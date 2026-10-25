# docs-site

Static **OpenDomain** documentation — no build step, deployable to GitHub Pages / Netlify / static Vercel. Restrained layout inspired by the opencode docs: slim bar, left sidebar, "On this page" right nav, prose first.

## Style

Quasi-monochrome palette — neutral paper, near-black ink, and a single desaturated accent (the structural red of `../opendomain.svg`) reserved for links, active markers and blocking verdicts:

| Role | Token | Hex |
|---|---|---|
| paper | `--paper` | `#F8F8F7` |
| paper-2 | `--paper-2` | `#EFEFEC` |
| sand | `--sand` | `#E2E1DC` |
| card | `--card` | `#FFFFFF` |
| ink | `--ink` | `#1B1B19` |
| accent | `--accent` | `#9E3D2B` |

Dark theme (`html[data-theme="dark"]`): `--paper #141413`, `--ink #E8E6E1`, `--accent #D0704C`. The theme is decided before first paint by a small inline head script — stored choice wins, otherwise the OS preference — so no wrong-theme flash ever shows.

Logo: `assets/opendomain.svg` (copy of `../opendomain.svg`).

## Structure

```text
docs-site/
  index.html              # Introduction
  docs/
    getting-started.html  # install, init, first verdict
    concepts.html         # knowledge states, evidence, anti-whitewashing
    artifacts.html        # record format + annotated example
    brownfield.html       # adopting the framework on an existing repository
    gates.html            # the 9 gates + rules
    cli-reference.html    # the registry commands, on one page
    opencode.html         # commands, roles, permissions
    harness.html          # agent-executed flow: six phases, run archive
    theory.html           # full specification (v0.20.26)
  css/style.css           # restrained theme + dark mode + responsive details
  js/main.js              # copy, theme toggle, skip link, scrim, mobile sidebar,
                          # on-this-page nav with scroll-spy, filter, Ctrl+K, Escape
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
