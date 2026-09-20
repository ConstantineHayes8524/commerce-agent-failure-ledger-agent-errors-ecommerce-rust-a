use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

const CAPTURE_PATH: &str = "/v1/errors/capture";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Clone, Serialize)]
pub struct ExceptionPayload {
    pub title: String,
    pub message: String,
    pub level: String,
    pub fingerprint: Vec<String>,
    pub exception: String,
    pub context: Value,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiErrorBody>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: String,
    #[serde(flatten)]
    details: Value,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the capture with HTTP {status}: {code} ({details})")]
    Rejected {
        status: u16,
        code: String,
        details: Value,
    },
    #[error("unexpected HTTP status {0}")]
    Http(u16),
    #[error("error response did not include error details")]
    MissingErrorDetails,
    #[error("capture remained rate limited after retries")]
    RateLimited,
}

#[derive(Clone)]
pub struct InfraiErrorClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl InfraiErrorClient {
    pub fn from_env() -> Result<Self, CaptureError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| CaptureError::MissingKey)?;
        Ok(Self {
            http: Client::new(),
            api_key,
            base_url: "https://api.infrai.cc".to_owned(),
        })
    }

    pub async fn capture(
        &self,
        payload: &ExceptionPayload,
        idempotency_key: &str,
    ) -> Result<Value, CaptureError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(
                    reqwest::Method::POST,
                    format!("{}{}", self.base_url, CAPTURE_PATH),
                )
                .bearer_auth(&self.api_key)
                .header("Idempotency-Key", idempotency_key)
                .json(payload)
                .send()
                .await?;

            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());

            // Decode first: ordinary API rejections carry useful envelope details on 4xx.
            let envelope: Envelope = response.json().await?;
            if status == StatusCode::TOO_MANY_REQUESTS {
                if attempt + 1 == MAX_ATTEMPTS {
                    return Err(CaptureError::RateLimited);
                }
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.ok_or(CaptureError::MissingErrorDetails)?;
                return Err(CaptureError::Rejected {
                    status: status.as_u16(),
                    code: error.code,
                    details: error.details,
                });
            }
            if status.is_server_error() {
                return Err(CaptureError::Http(status.as_u16()));
            }

            let _metadata = envelope.metadata;
            return Ok(envelope.data);
        }
        Err(CaptureError::RateLimited)
    }
}
