# Working in a project with an OpenDomain workspace

This project records what it knows about its domain in `opendomain/`, as linked
Markdown artifacts. The `opendomain` command reads that knowledge and checks the links
between artifacts. It is the authority on what this project has established: if
you want to know whether something here is known, inferred or assumed, ask `opendomain`.

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

Run `opendomain reference` for the identifier prefixes, the relations, and the vocabulary
of statuses and epistemic values. Do not guess a prefix.

## The commands

`opendomain --help` is authoritative. If a command you expected is missing, that is a
fact about the tool, not an invitation to assume it exists.

### Reading

| Question | Command |
|---|---|
| Where does the project stand, and what is blocking? | `opendomain status` |
| Is anything dangling, unjustified or unresolved? | `opendomain audit` |
| Are all declared links resolvable? | `opendomain link --check` |
| What do the identifiers and rules mean? | `opendomain reference` (63 rules, 9 gates, 37 kinds) |
| Give me one artifact in full | `opendomain show <ID>` |
| Why does this artifact exist? | `opendomain trace <ID>` |
| What implements and verifies it? | `opendomain trace <ID> --forward` |
| Who said so, nearest source first? | `opendomain trace <ID> --provenance` |
| What breaks if this changes? | `opendomain impact <ID>` |
| What does the traceability graph look like? | `opendomain graph [--root <ID>] [--depth N] [--dot]` |
| Give the whole knowledge base to a human? | `opendomain export [--format pdf\|html] [--graph mermaid\|dot\|none]` |
| Is it consistent enough to proceed? | `opendomain gate <gate>` |

`opendomain show` prints the record; `opendomain trace` explains it. Reach for `show` when you
want the artifact, and for `trace` when you want the argument for it.

`opendomain graph` renders Mermaid by default and Graphviz DOT under `--dot`. It is
capped at 200 nodes, so a large workspace will be truncated rather than
unreadable; raise `--limit` if you need the whole graph and accept the output.

`opendomain export` mirrors the whole `opendomain/` tree as one paginated document -
a status cover, the traceability graph (Mermaid by default, Graphviz DOT under
`--graph dot`), then every file in gate order, records unfolded as readable
tables. The PDF is printed headlessly by the machine's own browser (Edge or
Chrome are auto-detected; `--browser` or `OPENDOMAIN_BROWSER` pick another);
without one the styled HTML stays instead, and the command says so. The
printout lists the same numbers `status` does; a PDF is a printout, so never
cite it against a registry changed since - rerun the export or read the
registry.

### Checking and writing

| Command | What it does |
|---|---|
| `opendomain init [--name N] [--profile lean\|core\|full]` | Writes `opendomain/` and the agent contract; each generated tree (opendomain/, .opencode/) carries its own `.gitignore` — regenerable like the rest, delete it to version that tree (the registries are the F14/L9 evidence). |
| `opendomain gate <gate>` | Runs one gate. Exits non-zero on a blocking finding. |
| `opendomain gate --through <n>` | Runs every gate up to and including number `n` (0–8). |
| `opendomain link <ID> <relation> <TARGET>` | Records a link declared by the dependent artifact. |
| `opendomain link --check` | Verifies declared links resolve. |
| `opendomain new <KIND> [--scope S]` | Prints the next free identifier of a kind. Read-only; re-run before writing. |
| `opendomain advance <ID> <STATUS>` | Moves an artifact along its lifecycle. |
| `opendomain propose <slug> [--summary S]` | Opens a change proposal under `changes/`. |
| `opendomain apply <slug>` | Marks a change as applied. |
| `opendomain archive <slug>` | Moves a completed change under `changes/archive/`. |

### Verification (Sections 9/10)

