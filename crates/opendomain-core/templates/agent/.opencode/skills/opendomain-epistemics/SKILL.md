---
description: Use when choosing an epistemic value, recording a promotion, or checking gate-7/epistemic-laundering. The seven knowledge states and how a claim may get stronger.
---

# Saying how you know

Every record that makes a knowledge claim carries one `epistemic` label. There is no default: a record that makes a claim without saying how it is known is itself a blocking finding (`gate-1/undeclared-epistemic`). The framework cannot protect what it cannot see — so the very first duty is to label honestly.

## The seven states, and when to use each

- **`EXPLICIT`** — a source you actually read states this directly. You record where it came from (`derived-from`). This is the strongest label, and the only one that can carry a `VALIDATED` record.
- **`TACIT`** — real knowledge held in people's practice, written nowhere. Name the practice or the practitioner. It is not "weaker" than `EXPLICIT`, just located differently: what people actually do often matters more than what the document says they do. Record both, and mark the divergence instead of quietly resolving it.
- **`HYBRID`** — part stated in a source, part lived practice. Record both halves and the reasoning that joins them.
- **`INFERRED`** — you derived it by reasoning from evidence. Write down the chain and set a `confidence`. Code you observed in a repository is `INFERRED` at best — never `EXPLICIT`.
- **`ASSUMED`** — nobody knows yet, and you need a placeholder to move forward. Say so in the prose, with the condition that would retire it. User speech, general knowledge, and "typical in this industry" are all `ASSUMED`.
- **`UNKNOWN`** — a known gap. Track it as an uncertainty; do not fill it in with a plausible invention.
- **`CONFLICTING`** — reliable sources disagree. Record both positions and resolve nothing yourself.

## How a claim gets stronger — and how to record it

When a record's status moves to `VALIDATED` or `VERIFIED`, the move itself gets recorded:

```toml
promotions = [
  { from = "ASSUMED", to = "VALIDATED", evidence = ["EVD-014"], date = "2026-04-10" }
]
```

Without that entry, a genuine strengthening and a quiet status edit look identical — and telling them apart is precisely what the framework exists for. A promotion from `ASSUMED` or `INFERRED` to `VALIDATED`/`VERIFIED` that cites no evidence, cites an identifier that doesn't exist, or cites evidence that isn't itself `EXPLICIT` is blocked as `gate-7/epistemic-laundering`. Note there are two tripwires: the recorded history, and the current state alone — a record that reads `ASSUMED` and `VALIDATED` at the same time has been laundered whether or not anyone wrote it down. A workspace with no promotion history at all gets an advisory instead (`gate-7/no-transition-history`): absence of history doesn't prove laundering happened, it proves laundering would currently be undetectable — a smaller problem, and a different one.

## The two things to never do

Never write `EXPLICIT` for something you inferred, and never invent a `SRC-` or `EVD-` identifier to make a claim look sourced. Both turn an honest gap — which the gates know how to handle — into apparent fact, which they cannot.
