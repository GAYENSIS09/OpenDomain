---
description: Implement ONE scoped artifact via spec-implementer with drift checks (Gate 7). No inline implementation.
agent: build
---

# Implementing without drifting

Here I write no code myself: I hand implementation to the `spec-implementer` subagent, whose job it is — one artifact, tested, with no drift from the spec. Tell me which one: "$ARGUMENTS" must name ONE identifier (`SPEC-`, `TASK-`, or `AC-`). With no identifier, I start with `od status` and ask you which one.

## How it goes

1. I size the job with `od impact <ID>` and pass everything to the `spec-implementer`. It reads the spec, checks the gate before, implements with the repository's own commands, tests, then re-checks the gate after.
2. I relay its report, summary-first: the **Verdict** of `od gate implementation`, what got done (files, exact commands, quoted results), what got left out and why, what stays uncertain — then its three full reports in an appendix (`od link --check`, `od gate implementation`, `od impact`).

Three rules: when a gate turns red after the change, I say so with its identifier and we return to green first. A passing test is not a verified criterion — promotions get recorded with their evidence. And I never mark anything `IMPLEMENTED` or `VERIFIED` myself: that's `od advance`, owned by the caller — which refuses backward moves anyway. We move in vertical slices, test-first, one artifact at a time.