| Command | What it does |
|---|---|
| `opendomain rigour` | Prints the Section 9.7 rigour table; audits which requirement records declare a rigour today. |
| `opendomain provision [--dry-run]` | Stack detection (repository-only), the ordered selection rules, then generated `.feature` scaffolds with `@AC` tags and provenance — or a refusal recorded as a DEC artifact. Nothing is installed. |
| `opendomain bind` | Audit of binding provenance (GENERATED/ANNOTATED/ATTRIBUTED/INFERRED) and verification depth (executed/discovered/declared). Findings, not gate rules. |
| `opendomain provenance` | Verifies each record's `run = "…"` citation against the archived run it names — and lists the archives no record claims (F14/L9 bridge). |

Gates are named `context`, `research`, `domain`, `requirements`, `opportunity`,
`specification`, `plan`, `implementation`, `completion`. `opendomain gate <name>` takes
the name; `opendomain gate --through` takes the number (0–8), so `opendomain gate --through 3`
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
`opendomain gate --through 8` to check the whole chain, or `opendomain gate <n>` for one stage.

These gates guard what the project **claims** in its registries: a claim whose
chain is broken is a gate finding, not an opinion. Keep the same discipline in
whatever executes the work — no execution may rewrite a claim without the
evidence its gate requires, and a verification the party it verifies controls
is not a verification: never re-label `ASSUMED` as `EXPLICIT` without
intervening evidence. A `TEST` or `DRIFT` record stores its evidence in the
registries themselves, cited by identifier.

`opendomain reference` lists the **63 rules** these gates apply — 35 blocking, 28
advisory — grouped by gate, and `opendomain reference --json` returns the same list
machine-readably. Read it before you write artifacts rather than after a gate
fails: a rule you learn about by tripping it is a rule that has already cost you
a change.

`opendomain gate --strict` is accepted and changes nothing: a blocking finding always
fails the command and an advisory one never does, regardless of the flag.

`opendomain --json` works on every command, for a machine-readable result instead of a
rendered one. `-C <dir>` points any command at another project.

`opendomain propose`, `opendomain apply` and `opendomain archive` manage a change through the OpenSpec
layout: propose it, implement it, apply it, archive it. An agent working a task
should propose rather than edit registries directly, so that what changed and why
is recorded as a change rather than as an unexplained diff.

Only `init`, `link`, `advance`, `propose`, `apply`, `archive` and `export` write -
and `export` writes its own output file only. Every other command only reads, and
reads the registries as they are.

### When the work must be executed by agents

The door is one: `/opendomain-harness` (opencode) routes to the
`opendomain-harness` agent — the execution architect. A `TASK-###` record
becomes a planned, grounded and gated agent system; a finished or failed run
is recovered (`verify`, `attribute`, `resume`, `repatch`), never re-run
blind. Every implementation executed by agents goes through this verified
harness; evidence is archived under `opendomain/harness/runs/` and cited
`run = "…"`, then checked by `opendomain provenance`. There is no
free-standing implementer to call.

Roles and skills are different objects, and their names are not interchangeable.
The three roles (`context-completer`, `domain-researcher`, `opendomain-harness`)
live in `.opencode/agent/` and are invoked with the subagent tool
(`agent: "<role name>"`) or through the slash command that routes to them;
they are never loaded through the skill tool. The skills live in
`.opencode/skills/<name>/SKILL.md` and are loaded by exact directory name —
including `context-completer`, the Phase 0 doctrine of the interviewer role.
No skill named `opendomain-context-completer` exists, and no other name that
merely sounds plausible does either: when a skill fails to load, the reason is
a wrong name, and the correction is to check the index
(`.opencode/skills/opendomain/SKILL.md`) rather than to try variants. The same
applies to commands and agent identifiers: aliases are not inferred, they are
read where they are declared.

## Gates are not a formality

A gate failure is a finding with a stable rule slug, such as
`gate-3/unjustified-requirement`, and a failing gate exits non-zero so it works
as a CI step.

Findings come in two severities. **Blocking** means the stage produced nothing
usable, and `opendomain gate` exits non-zero. **Advisory** means a rule of discipline was
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

Run `opendomain link --check` and `opendomain gate <current-gate>`, then report what they say.
When a gate fails, say it fails.
