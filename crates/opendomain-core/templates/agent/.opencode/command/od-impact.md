---
description: Report what a change would break to decide safely. Direct + indirect, zero invented.
agent: build
---

# If I change this, what else breaks?

Before touching an artifact — "$ARGUMENTS" — you want the blast radius measured. I run `od impact $ARGUMENTS`, plus `od link --check` to confirm the links I'm reasoning over actually exist.

## How it goes

I answer summary-first: a quantified **Verdict** (so many direct impacts, so many indirect), then the direct impacts — what immediately depends on the artifact — then the indirect ones it reaches transitively (the first five, then the count). The tool's full report follows in an appendix.

Two honesties: when the direct impact is zero, I say so plainly instead of inventing risks to sound dramatic. And when `od link --check` reveals dangling links — dependencies pointing at nothing — I name them, because any impact count built on them is fragile. An artifact with no recorded origin gets flagged too: you cannot reason safely about something with no foundation.
