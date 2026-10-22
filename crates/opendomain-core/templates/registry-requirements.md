# Requirements

Requirements derived from validated evidence and domain analysis, `REQ-###`.

One requirement per artifact. If it needs an "and", it is probably two.

**A requirement is not a feature.** It is a statement of behaviour the system
must exhibit, traceable to evidence.

Required fields: `type`, `status`. Strongly required: `derived-from`,
`actor`, `epistemic`.

Optional: `rigour` — the verification mechanism this requirement demands,
declared against the Section 9.7 table (`"Business behaviour, acceptance"`,
`"Unit-level contract"`, `"Performance envelope"`, …). `opendomain rigour`
audits the declarations; `opendomain provision` refuses to scaffold BDD when
they are absent.

`type` is one of `BUSINESS` `FUNCTIONAL` `NON_FUNCTIONAL` `DATA` `SECURITY`
`INTEGRATION` `COMPLIANCE` `OPERATIONAL` `TECHNICAL_CONSTRAINT`.

<!--
## REQ-004 — Reject a receipt exceeding the outstanding quantity

```toml
type = "FUNCTIONAL"
status = "VALIDATED"
scope = "CORE"
priority = "HIGH"
epistemic = "EXPLICIT"
confidence = "HIGH"
actor = "ACT-001"
derived-from = ["EVD-014", "RULE-001"]
rules = ["RULE-001"]
processes = ["PROC-001"]
verifies = ["AC-004-01", "AC-004-02", "AC-004-03"]
implements = ["SPEC-004"]
```

**Statement**

The system SHALL reject a goods receipt whose accepted quantity exceeds the
remaining quantity of the referenced purchase order line, and SHALL report the
outstanding quantity to the operator.

**Rationale**

`RULE-001` forbids it. Accepting an over-receipt would inflate stock, inflate
goods-not-invoiced value and misstate the supplier's performance, corrupting
three domains from a single input.

**Acceptance criteria**

- `AC-004-01` — When the accepted quantity equals the remaining quantity, the
  receipt is confirmed and the remaining quantity becomes zero.
- `AC-004-02` — When the accepted quantity exceeds the remaining quantity, the
  receipt is rejected, no stock movement occurs, and the outstanding quantity
  is reported.
- `AC-004-03` — When two receipts are confirmed concurrently against the same
  line, at most one succeeds and the total accepted quantity never exceeds the
  original remaining quantity.

**Dependencies**

- `REQ-002` Purchase orders are stored
- `CNST-001` Operations run in a single region

**Status**

`VALIDATED`

-->

## The chain that produced this

```text
SRC-014 + SRC-021
  → EVD-014 (stock movements recorded; history used for reconciliation)
  → RULE-001 (receipted quantity may not exceed outstanding quantity)
  → REQ-004 (the system shall enforce RULE-001 for goods receipts)
  → SPEC-004 (observable behaviour)
  → AC-004-01..03 → TEST-021..023
```

Note what is *not* in the chain: the fact that competitors allow over-receipt.
That is a `COMPETITOR_CAPABILITY` in `EVD`, and it is evidence about a market,
not a requirement.

## Quality bar

Every requirement is checked for: clarity, atomicity, consistency, necessity,
feasibility, testability, traceability, unambiguity. A requirement that fails
goes back with status `DRAFT`, not forward with a caveat.

## Rejected phrasing

| Rejected | Because | Replace with |
|---|---|---|
| "The system should be fast." | Unmeasurable | "95th percentile response under 400 ms at 50 concurrent users" |
| "The interface should be intuitive." | Unobservable | "A first-time operator completes `UC-001` without help, measured in usability testing" |
| "The AI should be intelligent." | Unfalsifiable | "Classification precision ≥ 0.92 on a 500-item labelled sample" |
| "The system should be powerful." | Empty | Delete it, then find the requirement it was hiding |
