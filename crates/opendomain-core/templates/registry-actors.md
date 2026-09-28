# Actors

Roles and systems that interact with the software, `ACT-###`.

Actors are what the system sees. Everything else about a person lives in
`stakeholders.md`.

Required fields: `status`, `owned-by`.

<!--
## ACT-001 — Warehouse operator

```toml
status = "VALIDATED"
epistemic = "EXPLICIT"
owned-by = "DOM-002"
derived-from = ["EVD-004"]
```

**Role**

Physically receives goods against a purchase order and records quantities.

**Goals**

- Record what actually arrived, quickly
- Know what remains outstanding

**Permissions**

- Create and amend goods receipts
- Read purchase orders and item stock
- Cannot approve, cannot issue stock to a customer order

**Processes**

`PROC-001` Receive goods

**Pain points**

- Records quantities on paper when the scanner is offline, then re-keys them
-->

## Rules for this registry

- A system that calls this system is an actor too (`ACT-0xx` with role
  `SYSTEM`).
- Permissions belong here as a statement of intent; enforce them in the
  `security` specification and verify them with tests.
- If a permission is not justified by a business rule, it does not belong in
  the system.
