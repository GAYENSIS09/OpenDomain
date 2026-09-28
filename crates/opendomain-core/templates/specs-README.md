# Specifications

Behavioural specifications, `SPEC-###`. One file per capability.

A specification states **what the system does that can be observed**. It does
not state how, and it does not state why — the requirement already did that.

## File layout

```text
specs/
  receipts/
    SPEC-004-receiving-goods.md
  procurement/
    SPEC-002-purchase-orders.md
  overview.md
```

Each file is a registry containing one or more `SPEC-###` artifacts, so the
same parser reads it.

## The five obligatory sections

1. **Scenarios** — Given / When / Then, with the precondition made concrete
2. **Behavioural rules** — SHALL statements, numbered
3. **Acceptance criteria** — `AC-###-##` per scenario, one observable outcome
   each
4. **Edge cases** — what happens at zero, at the boundary, and on conflict
5. **Non-goals** — what this capability deliberately does not do

## Given / When / Then, used properly

The **Given** must make the precondition real: actual values, not placeholders.
The **Then** must be observable from outside: a value, a state, an event, an
absence of an effect.

<!--
## SPEC-004 — Reject a receipt exceeding the outstanding quantity

```toml
status = "APPROVED"
concept = "CPT-003"
derived-from = ["REQ-004", "UC-006"]
verifies = ["AC-004-01", "AC-004-02", "AC-004-03", "AC-004-04"]
epistemic = "EXPLICIT"
scope = "CORE"
```

### Scenarios

#### Scenario 1 — Receipt at exactly the outstanding quantity

**Given** purchase order `PO-2026-0012` line 2 has an outstanding quantity of
20 units

**When** a receipt of 20 units is confirmed against that line

**Then** the receipt is confirmed, the line's outstanding quantity becomes 0,
and the on-hand quantity of item `ITM-0031` increases by 20

#### Scenario 2 — Receipt above the outstanding quantity

**Given** the same line has an outstanding quantity of 20 units

**When** a receipt of 21 units is confirmed

**Then** the receipt is rejected, no stock movement is recorded, the on-hand
quantity is unchanged, and the response reports the outstanding quantity of 20

#### Scenario 3 — Two concurrent receipts

**Given** the same line has an outstanding quantity of 20 units

**When** receipts of 15 units and 10 units are confirmed concurrently

**Then** at most one is confirmed, the total accepted never exceeds 20, and the
rejected receipt reports the outstanding quantity at the time of rejection

#### Scenario 4 — Supplier-authorised over-receipt

**Given** price agreement `PA-2026-003` authorises over-receipt up to 10% and
`ACT-003` has approved this receipt with reference `APR-4417`

**When** a receipt of 22 units is confirmed against a line with 20 outstanding

**Then** the receipt is confirmed, the approval reference is stored with it, and
the report of over-receipt flags the event

### Behavioural rules

1. The system SHALL reject a confirmed receipt whose accepted quantity exceeds
   the line's outstanding quantity, unless an approval reference authorising the
   excess is present.
2. The system SHALL report the outstanding quantity on every rejection.
3. The system SHALL NOT record a stock movement for a rejected receipt.
4. The system SHALL ensure that the sum of accepted quantities against a line
   never exceeds the line's original ordered quantity, including under
   concurrent confirmation.

### Acceptance criteria

- `AC-004-01` — a receipt at exactly the outstanding quantity is confirmed and
  the outstanding quantity becomes zero
- `AC-004-02` — a receipt above the outstanding quantity is rejected with the
  outstanding quantity reported, and on-hand quantity is unchanged
- `AC-004-03` — concurrent confirmations against one line accept a total no
  greater than the original outstanding quantity
- `AC-004-04` — a receipt above the outstanding quantity with a valid approval
  reference is confirmed and the approval is retained

### Edge cases

- Accepted quantity of zero: rejected, with reason
- The item is not stock-managed: the stock step is skipped; the order is still
  updated
- The purchase order itself was cancelled between selection and confirmation:
  rejected
- The line was fully received by another operator moments earlier: rejected with
  the current outstanding quantity, which is zero

### Non-goals

- Choosing a supplier
- Amending a confirmed receipt. Confirmed receipts are reversed, not edited.
- Three-way invoice matching, which belongs to `SPEC-006`
-->

## Rules for these files

- **What, not how.** Gate 5 reports a specification that names a database, a
  framework or a message broker. The technology belongs in the plan or an ADR.
- Every scenario is executable as a test. If it cannot be, it is prose.
- A criterion is one outcome. "The system works correctly" is not a criterion.
- Write the **Non-goals** section every time. It is the section that stops the
  specification from quietly growing.
