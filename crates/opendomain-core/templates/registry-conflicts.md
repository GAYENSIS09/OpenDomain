# Source conflicts

Where sources disagree, `XCONF-###`. One artifact per conflict.

**Never arbitrarily select one source without documenting why.** A conflict that
is resolved silently becomes an undocumented assumption, and undocumented
assumptions are what audits find last and most expensively.

Required fields: `status`, `topic`, `source-a`, `source-b`, `difference`.

<!--
## XCONF-001 — Whether a receipt may exceed the outstanding quantity

```toml
status = "OPEN"
topic = "Over-receipt authorisation"
source-a = "SRC-014"
source-b = "SRC-027"
difference = "SRC-014 permits over-receipt with supplier approval; SRC-027 rejects any over-receipt unconditionally"
derived-from = ["EVD-014", "EVD-031"]
affects = ["RULE-001", "REQ-004"]
```

**Source A says**

A receipt may exceed the outstanding quantity when a price agreement covers the
excess and a named approver authorises it.

**Source B says**

Any receipt exceeding the outstanding quantity SHALL be rejected. Over-receipt
is corrected by a price adjustment, not by a receipt.

**Nature of the conflict**

A policy difference, not a factual one. Both sources describe systems that work.

**Context difference**

Source A describes a distribution operation with high-value, low-volume items
and weekly replenishment. Source B describes retail replenishment with
high-volume, low-value items and daily delivery. The regimes differ.

**Possible resolution**

Adopt A's conditional permission, since the project is closer to A's regime,
and record the assumption. Then verify the regime assumption with
`EXP-002`.

**Project decision**

*Unresolved. Do not implement `REQ-004` until this is closed.*

**Status**

`OPEN`

## Status values

`OPEN` — disagreement recorded, no decision taken
`RESOLVED` — a decision was taken and justified below
`CONTEXTUAL` — both are true, in different regimes; both rules are recorded
