# Hypotheses

Falsifiable claims, `HYP-###`. One artifact per hypothesis.

A hypothesis is not a belief. It is a sentence with a metric and a condition
that would prove it wrong. If you cannot write the failing condition, you have
an opinion, and the opinion belongs in a decision record instead.

Required fields: `status`, `statement`, `metric`, `fails-when`. Gate 4 blocks
any hypothesis missing the last two.

<!--
## HYP-005 — Scanning removes most receipt re-keying

```toml
status = "IMPLEMENTING"
statement = "Operators using scanning-first capture will re-key fewer than 2% of received line items, against a current 18%"
metric = "share of received line items that must be re-keyed manually, measured over one week of production-like operation"
fails-when = "The measured re-key rate is above 8%, or above 10% for any single supplier, in which case the fallback path dominates and CPT-004 should be selected instead"
derived-from = ["CPT-003", "EVD-021", "OPP-002"]
tested-by = ["EXP-002"]
confidence = "MEDIUM"
```

**Statement**

Replacing manual entry with scan-and-confirm will reduce manual re-keying of
received line items to under 2%, from the 18% measured in `EVD-021`.

**Metric**

Share of received line items requiring manual entry, measured over one week
with at least 200 receipts.

**Failing condition**

- The rate exceeds 8% overall, **or**
- It exceeds 10% for any single supplier

The second clause exists because a good average can hide one supplier whose
goods cannot be scanned, and one such supplier is enough to break the workflow.

**How it is tested**

`EXP-002`, a time-boxed trial with two operators.

**What the result would mean**

- **Pass** — `CPT-003` proceeds to specification
- **Fail** — `CPT-003` is rejected in favour of `CPT-004`
- **Partial** — the hypothesis is split: scanning for encoded suppliers, a
  streamlined manual form for the rest

-->

## Rules for this registry

- A hypothesis without a `fails-when` cannot be tested, so it is not a
  hypothesis. Gate 4 blocks it.
- Choose the failing condition **before** the experiment. A threshold chosen
  afterwards is not a threshold.
- A refuted hypothesis is a good outcome. Record it.
