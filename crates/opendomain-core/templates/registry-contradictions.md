# Contradictions

Places where the project contradicts itself, `CON-###`. One artifact per
contradiction.

A contradiction is not a source conflict. `XCONF` is two sources disagreeing;
`CON` is the project asserting two incompatible things. The second is almost
always more expensive.

Required fields: `status`, `statement`, `side-a`, `side-b`.

<!--
## CON-001 — Receipt must be fast and must be re-keyed to the ERP

```toml
status = "OPEN"
statement = "REQ-002 requires synchronous confirmation for operator feedback, but the ERP has a 30-minute unavailability window recorded in INT-001"
side-a = "REQ-002 — confirmation SHALL return within 2 seconds"
side-b = "INT-001 — the ERP is unavailable for up to 30 minutes during month-end close"
derived-from = ["INT-001", "REQ-002", "EVD-044"]
```

**Side A argues**

Synchronous confirmation is what makes the receipt screen trustworthy. If the
operator does not get an immediate answer, they re-key later, and the whole
point of the change is to stop re-keying.

**Side B argues**

The ERP being unavailable is a fact of operations, not a design constraint to
be wished away. A synchronous design has to degrade, and degrading it means
the operator loses the guarantee.

**Why this is a contradiction**

Both cannot be true. Either confirmation is conditional on the ERP, in which
case the 2-second guarantee has an exception, or it is not, in which case the
system accepts receipts while the ERP is down and owns the reconciliation.

**Resolution options**

- Local confirmation with a visible reconciliation queue, and the 2-second
  guarantee applies to local persistence
- Synchronous confirmation with an explicit "ERP unavailable" state, and the
  operator is told the receipt is provisional

**Decision owner**

`STK-004`

-->

## Status values

`OPEN` `RESOLVED` `ACCEPTED` — both sides are kept, with the cost acknowledged
