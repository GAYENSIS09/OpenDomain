# Spec 000 — Function specialization (executed example)

*Provided* example: a function-specialization case, executed end to end as
a demonstration. The matching DAG lives in `../../graph/graph.json` — not a
real request.

## T (task description)

Complete a Python function from a signature and a docstring:

```python
def has_close_elements(numbers: List[float], threshold: float) -> bool:
    """Check if any two numbers in the list are closer than the given threshold."""
```

## Contraintes
- exact signature and **verbatim** docstring in the output,
- one handling per edge case raised by the analyst (empty list, single
  element, threshold zero — **strict** inequality: `distance < threshold`),
- third-party imports banned,
- pass the target project's local tests.

## Critères d'acceptance (C_final)
- [ ] calls `analyste → strategie → synthese → verificateur` in that order,
- [ ] every `y_i` passed its gate before reaching the next agent,
- [ ] the final verdict is a justified PASS/FAIL, never a bare assertion.

## Décomposition prévue
             ┌──────────┐   ┌───────────┐   ┌──────────┐   ┌──────────────┐
             │ analyste │ ─▶│ strategie │ ─▶│ synthese │ ─▶│ verificateur │
             └──────────┘   └───────────┘   └──────────┘   └──────────────┘
