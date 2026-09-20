use crate::infrai_errors::{CaptureError, ExceptionPayload, InfraiErrorClient};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Order {
    pub order_id: String,
    pub customer_id: String,
    pub total_cents: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AgentStage {
    Checkout,
    Fulfillment,
    Receipt,
}

impl AgentStage {
    fn label(self) -> &'static str {
        match self {
            Self::Checkout => "checkout",
            Self::Fulfillment => "fulfillment",
            Self::Receipt => "receipt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CustomerOrderUpdate {
    Confirmed { order_id: String },
    ActionRequired { order_id: String, stage: AgentStage },
}

#[derive(Debug, Clone, Error)]
pub enum AgentStepError {
    #[error("payment authorization declined")]
    CheckoutDeclined,
    #[error("inventory reservation failed")]
    InventoryUnavailable,
    #[error("receipt delivery failed")]
    ReceiptDelivery,
}

#[derive(Debug, Error)]
pub enum OrderLoopError {
    #[error("{source}; failure capture also failed: {capture}")]
    Capture {
        source: AgentStepError,
        capture: CaptureError,
    },
}

pub trait CommerceSteps {
    fn checkout(&mut self, order: &Order) -> Result<(), AgentStepError>;
    fn fulfill(&mut self, order: &Order) -> Result<(), AgentStepError>;
    fn send_receipt(&mut self, order: &Order) -> Result<(), AgentStepError>;
}

pub struct OrderAgent {
    errors: InfraiErrorClient,
}

impl OrderAgent {
    pub fn new(errors: InfraiErrorClient) -> Self {
        Self { errors }
    }

    pub async fn run<S: CommerceSteps>(
        &self,
        order: &Order,
        steps: &mut S,
    ) -> Result<CustomerOrderUpdate, OrderLoopError> {
        if let Err(error) = steps.checkout(order) {
            return self
                .record_failure(order, AgentStage::Checkout, error)
                .await;
        }
        if let Err(error) = steps.fulfill(order) {
            return self
                .record_failure(order, AgentStage::Fulfillment, error)
                .await;
        }
        if let Err(error) = steps.send_receipt(order) {
            return self.record_failure(order, AgentStage::Receipt, error).await;
        }
        Ok(CustomerOrderUpdate::Confirmed {
            order_id: order.order_id.clone(),
        })
    }

    async fn record_failure(
        &self,
        order: &Order,
        stage: AgentStage,
        error: AgentStepError,
    ) -> Result<CustomerOrderUpdate, OrderLoopError> {
        let stage_name = stage.label();
        let payload = ExceptionPayload {
            title: format!("order-agent/{stage_name} failed"),
            message: error.to_string(),
            level: "error".to_owned(),
            fingerprint: vec!["order-agent".to_owned(), stage_name.to_owned()],
            exception: format!("{error:?}"),
            context: json!({
                "order_id": order.order_id,
                "customer_id": order.customer_id,
                "total_cents": order.total_cents,
                "stage": stage_name
            }),
        };
        let idempotency_key = format!("order-failure:{}:{stage_name}", order.order_id);
        self.errors
            .capture(&payload, &idempotency_key)
            .await
            .map_err(|capture| OrderLoopError::Capture {
                source: error,
                capture,
            })?;
        Ok(CustomerOrderUpdate::ActionRequired {
            order_id: order.order_id.clone(),
            stage,
        })
    }
}

pub fn decide_update(
    order_id: &str,
    outcomes: &[Result<(), AgentStepError>],
) -> CustomerOrderUpdate {
    let stages = [
        AgentStage::Checkout,
        AgentStage::Fulfillment,
        AgentStage::Receipt,
    ];
    for (stage, outcome) in stages.into_iter().zip(outcomes) {
        if outcome.is_err() {
            return CustomerOrderUpdate::ActionRequired {
                order_id: order_id.to_owned(),
                stage,
            };
        }
    }
    CustomerOrderUpdate::Confirmed {
        order_id: order_id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fulfillment_failure_blocks_receipt_confirmation() {
        let update = decide_update(
            "ord_1042",
            &[Ok(()), Err(AgentStepError::InventoryUnavailable), Ok(())],
        );

        assert_eq!(
            update,
            CustomerOrderUpdate::ActionRequired {
                order_id: "ord_1042".to_owned(),
                stage: AgentStage::Fulfillment,
            }
        );
    }
}
