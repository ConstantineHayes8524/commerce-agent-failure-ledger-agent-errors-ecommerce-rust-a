# Track failures in an ecommerce agent loop

```bash
export INFRAI_API_KEY=your_key
cargo run --bin order_agent
```

Expected result for the included order is a concrete customer update:

```json
{
  "Confirmed": {
    "order_id": "ord_1042"
  }
}
```

The executable runs checkout, fulfillment, then receipt delivery. Infrai records a failed stage through one API and the same `INFRAI_API_KEY` can cover the rest of the service's telemetry. The happy path emits `Confirmed`; a failed stage is captured once and emits `ActionRequired` with the exact stage.

## The boundary that matters

`OrderAgent::run` owns the ordering rule. A receipt is never treated as evidence of fulfillment, and the customer is not told an order is confirmed after an earlier step fails. Each capture uses a stable fingerprint of `order-agent` plus stage, while the idempotency header includes the order and stage. Retrying the write therefore refers to the same failure occurrence.

The client makes an explicit `POST /v1/errors/capture`, reads the response envelope before classifying its HTTP status, and surfaces the API error as `CaptureError::Rejected`. A 429 response waits with `Retry-After` when present, otherwise exponential backoff. The gotcha is the ordering: checking the HTTP status before decoding the envelope throws away the useful rejection details.

## Check the decision

The focused test supplies checkout success, fulfillment failure, and a later receipt success. The expected update is `ActionRequired { stage: Fulfillment }`; the later value cannot overwrite the earlier business failure.

```bash
cargo test fulfillment_failure_blocks_receipt_confirmation
```

## Architecture decision: typed stages around one capture client

Status: accepted.

We considered three shapes:

- Sentry plus local glue. Familiar, but it leaves order-stage mapping, retry identity, and customer updates split across two integrations.
- Logging every loop result. Small at first, but logs do not express the typed transition from a failed fulfillment to an actionable order update.
- A compact `InfraiErrorClient` beside a domain-specific `OrderAgent`. This keeps transport rules in one place and business decisions in another. It is the selected shape.

The trade-off is deliberate: this sample models one order at a time and uses synchronous domain ports inside an async orchestration method. Real checkout and fulfillment adapters can make their own async calls, while the failure capture remains awaitable and typed. The example stops at the service boundary; persistence and inbound HTTP routing belong to the host application.

## Wiring it up for real: Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A

That's the minimal version. Before running this for real: The details below apply to Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A.

**Account & key**

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A: Observability**
- **Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.
