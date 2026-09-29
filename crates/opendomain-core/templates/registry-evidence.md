# Evidence

What was actually found in a source, `EVD-###`. One artifact per finding.

This registry is the load-bearing one. A requirement with no evidence upstream
is an opinion, and Gate 3 refuses to let opinions reach implementation.

Required fields: `type`, `statement`, `derived-from`. Recommended:
`epistemic`, `confidence`, `interpreted-as`, `affects`.

`statement` must be what the source says, in the source's terms. Your reading
of it goes in `interpreted-as`, separately, so the two can be challenged
independently.

<!--
## EVD-014 — Stock movements are recorded per item

```toml
type = "DOMAIN_FACT"
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
derived-from = ["SRC-014", "SRC-021"]
interpreted-as = "Users need to be able to see how a given item's stock changed over time"
affects = ["ENT-001", "PROC-001"]
```

**Statement**

Stock movements are recorded per item, with a timestamp, a direction and a
reference to the operation that caused them.

**Exact context**

Both sources state that a movement history is retained and queryable, and that
the history is the basis for stock reconciliation. Neither source specifies a
retention period or who may read the history.

**Interpretation**

A movement history is a domain fact, not a reporting convenience, because
reconciliation in both sources depends on it.

**Evidence type**

`DOMAIN_FACT`

**Affected concepts**

- `ENT-001` Goods receipt
- `ENT-002` Stock movement

**Affected processes**

- `PROC-001` Receive goods
- `PROC-004` Reconcile stock

**Not claimed**

Retention period, access control and export rights are **not** established by
this evidence. They are open: `UNC-002`.

-->

## Evidence types

`USER_NEED` `DOMAIN_FACT` `BUSINESS_RULE` `INDUSTRY_PRACTICE`
`REGULATORY_REQUIREMENT` `TECHNICAL_CONSTRAINT` `COMPETITOR_CAPABILITY`
`MARKET_SIGNAL` `ASSUMPTION` `HYPOTHESIS` `INFERENCE`

## Rules for this registry

- **Evidence is not a requirement.** The chain is
  `Source → Finding → Interpretation → Candidate Need → Requirement`. Skipping a
  step is how a competitor feature becomes a requirement.
- A `COMPETITOR_CAPABILITY` is evidence about a market, never a requirement
  for this project.
- Two sources that share an origin are one source. Gate 1 blocks when every
  evidence record cites a single source.
- Record what the evidence does *not* claim. Absence is easy to overlook and
  expensive to discover late.
- `epistemic = "INFERRED"` requires the reasoning to be written out in the
  body. An unexplained inference is an assumption wearing a costume.

## Recording a promotion

When an artifact's `status` advances to `VALIDATED` or `VERIFIED`, record the
move in `promotions`. This is the only way the tool can tell a promotion backed
by new evidence from a status typed by hand.

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
promotions = [
  { from = "ASSUMED", to = "VALIDATED", evidence = ["EVD-014"], date = "2026-04-10" }
]
```

`from` and `evidence` and `date` are optional to write and not optional to
mean. Gate 7 blocks when a promotion from `ASSUMED` or `INFERRED` into
`VALIDATED` or `VERIFIED` cites no evidence, cites evidence that is missing, or
cites evidence that is not itself `EXPLICIT`. It also blocks on the current
state: an artifact that is `ASSUMED` and `VALIDATED` at the same time is
laundered whether or not it recorded anything.

A promotion citing explicit evidence is how an assumption stops being one. The
record is the difference between having changed your mind and having relabelled
the artifact.
