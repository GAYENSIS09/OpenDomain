# Plans

Technical plans, `PLAN-###`. One artifact per plan.

A plan says **how** the specification will be realised. This is the first
place where choosing a technology is legitimate.

Required fields: `status`, `approach`, `derived-from`. Recommended:
`authority`, `risks`, `alternatives`.

`approach` is the one-line shape of the solution. `authority` names the
artefact that wins when the plan and the specification disagree; Gate 6 reports a
plan with neither.

<!--
## PLAN-002 — Modular monolith with an outbox for ERP writes

```toml
status = "APPROVED"
approach = "One deployable unit on the contracted platform estate, with domain modules and a transactional outbox for ERP writes"
authority = "SPEC-004"
alternatives = ["Event-driven services per domain", "Sync-and-retry without an outbox"]
risks = "The outbox adds a component that must be monitored; if it stalls, receipts queue silently"
derived-from = ["SPEC-004", "SPEC-002", "CDR-002", "CNST-001"]
adrs = ["ADR-002", "ADR-005"]
```

**What this plan covers**

`SPEC-004` receiving goods, and the ERP write path that `CDR-002` forced us to
decide.

**The approach**

A single deployable unit, with explicit internal module boundaries following the
domain map. ERP writes go through a transactional outbox so a receipt is
persisted locally even when the ERP is unavailable, which resolves `CDR-002` in
favour of local confirmation with a visible queue.

**Why the outbox**

`INT-001` records a 30-minute ERP unavailability window. Without an outbox, a
synchronous write either blocks the operator for 30 minutes or loses the
receipt. The outbox makes "the receipt is saved, the ERP has not seen it yet" a
state the system can show honestly.

**Rejected alternatives**

- **Event-driven services per domain** — rejected on cost and on `ASM-004`: the
  single-region assumption is unvalidated, and distributed transactions would
  make `RULE-001`'s invariant depend on network partitions
- **Synchronous write with retry** — rejected because it is exactly the
  contradiction `CON-001` records

**Risks and mitigations**

- The outbox stalls and nobody notices → monitor queue depth, alert at 80% of
  the retention window
- Module boundaries decay into a distributed monolith → enforce import
  direction in CI, recorded as a follow-up task

**Out of scope**

Data migration from the old spreadsheets, which `GAP-002` tracks separately.

-->

## Rules for this registry

- One plan per capability area, not one plan per project. A plan that covers
  everything decides nothing.
- Every plan names its `authority`. If a developer and the specification
  disagree, one of them is going to win by accident; name which.
- A plan that contains no rejected alternative has not been a decision.
