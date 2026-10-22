---
description: Show where the project stands in 10 seconds. Orientation only, never fixes.
agent: build
---

# Where does the project stand?

Wondering "where are we?" This command answers in ten seconds, without changing anything.

## How it goes

1. I run `opendomain status`, which counts the artifacts and checks every gate.
2. I give you the essentials, summary-first: a **Verdict** (the blocking gate plus the artifact count), then the top blockers with their identifiers (for example `gate-0/no-stakeholders`), then exactly **one** next action — `/opendomain-project` when context is missing, `/opendomain-research` when evidence is missing, `/opendomain-record` for writing.
3. The full detail stays available in an appendix, trimmed to failing gates and warnings.

## What I don't do

I don't fix anything myself. When something blocks, I tell you what to run next — the fix happens as a separate, explicit step. If no workspace exists yet, I suggest `opendomain init` followed by `/opendomain-project`. And when a gate fails, I say so plainly: a failure is never softened into "mostly ready".

$ARGUMENTS
