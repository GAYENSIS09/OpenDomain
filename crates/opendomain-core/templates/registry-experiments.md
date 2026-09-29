# Experiments

Time-boxed tests of a hypothesis, `EXP-###`. One artifact per experiment.

An experiment is the cheapest way to be wrong. It is not a proof, and it is not
a spike: a spike explores, an experiment decides.

Required fields: `status`, `hypothesis`, `method`, `success-criteria`.
Recommended: `duration`, `cost`, `outcome`, `learned`.

<!--
## EXP-002 — Time-boxed trial of scanning-first capture

```toml
status = "IMPLEMENTING"
hypothesis = "HYP-005"
method = "Two warehouse operators receive goods using the scan-and-confirm flow for one week; the existing form stays available and the chosen path is recorded per receipt"
success-criteria = "Manual re-key rate under 2% overall, under 10% for any single supplier, over at least 200 receipts"
duration = "1 week"
cost = "low"
derived-from = ["HYP-005", "CPT-003"]
```

**What is being tested**

Whether `HYP-005` holds for real operators on real goods, including the
suppliers whose labels may not encode the needed fields.

**Method**

- Both operators use the scan-first flow for every receipt
- The manual form remains reachable, and which path was used is recorded per
  receipt, so a fallback is data rather than a guess
- Daily check of the re-key rate against the success criteria

**Why this method**

The hypothesis is about operator behaviour under time pressure, so the trial
has to involve operators under time pressure. A synthetic benchmark would test
the scanner, not the hypothesis.

**Confounders to watch**

- The two operators are the most experienced staff available; the result may
  not transfer
- One week may not surface supplier-specific problems that appear seasonally
- Both operators know they are being observed

**Outcome**

*To be recorded. Do not update this section while the experiment is running.*

**Learned**

*To be recorded.*

-->

## Rules for this registry

- State the success criteria before running. A result that is only interesting
  is not a result.
- Record what would have made you stop early.
- An experiment that cannot change a decision is a demo. Do not run it.
