# Architecture decision records

Technical decisions, `ADR-###`. One artifact per decision.

An ADR is immutable once accepted. To change your mind, write a new one that
supersedes the old one and say so. Editing history is how a codebase loses the
reasoning that made it survive its first hard year.

Required fields: `status`, `decision`, `context`, `consequences`. Recommended:
`alternatives`, `derived-from`, `date`.

`status` is one of `DRAFT` `APPROVED` `SUPERSEDED` `DEPRECATED`.

<!--
## ADR-002 — Deploy as a modular monolith on the contracted platform estate

```toml
status = "APPROVED"
decision = "One deployable unit, with internal module boundaries enforced in CI, on the company platform estate"
context = "CNST-001 forbids a bespoke deployment, and ASM-004 (single region) is unvalidated"
consequences = "Cross-domain transactions are cheap, so RULE-001 can be enforced in one place. Module boundaries depend on CI enforcement because nothing else keeps them honest. Scaling is per-unit, not per-domain."
alternatives = ["Services per domain", "Shared database with no module boundaries"]
derived-from = ["CNST-001", "ASM-004", "PLAN-002"]
date = "2026-04-10"
supersedes = "ADR-001"
```

**Context**

`CNST-001` restricts deployment to the company estate. `ASM-004` assumes a
single region and is not validated. A per-domain service design would make both
assumptions load-bearing in the transaction path.

**Decision**

One deployable unit. Internal modules follow the bounded contexts in
`domain/domain-map.md`. Dependencies point inwards to a shared kernel.

**Alternatives considered**

- **Services per domain** — rejected: the operational cost is real, and the
  distributed-transaction problem lands directly on `RULE-001`, which requires
  the outstanding quantity never to be exceeded. Enforcing that across a network
  partition is strictly harder than enforcing it in one process.
- **Shared database, no module boundaries** — rejected: cheaper still, and it
  makes the domain map a lie. The map is the thing that keeps shared concepts
  from diverging.

**Consequences**

Good:

- `RULE-001` is enforceable in a single transaction
- Deployment is one unit, which matches the operational capacity described by
  `STK-005`

Bad:

- Module boundaries are maintained by CI alone; a disabled check silently
  returns the system to a distributed monolith
- Scaling cannot isolate one heavy domain; this becomes costly only if
  concurrent users exceed 50 (`NFR-001`)

**Revisit when**

Concurrent users exceed 50, or a second region is confirmed.

-->

## Rules for this registry

- One decision, one ADR. Do not bundle.
- Write the rejected options. An ADR without them is a record of what you
  ended up with, not a record of why.
- Consequences are consequences, including the bad ones. A decision record with
  only benefits has not been read honestly.
- These are *technical* decisions. A decision that changes what the business can
  do is a `DEC-###`.
