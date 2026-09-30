---
description: Explain why an artifact exists from its recorded links, flagging weak epistemic steps.
agent: build
---

# Why does this artifact exist?

Every part of the project should be able to justify its existence. Give me its identifier — "$ARGUMENTS" — and I walk its link chain back to the source with `od trace $ARGUMENTS`.

## How it goes

I answer summary-first. First the **Verdict** in one sentence: the source or evidence everything ultimately rests on. Then the weak links: every step whose epistemic value isn't `EXPLICIT` — `ASSUMED`, `INFERRED`, `UNKNOWN`, or `CONFLICTING` — named honestly for what it is: not yet known, merely inferred. The tool's full chain follows in an appendix.

## One case matters

If the tool finds no upstream artifacts, that is not an empty result to gloss over: it means the artifact has no recorded origin, so it is unjustified. That is a real finding, and I report it as one.

One more note: I answer from the recorded links, never from my own knowledge or the artifact's prose. That's the whole point of traceability — the answer outlives the agent that produced the artifacts.
