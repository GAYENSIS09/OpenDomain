# Uncertainties

What is not known, `UNC-###`. One artifact per open question.

**This registry is a success, not a failure.** An assumption that is written
down and tracked can be revisited; one that is buried in an implementation gets
to become a load-bearing myth.

Required fields: `status`, `question`, `why-unknown`, `impact`, `needed-by`.

<!--
## UNC-002 — Retention period for the stock movement history

```toml
status = "OPEN"
epistemic = "UNKNOWN"
question = "How long must the stock movement history be retained?"
why-unknown = "No source consulted states a retention period; the applicable regulation was not identified"
confidence = "LOW"
impact = "Affects ENT-002, the validation scenario for archiving, and the storage budget"
needed-by = "Before PROC-004 Reconcile stock is specified"
derived-from = ["EVD-014"]
resolves = ["NFR-004"]
```

**Evidence available**

- `EVD-014` establishes that the history exists and is used for reconciliation
- No source establishes a retention period

**Possible interpretations**

- Indefinite retention, pruning nothing
- Retention aligned to the fiscal year close plus a statutory delay
- Retention aligned to the warranty period of the item

**Impact if wrong**

- Too short: reconciliation disputes cannot be settled after the fact
- Too long: unbounded storage growth and an unnecessary archiving subsystem

**Resolution needed**

- Identify the applicable accounting and customs regulation for the operating
  region
- Ask the finance stakeholder to confirm the audit window

**How to resolve**

Research, then a project decision recorded as `DEC-###`.

-->

## Status values

`OPEN` — active question
`INVESTIGATING` — research under way
`RESOLVED` — answered; link the answering artifact
`WONTFIX` — deliberately accepted as unknown; requires a stated consequence
`BLOCKED` — cannot be answered without an authorisation or an external party
