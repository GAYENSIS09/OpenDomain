---
description: Drive the harness — build the agent system from a TASK record, or recover a run (verify, attribute, resume, repatch). Routes to opendomain-harnais.
agent: build
---

# Driving the harness

The verified-execution flow has one door: "$ARGUMENTS" tells me which. With no argument, I show the last run's verdict (`opendomain provenance` + the newest `runs/run-*`) and ask.

## A `TASK-` — build and execute

I hand the task to the `opendomain-harnais` subagent (the harness architect; doctrine: the `opendomain-harness` skill): `opendomain plan TASK-012` → `ground` → `build` (the node-agents are defined here, reviewable) → `run` — the gates `y_i ∈ C_i` fire, everything archives. I relay its report résumé-first: the **Verdict** (run PASS + directory, or the terminal failure typed), the nodes validated, the blockers.

## A `run-` — recover, never re-run blind

1. `opendomain verify <run>` first: the gates replay offline — a run that verifies now needs no recovery.
2. `opendomain attribute <run>` types the terminal failure: **local** → fix the node, `opendomain run <spec> --resume <run>` (validated nodes stay frozen); **upstream** → re-ground the step, re-plan that node alone; **structural** → `opendomain repatch <run>` — a partial return to the plan, the revised node re-admitted like any plan graph.
3. The lesson lands in `memory/registre.md` (typed, dated); the lessons that matter become the doctrine's next fix — I name the one this failure earned.

## Always

The **Verdict** first, then the recovered run's path, then what the caller should write where — a TEST/DRIFT record citing the run (`run = "runs/run-NNN-…"`; `opendomain provenance` verifies the citation) — then the appendix: `verify.log`, the attribution record, `opendomain link --check`. I never mark anything `IMPLEMENTED` or `VERIFIED` myself: that's `opendomain advance`, owned by the caller.
