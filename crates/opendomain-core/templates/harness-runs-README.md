# `runs/` — execution traces (append-only)

One folder per run, named `run-<NNN>-<slug>`:

```
runs/run-002-migration/
├── trace.jsonl      # events: Start, Passed, Retry, Failed
├── context/         # validated y_i (one JSON per agent)
├── gates.log        # gate verdicts y_i ∈ C_i
└── attribution.md   # typed errors + recovery directives
```

Insert-only: never rewritten after the fact — the basis for commits,
rollbacks and diffs between strategies.
