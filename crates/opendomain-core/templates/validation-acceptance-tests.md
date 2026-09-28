# Acceptance tests

Executable verification, `TEST-###`. One artifact per test.

A test is the executable form of an acceptance criterion. If a criterion cannot
be written as a test, the criterion is not finished.

Required fields: `status`, `verifies`, `level`, `assertions`. Recommended:
`automation`, `environment`, `tolerance`.

`level` is one of `UNIT` `INTEGRATION` `CONTRACT` `E2E` `LOAD` `SECURITY`.

`status` uses the normal lifecycle. **`VERIFIED` means the test has been run
and passed against the current implementation** — not that it was written.
Gate 8 checks for this.

<!--
## TEST-023 — Concurrent receipts cannot exceed the outstanding quantity

```toml
status = "DRAFT"
verifies = ["AC-004-03", "SPEC-004"]
level = "INTEGRATION"
automation = "cargo test --test concurrency_receipts"
environment = "staging, single database instance"
tolerance = "none: the invariant is exact"
derived-from = ["TASK-011", "AC-004-03"]
```

**What this proves**

`SPEC-004` scenario 3: two receipts confirmed concurrently against one line
accept a total no greater than the outstanding quantity.

**Setup**

A line with an outstanding quantity of 20 units. Two independent transactions
each attempting to confirm 15 units, started at the same instant.

**Assertions**

- At most one transaction is confirmed
- The total accepted quantity is at or below 20
- The rejected transaction's response reports the outstanding quantity
- On-hand quantity equals its starting value plus the accepted quantity, and
  nothing else changed

**How it is run**

`cargo test --test concurrency_receipts -- --ignored`, against staging rather
than in-process, because a unit test with a single-threaded connection cannot
produce the race it is meant to reproduce.

**Failure interpretation**

If this test fails intermittently, the guard is not correctly serialised. Do
**not** add a retry; a retry hides the defect and does not remove it.

## The difference from a unit test

- A **unit test** verifies a function. A **contract test** verifies a boundary
  against its declared schema. A **scenario test** (`SCN`) verifies that
  several correct behaviours compose.
- A test that cannot fail is worse than no test, because it consumes the
  confidence that a real test would provide.

## Rules for this registry

- No test without a `verifies` link. Gate 7 blocks an orphan test: a test that
  verifies nothing is a maintenance liability, not evidence.
- Only mark a test `VERIFIED` after running it. This is the single easiest
  value in the framework to fake, and faking it makes the entire trace
  meaningless.
- Record the tolerance. A test with no tolerance quietly grows one.
