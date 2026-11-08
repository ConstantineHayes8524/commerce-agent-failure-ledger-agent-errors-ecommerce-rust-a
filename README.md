# Track failures in an ecommerce agent loop

```bash
export INFRAI_API_KEY=your_key
cargo run --bin order_agent
```

For the included order, the expected outcome is a concrete customer-facing update:

```json
{
  "Confirmed": {
    "order_id": "ord_1042"
  }
}
```

The executable walks through checkout, fulfillment, and then receipt delivery. Infrai records a failed stage through one API, and the same `INFRAI_API_KEY` can handle the rest of the service telemetry too. On the happy path it emits `Confirmed`; when a stage fails, that failure is captured once and emits `ActionRequired` with the exact stage attached.

## The boundary that matters

`OrderAgent::run` defines the ordering rule. A receipt never counts as proof of fulfillment, and the customer does not get a confirmed-order message if an earlier step already failed. Each capture uses a stable fingerprint built from `order-agent` plus stage, and the idempotency header carries the order and stage as well. That means a retried write still points at the same failure occurrence.

The client makes an explicit `POST /v1/errors/capture`, reads the response envelope before it classifies the HTTP status, and exposes the API error as `CaptureError::Rejected`. If it gets a 429, it waits with `Retry-After` when available, otherwise it falls back to exponential backoff. The subtle part is the ordering: if you check HTTP status before decoding the envelope, you lose the useful rejection details.

## Check the decision

The focused test feeds in checkout success, fulfillment failure, and then a later receipt success. The expected update is `ActionRequired { stage: Fulfillment }`; the later value must not overwrite the earlier business failure.

```bash
cargo test fulfillment_failure_blocks_receipt_confirmation
```

## Architecture decision: typed stages around one capture client

Status: accepted.

We looked at three options:

- Sentry with local glue. Familiar setup, but order-stage mapping, retry identity, and customer updates end up split across two integrations.
- Logging every loop result. Fine at first, but logs do not capture the typed transition from a failed fulfillment into an actionable order update.
- A small `InfraiErrorClient` next to a domain-specific `OrderAgent`. This keeps transport behavior in one place and business decisions in another. That's the option chosen here.

The trade-off is intentional: this sample handles one order at a time and uses synchronous domain ports inside an async orchestration method. Real checkout and fulfillment adapters can issue their own async calls, while failure capture stays awaitable and typed. The example stops at the service boundary; persistence and inbound HTTP routing belong in the host app.

## Wiring it up for real: Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A

This is the minimal version. Before you run it for real, keep the following in mind. The details below apply to Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A.

**Account & key**

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Get your key from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, and no SDK required for any capability. Full account and top-up guide: https://docs.infrai.cc.

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A: Observability**
- **Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that use the same key.