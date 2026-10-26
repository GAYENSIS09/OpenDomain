---
description: Use when completing or refreshing project context (Phase 0) - project.md, the [project] table, the first DOM/STK records. The context-completer doctrine in loadable form; delegate to the role when it can be spawned, apply the doctrine here when it cannot.
---

# Completing project context

The role `context-completer` (`.opencode/agent/context-completer.md`) owns Phase 0. This skill is the same doctrine in loadable form: when the role can be spawned, delegate to it — the `subagent` tool with `agent: "context-completer"`, or `/opendomain-project`, which does exactly that. When no subagent can be spawned, do the work yourself under these rules, never under invented ones.

## One name, exactly

The skill's loadable name is `context-completer` — the role's name, without a prefix. There is no `opendomain-context-completer` skill and never was: a failed load under that name is a wrong name, not a missing file. The emitted skills are listed in the index `.opencode/skills/opendomain/SKILL.md`; read it instead of trying variants that sound plausible.

## Phase 1 — read the repository first

Read what actually exists, citing `path:line` for every fact — identity, languages and frameworks, services and entry points, the data model, the documentation, the conventions visible in git history, whatever context is already recorded. Stay out of target, node_modules and build directories. Note secret paths and key names, never values.

## Phase 2 — research the outside through the researcher

Pull 5–12 keywords — domain nouns, the suspected jurisdiction, the stack, any regulation glimpsed in code — and delegate each direction to `domain-researcher`; when that role cannot be spawned, search it yourself one direction at a time, in parallel. Regulation, official documentation, industry practice, competitors as clues. Fold findings into your work when they return, and never record on the researcher's behalf.

## Phase 3 — write the context honestly

Fill every section of `project.md`, copy the essence into short `[project]` strings in `opendomain.toml`, and create `DOM-001`/`STK-001` only when the registry holds none yet — anything already written stays untouched. What you read in code is `INFERRED`; what comes from speech or general knowledge is `ASSUMED`; `EXPLICIT` requires a source you read, linked. Holes become tracked uncertainties; working hypotheses get dated; disagreements become unsettled conflicts.

## Phase 4 — ask what no reading can answer

The real jurisdiction, the actual users, the budget, the deadlines, the known problems: the repository and the web cannot answer those. These **Human-only gaps** go back grouped, with your best guesses as defaults. With no channel to ask through, they are written as `UNKNOWN` with their uncertainties, listed neatly in the report.

## What you hand back

Summary-first, like every other phase: the `opendomain gate context` verdict in one line, then up to three blockers with their identifiers, then what was filled in (sections, keys, records created or kept), then the sources line by line (identifier, origin, epistemic value), then the Human-only gaps with what would close each one. `opendomain link --check` and the full gate ratify it in an appendix.

The role body keeps the full contract. If this file and the role ever disagree, correct both in the same change — a doctrine that diverges from itself is two doctrines, and one of them is wrong.
