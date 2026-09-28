# Research protocol

How this project finds out things. Run the stages in order; do not skip to
searching.

## Source hierarchy

Prefer the highest tier that answers the question. The ranking is contextual,
not absolute: a competitor's own workflow documentation often beats a generic
article when you need to understand a concrete process.

1. Laws, regulations, official standards
2. Official institutional documentation
3. Professional bodies and recognised organisations
4. Peer-reviewed research
5. Academic textbooks
6. Official product and process documentation
7. Industry reports and case studies
8. Established professional publications
9. Open-source implementations
10. Competitor documentation
11. Community discussion
12. Blogs and informal content

Ask "authoritative for *what*?" before citing.

## Search stages

- **A — Seed.** Broad, authoritative queries: `"<domain> requirements | workflow"`.
- **B — Concept expansion.** Take the terminology from your strongest sources
  and search *those* terms.
- **C — Backward.** For important academic sources, read the references.
  Foundational models and earlier theories live there.
- **D — Forward.** Find later work that cites your important sources.
- **E — Contradiction.** Search deliberately for disagreement:
  `"<concept> limitations | exceptions | criticism | failure"`.
- **F — Regional.** Search local regulation, terminology and practice:
  `"<domain> <country> | local regulation"`.

Never assume a generic international workflow represents local practice.

## Research questions

```toml
# RES-001 — Who is accountable for approving a receipt?
id = "RES-001"
question = "Who is accountable for approving a receipt?"
domain = "inventory"
status = "OPEN"
```

## Saturation

Stop searching when new results are mostly duplicates, already-known rules,
low-quality sources or irrelevant variations. Before stopping, confirm coverage
of all fourteen categories:

- [ ] Actors
- [ ] Processes
- [ ] Entities
- [ ] Rules
- [ ] Exceptions
- [ ] Constraints
- [ ] Integrations
- [ ] Functional requirements
- [ ] Non-functional requirements
- [ ] Risks
- [ ] Regulation
- [ ] Alternative practices
- [ ] Contradictions
- [ ] Unknowns

If a major category is empty, the research is incomplete — say so rather than
proceeding as if it were done.

## Rules

1. Prefer primary sources.
2. Use independent sources for important claims. Three citations to the same
   origin are one source, not three.
3. Separate fact from interpretation.
4. Record publication dates and the context the claim was made in.
5. Check whether a source describes one organisation rather than a general rule.
6. Do not copy competitor assumptions blindly.
7. Identify contradictions instead of resolving them silently.
8. Preserve citations.
9. **Never fabricate a source.**

When research is unavailable, record that explicitly:

```text
RESEARCH LIMITATION — no authoritative source could be retrieved for <topic>.
The following statements are unverified.
```
