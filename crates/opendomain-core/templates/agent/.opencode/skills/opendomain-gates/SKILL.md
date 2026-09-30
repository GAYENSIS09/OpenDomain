---
description: Use when reading any od gate/status/audit output. Summary-first rendering, slug-to-action table, and the no-rationalisation rule.
---

# Reading a gate result — and acting on it

`od` reads your records and checks the links; it never rewrites them (except through the few commands that say so). Every gate is a pure function: the same workspace always gives the same answer. A blocking finding fails the command with a non-zero exit — which is what makes gates usable as CI steps — while an advisory is reported without failing. Learn both by heart: neither is negotiable at the call site.

## Answer summary-first, never wall-of-text

A raw gate dump is noise; a good report is signal. Always render in this order:

1. **Verdict** — one line: `PASS`, or `FAIL` with the counts (`N blocking, M advisory`).
2. **Top blockers** — at most three, each with the stable identifier the tool printed (like `gate-3/unjustified-requirement`) plus one sentence in your own words. Keep the identifiers verbatim — they are how findings get cited later.
3. **Next action** — exactly one smallest step: the file to open, the source to read, the link to record.
4. **Appendix** — the tool's full output, unedited, for the record.

And three refusals that are not optional: never soften a `FAIL` into "mostly ready"; never describe a blocking finding as advisory because the work feels complete; never edit a record just to silence a finding — that edit is the laundering the framework detects, performed by you. When a gate fails, say it fails, then fix the cause.

## From identifier to action

| What the tool says | What you do |
|---|---|
| `gate-0/no-domain-map`, `gate-0/no-stakeholders` | Complete the context (`/od-project`) |
| `gate-1/no-sources`, `gate-1/orphan-evidence`, `gate-1/undeclared-epistemic` | Research the missing finding (`/od-research`) |
| `gate-1/single-source` | Find a second *independent* source — not a third record on the same one |
| `gate-2/empty-registry`, `gate-2/duplicate-term` | Model the missing piece or merge the duplicates (`/od-domain`) |
| `gate-3/unjustified-requirement`, `gate-3/unresolved-provenance` | Trace to evidence or mark assumed with a tracked uncertainty |
| `gate-5/untraced-specification`, `gate-5/how-not-what` | Derive from the requirement; move the HOW to the plan |
| `gate-6/untraced-task`, `gate-6/dependency-cycle` | Trace the tasks; break the cycle |
| `gate-7/epistemic-laundering`, `gate-7/orphan-test` | Bring explicit evidence or record the promotion honestly; link a test |
| `gate-8/dangling-reference`, `gate-8/assumption-reached-production` | Check the links; resolve the assumption or track it as a risk |

Two syntax notes that save real time: `od gate --through N` takes the gate *number* (0–8), never its name. And `od reference` lists every rule — read it before writing, not after tripping.
