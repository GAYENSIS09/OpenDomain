# Domain glossary

The canonical vocabulary. One term per artifact, `TERM-###`.

Two words is not a synonym until this file says so. Two words meaning different
things is a defect until this file says that too. Both failures are expensive
downstream, which is why Gate 2 flags duplicate term titles.

Required fields: `status`. Recommended: `epistemic`, `confidence`,
`derived-from`, `owned-by`.

<!--
## TERM-001 — Purchase order

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
owned-by = "DOM-002"
derived-from = ["EVD-004", "EVD-011"]
```

**Definition**

A document issued by a buyer to a supplier authorising the delivery of a
specified quantity of goods within a stated period.

**Synonyms**

- PO
- Bon de commande
- Order form

**Do not confuse with**

- *Purchase requisition*: the internal request that may lead to a purchase
  order. Different lifecycle stage, different owner, different approver.
- *Invoice*: a request for payment issued by the supplier after delivery.

**Context**

Used by Procurement. Consumed by Inventory on receipt and by Finance on
invoice match.
-->

## Rules for this registry

- One concept, one term. If two words are truly the same, keep one and list the
  other under **Synonyms**.
- If two words look the same but are not, keep both and write a **Do not
  confuse with** section.
- Never merge an ambiguous pair silently.
- Every term that a domain owns links `owned-by` a `DOM`.
