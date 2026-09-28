# Business rules

Rules the domain imposes, `RULE-###`. A rule is a statement that must hold, not
a step to perform.

Required fields: `type`, `status`. Recommended: `derived-from`, `scope`,
`owned-by`, `epistemic`, `confidence`.

`type` is one of `LEGAL` `REGULATORY` `ORGANIZATIONAL` `OPERATIONAL`
`CALCULATION` `SECURITY` `DATA_INTEGRITY` `WORKFLOW`.

<!--
## RULE-001 — Receipted quantity may not exceed the outstanding quantity

```toml
type = "OPERATIONAL"
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
scope = "CORE"
owned-by = "DOM-002"
derived-from = ["EVD-004"]
constrains = ["PROC-001", "REQ-004"]
```

**Statement**

The accepted quantity of a goods receipt SHALL be less than or equal to the
remaining quantity of the purchase order line it references.

**Why**

Short receipt is a real operational condition, not an error: the supplier
under-delivers. Over-receipt is a data-entry error and inflates stock and
invoiced value, which corrupts three domains at once.

**Exceptions**

- A supplier-authorised over-receipt is permitted only when a price agreement
  covers it, is approved by `ACT-003`, and records the approval reference.

**Validation method**

Boundary test: receive exactly the remaining quantity (accepted), receive one
more (rejected), receive zero (rejected).

**Affected**

- `PROC-001` Receive goods
- `ENT-001` Goods receipt
- `REQ-004` Reject an over-receipt
-->

## Rules for this registry

- A rule that is not written down does not exist. If the implementation enforces
  something, it belongs here.
- Never encode an inferred rule into code without recording it here first.
  `epistemic = "INFERRED"` is a legitimate state; silence is not.
- Every rule needs a **validation method**. A rule you cannot test is either not
  a rule or an unfinished one.
- A `LEGAL` or `REGULATORY` rule links the regulation in `derived-from`.
