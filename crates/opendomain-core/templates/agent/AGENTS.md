# Working in a project with an OpenDomain workspace

This project records what it knows about its domain in `opendomain/`, as linked
Markdown artifacts. The `od` command reads that knowledge and checks the links
between artifacts. It is the authority on what this project has established: if
you want to know whether something here is known, inferred or assumed, ask `od`.

## The one rule

Every artifact declares an `epistemic` value. Use the honest one.

| Value | Meaning |
|---|---|
| `EXPLICIT` | A cited source states this directly. Record `derived-from`. |
| `TACIT` | Known by practice, not written anywhere. |
| `HYBRID` | Partly stated, partly practice. |
| `INFERRED` | Derived by reasoning. Write the chain and set `confidence`. |
| `ASSUMED` | Nobody knows yet. Say so in the prose. |
| `UNKNOWN` | A known gap. Track it; do not fill it in. |
| `CONFLICTING` | Sources disagree. Record both, resolve nothing. |

**Never write `EXPLICIT` for something you inferred.** A confident domain rule
built on general knowledge, with no source behind it, is `ASSUMED` with the
reasoning written down. Writing it as `EXPLICIT` is epistemic laundering, and it
is the specific failure this framework exists to catch.

**Never invent a `SRC-` or `EVD-` identifier** to make a claim look sourced. A
fabricated citation is worse than an honest gap: it propagates through every
downstream gate and becomes indistinguishable from a real one. If you could not
find a source, say so.

## Reading and writing artifacts

Each registry is a Markdown file of `##` headings, one per artifact, with a
fenced `toml` block that is machine-checked and prose that is never parsed:

````markdown
## REQ-004 — Reject a receipt exceeding the outstanding quantity

```toml
type = "FUNCTIONAL"
status = "VALIDATED"
epistemic = "EXPLICIT"
derived-from = ["EVD-014", "RULE-001"]
```

**Statement**

The system SHALL reject a goods receipt whose accepted quantity exceeds the
remaining quantity of the referenced purchase order line.
````

A malformed `toml` block is reported with a line number and never guessed at, so
you can write as much prose as the subject deserves without risking a parse
error.

Links are declared by the artifact that depends on the other, not the reverse:
`REQ-004 derived-from RULE-001`, never `RULE-001 derived-from REQ-004`.

Run `od reference` for the identifier prefixes, the relations, and the vocabulary
of statuses and epistemic values. Do not guess a prefix.

## The commands

`od --help` is authoritative. If a command you expected is missing, that is a
fact about the tool, not an invitation to assume it exists.

### Reading

| Question | Command |
|---|---|
| Where does the project stand, and what is blocking? | `od status` |
| Is anything dangling, unjustified or unresolved? | `od audit` |
| Are all declared links resolvable? | `od link --check` |
| What do the identifiers and rules mean? | `od reference` (63 rules, 9 gates, 37 kinds) |
| Give me one artifact in full | `od show <ID>` |
| Why does this artifact exist? | `od trace <ID>` |
| What implements and verifies it? | `od trace <ID> --forward` |
| Who said so, nearest source first? | `od trace <ID> --provenance` |
| What breaks if this changes? | `od impact <ID>` |
| What does the traceability graph look like? | `od graph [--root <ID>] [--depth N] [--dot]` |
| Is it consistent enough to proceed? | `od gate <gate>` |

`od show` prints the record; `od trace` explains it. Reach for `show` when you
want the artifact, and for `trace` when you want the argument for it.

`od graph` renders Mermaid by default and Graphviz DOT under `--dot`. It is
capped at 200 nodes, so a large workspace will be truncated rather than
unreadable; raise `--limit` if you need the whole graph and accept the output.

### Checking and writing

