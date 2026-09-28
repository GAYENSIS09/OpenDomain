---
description: Explain why an artifact exists, following the derivation chain to its source.
agent: build
---

The user asked why an artifact exists: "$ARGUMENTS"

Run `od trace $ARGUMENTS` and show the chain verbatim.

Then add, in your own words:

- Where the chain starts: which source or evidence the claim ultimately rests on.
- Any step where the epistemic value is `ASSUMED`, `INFERRED`, `UNKNOWN` or
  `CONFLICTING`. Say plainly that the claim is not known, it is inferred.
- If the tool reports no upstream artifacts, say that the artifact has no
  recorded origin and is therefore unjustified. That is a real finding, not an
  empty result to be glossed over.

Do not answer the question yourself from the artifact's prose. The point of
`od trace` is to answer it from the recorded links, so that the answer survives
the agent that produced the artifacts.
