# Stakeholders

People and organisations with an interest in the system's behaviour, `STK-###`.

A stakeholder is not an actor. A stakeholder may never log in; an actor is
always someone or something that does.

Required fields: `status`.

<!--
## STK-001 — Head of Procurement

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
```

**Objectives**

- Buy at the lowest total cost without stockouts
- Keep supplier performance visible

**Responsibilities**

- Owns the approval policy
- Signs off on purchase orders above a threshold

**Problems today**

- Cannot see which orders are late because the ERP report is weekly
- Re-keys supplier data between the ERP and the spreadsheet

**Information needed**

- Open order value by supplier
- Delivery reliability per supplier

**Decisions**

- Whether to raise a purchase order manually
- Which supplier to place an order with

**Interactions**

- Approves `UC-004` Place a purchase order
- Reviews the procurement dashboard
-->

## Rules for this registry

- Distinguish: human role, system actor, domain entity, permission,
  organisation.
- A person who uses the system is not automatically the person accountable for
  the process behind it. Record both when they differ.
- Stakeholders who cannot be interviewed are reconstructed from evidence, and
  the reconstruction is marked `INFERRED`.
