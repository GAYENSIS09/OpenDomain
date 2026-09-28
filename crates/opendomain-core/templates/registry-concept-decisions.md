# Concept decisions

Which concept was chosen, and why, `CDR-###`. One artifact per decision.

**The purpose of this registry is to make the discarded option visible.** A
decision record that only contains the winner cannot be reviewed by the person
who will have to maintain the loser in six months.

Required fields: `status`, `chosen`, `rationale`, `rejected`. Gate 4 blocks a
decision that cites nothing.

<!--
## CDR-002 — Adopt scanning-first capture for goods receipt

```toml
status = "APPROVED"
chosen = "CPT-003"
rationale = "EXP-002 measured a 3.1% manual re-key rate against a 2% target and an 8% failure threshold; the per-supplier maximum was 14%, above the 10% clause, so the hypothesis is partially met"
rejected = ["CPT-004"]
rejected-because = "CPT-004 keeps the general form and adds a lot and expiry field. It does not remove the re-key loss, because the loss comes from the re-key step itself, which CPT-004 leaves in place"
decided-by = "STK-004"
date = "2026-04-02"
derived-from = ["EXP-002", "HYP-005", "CPT-003", "EVD-021"]
```

**Decision**

Adopt `CPT-003`, scanning-first capture, with a streamlined manual fallback for
labels that do not encode the required fields.

**What the experiment actually showed**

`EXP-002` measured a 3.1% overall re-key rate over 214 receipts. The overall
number is above the 2% target and below the 8% failure threshold, so
`HYP-005` is **partially** met. One supplier reached 14%, above the per-supplier
clause, so the hypothesis is refuted for that supplier.

**What was discarded**

`CPT-004` — a general form with better lot and expiry fields — is rejected on
mechanism, not on measurement. Adding fields to a form does not remove the
re-key step, and the re-key step is where `EVD-021` shows the data is lost.

**Consequences accepted**

- 3.1% of receipts still need manual entry, so the fallback must stay first-class
- Supplier `SUP-007` needs a procurement conversation about label conformance
- The hypothesis is not resolved for `SUP-007`; recorded as `ASM-005`

**What would reverse this decision**

If the manual path exceeds 10% overall after `SUP-007` is addressed, revisit
`CPT-004`.

## Status values

`PROPOSED` `APPROVED` `REJECTED` `SUPERSEDED`
