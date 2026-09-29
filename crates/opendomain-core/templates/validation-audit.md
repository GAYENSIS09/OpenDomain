# Audit

The record of what was actually promised and delivered.

Run `od audit` to regenerate the machine-checkable parts. This file holds the
narrative that a machine cannot infer: what the audit was checking, who asked,
and what it concluded.

## Audit questions

<!--
### Pre-release audit of the receiving capability

```toml
type = "PRE_RELEASE"
date = "2026-04-20"
auditor = "STK-004"
scope = ["DOM-002 Inventory", "SPEC-004", "PLAN-002"]
conclusion = "PASS"
gates-passed = ["context", "research", "domain", "requirements", "opportunity", "specification", "plan", "implementation", "completion"]
```

**Questions asked**

1. Can every confirmed receipt be explained by an acceptance criterion?
2. Is any requirement implemented without a verified test?
3. Are there open conflicts or assumptions that reached the specification
   stage?
4. Is the domain model still what the team thinks it is?

**Findings**

- Every criterion in `SPEC-004` traces to a `VERIFIED` test.
- `DRIFT-004` was open at the time of the audit. It was closed as
  `SPEC_UPDATED` before the release, and the new criterion was added and
  verified.
- `ASM-004` (single region) is still `OPEN` and did not affect any implemented
  behaviour; it is recorded as a release risk.
- `UNC-002` (movement retention) is still `OPEN`. `DEC-004` accepts indefinite
  retention for the first release.

**Conclusion**

`PASS`, with two accepted unknowns recorded as risks. Neither affects the
behaviour under review.

-->

## What an audit is for

An audit answers one question: **can you show that what was built is what was
asked for?** A trail that cannot answer it is not a documentation problem; it
is an accountability problem, and it surfaces at the worst possible time.

## Rules for this registry

- Record the question, not just the verdict. An audit whose scope is not stated
  cannot be re-run.
- An audit that only ever concludes `PASS` is not an audit.
- Accepted unknowns belong here, in writing. The alternative is discovering them
  during the incident.
