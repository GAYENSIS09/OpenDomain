---
description: Show where the project actually stands, and what is blocking progress.
agent: build
---

Run the OpenDomain status report and summarise it. The user asked: "where are we?"

Do this:

1. Run `od status` and show the output verbatim. Do not paraphrase the gate lines.
2. If any gate reports blocking findings, name the single most important one and
   stop. Do not attempt to fix it — that is what `/od-context` and the other
   commands are for.
3. If `od` reports the workspace is missing, say so and suggest `od init`.

Report faithfully. If a gate FAILS, say it FAILS. Do not soften a failure into
"mostly ready", and do not describe an unresolved epistemic state as a minor gap.

$ARGUMENTS
