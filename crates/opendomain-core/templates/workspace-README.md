# This workspace

`opendomain/` is the project's engineering knowledge base. Everything the
framework knows about the domain, the requirements, the specification and the
verification lives here, in plain Markdown, addressable by identifier.

## Where things live

| Path | Holds | Stage |
|---|---|---|
| `project.md` | Phase 0: what this project is, what already exists, what is unknown | Context |
| `domain/research/` | `SRC` sources, `EVD` evidence, and the research protocol | 1 |
| `domain/` | `TERM` `STK` `ACT` `PROC` `ENT` `EVENT` `RULE` `INT` `DOM`, plus `UNC` `XCONF` `DEC` | 1 |
| `opportunity/` | `OPP` `ASM` `CON` `CPT` `HYP` `EXP` `CDR` | 2 |
| `requirements/` | `REQ` `NFR` `CNST`, and the `BAS` baselines | 1 → 3 |
| `use-cases/` | `UC` use cases | 3 |
| `specs/` | `SPEC` behavioural specifications, grouped by capability | 3 |
| `plans/` | `PLAN` technical plans | 3 |
| `tasks/` | `TASK` implementation tasks | 3 |
| `contracts/` | `CTR` machine-checkable contracts | 3 |
| `architecture/` | `architecture.md` and `ADR` decision records | 3 |
| `validation/` | `SCN` scenarios, `TEST` acceptance tests, drift findings, audits | 3 |
| `changes/` | In-flight change proposals, one directory each | all |
| `harness/` | Verified execution: task specs, grounding, the agent DAG, runs, memory; governed by `opendomain.toml` (`[gates]`, `[sandbox]`) | execution |

When implementation is executed by agents, it runs through `harness/`: every
agent output passes its gate before it reaches the next agent, and each run is
archived under `harness/runs/` — the registries' `TEST` and `DRIFT` records
may cite those archives as their provenance. The full execution doctrine is
the framework's documentation; the directory contract is `harness/README.md`.

## The file format

Every registry is a Markdown file of `##`-level headings, one per artifact. The
heading opens with the identifier. A fenced `toml` block carries the
machine-checked fields; the prose below carries everything a human reads.

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

The prose is never machine-read, so you can write as much explanation as the
subject deserves without risking a parse error. The TOML block is load-bearing:
a malformed block is reported, never guessed at.

## Rules that are enforced

These are checked by `opendomain gate` and `opendomain audit`, not by good intentions:

- Every requirement has `derived-from` an evidence record or a business rule.
- Evidence is `derived-from` a source. An unsourced claim is not evidence.
- Three or more evidence records citing one source is a *single-source* failure,
  not corroboration.
- A requirement whose `epistemic` is `ASSUMED`, `UNKNOWN` or `CONFLICTING` may
  not reach the specification stage.
- A specification states observable behaviour. Mentioning a database or a
  framework in a specification is reported as plan content that leaked in.
- A hypothesis without a metric and a failure condition is not a hypothesis.
- A concept justified by competitor parity is rejected as feature cloning.
- Task dependencies may not form a cycle.
- A requirement marked `IMPLEMENTED` with no verified test downstream fails the
  completion gate.

## Getting started

`opendomain` reads and checks; it does not author. You write the artifacts, `opendomain` tells
you whether they are consistent. Every command below is real — run `opendomain --help`
or `opendomain <command> --help` for the full list.

```bash
opendomain status                        # where does the project actually stand?
opendomain gate context                  # what is missing before the domain can start?
opendomain gate --through domain         # run every gate up to the domain stage

# write artifacts, then check them
opendomain link --check                  # every referenced id must exist
opendomain link REQ-012 derived-from EVD-014   # declare an edge (writes the TOML block)
opendomain advance REQ-012 VALIDATED     # move along the lifecycle
opendomain show REQ-012                  # one artifact's full record

opendomain trace REQ-012                 # why does this exist?
opendomain trace --forward REQ-012       # what implements and verifies it?
opendomain impact RULE-007               # what breaks if this changes?
opendomain graph --root REQ-012 --depth 3
opendomain audit                         # traceability health across the workspace
```

Gates are named, not numbered, in the commands: `context`, `research`, `domain`,
`requirements`, `opportunity`, `specification`, `plan`, `implementation`,
`completion`. Numbers 0-8 are also accepted and mean the same order.

`--json` works on every command that reports, for use in scripts and CI.
`opendomain reference` prints the full gate list and the 37 artifact kinds.
