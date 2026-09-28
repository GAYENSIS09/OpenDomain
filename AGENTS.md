# OpenDomain — agent orientation

This repository contains the OpenDomain framework: a Rust workspace
(`opendomain-core`, `opendomain-cli`) and the specification paper.

## The problem this addresses

An AI coding assistant produces complete, consistent, test-passing software
whose **domain semantics were invented rather than known**. That is an
epistemic failure, not a modelling one, and it is invisible without a
representation of what is known versus assumed.

## The one rule

Every artifact declares an `epistemic` value. Use the honest one.

- `EXPLICIT` — a source states this directly. Record `derived-from`.
- `INFERRED` — derived by reasoning. Record the chain and a `confidence`.
- `ASSUMED` — nobody knows yet. Say so in the prose.
- `UNKNOWN` — a known gap. Track it; do not quietly fill it in.
- `CONFLICTING` — sources disagree. Record both, resolve nothing.

**Never write `EXPLICIT` for something you inferred**, and never invent a
`SRC-`/`EVD-` identifier to make a claim look sourced. A fabricated citation
propagates downstream and becomes indistinguishable from a real one. An honest
gap is the better result.

## The tool

`od` reads the knowledge and checks the links. It is authoritative — if a
command you expect is missing, `od --help` decides, not your expectation.

```bash
od status            # where does the project actually stand?
od trace <ID>        # why does this exist?
od impact <ID>       # if this changes, what breaks?
od gate <gate>       # is it consistent enough to proceed?
od audit             # anything dangling, unjustified, unresolved?
od reference         # the 37 artifact kinds and the 9 gates
```

Gates are named: `context`, `research`, `domain`, `requirements`, `opportunity`,
`specification`, `plan`, `implementation`, `completion`. A failing gate exits
non-zero, so it works as a CI step.

## Slash commands available here

| Command | Purpose |
|---|---|
| `/od-status` | Where the project stands, and what is blocking |
| `/od-gate <gate>` | Run a gate, report findings without rationalising them |
| `/od-trace <ID>` | Why an artifact exists, from recorded links |
| `/od-impact <ID>` | Blast radius of a proposed change |
| `/od-audit` | Full traceability health before claiming completion |
| `/od-record <topic>` | Write a domain artifact, keeping epistemic state honest |

Subagents: `domain-researcher` (sources and evidence, never invents citations)
and `spec-implementer` (one scoped task, gate-checked, reports what it skipped).

## When the gates say no

A gate failure is a finding with a stable rule slug, such as
`gate-3/unjustified-requirement`. Report it as it is. Do not describe a
blocking finding as advisory because the work looks finished, and do not edit an
artifact to silence a finding — that is the laundering the framework detects.

## Working on this repository

```bash
cargo test --workspace      # 91 unit tests + 2 doctests
cargo fmt --all -- --check
pdflatex opendomain-paper.tex && pdflatex opendomain-paper.tex   # the paper
```

`opendomain-paper.tex` is the authoritative statement of intent. When code and
paper disagree, the paper's traceability tables record which side is ahead —
keep that honest in both directions.
