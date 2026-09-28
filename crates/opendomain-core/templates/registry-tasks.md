# Tasks

Implementation tasks, `TASK-###`. One artifact per task.

A task is a unit of work with an owner, a size and a definition of done. It is
not a milestone and not a wish.

Required fields: `status`, `derived-from`, `plan`. Recommended: `depends-on`,
`verifies`, `acceptance`, `estimate`, `owner`.

A task must point at a specification, and — once it is out of draft — at a
test. Gate 6 blocks a task that implements nothing.

<!--
## TASK-011 — Enforce the outstanding-quantity invariant

```toml
status = "DRAFT"
plan = "PLAN-002"
derived-from = ["SPEC-004", "REQ-004"]
verifies = ["TEST-021", "TEST-022", "TEST-023"]
depends-on = ["TASK-009"]
owner = "unassigned"
estimate = "M"
acceptance = "AC-004-01, AC-004-02, AC-004-03 are green and the concurrency test fails when the guard is removed"
```

**What to build**

The check that a confirmed receipt does not exceed the line's outstanding
quantity, including under concurrent confirmation.

**Why it exists**

`SPEC-004` scenario 2 and 3. Without it, `RULE-001` is unenforced.

**Definition of done**

- `AC-004-01` passes
- `AC-004-02` passes, and the response includes the outstanding quantity
- `AC-004-03` passes; removing the concurrency guard makes `TEST-023` fail
- `TEST-024` (approved over-receipt) passes
- No scenario depends on a named technology; if one appears in the
  implementation, the specification was wrong and `DRIFT` is filed

**Dependencies**

`TASK-009` establishes the line quantity accessor this uses.

**Not included**

The outbox write, which is `TASK-014`.

## Rules for this registry

- Every task is verifiable. A task with no test attached is a task with no
  definition of done.
- Tasks are small enough to finish in a day or two. Anything larger is a
  milestone, and milestones belong in the plan.
- If a task's `derived-from` is a requirement rather than a specification, the
  behaviour was never specified. Fix that before building it.
