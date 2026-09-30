---
description: Derive 3-7 justified, testable requirements through the evidence gate (Gate 3).
agent: build
---

# Deriving justified requirements

A requirement is only accepted when we know where it comes from. This command derives a small batch — three to seven, not the whole specification — each with its recorded origin. The scope: "$ARGUMENTS".

## How it goes

1. I start with `od gate requirements`, summarized in a few lines.
2. For each candidate I follow the full chain: a source, what it literally says, what I understand from it for our case, the need it would serve, then the requirement as an atomic SHALL statement with its `derived-from` to evidence or a rule. A competitor's page is a clue, never a justification. Vague wording ("fast", "intuitive", "scalable") gets replaced with measurable criteria, and non-functional requirements ride along in the same batch.
3. When the origin is missing, I first hand the question to the `domain-researcher` — and meanwhile the requirement stays `ASSUMED`, never `EXPLICIT`. The writing itself goes through `/od-record`.
4. I finish with `od link --check` and `od gate requirements`, answering summary-first: the **Verdict**, what got written with its origins, the main blockers (`unjustified-requirement`, `unresolved-provenance`, `duplicate-requirement`...) — then detail in an appendix.
