# docs-site

Static **OpenDomain** documentation — no build step, deployable to GitHub Pages / Netlify / static Vercel. Restrained layout inspired by the opencode docs: slim bar, left sidebar, "On this page" right nav, prose first.

## Style

Palette drawn from `../opendomain.svg`:

| Role | Token | Hex |
|---|---|---|
| paper | `--paper` | `#FBF8F1` |
| paper-2 | `--paper-2` | `#F1ECE1` |
| sand | `--sand` | `#D9CFBA` |
| ink | `--ink` | `#1B1A17` |
| structural red | `--red` | `#D9432B` |

Logo: `assets/opendomain.svg` (copy of `../opendomain.svg`).

## Structure

```text
docs-site/
  index.html              # Introduction
  docs/
    getting-started.html  # install, init, first verdict
    concepts.html         # knowledge states, evidence, anti-whitewashing
    artifacts.html        # record format + annotated example
    gates.html            # the 9 gates + rules
    cli-reference.html    # od reference (the only command-centered page)
    opencode.html         # commands, roles, permissions
    theory.html           # full specification (v0.20.26)
  css/style.css           # restrained theme
  js/main.js              # copy, mobile sidebar, on-this-page nav, filter, Ctrl+K
  assets/opendomain.svg   # logo
```

## Conventions

- Prose first: command blocks are reserved for essential spots (install, format example, CLI reference, permissions).
- No marketing hero, no badges, no decorative icons.
- Every page: left sidebar (Guide / Reference groups), right nav generated in JS from `h2`/`h3`, footer with "Edit / Report" links.
- `Ctrl K` search: filters the sidebar entries.

## Preview

```bash
python -m http.server 8080 --directory docs-site
# then http://localhost:8080
```
