# Drift

Where the implementation and the specification disagree, `DRIFT-###`. One
artifact per drift.

**Drift is not a failure of discipline to be hidden. It is a fact discovered
and recorded.** The project that files drift honestly can still be wrong; the
project that quietly edits the specification after the code was written has
lost the ability to tell anyone what was agreed.

Required fields: `status`, `type`, `specification`, `implementation`, `decision`.

`type` is one of `OMISSION` `ADDITION` `DIVERGENCE` `REVERSAL`.

`status` is one of `OPEN` `ACCEPTED` `SPEC_UPDATED` `IMPLEMENTATION_UPDATED` `REJECTED`.

<!--
## DRIFT-004 — Optimistic locking implemented without a specification

```toml
status = "OPEN"
type = "ADDITION"
specification = "SPEC-004"
implementation = "A version column on the receipt row, added during TASK-011"
decision = "SPEC_UPDATED"
rationale = "The concurrency guard required an atomic compare-and-set that the specification never described. The behaviour was implemented first and the specification was written afterwards."
detected-by = "TASK-011 review"
detected-on = "2026-04-18"
derived-from = ["SPEC-004", "TASK-011", "TEST-023"]
```

**What the specification says**

`SPEC-004` scenario 3 states the observable outcome: at most one concurrent
receipt is confirmed, and the total accepted never exceeds 20.

**What the implementation does**

Introduces a version column and rejects a stale write with a generic
conflict error.

**Why this is drift**

The observable outcome matches, so the behaviour is arguably right. But the
specification never says the conflict is surfaced as a *distinct, named* error
to the operator, and the implementation does surface it that way. An
undocumented behaviour is a behaviour nobody agreed to.

**Decision taken**

`SPEC_UPDATED`. `SPEC-004` gains an explicit scenario 5 stating that a stale
concurrent write is rejected with a named conflict, distinguishable from a
validation failure, and the error is added to `AC-004-05`.

**Why not `IMPLEMENTATION_UPDATED`**

Because the implementation's behaviour is the better one, and because the error
is genuinely more useful to an operator than a silent failure. Retro-fitting
the code to the original wording would make the code worse in order to make the
paper look consistent, which is the wrong direction of travel.

**Why not `REJECTED`**

The behaviour is defensible. It is undocumented, not wrong.

**Lesson**

The specification was written for a scenario whose difficulty was underestimated.
Write the concurrency scenario before implementing it, not after.

## Rules for this registry

- Drift is filed when it is *discovered*, not when it is fixed. An open drift
  record is a live signal; Gate 7 blocks on open drift.
- Say explicitly which side changed and why.
- A drift with a `decision` but no `rationale` is a drift that will be
  relitigated at the worst possible moment.
