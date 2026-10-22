# `opendomain/harness/` — the harness contract

This folder **is** your agent system: every subfolder is an artifact the
harness reads or writes. Nothing is decorative here; everything is checkable.

| path             | role                                                              |
|------------------|-------------------------------------------------------------------|
| `specs/`         | task descriptions `T` (one folder per job, `spec.md`)             |
| `grounding/`     | external knowledge collected **at design time**                   |
| `graph/graph.json` | the DAG of agents `σ_i` with I/O contracts **and their criteria `C_i`** — the single source of truth for the gates |
| `agents/`        | the *prompt* version of each agent, executor-agnostic             |
| `contracts/`     | JSON schemas shared between several agents                        |
| `runs/`          | execution traces: events, context, gates (append-only)            |
| `memory/`        | lessons: typed failures → recovery hints                          |

Canonical loop: **Generate → Verify → Attribute → Refine**.
Full execution doctrine (canonical loop, six pillars, M1–M7 milestones):
https://github.com/GAYENSIS09/OpenDomain/blob/main/docs-site/docs/harness.html
