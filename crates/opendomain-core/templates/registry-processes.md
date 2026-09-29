# Processes

Business processes, `PROC-###`. One artifact per process.

Required fields: `status`, `derived-from` (at least one evidence record).

For every process, ask: what happens before this? After this? Who is
responsible? What information changes? What other domain is affected?

Do not model only the happy path. Every process must answer the nine exception
prompts below.

<!--
## PROC-001 — Receive goods against a purchase order

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
trigger = "Goods arrive at the loading bay"
actor = "ACT-001"
owned-by = "DOM-002"
derived-from = ["EVD-004", "EVD-011"]
rules = ["RULE-001", "RULE-003"]
events = ["EVENT-002"]
```

**Purpose**

Record what physically arrived so that stock, order status and invoicing all
reflect reality.

**Preconditions**

- A purchase order exists and is at least partially open
- The operator is authenticated

**Inputs**

- Purchase order line
- Delivered quantity, lot and expiry where applicable

**Main flow**

1. Operator selects the purchase order line to receive against.
2. Operator records the delivered quantity.
3. System rejects the receipt if the delivered quantity exceeds the remaining
   quantity (`RULE-001`).
4. System increases on-hand stock by the accepted quantity.
5. System decreases the order's remaining quantity.
6. System records the movement and emits `EVENT-002` GoodsReceived.

**Alternative flows**

- Partial delivery: remaining quantity decreases by the accepted amount only.
- Multiple deliveries against one line: each is a separate receipt.

**Exception flows**

- Delivered quantity is zero: the receipt is rejected as meaningless.
- No open quantity remains: rejected with the reason, no stock movement.
- The item is not stock-managed: stock step is skipped, order still updated.
- A concurrently placed receipt already consumed the remaining quantity: the
  second receipt is rejected and both receipts are reported to the operator.

**Outputs**

- Goods receipt record
- Stock movement
- Updated purchase order status

**Postconditions**

- On-hand stock equals its previous value plus accepted quantity
- Order remaining quantity is never negative
- Every movement is attributable to an actor and a timestamp

**Affected domains**

- `DOM-002` Inventory — stock
- `DOM-001` Procurement — order status
- `DOM-005` Finance — goods-not-invoiced value

-->

## The nine exception prompts

Apply these to every process before calling it modelled.

1. What if required data is missing?
2. What if the operation fails midway?
3. What if an approval is rejected?
4. What if stock or capacity is insufficient?
5. What if two users act at the same time?
6. What if an external system is unavailable?
7. What if the process is cancelled after starting?
8. What if the operation must be reversed?
9. What if the information later turns out to be invalid?
