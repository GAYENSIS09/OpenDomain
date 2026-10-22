# Changes

In-flight work lives here, one directory per change proposal.

A **change** is a modification to already-agreed behaviour. The registries
record what the project currently believes; a change records a proposal to
alter that, reviewed and then applied or rejected.

## Why changes are separate

Keeping proposals out of the registries is what makes `opendomain status` meaningful. If
`REQ-004` could be edited directly, the registry would always describe an
ambiguous mix of agreed and proposed behaviour, and no gate could tell them
apart.

## Layout

```text
changes/
  add-over-receipt-approval/
    proposal.md      # why, what changes, which IDs are affected
    design.md        # how, for changes large enough to need it
    tasks.md         # the ordered work
    specs/           # capability deltas, same format as specs/
    tests.md         # what will verify it
```

## Lifecycle

```text
PROPOSED -apply-> APPLIED -archive-> ARCHIVED
    |                                      |
    +-reject-> REJECTED                     +- superseded by a later change
```

A change is never edited in place after it is applied. Correcting an applied
change means another change.

## Rules

- Every change names the IDs it affects. A change that cannot say what it
  touches is a rewrite, not a change.
- `proposal.md` must state *why* before *what*. A proposal that leads with
  implementation has already skipped the interesting part.
- Anything that survives application is copied into the registries by
  `opendomain apply`; the change directory is then the historical record of how it
  arrived.
- A rejected change is archived, not deleted. Knowing what was considered and
  why it lost is worth more than a clean directory.
