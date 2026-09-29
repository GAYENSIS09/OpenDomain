# Non-functional requirements

`NFR-###`. Quality requirements, stated as measurable constraints.

**Never postpone quality requirements.** A quality attribute that is not
specified is not a nice-to-have; it is an undiscovered defect with a deadline.

Required fields: `type`, `status`, `measure`. `measure` is mandatory: an NFR
without a measurement is an aspiration.

<!--
## NFR-001 — Stock read latency under concurrent receipt posting

```toml
type = "PERFORMANCE"
status = "VALIDATED"
measure = "95th percentile ≤ 400 ms at 50 concurrent users, measured over 15 minutes"
epistemic = "EXPLICIT"
confidence = "MEDIUM"
derived-from = ["EVD-052", "STK-001"]
scope = "CORE"
verifies = ["TEST-031"]
```

**Statement**

The 95th percentile latency of a stock read SHALL be at most 400 ms, measured at
50 concurrent users over a 15-minute window during normal operation.

**Why this number**

`STK-001` reported that operators abandon the receipt screen if it lags. The
threshold was set below the observed attention-switch cost, not from a
general rule of thumb.

**Measurement**

Load test `TEST-031`, run in the staging environment with production-like data
volume.

**Degradation behaviour**

Above 50 concurrent users the system SHALL continue to function; the latency
target applies only up to that concurrency level.

-->

## Quality dimensions

`PERFORMANCE` `SECURITY` `RELIABILITY` `AVAILABILITY` `USABILITY` `MAINTAINABILITY` `COMPATIBILITY` `SCALABILITY` `OBSERVABILITY` `AUDITABILITY` `COMPLIANCE` `MAINTAINABILITY` `PORTABILITY` `USABILITY`

## Rules for this registry

- Every NFR needs a number, a workload, and a measurement method.
- Select only the attributes the domain actually needs. Applying all of them
  automatically produces a specification nobody implemented.
- An NFR derived from a stakeholder's preference rather than from a constraint
  is `epistemic = "ASSUMED"` and must say so.
