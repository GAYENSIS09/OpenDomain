# Use case model

How actors and entities collaborate, `UC-###`. One artifact per use case.

Use cases describe **what a goal is achieved with**, not what the system does on
a click. A use case is close to the requirement; the specification is closer
still.

Required fields: `status`, `primary-actor`, `trigger`, `outcome`.

<!--
## UC-004 — Place a purchase order

```toml
status = "VALIDATED"
primary-actor = "STK-001"
secondary-actors = ["ACT-003", "ACT-001"]
trigger = "A requisition has been approved"
outcome = "An approved purchase order exists with a supplier and a committed delivery date"
scope = "CORE"
derived-from = ["EVD-014", "EVD-027", "STK-001"]
```

**Primary actor**

`STK-001` Head of Procurement

**Secondary actors**

- `ACT-003` Financial approver — authorises orders above a threshold
- `ACT-001` Warehouse operator — receives against the order

**Trigger**

A requisition reaches the approved state.

**Preconditions**

- A requisition exists and is approved
- A supplier is active and has a price agreement
- A stock record exists for each ordered item

**Main flow**

1. Procurement selects the approved requisition and chooses a supplier.
2. The system presents the agreed prices and available delivery dates.
3. Procurement adjusts quantities within the price agreement's bounds.
4. Procurement submits the order.
5. The system routes it for approval when its value exceeds `RULE-002`.
6. The approver approves, and the system emits `EVENT-001`.

**Alternative flows**

- Supplier has no price agreement: the order cannot be submitted until
  procurement records one
- The requisition changes after submission: the order is superseded, not edited
- Approval is refused: the order returns to draft with the approver's reason

**Exception flows**

- Supplier becomes inactive between selection and submission: rejected
- Delivery date falls outside the agreement's window: rejected
- Two orders are submitted for the same requisition: the second is rejected

**Postconditions**

- Exactly one order exists per requisition
- The requisition is marked ordered
- The commitment is visible to `DOM-002` Inventory

**Out of scope**

Choosing the supplier. That is a separate goal, `UC-005`.

-->

## The nine patterns

Every use case answers these, and the answer is allowed to be "not applicable" —
but only if it is written down.

1. Basic flow
2. Alternative flows
3. Preconditions and postconditions
4. Primary and secondary actors
5. Business rules applied
6. Data created, updated, deleted
7. Events emitted
8. Failure and recovery
9. Scope boundaries — and what is explicitly **not** covered

## Rules for this registry

- A use case is a goal, not a screen flow. "Click the button" is a gesture;
  "place an order" is a goal.
- If the use case needs "and" in its name, it is probably two.
- A use case with no exception flows has not been thought about.
