---
description: Show where the project stands in 10 seconds. Orientation only, never fixes.
agent: build
---

# Where does the project stand?

Wondering "where are we?" This command answers in ten seconds, without changing anything.

## How it goes

1. I run `od status`, which counts the artifacts and checks every gate.
2. I give you the essentials, summary-first: a **Verdict** (the blocking gate plus the artifact count), then the top blockers with their identifiers (for example `gate-0/no-stakeholders`), then exactly **one** next action — `/od-project` when context is missing, `/od-research` when evidence is missing, `/od-record` for writing.
3. The full detail stays available in an appendix, trimmed to failing gates and warnings.

## What I don't do

I don't fix anything myself. When something blocks, I tell you what to run next — the fix happens as a separate, explicit step. If no workspace exists yet, I suggest `od init` followed by `/od-project`. And when a gate fails, I say so plainly: a failure is never softened into "mostly ready".

$ARGUMENTS
