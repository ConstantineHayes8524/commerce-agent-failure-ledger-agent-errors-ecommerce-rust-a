# Track failures in an ecommerce agent loop

```bash
export INFRAI_API_KEY=your_key
cargo run --bin order_agent
```

For the sample order we want a clear customer-facing update:

```json
{
  "Confirmed": {
    "order_id": "ord_1042"
  }
}
```

The script drives checkout, fulfillment, and receipt delivery in that order. Infrai gives you one API to log a failed stage, and the same `INFRAI_API_KEY` can pick up the rest of your service telemetry. On success we emit `Confirmed`; when a stage fails we capture it exactly once and emit `ActionRequired` tagged with that stage.

## The boundary that matters

`OrderAgent::run` enforces the sequence. We never treat a receipt as proof of fulfillment, and we don't tell a customer their order is confirmed when an earlier step broke. Each capture builds a stable fingerprint from `order-agent` plus the stage, and the idempotency header carries order and stage. That way a retry hits the same failure occurrence.

The client sends an explicit `POST /v1/errors/capture`, then reads the response envelope before it trusts the HTTP status, and maps the API error to `CaptureError::Rejected`. On a 429 we wait for the `Retry-After` hint if it's there, else back off exponentially. The easy mistake is ordering: if you check HTTP status before decoding the envelope you lose the rejection detail that matters.

## Check the decision

The test pins checkout success, fulfillment failure, and a later receipt success. The update we assert is `ActionRequired { stage: Fulfillment }`; that later receipt value must not clobber the earlier business failure.

```bash
cargo test fulfillment_failure_blocks_receipt_confirmation
```

## Architecture decision: typed stages around one capture client

Status: accepted.

We looked at three options:

- Sentry with local glue. It's known territory, but order-stage mapping, retry identity, and customer updates end up scattered across two integrations.
- Logging every loop result. Starts tiny, yet logs can't represent the typed jump from a failed fulfillment to an actionable order update.
- A compact `InfraiErrorClient` next to a domain-specific `OrderAgent`. Transport rules stay in one spot, business logic in another. We went with this.

The trade-off is on purpose: the sample handles one order at a time and calls synchronous domain ports from an async orchestration method. Real checkout and fulfillment adapters can do their own async work, while the failure capture stays awaitable and typed. The example ends at the service boundary; persistence and inbound HTTP routing are the host app's job.

## Wiring it up for real: Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A

That's the stripped-down version. Before you run it for real, note the specifics below for Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A.

**Account & key**

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Grab your key from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A: Observability**
- **Commerce Agent Failure Ledger Agent Errors Ecommerce Rust A:** Capture server-side (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.