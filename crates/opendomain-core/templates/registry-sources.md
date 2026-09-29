# Sources

External sources the project relies on, `SRC-###`. One artifact per source.

Required fields: `title`, `type`, `url`, `retrieved`. Recommended:
`organization`, `date`, `authority`, `domain`, `reliability`.

`retrieved` is not optional. A source without a retrieval date cannot be
re-checked, and an unverifiable claim cannot be defended in an audit.

<!--
## SRC-014 — GS1 General Specifications, release 25.0

```toml
title = "GS1 General Specifications, release 25.0"
type = "STANDARD"
organization = "GS1"
url = "https://www.gs1.org/standards/barcode"
date = "2025-01"
retrieved = "2026-02-11"
authority = "Industry standards body for barcode identification"
domain = "inventory"
reliability = "HIGH"
epistemic = "EXPLICIT"
```

**Relevant sections**

- §7.3 GTIN allocation
- §9.4 Application identifiers for quantity and expiry

**Claims used**

- `- stock items carry a globally unique identifier`
- `- a lot/batch identifier and expiry date are encoded as application
  identifiers on a serialized item`

**Context note**

Describes the general case worldwide. Does not describe any specific local
practice, and does not address local customs declaration formats.

-->

## Source types

`STANDARD` `REGULATION` `LAW` `OFFICIAL_DOC` `ACADEMIC` `BOOK`
`PRODUCT_DOC` `CASE_STUDY` `INDUSTRY_REPORT` `OPEN_SOURCE` `COMPETITOR` `COMMUNITY` `BLOG`

## Rules for this registry

- **Never fabricate a source.** An invented citation is worse than a recorded
  gap, because it looks like evidence.
- Record the *relevant section*, not just the document. "I read it" is not
  traceability.
- Note when a source describes one organisation rather than a general rule.
- If a source could not be retrieved, say so here rather than in the chat.
