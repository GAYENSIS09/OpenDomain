---
description: Grow the domain model by ONE bounded increment (Gate 2). Routes writing to od-record, sourcing to domain-researcher.
agent: build
---

# Growing the domain model

A domain model grows little by little: one bounded increment at a time — a context, a process, a group of entities — never the whole model at once. Tell me the scope: "$ARGUMENTS".

## How it goes

1. I start with `od gate domain`, summarized in a few lines, and I re-read the registries involved under `opendomain/domain/`: the glossary, the actors, the processes, the entities with their owners, the business rules with their evidence, the context map.
2. A few working rules: every important concept has exactly one owner; two contexts claiming the same word (`SalesCustomer` versus `InventoryCustomer`) get merged or their distinction recorded; every rule traces back to evidence; behavior lives in entities, not in services. Holes become tracked uncertainties, disagreements become unsettled conflicts.
3. Writing goes through `/od-record` with its discipline. And when a source is missing to support something, I hand that precise question to the `domain-researcher` — and wait for its evidence before writing `EXPLICIT` anywhere.
4. I finish with `od link --check` and `od gate domain`, answering summary-first: the **Verdict**, what got written, the main blockers (for example `gate-2/duplicate-term`) — then detail in an appendix.
