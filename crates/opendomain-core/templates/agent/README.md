# Agent integration

`od init` writes these files so that a coding agent working in this project
discovers that the workspace exists and what the discipline is. Without them,
`od init` produces a set of registries that no agent knows to look for, and the
epistemic discipline is silently absent from the conversation.

Two kinds of file, deliberately:

- **`AGENTS.md` at the project root** is the portable one. It is read by several
  coding agents without configuration, and it is what makes the framework
  discoverable by tools that know nothing about `od`.
- **`.opencode/`** wires the same discipline into this specific agent: slash
  commands, two subagents, and a skill. Other tools get `AGENTS.md`; this one
  gets the full set.

Both are generated from templates compiled into the `od` binary, so they cannot
drift from the tool's actual behaviour the way hand-maintained prose does. If a
command named here does not exist, that is a bug in the template, not a
discovered limitation of the tool.

## What `od init` does and does not overwrite

Existing files are never overwritten without `--force`. If you already have an
`AGENTS.md`, `od init` reports it as skipped and leaves it alone. That is
deliberate: your instructions are yours, and a scaffolder that overwrites them
would be a scaffolder you run once.

## Keeping the two copies honest

This repository is itself a workspace with an OpenDomain workspace, so it carries
its own copy of these files under `.opencode/` and `AGENTS.md`. The copy under
`crates/opendomain-core/templates/agent/` is the source the binary ships. A test
in `scaffold.rs` asserts the two agree, so editing one without the other fails
the build rather than producing two subtly different agent contracts.
