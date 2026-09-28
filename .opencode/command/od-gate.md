---
description: Run a quality gate and report blocking findings without working around them.
agent: build
---

Run the gate: `od gate $ARGUMENTS` — with no argument, `od gate` runs `context`.

Show the output verbatim. The gate's own words are the finding; do not
summarise them away.

Then:

- List every BLOCKER as a separate item, each with the rule slug the tool
  printed (for example `gate-3/unjustified-requirement`). The slug is how the
  finding gets cited later, so keep it.
- List WARNING findings separately from BLOCKERs. A warning does not stop
  progress but requires a recorded decision.
- State plainly whether the gate passed or failed.

**Do not attempt to clear a finding by editing files.** The purpose of running a
gate is to learn what is missing. If the user asks you to fix a specific
finding, do that as a separate, explicit step.

**Never** describe a failing gate as passing, or a blocking finding as advisory,
because the work in front of you feels complete. The framework exists to catch
exactly that error, and an agent that rationalises a gate result has defeated it.

If the user wants every gate up to a stage, that is
`od gate --through <gate-name>`.
