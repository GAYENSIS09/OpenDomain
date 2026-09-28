# Integrations

External systems this domain exchanges data with, `INT-###`. One artifact per
integration.

Required fields: `status`, `system`, `direction`. Recommended: `owner`,
`contract`, `failure-mode`, `derived-from`.

<!--
## INT-001 — ERP (supplier and invoice master data)

```toml
status = "VALIDATED"
system = "ERP"
direction = "BOTH"
owner = "STK-006"
contract = "CTR-001"
failure-mode = "ERP unavailable for up to 30 minutes during month-end close"
epistemic = "EXPLICIT"
derived-from = ["EVD-044"]
```

**What it owns**

Supplier master data, price agreements, invoice issuance.

**What we own**

Goods receipt and stock position.

**Direction in**

- Supplier creation and amendment
- Price agreement changes
- Invoices for received goods

**Direction out**

- Goods receipt, for three-way matching
- Stock position, for available-to-promise

**Synchronisation**

Near-real-time for supplier changes; batch on receipt for stock.

**Contract**

`CTR-001` OpenAPI, with an idempotency key on inbound writes.

**Failure behaviour**

If the ERP is unavailable, the system accepts receipts locally and queues them.
The queue is visible, has a bound, and alerts at 80% capacity. It never retries
silently forever.

**What happens if this integration is wrong**

Three-way match fails; invoices are not matched to receipts; the finance
reconciliation reports unmatched invoices. Detected by `TEST-021`.

## Rules for this registry

- Every integration needs a **failure mode**. An integration without one is not
  designed, it is wished for.
- Name the system of record for each concept. Two systems both being the system
  of record is the definition of a data corruption incident.
- Record who owns the integration on the other side. An unowned integration has
  no upgrade path.