| Command | What it does |
|---|---|
| `od init [--name N] [--profile lean\|core\|full]` | Writes `opendomain/` and the agent contract. |
| `od gate <gate>` | Runs one gate. Exits non-zero on a blocking finding. |
| `od gate --through <n>` | Runs every gate up to and including number `n` (0–8). |
| `od link <ID> <relation> <TARGET>` | Records a link declared by the dependent artifact. |
| `od link --check` | Verifies declared links resolve. |
| `od new <KIND> [--scope S]` | Prints the next free identifier of a kind. Read-only; re-run before writing. |
| `od advance <ID> <STATUS>` | Moves an artifact along its lifecycle. |
| `od propose <slug> [--summary S]` | Opens a change proposal under `changes/`. |
| `od apply <slug>` | Marks a change as applied. |
| `od archive <slug>` | Moves a completed change under `changes/archive/`. |

Gates are named `context`, `research`, `domain`, `requirements`, `opportunity`,
`specification`, `plan`, `implementation`, `completion`. `od gate <name>` takes
the name; `od gate --through` takes the number (0–8), so `od gate --through 3`
runs every gate up to and including `requirements`. The two are not
interchangeable, and passing a name to `--through` is an error rather than a
silent no-op.

| # | Gate | The question it answers |
|---|---|---|
| 0 | `context` | Do we know what we are building, and what is already true? |
| 1 | `research` | Is the domain understood well enough to derive requirements? |
| 2 | `domain` | Is the domain model coherent, owned and cross-linked? |
| 3 | `requirements` | Is every requirement justified, testable and traced? |
| 4 | `opportunity` | Has anything been challenged, and is the chosen concept earned? |
| 5 | `specification` | Does the specification describe observable behaviour and nothing else? |
| 6 | `plan` | Is there a bounded, ordered plan that can realise the specification? |
| 7 | `implementation` | Did implementation satisfy the acceptance criteria without drift? |
| 8 | `completion` | Is the whole chain coherent, tested and free of critical unknowns? |

Gates are cumulative: reaching gate 8 means 0 through 7 passed too. Run
`od gate --through 8` to check the whole chain, or `od gate <n>` for one stage.

`od reference` lists the **63 rules** these gates apply — 35 blocking, 28
advisory — grouped by gate, and `od reference --json` returns the same list
machine-readably. Read it before you write artifacts rather than after a gate
fails: a rule you learn about by tripping it is a rule that has already cost you
a change.

`od gate --strict` is accepted and changes nothing: a blocking finding always
fails the command and an advisory one never does, regardless of the flag.

`od --json` works on every command, for a machine-readable result instead of a
rendered one. `-C <dir>` points any command at another project.

`od propose`, `od apply` and `od archive` manage a change through the OpenSpec
layout: propose it, implement it, apply it, archive it. An agent working a task
should propose rather than edit registries directly, so that what changed and why
is recorded as a change rather than as an unexplained diff.

Only `init`, `link`, `advance`, `propose`, `apply` and `archive` write. Every
other command only reads, and reads the registries as they are.

## Gates are not a formality

A gate failure is a finding with a stable rule slug, such as
`gate-3/unjustified-requirement`, and a failing gate exits non-zero so it works
as a CI step.

Findings come in two severities. **Blocking** means the stage produced nothing
usable, and `od gate` exits non-zero. **Advisory** means a rule of discipline was
broken that does not make the output unusable; it is reported and recorded, and
it does not fail the command. Neither is negotiable at the call site: an agent
cannot decide a finding is advisory, and `--strict` does not change it.

Report a failure as a failure. Do not describe a blocking finding as advisory
because the surrounding work looks finished, and do not edit an artifact to
silence a finding — an artifact edited to quiet a gate is the laundering the
framework detects, performed by you.

## Recording how a claim got stronger

When an artifact's status moves to `VALIDATED` or `VERIFIED`, record the move:

```toml
promotions = [
  { from = "ASSUMED", to = "VALIDATED", evidence = ["EVD-014"], date = "2026-04-10" }
]
```

Without that record, a change of epistemic state and a change of status are
indistinguishable from each other. A promotion that cites no evidence, cites an
identifier that does not exist, or cites evidence that is not itself `EXPLICIT`
is a blocking finding.

## Before you finish

Run `od link --check` and `od gate <current-gate>`, then report what they say.
When a gate fails, say it fails.
