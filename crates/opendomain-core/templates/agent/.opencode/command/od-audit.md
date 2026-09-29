---
description: Audit traceability health across the whole workspace before claiming work is done.
agent: build
---

The user asked for a full audit. This is the check that runs before declaring
anything complete.

1. Run `od audit` and show the output verbatim.
2. Run `od link --check` and show the result. Dangling references mean stated
   dependencies point at artifacts that do not exist.
3. Run `od gate --through 8` and show which gates fail. `--through` takes the
   gate number 0–8, not the name; `completion` is gate 8.

Then report, in this order:

- Dangling references, with the identifiers involved.
- Requirements that are unjustified — no recorded origin.
- Unresolved conflicts and unresolved uncertainties.
- Coverage: how many requirements are specified, traced, and verified.
- The highest gate reached, and the first gate that fails.

Be exact about the difference between "traced" and "verified". A requirement
that is traced has a recorded origin; a verified one has a test that passes. A
traced requirement is not a verified requirement, and conflating them is the
specific overclaim this audit exists to prevent.
