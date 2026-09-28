# OpenDomain

OpenDomain is a domain-first, evidence-driven software engineering framework that transforms real-world knowledge into validated software systems through traceable domain modeling, opportunity discovery, specification, behavioral verification, test-driven implementation, and continuous audit.

It exists to catch one specific failure: an AI coding assistant produces syntactically complete, internally consistent, test-passing software whose **domain semantics were invented rather than known**. That is not a modelling error, it is an epistemic one. The assistant has no way to distinguish a domain rule it reconstructed from a plausible-sounding hallucination, because nothing in the workspace records what is known, what is assumed, what is inferred, and what is unknown.

OpenDomain stores that distinction explicitly and enforces it mechanically.

## The core idea

Every engineering artifact carries an **epistemic value** — one of `EXPLICIT`, `TACIT`, `HYBRID`, `INFERRED`, `ASSUMED`, `UNKNOWN`, `CONFLICTING`. Artifacts are plain Markdown with a machine-checked TOML block, and they are wired into one ordered traceability spine from source to test.

That makes two questions mechanically answerable:

```bash
od trace REQ-012          # why does this exist?
od impact RULE-007        # if this changes, what breaks?
```

Both are graph queries over declared relations. Neither requires you to remember anything.

## Install

Requires Rust 1.75 or newer.

```bash
git clone <this-repository>
cd OpenDomain
cargo build --release
```

The binary lands at `target/release/od.exe` (Windows) or `target/release/od`
(Unix). Put it on your `PATH`, or invoke it by full path.

## Start here

```bash
mkdir my-project && cd my-project
od init --name my-project
od status
```

`od init` creates an `opendomain/` directory: 41 files, one per registry, plus
configuration and agent instructions. It is idempotent — re-running it reports
what already exists and changes nothing.

```bash
od init --profile lean      # start at Stage 3; the domain model exists elsewhere
od init --profile core      # stages 0, 1, 3-8 (no opportunity discovery)
od init --profile full      # everything (default)
od init --dry-run           # report what would happen, write nothing
```

Other init flags: `--repository greenfield|brownfield|hybrid` and
`--authority spec-first|spec-anchored|spec-as-source`.

## Working with `od`

`od` **reads and checks**; it does not author. You write the artifacts, `od`
tells you whether they are consistent.

```bash
od status                        # where does the project actually stand?
od gate context                  # what is missing before the domain can start?
od gate --through domain         # run every gate up to the domain stage

od link --check                  # every referenced id must exist
od link REQ-012 derived-from EVD-014   # declare an edge (writes the TOML block)
od advance REQ-012 VALIDATED     # move along the lifecycle
od show REQ-012                  # one artifact's full record

od trace REQ-012                 # why does this exist?
od trace --forward REQ-012       # what implements and verifies it?
od impact RULE-007               # what breaks if this changes?
od graph --root REQ-012 --depth 3
od audit                         # traceability health across the workspace
od reference                     # the gates and the 37 artifact kinds
```

Change proposals follow the OpenSpec layout:

```bash
od propose tighten-reservation --summary "Tighten the reservation window"
od apply tighten-reservation
od archive tighten-reservation
```

### The nine gates

| Gate | Question it asks |
|---|---|
| `context` | Do we know what we are building, and what is already true? |
| `research` | Is the domain understood well enough to derive requirements? |
| `domain` | Is the domain model coherent, owned and cross-linked? |
| `requirements` | Is every requirement justified, testable and traced? |
| `opportunity` | Has anything been challenged, and is the chosen concept earned? |
| `specification` | Does the specification describe observable behaviour and nothing else? |
| `plan` | Is there a bounded, ordered plan that can realise the specification? |
| `implementation` | Did implementation satisfy the acceptance criteria without drift? |
| `completion` | Is the whole chain coherent, tested and free of critical unknowns? |

`od reference` prints this list as JSON.

Findings carry one of three severities. `BLOCKER` prevents progression,
`WARNING` requires a recorded decision, `NOTE` is advisory.

Gates are pure functions from the workspace to a report: they never mutate
state, so running one twice gives the same answer, and a failure names the exact
condition that broke.

### Scripts and CI

`--json` works on every command that reports. A failing gate exits non-zero,
so `od gate --through specification` works as a CI step.

```bash
od gate --through specification --json > gate-report.json
```

`--strict` treats advisories as failures. `--no-color` and `-C <dir>` are
available everywhere.

## Using it with opencode

This repository ships a working opencode integration. If you point opencode at a
project that has an `opendomain/` workspace, the following become available
automatically.

**Slash commands**

| Command | What it does |
|---|---|
| `/od-status` | Where the project stands, and what is blocking |
| `/od-gate <gate>` | Runs a gate and reports the findings verbatim |
| `/od-trace <ID>` | Explains why an artifact exists, from its recorded links |
| `/od-impact <ID>` | What a change to an artifact would break |
| `/od-audit` | Traceability health before claiming work is done |
| `/od-record <topic>` | Writes a domain artifact, keeping its epistemic state honest |

**Subagents**

- `domain-researcher` — finds and records `SRC-`/`EVD-` artifacts. Its bash
  permission is `deny` by default with `od *` allowed, so it can check its own
  work but cannot run anything else.
