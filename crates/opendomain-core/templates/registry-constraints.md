# Constraints

Boundaries the project does not choose, `CNST-###`. One artifact per constraint.

A constraint narrows the solution space from outside. A decision narrows it
from inside. Recording a decision as a constraint is how a project ends up
pretending it had no choice.

Required fields: `status`, `type`, `statement`. Recommended: `source`,
`derived-from`, `implications`.

`type` is one of `TECHNICAL` `LEGAL` `REGULATORY` `FINANCIAL` `ORGANIZATIONAL` `TEMPORAL` `PHYSICAL` `CONTRACTUAL`.

<!--
## CNST-001 — The target platform is the company's standard hosting estate

```toml
status = "VALIDATED"
type = "ORGANIZATIONAL"
statement = "The system SHALL deploy only on the managed platform estate already contracted by the company; a bespoke deployment is not funded"
source = "STK-005"
derived-from = ["STK-005", "EVD-052"]
implications = "No self-managed Kubernetes cluster, no custom operating system, no purchase of a new licence outside the existing agreement"
epistemic = "EXPLICIT"
```

**Statement**

Deployment is restricted to the company's contracted managed platform estate.

**Origin**

Procurement and platform engineering, stated by `STK-005` in the phase-0
interview.

**Consequences**

- Available compute, database and queueing services are limited to what the
  estate offers
- A requirement that needs a capability outside the estate is either
  renegotiated with the platform team or dropped
- This constraint was considered and accepted in `ADR-002`

**What it does not constrain**

Domain behaviour. `REQ-004` does not change because of CNST-001; only how it is
built does.

-->

## Rules for this registry

- Every constraint needs an origin. An unsourced constraint is a preference
  wearing a bureaucratic costume.
- State the consequences, not just the boundary. A constraint whose cost is not
  written down will be dropped under pressure.
- A constraint that is really a decision belongs in `DEC-###` or an `ADR`.
