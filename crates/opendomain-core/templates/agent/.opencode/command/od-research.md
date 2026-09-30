---
description: Run Gate 1 research. Fans out parallel axes to domain-researcher, merges, checks corroboration.
agent: build
---

# Doing the documentary research

Before writing a single requirement, you need evidence. This command runs the Gate 1 research: several directions explored in parallel, then merged and checked. The goal isn't a pile of notes — it's saturation, several independent sources corroborating each other.

## How it goes

1. I start with `od status` and `od gate research`, summarized in a few lines, and I re-read `project.md` for keywords: the domain's nouns, the jurisdiction, the tech stack.
2. From those I derive 3 to 6 directions — regulation or standards, official vendor documentation, industry practice, competitors (as clues only, never as justification), textbooks and academic references — and I hand each direction in parallel to the `domain-researcher` subagent, with one sharp question and its keyword cluster. Each comes back with sources and findings, reading dates and exact contexts attached. No citation is invented; dead ends go to retrieval failures.
3. At merge time I verify: every finding points at its source, three findings on a single source don't count as corroboration, disagreements become conflicts to settle later, holes become tracked uncertainties, and open questions get either answered or tracked.
4. I finish with `od link --check` and `od gate research`, answering summary-first: the **Verdict**, the main blockers with identifiers (for example `gate-1/orphan-evidence`), and the saturation state direction by direction — then the detail in an appendix.

And most importantly: research stops at findings. Turning them into requirements is `/od-requirements`' job — another step, another moment.
