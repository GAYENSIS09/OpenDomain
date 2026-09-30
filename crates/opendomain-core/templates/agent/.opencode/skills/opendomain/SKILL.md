---
description: Use when working in a project that has an opendomain/ workspace. Explains the artifact format, the epistemic discipline, and which od command to reach for.
---

# Working under OpenDomain

This project keeps its domain knowledge in `opendomain/` as linked Markdown
artifacts, and `od` enforces the links. `od` reads that knowledge; it never
rewrites registries unless a command explicitly says it does.

## The one thing that matters

Every artifact declares an `epistemic` value, and it is the honest label for
how the claim is known:

`EXPLICIT` (a source states it) · `TACIT` · `HYBRID` · `INFERRED` (derived,
not stated) · `ASSUMED` (nobody knows yet) · `UNKNOWN` (a known gap) ·
`CONFLICTING` (sources disagree)

**Do not write `EXPLICIT` for something you inferred.** That is epistemic
laundering — asserting invented domain semantics as known fact — and it is the
failure mode this framework exists to catch. A confident rule built on general
knowledge, with no source, is `ASSUMED` with the reasoning written down.

Never invent a `SRC-` or `EVD-` identifier to make a claim look sourced. A
fabricated citation is worse than an honest gap: it propagates downstream and
becomes indistinguishable from a real one.

## Artifact format

Each registry is a Markdown file of `##` headings, one per artifact. When the
registry is still empty — only the commented example and the Rules section —
follow the example: empty is the normal starting state, not a reason to stop.
Allocate the id with `od new <KIND>` rather than guessing the next number.

````markdown
## REQ-012 — Consult the movement history of an item

```toml
type = "FUNCTIONAL"
status = "DRAFT"
epistemic = "INFERRED"
confidence = "MEDIUM"
actor = ["ACT-002"]
derived-from = ["EVD-014"]
```

**Statement**

The system SHALL allow authorized users to consult the chronological movements
of an item.
````

The `toml` block is load-bearing and machine-checked; a malformed block is
reported with a line number, never guessed at. The prose is never parsed, so
write as much explanation as the subject deserves.

Relations are declared by the artifact that depends on the other:
`REQ-004 derived-from RULE-001`, not the reverse.

## Which command to use

Ask the CLI rather than guessing — these are the real commands:

| Question | Command |
|---|---|
| Where does the project stand? | `od status` |
| Why does this exist? | `od trace <ID>` |
| What breaks if this changes? | `od impact <ID>` |
| Is it consistent enough to proceed? | `od gate <gate>` |
| Is anything dangling or unjustified? | `od audit`, `od link --check` |
| What do the identifiers mean? | `od reference` |
| Record a link | `od link <ID> <relation> <TARGET>` |
| Next free id of a kind | `od new <KIND> [--scope S]` (read-only; re-run before writing) |
| Move along the lifecycle | `od advance <ID> <STATUS>` |

Gates are named: `context`, `research`, `domain`, `requirements`, `opportunity`,
`specification`, `plan`, `implementation`, `completion`. `od gate --through
<gate>` runs everything up to that stage. Numbers 0–8 also work.

If a command you expected does not exist, `od --help` is authoritative. Do not
assume a command exists because it would be convenient.

## Gates are not a formality

A failing gate is a finding with a rule slug attached, such as
`gate-3/unjustified-requirement`. Report it as it is. Do not describe a
blocking finding as advisory because the surrounding work looks finished, and do
not work around a finding by editing the artifact to silence it.

## Writing to registries

Write the artifact, then run `od link --check` and `od gate <current-gate>`, and
report what they say. When a gate fails, say it fails.
