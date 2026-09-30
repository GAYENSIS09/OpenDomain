---
description: Full traceability diagnostic. The ONLY command that renders the whole chain; od-complete calls it for the release decision.
agent: build
---

# Full health check

Before declaring anything finished, you need an honest checkup. That's this command's job — and the only one to paint the whole picture. (`/od-complete` will reuse this checkup to make the release call, rather than recomputing it.)

## How it goes

1. I run `od audit` and `od link --check`. No need to add `od gate --through 8`: the audit already contains all 9 gates, from a clean PASS to a documented FAIL.
2. I answer summary-first. The **Verdict** first: the highest gate reached and the first one failing. Then, in order: dangling references with the identifiers involved; unjustified requirements — with no recorded origin; open conflicts and uncertainties; coverage — how many requirements are specified, traced, verified.

On that last point, hold me to the same standard as yourself: "traced" means an origin is recorded, "verified" means a test passes. A traced requirement is not a verified requirement, and conflating them is exactly the kind of overclaim this audit exists to prevent. The full detail follows in an appendix, trimmed to failing gates.
