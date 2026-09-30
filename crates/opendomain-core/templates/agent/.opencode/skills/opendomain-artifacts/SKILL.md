---
description: Use when writing or repairing any registry artifact. Heading + toml + prose format, id allocation, relation vocabulary, per-kind rules.
---

# Writing a record people and machines can trust

Each registry is a Markdown file made of `##` headings — one per record. The fenced `toml` block is load-bearing and machine-checked (a malformed block is reported with a line number, never guessed at). The prose underneath is never parsed, so write as much explanation as the subject deserves without fear of breaking anything.

````markdown
## REQ-012 — Consult the movement history of an item

```toml
type = "FUNCTIONAL"
status = "DRAFT"
epistemic = "INFERRED"
confidence = "MEDIUM"
actor = ["ACT-002"]
derived-from = ["EVD-014"]
```

**Statement**

The system SHALL allow authorized users to consult the chronological movements
of an item.
````

## Six habits for clean records

1. **Headings follow the pattern exactly**: `## <PREFIX>-<NNN> — Title`. Ask `od new <KIND>` for the next free number (with `--scope` for scoped ids like `REQ-INVENTORY-014`) instead of counting by hand — and re-run it right before writing, since it prints but never reserves.
2. **Links are declared by the dependent side**: `REQ-004 derived-from RULE-001`, never the reverse. If you're unsure which prefix a kind uses or which relations exist, `od reference` knows — don't guess.
3. **Statuses move forward along a lifecycle** — `DRAFT → RESEARCHED → SUPPORTED → VALIDATED → APPROVED → IMPLEMENTING → IMPLEMENTED → VERIFIED` — plus a few terminal states. `od advance` walks an artifact forward and refuses to walk it back; a backwards move means editing the file by hand, deliberately.
4. **An empty registry is a normal starting state**, not a reason to stop: when a file holds only its commented example and its Rules section, follow the example.
5. **One concept, one owner.** If two contexts claim the same word (`SalesCustomer` versus `InventoryCustomer`), merge them or record the distinction as a decision — silent duplication becomes a blocking finding downstream.
6. **Close the loop**: after writing, run `od link --check` and the current gate, then report the verdict with the top blockers quoted by their identifiers.
