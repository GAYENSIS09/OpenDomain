---
description: Write or strengthen a domain artifact, keeping its epistemic state honest.
agent: build
---

The user asked to work on: "$ARGUMENTS"

Before writing anything, establish what is actually known.

1. Run `od status` to see which stage the workspace is in, and which gate is
   currently blocking.
2. Read the relevant registry under `opendomain/` (for example
   `opendomain/domain/business-rules.md`). Match the existing heading format
   exactly: `## <PREFIX>-<NNN> — Title`, then a fenced `toml` block, then prose.
3. Run `od reference` if you are unsure which prefix a kind uses, or which
   relations exist.

## The rule that matters

Every artifact declares an `epistemic` value. Choose it honestly:

| Value | Use when |
|---|---|
| `EXPLICIT` | A cited source states this directly. Record `derived-from`. |
| `INFERRED` | Derived by reasoning from evidence. Record the chain and the `confidence`. |
| `ASSUMED` | Nobody knows yet; this is a placeholder. Say so in the prose. |
| `UNKNOWN` | A known gap. Must be tracked, not quietly filled in. |
| `CONFLICTING` | Sources disagree. Record both, and do not resolve it yourself. |

**Never write `EXPLICIT` for something you inferred.** That is epistemic
laundering: the failure mode this whole framework exists to catch. If you
believe a rule from general knowledge and no source, that is `ASSUMED` with the
reasoning written down, not `EXPLICIT`.

## After writing

Run `od link --check` and `od gate <current-gate>`. Report what they say.

If you could not find a source, say so explicitly to the user rather than
producing a confident rule with a plausible citation-shaped artifact ID. A
fabricated `SRC-` or `EVD-` identifier is worse than an honest gap, because it
will propagate through every downstream gate.
