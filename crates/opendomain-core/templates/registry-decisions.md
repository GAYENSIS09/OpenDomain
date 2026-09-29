# Domain decisions

Business and domain decisions, `DEC-###`. One artifact per decision.

This is the *domain* decision log. Technical decisions live as `ADR-###` in
`architecture/decisions/records.md`. A technical decision must never silently
override a business requirement.

Required fields: `status`, `decision`, `context`, `reason`, `consequences`.

<!--
## DEC-004 — Retain the stock movement history indefinitely for now

```toml
status = "APPROVED"
decision = "The stock movement history is retained without pruning for the first release"
context = "UNC-002 is open: no retention requirement has been identified"
reason = "Indefinite retention is the only option that cannot lose information; pruning is a one-way door"
consequences = "Storage grows without bound. A pruning job is out of scope until UNC-002 resolves"
alternatives = "Prune at 24 months (rejected: may discard data a later audit needs)"
derived-from = ["UNC-002"]
affects = ["ENT-002", "NFR-004"]
decided-by = "STK-004"
date = "2026-03-10"
```

**Alternatives considered**

- Prune after 24 months — rejected: the applicable regulation is unknown, and
  pruning is irreversible
- Prune after the fiscal close — rejected: depends on the unresolved fiscal
  calendar

**What this decision is not**

It is not a claim that indefinite retention is correct. It is a decision to
defer the question without losing data.

-->

## Rules for this registry

- Record the decision you did *not* make, and why. An ADR with no rejected
  options has not been thought about.
- State consequences as consequences, not as justifications.
- A decision taken to unblock work while a question is open must say so
  explicitly, and must be revisited when the question closes.
