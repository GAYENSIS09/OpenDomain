---
description: Write ONE artifact with honest epistemics. Transverse utility used by all production commands.
agent: build
---

# Recording one artifact

Every production command ends up writing the same thing at some point: one well-formed artifact with an honest epistemic state. This command is that shared utility — a single artifact at a time, never a whole registry. Today's subject: "$ARGUMENTS".

## How it goes

1. I start with `od status` to see which stage we're at, then I read the target registry under `opendomain/` (for example `domain/business-rules.md`), matching its format exactly: a `## PREFIX-001 — Title` heading, then a machine-checked `toml` block, then free prose. When the registry is still empty — just the commented example and the Rules section — that's normal: I follow the example. Unsure about a prefix or a relation? `od reference` settles it. And the identifier? I ask `od new <KIND>` (for example `od new RULE`) for the next free number — never counted by hand.
2. For the epistemic value, one golden rule: `EXPLICIT` only when a source I read states it directly, with the link recorded. Reasoning from evidence is `INFERRED`, with the chain written down. Nobody knows? `ASSUMED`, said out loud. A known hole? `UNKNOWN`, to track. Sources disagreeing? Both positions, no ruling. Writing `EXPLICIT` for something inferred is called epistemic laundering — the failure mode this entire framework exists to catch.
3. I finish with `od link --check` and the current gate, answering summary-first: the **Verdict**, what got written and where, the epistemic value chosen and why — then detail in an appendix.

One last thing: when I can't find a source, I say so explicitly. I never invent a `SRC-` or `EVD-` identifier to make a claim look sourced — a fabricated citation is worse than an honest gap, because it propagates through every downstream gate with no way to tell it apart from a real one.
