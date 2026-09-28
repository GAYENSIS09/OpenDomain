# Contracts

Interfaces, `CTR-###`. One artifact per contract.

A contract is an agreement about a boundary: an API, an event, a file format, a
database shape that another system reads. If nobody else is affected, it is
internal design, and it belongs in the plan.

Required fields: `status`, `type`, `derived-from`, `version`. Recommended:
`verification`, `consumers`, `breaking-change`.

`type` is one of `API` `EVENT` `DATA` `FILE_FORMAT` `LIBRARY`.

<!--
## CTR-001 — ERP purchase-order receipt feed

```toml
status = "IMPLEMENTED"
type = "EVENT"
version = "1.2"
derived-from = ["SPEC-004", "INT-001", "PLAN-002"]
consumers = ["ERP", "DOM-005 Finance"]
verification = "TEST-021"
breaking-change = "none; 1.2 adds optional `lotNumber` and `expiryDate` fields"
epistemic = "EXPLICIT"
```

**Contract**

A confirmed goods receipt is published to the ERP as `GoodsReceived`, with the
fields listed below.

**Direction**

This system publishes; the ERP consumes.

**Schema**

```json
{
  "receiptId": "GR-2026-0041",
  "orderId": "PO-2026-0012",
  "lineId": "PO-2026-0012-2",
  "acceptedQuantity": 20,
  "unit": "unit",
  "lotNumber": "LOT-77A",
  "expiryDate": "2027-01-31",
  "occurredAt": "2026-03-04T09:14:22Z"
}
```

**Guarantees**

- At-least-once delivery
- `receiptId` is stable across retries, so consumers can be idempotent
- Publication is ordered per `orderId`, not globally
- Publication is never transactional with the local receipt; the outbox makes
  that explicit rather than pretending otherwise

**Consumers must**

- Ignore unknown fields, so a minor version cannot break them
- Be idempotent on `receiptId`

**Verified by**

`TEST-021`, which asserts the published payload against this schema and asserts
that a retry publishes the same `receiptId`.

**Owner**

This system. The ERP side is owned by `STK-006`.

## Rules for this registry

- No contract without a version and a verification test. Gate 7 reports
  contracts that no test touches.
- Record the delivery guarantee explicitly. "It depends on the broker" is not a
  guarantee.
- State the breaking-change policy. A contract that can change silently is a
  contract that will.
