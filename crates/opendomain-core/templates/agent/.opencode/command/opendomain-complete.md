---
description: Release decision for Gate 8. Calls opendomain-audit for the diagnostic, then decides. Never re-renders the chain itself.
agent: build
---

# May we ship?

The final question, and the most serious one: is the whole chain — knowledge, requirement, spec, plan, code, test — coherent and free of critical unknowns? This command makes the call: we ship, or we don't. The candidate: "$ARGUMENTS".

## How it goes

1. I recompute nothing myself: I call `/opendomain-audit` for the full diagnostic and start from its summary-first report — the highest gate reached, the first one failing, dangling links, unjustified requirements, conflicts and uncertainties, coverage (remembering that having an origin means traced, having a passing test means verified — two different things).
2. Then I decide. All green: I propose the evidence-complete vertical increment and archive the finished changes (`opendomain archive <slug>`). Red: we stop here — I list the blockers with their identifiers, and no release gets declared.

And when the workspace is empty? The gate says so politely (`nothing-completed`): an empty project isn't a finished project, it's a project not started. The **Verdict** always opens my answer; detail follows in an appendix.
