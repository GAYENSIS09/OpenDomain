---
description: Inspect ONE quality gate and report blockers without working around them.
agent: build
---

# Putting one gate under the microscope

Want to know whether a stage holds up? I run the requested gate — `opendomain gate $ARGUMENTS` — and tell you exactly what is missing, without working around anything.

With no argument, I check the `context` gate. To sweep every gate up to a stage, the syntax is `opendomain gate --through <n>`, where `n` is the number from 0 to 8 (a name won't work there — it's an error, not a silent no-op).

## How it goes

I answer summary-first: a clear **Verdict** (`PASS`, or `FAIL` with the blocking and advisory counts), then the blockers one by one — three at most — each with the stable identifier the tool printed (for example `gate-3/unjustified-requirement`, keep it verbatim for later), then exactly **one** next action. Advisories are listed separately: they ask for a recorded decision but don't stop progress. The tool's full report follows in an appendix, untouched.

## What I don't do

Running a gate means learning what is missing — not making it disappear. I touch no file to clear a finding, and I never describe a failing gate as passing because the work "looks done". That is precisely the mistake the framework exists to catch.
