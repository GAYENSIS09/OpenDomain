---
description: Use when gathering external evidence. Parallel keyword fan-out, rich SRC/EVD records, triangulation, saturation, conflicts and gaps.
---

# From keywords to evidence you can build on

Good research ends in saturation and corroboration — not in a pile of notes from a single source. The path has three stages: fan out widely, record richly, triangulate and stop.

## 1. Fan out from keywords, in parallel

Start from 5–12 keywords drawn from `project.md`, the `[project]` table, and the question at hand — domain nouns, the jurisdiction, the stack, any regulation glimpsed in code or docs. Turn them into 3–6 search directions and work each one in parallel: the regulation or standard, the vendor's official documentation, industry practice, competitors (as clues only — a competitor page is evidence, never a justification), and textbooks or academic references. Give each thread one sharp question plus its keyword cluster, and keep threads independent: they never share citations.

Trust has an order: a regulation or standard outranks official docs, which outrank textbooks, which outrank heard practice, which outranks competitor marketing. Note web sources with their URL and retrieval date, repo files with path and revision — and never cite a file you haven't opened yourself.

## 2. Record each source richly

Every accepted source becomes a `SRC-` record (`opendomain new SRC`): title, type, address, today's date as `retrieved`, and the exact sections you used. Then each finding becomes its own `EVD-` record (`opendomain new EVD`): a `type` from the vocabulary (`USER_NEED`, `DOMAIN_FACT`, `BUSINESS_RULE`, `INDUSTRY_PRACTICE`, `REGULATORY_REQUIREMENT`, `TECHNICAL_CONSTRAINT`, `COMPETITOR_CAPABILITY`, `MARKET_SIGNAL`, `ASSUMPTION`, `HYPOTHESIS`, `INFERENCE`), a `Statement` phrased in the source's own terms, an `Exact context` (section, page, revision), a `Not claimed` section saying what the source does *not* establish, a `derived-from` link to the source, and an honest epistemic value with its confidence.

## 3. Triangulate, then stop

Corroboration means two or more *independent* sources saying the same thing. Three records resting on one source is restatement, not corroboration — the gates block it as `gate-1/single-source`. When sources disagree, open a conflict record with both positions and leave the resolution pending; resolve nothing yourself. A gap becomes a tracked uncertainty. An unreadable source goes under retrieval failures and is never cited.

Stop a direction when new sources only restate what you already have — that's saturation. Report each direction as saturated or still thin, and name the exact source that would close the thin ones. Research ends at findings; turning them into requirements is a later step, with its own discipline.
