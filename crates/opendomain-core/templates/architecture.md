# Architecture

Technical decisions that are not tied to one plan.

This file holds the cross-cutting structure: the module map, the data
boundaries, the deployment shape, and the cross-cutting quality mechanisms.

Individual decisions belong in `architecture/decisions/records.md` as `ADR-###`.

## Principles

<!--
### Modules follow the domain, not the layers

Recorded for real as an `ADR-###` in `architecture/decisions/records.md`. What
belongs in this file is the reasoning, not the identifier.

```toml
status = "APPROVED"
decision = "Internal modules are organised by bounded context; the domain module is allowed to depend on the shared kernel and on nothing else in the application"
derived-from = ["DOM-001", "DOM-002", "PLAN-002"]
```

**Decision**

The code is organised by bounded context, and dependencies only point inwards
towards the shared kernel.

**Why**

`domain/domain-map.md` says `DOM-002` owns the concept of a goods receipt and
`DOM-001` references it. An organisation that does not follow the domain map
turns every shared concept into a lock fight, and lock fights show up as
business rules being quietly duplicated.

**What it costs**

A shared kernel that grows needs pruning, or every module ends up depending on
everything.

**How it is enforced**

An import-direction check in CI. Recorded as a follow-up task in `PLAN-002`.

-->

## Quality attribute scenarios

Every quality attribute is written as a scenario, because an attribute with no
scenario is an aspiration.

<!--
### Latency under load

```toml
status = "APPROVED"
decision = "Stock reads are served from the primary store; no read cache is introduced in the first release"
derived-from = ["NFR-001", "BAS-001"]
```

**Scenario**

At 50 concurrent users, 95th percentile stock-read latency stays at or below
400 ms over a 15-minute window.

**Why not a read cache**

The invalidation cost of a stock cache is the correctness of `RULE-001`, and
`NFR-001` is met by the primary store at the expected volume. Adding a cache
before a measurement says it is needed trades a proven requirement for an
unproven optimisation.

**Revisit when**

Concurrent users exceed 50, or the 400 ms target is missed at that level.
-->

## Deployment

One deployable unit on the contracted platform estate (`CNST-001`). No bespoke
deployment.

## Data boundaries

| Data | System of record | Written by | Notes |
|---|---|---|---|
| Goods receipt | This system | `PROC-001` | Published to the ERP via `CTR-001` |
| Supplier | ERP | ERP | Replicated in; never authored here |
| Price agreement | ERP | ERP | Read-only here |

Two systems must never both be the system of record for one concept. That is
the definition of a data corruption incident, and it is cheaper to prevent in
a table than to diagnose from a reconciliation report.

## Cross-cutting mechanisms

- **Identity and access** — operator identity is owned by the company directory
  (`INT-002`); roles map to the permissions in `domain/actors.md`
- **Audit** — every confirm, reverse and approval is recorded with actor and
  timestamp; the audit log is append-only
- **Outbox** — external writes leave through a transactional outbox, per
  `PLAN-002`
- **Observability** — structured logs, one correlation identifier per receipt,
  and a metric for outbox depth
