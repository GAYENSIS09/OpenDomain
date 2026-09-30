---
description: Use when working in a project that has an opendomain/ workspace. The front door: shared discipline plus a map to the four focused skills.
---

# Working under OpenDomain

Welcome. This project keeps what it knows about its domain in `opendomain/` — small linked records written in Markdown — and the `od` command checks that the links hold. Think of `od` as the project's memory: whenever you wonder whether something here is known, inferred, or merely assumed, ask `od` instead of guessing.

`od` reads that knowledge. It never rewrites your records, except through the few commands that say so explicitly (`init`, `link`, `advance`, `propose`, `apply`, `archive`).

## Where to look, depending on your task

This file states the one discipline everyone shares. The operative know-how lives in four focused skills — load the one your task needs:

| When you need to... | Load this skill |
|---|---|
| Choose an `epistemic` value, record a promotion, check for laundering | `.opencode/skills/opendomain-epistemics/SKILL.md` |
| Write or repair any artifact — format, ids, relations | `.opencode/skills/opendomain-artifacts/SKILL.md` |
| Gather external evidence — parallel search, triangulation | `.opencode/skills/opendomain-research/SKILL.md` |
| Read a gate, status, or audit output — and fix what it reports | `.opencode/skills/opendomain-gates/SKILL.md` |

## The one discipline everyone shares

Every record that makes a knowledge claim carries an `epistemic` label saying how the claim is known: `EXPLICIT` (a source you read says so) · `TACIT` (lived practice, unwritten) · `HYBRID` (a bit of both) · `INFERRED` (you derived it — show the chain) · `ASSUMED` (nobody knows yet — say so) · `UNKNOWN` (a known gap — track it, don't fill it) · `CONFLICTING` (sources disagree — record both, resolve nothing).

Two things are never acceptable. Writing `EXPLICIT` for something you inferred is called epistemic laundering — presenting an invented domain rule as established fact — and it is the exact failure this whole framework exists to catch. A confident rule built on general knowledge, with no source behind it, is `ASSUMED` with the reasoning written down. And never invent a `SRC-` or `EVD-` identifier to make a claim look sourced: a fabricated citation is worse than an honest gap, because it flows downstream through every gate and becomes indistinguishable from a real one.

## Which `od` command answers your question

| Your question | The command |
|---|---|
| Where does the project stand? | `od status` |
| Why does this artifact exist? | `od trace <ID>` |
| What implements and verifies it? | `od trace <ID> --forward` |
| Who said so, nearest source first? | `od trace <ID> --provenance` |
| What breaks if this changes? | `od impact <ID>` |
| What does the graph look like? | `od graph [--root <ID>] [--depth N] [--dot]` |
| Is it consistent enough to proceed? | `od gate <gate>` |
| Is anything dangling or unjustified? | `od audit`, `od link --check` |
| What do the identifiers mean? | `od reference` |
| Record a link / next free id / move along the lifecycle | `od link`, `od new`, `od advance` |

The gates are named `context`, `research`, `domain`, `requirements`, `opportunity`, `specification`, `plan`, `implementation`, `completion`. If a command you expected does not exist, `od --help` is authoritative — don't assume a convenient command into existence.

## When you report

A failing gate carries a stable identifier, like `gate-3/unjustified-requirement`. Quote it as-is. Never describe a blocking finding as advisory because the surrounding work looks finished, and never edit a record just to silence a finding — that edit is the laundering, performed by you. When a gate fails, say it fails, then fix the cause.
