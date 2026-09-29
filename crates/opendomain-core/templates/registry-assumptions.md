# Assumptions

Things taken as true without proof, `ASM-###`. One artifact per assumption.

**An unrecorded assumption is a bug with a delivery date.** The purpose of this
registry is not to eliminate assumptions; it is to make them visible, dated
and owned.

Required fields: `status`, `assumption`, `confidence`, `impact-if-false`.

<!--
## ASM-004 — The warehouse operates in a single region

```toml
status = "OPEN"
assumption = "The operating warehouse is in one country, so legal and currency constraints need to be modelled once"
confidence = "MEDIUM"
impact-if-false = "Multi-region rollout would make CNST-001 and the tax and customs rules multi-valued, and RULE-001's approval threshold region-specific"
derived-from = ["STK-001", "EVD-052"]
epistemic = "ASSUMED"
```

**Why assumed**

The expansion plan is not confirmed and no source describes a second site.

**How to validate**

Ask `STK-004` whether a second warehouse is committed for the next two years.

**If falsified**

`CNST-001` splits into per-region constraints, and `RULE-001` gains a
region-scoped variant. The cost is a change late in the project, which is
exactly why it is written down now.

**Decision needed by**

Before `SPEC-002` is approved.

-->

## Status values

`OPEN` — unvalidated
`VALIDATING` — an experiment or research task is under way
`VALIDATED` — confirmed; link the confirming evidence
`REFUTED` — shown to be false; link what replaced it
`BLOCKED` — cannot be validated without an external party
