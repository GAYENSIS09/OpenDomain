---
description: Implement ONE scoped artifact through the verified harness (Gate 7). SPEC/AC route through their TASK; TASK routes to opendomain-harnais. No inline implementation.
agent: build
---

# Implementing without drifting

Here I write no code myself, and I route every artifact through the verified-execution path — no agent-authored code outside the harness. Tell me which one: "$ARGUMENTS" must name ONE identifier (`TASK-`, `SPEC-`, or `AC-`). With no identifier, I start with `opendomain status` and ask you which one.

## The one path — through the harness

1. **A `TASK-` goes straight to the `opendomain-harnais` subagent** — the harness architect: it compiles the record into a DAG, grounds, builds the kits, and drives the run under the gates. Its doctrine is the `opendomain-harness` skill; my job is to relay its report, résumé-first: the **Verdict** (the run PASS + its directory, or the terminal failure typed + attributed), the nodes validated, what was retried — so the TEST record that cites the run (`run = "runs/run-NNN-…"` — `opendomain provenance` verifies the citation) lands in the registry it belongs to.
2. **A `SPEC-` or an `AC-` has no task yet**: stage 6 comes first. I check with `opendomain impact <ID>` and `opendomain trace --forward <ID>` whether a TASK already carries it; if none, I create the TASK record (through `/opendomain-record`, derived-from the spec, with the plan's order), then route it as above. A specification implemented without a task is exactly the drift the framework blocks.
3. **The evidence lands in the registries**: the architect hands back the run directory; the TEST record that cites it (`run = "runs/run-NNN-…"` — `opendomain provenance` verifies the citation) is written through `/opendomain-record`, then `opendomain gate implementation` says the truth.

## Three rules

When a gate turns red after the change, I say so with its identifier and we return to green first. A passing test is not a verified criterion — promotions get recorded with their evidence. And I never mark anything `IMPLEMENTED` or `VERIFIED` myself: that's `opendomain advance`, owned by the caller — which refuses backward moves anyway. We move in vertical slices, one artifact at a time, everything archived as a run.