- `spec-implementer` — one scoped task against an existing specification, gated
  before and after.

**Skill.** `.opencode/skills/opendomain/SKILL.md` loads on demand and carries
the artifact format and the epistemic discipline. `AGENTS.md` holds the short
version that is always in context, so the skill does not have to be.

To adopt this in your own project, copy `.opencode/` and `AGENTS.md` across and
put the `od` binary on `PATH`. `opencode.json` allows `od *` and asks before the
commands that write — `init`, `link`, `advance`, `apply`, `archive` — so reading
the project is frictionless and mutating it still needs consent.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "bash": {
      "od *": "allow",
      "od link *": "ask"
    }
  }
}
```

> opencode reads its configuration at startup. After adding or changing
> `opencode.json`, agents, commands or skills, **restart opencode** or the new
> definitions will not be picked up.

## The artifact format

Each registry is a Markdown file of `##` headings, one per artifact. A fenced
`toml` block carries the machine-checked fields; the prose below is for humans
and is never parsed.

````markdown
## REQ-012 — Consult the movement history of an item

```toml
type = "FUNCTIONAL"
status = "DRAFT"
epistemic = "INFERRED"
confidence = "MEDIUM"
actor = ["ACT-002"]
derived-from = ["EVD-014"]
rules = ["RULE-007"]
```

**Statement**

The system SHALL allow authorized users to consult the chronological movements
of an item.
````

A malformed TOML block is reported with a line number, never guessed at. The
prose is free-form, so you can write as much explanation as the subject
deserves without risking a parse error.

Identifiers carry their kind as a prefix — `REQ`, `EVD`, `SRC`, `RULE`, `AC`,
`SPEC` and 31 others (`od reference` lists them all) — optionally scoped, as in
`REQ-INVENTORY-014` and `AC-INVENTORY-014-01`. Links are declared by the
artifact that depends on the other, not the reverse: `REQ-004 derived-from
RULE-001`.

## What gets enforced

The nine gates carry 55 named rules, each identified by a stable slug such as
`gate-3/unjustified-requirement` so a failure can be cited rather than
paraphrased. A representative selection:

| Rule | What it forbids |
|---|---|
| `gate-3/unjustified-requirement` | A requirement with no `derived-from` an evidence record or business rule |
| `gate-1/orphan-evidence` | An evidence record not `derived-from` a source. An unsourced claim is not evidence |
| `gate-1/single-source` | Three or more evidence records citing one source — that is a *single-source* failure, not corroboration |
| `gate-8/assumption-reached-production` | A requirement whose `epistemic` is `ASSUMED`, `UNKNOWN` or `CONFLICTING` reaching the specification stage |
| `gate-5/how-not-what` | A specification that states mechanism rather than observable behaviour |
| `gate-4/unfalsifiable-hypothesis` | A hypothesis with no metric and no failure condition |
| `gate-4/feature-cloning` | A concept justified only by competitor parity |
| `gate-6/dependency-cycle` | Task dependencies forming a cycle |
| `gate-8/requirement-without-passing-test` | A requirement marked `IMPLEMENTED` with no verified test downstream |
| `gate-8/dangling-reference` | A link pointing at an identifier that does not exist |

`od audit` reports traceability health across the whole workspace, including
coverage ratios, unjustified artifacts, dangling links, and open conflicts.

## Repository layout

```
crates/opendomain-core/   artifact model, parser, workspace index, graph, gates
crates/opendomain-cli/    the `od` binary
opendomain-paper.tex      the framework specification (LaTeX source)
opendomain-paper.pdf      compiled paper
note.txt                  original framework notes
AGENTS.md                 agent orientation, loaded automatically by opencode
opencode.json             opencode configuration for this repository
.opencode/command/        slash commands (/od-status, /od-gate, ...)
.opencode/agent/          subagents (domain-researcher, spec-implementer)
.opencode/skills/         on-demand skill describing the artifact format
```

## Building and testing

```bash
cargo test --workspace      # 91 unit tests + 2 doctests
cargo fmt --all -- --check
```

The paper is LaTeX and compiles with a stock TeX Live or MiKTeX:

```bash
pdflatex opendomain-paper.tex && pdflatex opendomain-paper.tex
```

## Current status

The core and the CLI are implemented and tested. The verification subsystem of
the paper (feature-file generation, contract testing, binding provenance) and
the toolchain provisioner are **designed but not implemented** — the paper's
Section 11.7 marks each unimplemented claim explicitly rather than leaving it to
be inferred.

Two known discrepancies between the paper and the implementation are recorded
rather than papered over:

- **Gate numbering.** The paper numbers the gates 1–9, the implementation 0–8.
  The dangling-reference rule therefore sits in `gate-8` (completion) in code,
  where the paper places it in Gate 8 (implementation).
- **Condition count.** The paper defines 77 numbered conditions across its nine
  gate tables; the implementation ships 55 rule slugs. The paper merges some
  conditions into single checks (its four Gate 1 conditions map onto three
  slugs, for instance), and it counts conditions that are designed but not
  implemented, such as `G8-B5` through `G8-B7`.

Neither is a documentation fix: reconciling the numbering, and deciding which
designed conditions should be implemented, are design decisions.

## License

MIT — see [LICENSE](LICENSE).
