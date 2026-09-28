# Entities

Domain entities, `ENT-###`. The conceptual model, not the database.

Required fields: `status`, `owned-by`.

For every concept ask:

- Does this exist in the domain, independently of our software?
- What evidence supports it?
- What behaviour or rule requires it?
- Is it actually a separate concept?
- Would it be a value object instead?
- Is it an implementation artefact rather than domain knowledge?

**Do not create a class because a noun appears in a requirement.**

<!--
## ENT-001 — Goods receipt

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
owned-by = "DOM-002"
derived-from = ["EVD-004"]
rules = ["RULE-001"]
states = "DRAFT,CONFIRMED,REVERSED"
```

**Definition**

A record asserting that a quantity of goods was physically delivered against a
purchase order line.

**Identity**

- `receiptId`, assigned on confirmation

**Attributes**

- purchase order line
- accepted quantity (integer, greater than zero)
- lot number and expiry date, where the item is batch-managed
- received at, received by
- reversal reference, when reversed

**Value objects**

- `Quantity` — a non-negative integer with an explicit unit. Not a float.

**Invariants**

- Accepted quantity is greater than zero
- Accepted quantity never exceeds the line's remaining quantity (`RULE-001`)
- A confirmed receipt is never edited; it is reversed and re-recorded

**Lifecycle**

```text
DRAFT ──confirm──▶ CONFIRMED ──reverse──▶ REVERSED
  │                                     (terminal)
  └──discard──▶ (none)
```

**Ownership**

`DOM-002` Inventory owns this concept. `DOM-001` Procurement references it and
must not redefine it.

## Rules for this registry

- Distinguish entity, value object and aggregate root explicitly.
- An entity needs a lifecycle worth modelling only if it has states; if it has
  exactly one state, do not invent a state machine for it.
- Shared entities have exactly one owner. Consumers link to it.
