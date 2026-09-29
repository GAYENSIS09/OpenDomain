# Domain events

Events the domain emits when something meaningful happens, `EVENT-###`.

Required fields: `status`, `producer`. Recommended: `consumers`, `derived-from`.

**Do not introduce an event bus or asynchronous infrastructure merely because
events exist conceptually.** A domain event is a statement about the past, not a
deployment decision.

<!--
## EVENT-002 — GoodsReceived

```toml
status = "VALIDATED"
producer = "DOM-002"
consumers = ["DOM-005", "DOM-006"]
derived-from = ["EVD-004"]
epistemic = "EXPLICIT"
```

**Name**

`GoodsReceived`

**Producer**

`PROC-001` Receive goods

**Consumers**

- `DOM-005` Finance — updates goods-not-invoiced value
- `DOM-006` Reporting — refreshes the receipt ledger

**Payload**

```json
{
  "receiptId": "GR-2026-0041",
  "orderId": "PO-2026-0012",
  "lineId": "PO-2026-0012-2",
  "acceptedQuantity": 20,
  "unit": "unit",
  "occurredAt": "2026-03-04T09:14:22Z",
  "itemId": "ITM-0031"
}
```

**Business meaning**

Goods have been received and are now owned by the organisation. This is the
point at which inventory liability begins, not when the invoice is received.

**Side effects**

None in the producing transaction. Consumers react in their own context.

**Delivery semantics**

Decide deliberately: at-most-once, at-least-once or exactly-once, and whether
consumers must be idempotent. "It depends on the broker" is not a decision.

-->

## Rules for this registry

- An event represents real domain meaning. "UserClicked" is an interaction, not
  a domain event.
- An event is in the past tense.
- If no consumer exists yet, the event is a model, not an integration. Keep it
  as documentation and do not build infrastructure for it.
