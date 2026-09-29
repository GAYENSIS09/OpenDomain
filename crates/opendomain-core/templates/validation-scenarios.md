# Validation scenarios

End-to-end validation, `SCN-###`. One artifact per scenario.

A scenario walks the system the way a stakeholder does, crossing contexts, to
prove that the individually-correct behaviours compose.

Required fields: `status`, `derived-from`, `preconditions`, `steps`, `expected`.

<!--
## SCN-002 — Receive a partial delivery, then the remainder

```toml
status = "DRAFT"
derived-from = ["SPEC-004", "UC-006", "REQ-004"]
preconditions = "PO-2026-0012 line 2 has 40 units outstanding, 20 already received"
steps = "Receive 15 units, then receive 25 units"
expected = "The first receipt is confirmed; the second is rejected with 25 outstanding reported; on-hand quantity rises by 15 only"
verification = "TEST-031, run against staging with the ERP unavailable"
```

**Preconditions**

`PO-2026-0012` line 2 was ordered for 40 units. 20 units were received on
2026-03-01. Outstanding quantity is therefore 20.

**Steps**

1. Operator scans 15 units and confirms.
2. Operator scans 25 units and confirms.

**Expected**

1. The first receipt is confirmed. Outstanding quantity becomes 5. On-hand
   quantity rises by 15.
2. The second receipt is **rejected**, because 25 exceeds 5, and the response
   reports the outstanding quantity of 5. On-hand quantity is unchanged.

**Note on the intent of the data**

The step values are chosen so that the second receipt exceeds the *remaining*
quantity rather than the original. A test that only checks the original ordered
quantity would pass here and would not exercise `SPEC-004` at all.

**Verified by**

`TEST-031`, executed against staging with the ERP deliberately unavailable, so
that the local-confirmation path in `CDR-002` is covered too.

-->

## The three failure modes this registry exists for

1. **Each unit is right, the flow is wrong.** Composition is where most defects
   actually live.
2. **The happy path is tested, the exception is not.** Scenarios must include
   at least one failure per critical flow.
3. **Nobody checks the total.** A scenario that only asserts one step is a unit
   test wearing a scenario's clothes.

## Rules for this registry

- A scenario crosses at least two contexts, or it is a test, not a scenario.
- Always include one scenario per critical flow that ends in failure.
- Assert the whole expected result, not just the last step. The "and on-hand
  quantity rises by 15 only" is the part that catches the real bugs.
