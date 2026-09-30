---
description: Complete Phase 0 context (Gate 0). Delegates to context-completer, asks human-only gaps.
agent: build
---

# Setting the project context

Everything starts here: what are we building, for whom, and what is already true? This command fills in the Phase 0 context — the `project.md` file, the `[project]` table, and the first `DOM-001`/`STK-001` records — without inventing anything.

## How it goes

1. I start with `od status`, to record the starting point in a few lines.
2. I hand the work to the `context-completer` subagent, with your hints in `$ARGUMENTS` (the repository path, whatever you already know about the purpose or jurisdiction). It reads the whole repository, gets external research done through the `domain-researcher`, then writes the files.
3. Some things live neither in code nor on the web — the real jurisdiction, the actual users, budget constraints, known problems. For those, I come back to you with its questions, grouped together and served with its best guesses. Your answers get recorded honestly as user speech, never as established fact.
4. I finish with `od link --check` and `od gate context`, answering summary-first: the gate's **Verdict**, its main blockers with identifiers, what got filled in and what stays open — then the detail in an appendix.

A useful reminder: `od init` installs the project but never records any context in it. Code shows what the system does, never what it was meant to do — so whatever I conclude from reading it stays an inference until you confirm it.
