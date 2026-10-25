---
description: Use when the work must be executed by agents — building the agent system from a task, then driving the run under gates. The execution doctrine of opendomain/harness/.
---

# Driving the harness (execution doctrine)

You drive, you do not code around the harness: the `opendomain` CLI implements
this doctrine itself. Your output is a verified agent system, then an archived
run — never an unverified claim.

The loop this doctrine drives is the harness loop, mapped onto the phases
below: plan → identify needs → build each agent and subagent → decide models
per task → tool up (install, verify) → execute by delegation → improve,
amend, iterate.

## The substrate — no new runtime

Nothing below adds a runtime: every capability assembles from OpenCode
primitives, which all reload from disk on save (`/reload` forces a full
reload). The reference frame, translated once and only once:

| What the factory needs | The primitive that provides it |
|---|---|
| Agents + orchestration | `.opencode/agent/<id>.md` files, `mode: primary\|subagent`; a parent admits children through its `subagent` permission — an allowlist of child ids, so a recipe without the entry delegates to nobody (delegation depth ≤ 2, enforced by omission) |
| Per-task model choice | the agent frontmatter `model: provider/model#variant` — the pin the plan report cites, per command override allowed |
| Install & swap tools/skills | skills `.opencode/skills/<id>/SKILL.md` (+ scripts), commands `.opencode/command/<id>.md`, MCP servers `mcp.servers` (local `command[]` / remote url) |
| Sandbox & session layer | ordered permissions `{action, resource, effect}` — the LAST matching rule wins; the shell scanner splits compound commands, so one allow per simple command is the safe recipe (deny first, allows after) |
| Self-improvement | the flow itself rewrites `build`'s node-agents and the registries via `propose`/`apply`/`repatch`; definitions picked up on save — no restart mid-run |

The table is the frame, not the authority: OpenCode V2 evolves, and the
installed version decides. When opencode itself misbehaves — a permission
that did not bite, an MCP server that will not start, a definition that did
not reload — go in this order: the installed binary first (`opencode
--help`, `--version`, `opencode mcp list`, `opencode auth list`), then the
doc index at <https://opencode.ai/v2/llms.txt> (agents, skills, commands,
permissions, MCP servers, config, troubleshooting — one page per question),
then the `opencode` skill when the project or user installs it. Never retype
what the help already answers; when the table and the installed opencode
disagree, the binary wins and a lesson records which line drifted.

## Phase 1 — Construction (before any business code)

1. **Task `T`** — it lives in the registries. `opendomain plan TASK-012`
   resolves the task record as `T` (title, objective/constraints/criteria
   prose, `derived-from SPEC/RULE` links) and compiles the DAG from it. For
   work outside the registries, `opendomain spec new <slug>` writes
   `opendomain/harness/specs/<slug>/spec.md` — objective, constraints
   (`interdiction de X` lines), acceptance criteria.
2. **Grounding**: `opendomain ground <slug>` collects repo, manifests and
   toolchain into `grounding/<slug>/repo-scan.md` + fingerprint. Manual notes
   go in the same folder.
3. **DAG**: `opendomain plan <slug>` compiles via opencode (the real
   decomposition, verified by the loop); `opendomain plan <slug> --offline`
   asks the deterministic fallback explicitly (CI, no network). Incomplete
   agents are refused at admission.
4. **Execution kit**: `opendomain build <slug>` equips every agent in
   `agents/<slug>/` — `AGENTS.md` (the shared contract: loop, tools, context
   & memory, sandbox, hooks, skills), `<id>.md` (σ_i: role, tool whitelist,
   I/O contract, criteria), `<id>.kit.json` (machine manifest) —
   **resolves every node's kit** against the component registry (`kit`:
   skills, commands, protocols — a capacity without a provider is a typed
   construction fault; `--fetch` admits explicit web-provisioning with
   provenance), and **defines the node-agents** in `.opencode/agent/`: one
   file per node, deny-universal first, one allow per whitelisted tool, the
   output contract and the resolved kit in the body. Visible and reviewable
   BEFORE the run; the run invokes them, it never rewrites them. Targeted
   fixes only; `opendomain validate` replays the static graph checks.

## Phase 2 — Execution under gates

5. `opendomain run <slug>` — the executor is opencode, the real, visible,
   archived path (mock stays the offline self-check via `opendomain bench`);
   agents run in topological order; the coordinator applies the gate
   `y_i ∈ C_i` (schema, patterns, test commands executed in the sandbox)
   **before** any propagation; attempts replay with typed feedback, bounded
   by `[gates] max_attempts` in `opendomain.toml`.
6. The run archives append-only under `runs/run-NNN-<slug>/`:
   `trace.jsonl` (Start/Passed/Retry/Failed), `context/` (the `y_i`),
   `gates.log`, the frozen graph. Nothing is lost; everything replays.
7. **Terminal failure → attribution**: `opendomain attribute <run>` types it —
   `criteria_failure` / `spec_mismatch` → **local**: fix, then
   `opendomain run <slug> --resume <run>`; `grounding_failure` →
   **upstream**: re-ground, re-plan that node alone; `contract_violation` →
   **structural**: re-plan the sub-graph (`opendomain repatch <run>`), a
   partial return to phase 1.
8. **A posteriori control**: `opendomain verify <run>` replays every gate on
   the archived outputs — at any time, without an LLM.

## Prohibitions

- propagate unverified output (no exception — `gates.strict = true`);
- invent a tool outside an agent's whitelist (least privilege is projected
  into the executor: deny first, one allow per tool);
- run a command outside the sandbox while `[sandbox] enabled = true`;
- produce final code with no **verifier** node in the graph;
- blind retry without typed feedback (each attempt is directed by the gate);
- re-plan the whole system for a local error — recovery cost follows the
  locality of the error.
