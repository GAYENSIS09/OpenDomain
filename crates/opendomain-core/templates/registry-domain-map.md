# Domain map

The bounded contexts of the system, `DOM-###`. One line of the map per context.

A context is a region of the domain where a particular meaning of a term holds
and one set of rules applies. Modules are not contexts; a module that serves two
contexts is a coupling problem, not a modelling win.

Required fields: `purpose`, `status`.

<!--
## DOM-001 — Procurement

```toml
purpose = "decide what to buy, from whom, and at what price"
status = "VALIDATED"
owned-by = "STK-001"
depends-on = ["DOM-002"]
```

**Responsibilities**

- Raise requisitions, convert them to purchase orders
- Approve suppliers and price agreements
- Own the canonical `Supplier` concept

**Owned concepts**

`TERM-001` Purchase order, `ENT-004` Supplier, `RULE-002` Order approval limit

**Depends on**

- `DOM-002` Inventory — for demand and stock position
- `DOM-005` Finance — for budget availability

**External systems**

- `INT-001` ERP

**Events emitted**

- `EVENT-001` PurchaseOrderApproved
-->

## Rules for this registry

- Every important concept has exactly one owning context. If two contexts both
  claim it, that is a modelling defect, not a shared utility.
- `depends-on` is mandatory for every context in a multi-domain project. Gate 2
  reports a context that was never cross-analysed.
- Avoid `SalesCustomer` / `InventoryCustomer` / `ProcurementCustomer`. Use one
  canonical `Customer` and let the contexts reference it.
- A new module is never an isolated application. Adding one means re-running
  cross-domain analysis and updating this map.
