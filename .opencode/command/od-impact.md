---
description: Report what a change to an artifact would break, directly and indirectly.
agent: build
---

The user is considering a change to: "$ARGUMENTS"

Run `od impact $ARGUMENTS` and show the result verbatim.

Then add:

- Direct impact: what depends on it immediately.
- Indirect impact: what it reaches transitively.
- Anything the tool reports as having no recorded origin, since an artifact with
  no origin cannot be reasoned about safely.
- If `od audit` reports dangling references, name them. A dangling link means a
  stated dependency points at something that does not exist, so any impact count
  derived from it is unreliable.

If the direct impact is zero, say so directly. Do not invent downstream
consequences to make the change sound risky.
