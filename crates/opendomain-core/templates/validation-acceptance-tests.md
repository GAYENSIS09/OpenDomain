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

### Verification depth and binding provenance

Two optional fields make a test's claim inspectable (Sections 10.9/10.10):

```toml
verification-depth = "executed"   # executed | discovered | declared
binding = { to = "AC-004-03", provisionance = "ANNOTATED", confirmed = false }
```

- `verification-depth` — `executed`: the suite was run against the current
  implementation and its result observed; `discovered`: the artifact exists
  and is bound, but was not run here; `declared`: only the link is asserted.
  `opendomain bind` states the depth it audited from, and a VERIFIED record
  without `automation` reads as claimed-versus-actual.
- `binding` — how the `verifies` edge was established: `GENERATED`
  (provisioner tag), `ANNOTATED` (`// odc:<ID>` comment), `ATTRIBUTED`
  (language-native marker), or `INFERRED` (naming convention, LOW
  confidence). An INFERRED binding cannot close a traceability edge until
  `confirmed = true` after human review; the framework never presents the
  origins as equivalent.

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

-->

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
- When the implementation under test was executed by agents, cite the archived
  run that verified it — with the first-class field:
  `run = "runs/run-<NNN>-<slug>"` (the run directory
  `opendomain/harness/runs/run-<NNN>-<slug>/`), and, when the claim rests on
  one node's verdict, `run-node = "…"` — that node's last `gates.log` line is
  what makes the citation checkable. `opendomain provenance` verifies every
  citation against the archive it names: a citation that names no archive is
  an appeal, not evidence. A run archive is evidence in the same sense as
  this registry — append-only, never rewritten after the fact.
