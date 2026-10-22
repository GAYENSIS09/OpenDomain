---
description: Implement ONE scoped artifact via spec-implementer with drift checks (Gate 7). No inline implementation.
agent: build
---

# Implementing without drifting

Here I write no code myself: I hand implementation to the `spec-implementer` subagent, whose job it is — one artifact, tested, with no drift from the spec. Tell me which one: "$ARGUMENTS" must name ONE identifier (`SPEC-`, `TASK-`, or `AC-`). With no identifier, I start with `opendomain status` and ask you which one.

## When the artifact is a TASK — the agent-system path

A `TASK-###` is not implemented directly: it compiles into a DAG of agents and executes under the harness gates. In that case I load the `opendomain-harness` skill (the execution doctrine, one home — no persona) and drive the CLI: `opendomain plan TASK-012` (the task record **is** the task description `T`; its constraints feed the criteria, its links feed the traceability), `opendomain build`, `opendomain run <slug> --executor opencode|claude|mock`. Every agent output passes its gate before it propagates; the run archives append-only under `opendomain/harness/runs/`, and the TEST record it produces cites that run (`run = "runs/run-NNN-…"` — `opendomain provenance` verifies the citation). A terminal failure is typed (`opendomain attribute`): local → resume, upstream → re-ground, structural → re-plan the sub-graph only.

## How it goes

1. I size the job with `opendomain impact <ID>` and pass everything to the `spec-implementer`. It reads the spec, checks the gate before, implements with the repository's own commands, tests, then re-checks the gate after.
2. I relay its report, summary-first: the **Verdict** of `opendomain gate implementation`, what got done (files, exact commands, quoted results), what got left out and why, what stays uncertain — then its three full reports in an appendix (`opendomain link --check`, `opendomain gate implementation`, `opendomain impact`).

Three rules: when a gate turns red after the change, I say so with its identifier and we return to green first. A passing test is not a verified criterion — promotions get recorded with their evidence. And I never mark anything `IMPLEMENTED` or `VERIFIED` myself: that's `opendomain advance`, owned by the caller — which refuses backward moves anyway. We move in vertical slices, test-first, one artifact at a time.
