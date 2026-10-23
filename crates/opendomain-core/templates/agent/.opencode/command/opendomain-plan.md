---
description: Produce ONE bounded, ordered plan with decisions (Gate 6). Architecture derived, never fashionable.
agent: build
---

# Planning the work, properly

A good plan is bounded, ordered, and decided: we know what to do, in which order, and why these choices. One plan per pass (`changes/<slug>/tasks.md` plus the design), never an open-ended task list. The scope: "$ARGUMENTS".

## How it goes

1. I start from the specs in scope (`opendomain trace --forward`) and the existing architecture with its decisions, summarized in a few lines.
2. I write the `PLAN-` with its approach and authority model — spec-first, spec-anchored, or spec-as-source (knowing that LLM-written application code is never spec-as-source) — then the `TASK-` records attached to their specs, ordered by dependencies, with no cycles (`dependency-cycle`), no orphan tasks (`untraced-task`), no tasks without a plan. Architecture is derived from domain boundaries, never from fashion. Each decision notes its context, its content, and its consequences. Writing goes through `/opendomain-record`, and when feasibility is doubtful, I run a read-only spike myself (build/test commands, never a write): the plan stays the master of the DAG.
3. I finish with `opendomain link --check` and `opendomain gate plan`, answering summary-first: the **Verdict**, what got written, the main blockers — then detail in an appendix.
