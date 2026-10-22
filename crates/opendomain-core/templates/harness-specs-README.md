# `specs/` — one task, one folder

Each folder `NNN-slug/spec.md` holds the task description `T`.
The actual agent decomposition lives in `graph/graph.json`: the spec states
the *what* and the criteria, the graph states the *how*.

Create a new spec: `opendomain spec new 001-my-spec`.

Section headings inside `spec.md` (`## Objectif`, `## Contraintes`,
`## Critères d'acceptance`, and `## Besoins` for component needs) are the
parser contract of `plan` and `resolve` — write under them, don't rename them.
