---
description: Specify ONE capability as observable behaviour + acceptance criteria (Gate 5). WHAT, never HOW.
agent: build
---

# Specifying one capability — and nothing else

A specification describes the WHAT — observable behavior — never the HOW. One capability at a time, proposed through `opendomain propose <slug>`, with its deltas filed under `changes/<slug>/specs/` rather than a restatement of current behavior. The capability: "$ARGUMENTS".

## How it goes

1. I re-read the source requirement (`opendomain show` then `opendomain trace` on the `REQ-`), summarized in a few lines.
2. I write the `SPEC-` attached to its requirement, with nominal scenarios, edge cases, and errors — and no technology names (postgres, kafka, microservices... all of that belongs to the plan). Then its acceptance criteria, each with an observable outcome (or it trips `empty-criterion`), linked with `verifies`. Writing goes through `/opendomain-record`.
3. I finish with `opendomain link --check` and `opendomain gate specification`, answering summary-first: the **Verdict**, what got written, the main blockers (`untraced-specification`, `how-not-what`...) — then detail in an appendix.
