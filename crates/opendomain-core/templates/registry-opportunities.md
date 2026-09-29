# Opportunities

Worth exploring, `OPP-###`. One artifact per opportunity.

An opportunity is a place where the current design is likely to be wrong,
incomplete or worth leaving. It is not a feature list and not a project goal.

Required fields: `status`, `statement`. Recommended: `evidence`,
`value-hypothesis`, `derived-from`.

<!--
## OPP-002 — The receipt screen is the true bottleneck

```toml
status = "VALIDATED"
statement = "Operators abandon goods receipts when the screen is slow, so the cost of a slow screen is paid in missed receipts rather than in complaints"
value-hypothesis = "Making the receipt screen fast enough removes the most common cause of late bookkeeping"
opportunity = "Reconsider whether the receipt screen must support the general order-entry case at all"
derived-from = ["EVD-014", "EVD-021", "STK-001"]
priority = "HIGH"
```

**Why this is an opportunity**

The team has been treating the receipt screen as a thin form over the ERP. Both
interviews and the movement log suggest the real cost is elsewhere: operators
abandon it and re-key later, and the re-keying loses the lot and expiry data
that reconciliation depends on.

**Why it is not a requirement**

"Ignore the slow screen" is not a statement about behaviour. The opportunity is
the *question* of whether the general form is the right interaction at all.

**How it is being closed**

`CPT-003` proposes a scanning-first interaction. `HYP-005` makes its benefit
falsifiable. `EXP-002` measures the re-key rate before and after.

-->

## Status values

`OPEN` `INVESTIGATING` `VALIDATED` `REJECTED` `DEFERRED`
