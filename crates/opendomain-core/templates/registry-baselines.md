# Baselines

What "good enough" means, `BAS-###`. One artifact per baseline.

A baseline is a number that a requirement can be measured against. Without one,
every quality discussion is a matter of taste.

Required fields: `status`, `measure`, `target`.

<!--
## BAS-001 — Receipt throughput

```toml
status = "VALIDATED"
measure = "Goods receipt lines confirmed per operator-hour, at normal staffing"
target = "≥ 45 lines per operator-hour"
baseline = "11 lines per operator-hour, measured 2026-02-18 in the manual process"
derived-from = ["EVD-021", "STK-001"]
epistemic = "EXPLICIT"
```

**Measure**

Goods receipt lines confirmed per operator-hour under normal staffing.

**Target**

At least 45 lines per operator-hour.

**Current baseline**

11 lines per operator-hour, measured on 2026-02-18 by observing three operators
for one morning of receiving.

**Why this target**

`EVD-021` records that a scan takes roughly two seconds against a manual entry
of roughly twenty. 45 is deliberately below the arithmetic maximum, because the
work is not only entry: counting, checking damage and resolving discrepancies
consume the rest of the operator's time.

**Used by**

- `NFR-002` sustained receipt throughput
- `HYP-005` as the reason the 2% re-key target is acceptable
- `EXP-002` as a secondary observation, not a success criterion

-->

## Rules for this registry

- A baseline without a measured starting point is a target wearing a baseline's
  name.
- Baselines are measured before they are improved against. Measuring only after
  the change is how numbers get invented.
