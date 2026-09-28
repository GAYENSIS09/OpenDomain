# Concepts

Candidate solutions worth debating, `CPT-###`. One artifact per concept.

A concept is a *shape* of solution, not a task list. Two concepts are worth
writing down when the team disagrees about which is better, because then the
disagreement is a real decision with real consequences.

Required fields: `status`, `class`, `rationale`. Recommended: `mechanism`,
`key-difference`, `derived-from`.

`class` is one of `INCREMENTAL` `REVOLUTIONARY` `REPLICATION` `SUBSTITUTION`.

<!--
## CPT-003 — Scanning-first receipt capture

```toml
status = "APPROVED"
class = "REVOLUTIONARY"
mechanism = "The operator scans the item label; quantity, lot and expiry are read from the code, and the only manual input is a confirmation"
key-difference = "Competitors require manual entry of quantity and lot; this removes it entirely, which is the specific behaviour the re-key loss comes from"
rationale = "EVD-014 shows reconciliation depends on lot and expiry, and EVD-021 shows those are exactly the fields lost in re-keying. A form cannot prevent that loss; removing the input can."
derived-from = ["OPP-002", "EVD-014", "EVD-021"]
risks = "Requires GS1 Application Identifiers, which EVD-021 confirms only some suppliers encode"
```

**The idea**

Replace the receipt form with a scan-and-confirm interaction. Barcode content
supplies quantity, lot and expiry; the operator confirms rather than types.

**Why it might work**

- It removes the input step that re-keying loses
- It makes the loss visible: an unencoded label is detected at scan time rather
  than at reconciliation
- It matches `EVD-021`'s finding that scanning is already used for identity

**Why it might not**

- Suppliers that do not encode quantity in a scannable form fall back to manual
  entry, so the mechanism is partial
- It depends on `SRC-014`'s application identifier scheme being present on
  received goods, which `EVD-021` does not establish

**What would make this wrong**

If `EXP-002` shows the fallback path is the majority of receipts, the mechanism
does not pay for itself and `CPT-004` should win.

**Falsifiable claim**

`HYP-005`

## Rules for this registry

- A concept with no `mechanism` and no `key-difference` is a slogan. Gate 4
  reports it.
- `class = "REPLICATION"` justified by "our competitor has it" is feature
  cloning. Gate 4 blocks it. Competitor functionality is evidence about a
  market, not a reason to build.
- Write the concept that you expect to lose. A registry where every concept
  wins is a registry where nothing was debated.
