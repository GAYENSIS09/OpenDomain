//! Agent integration files, written to the project root rather than into
//! `opendomain/`.
//!
//! These exist because a workspace nobody's agent knows about is a workspace the
//! discipline does not reach. `opendomain init` creates forty-one registries and, without
//! these, produces a project where an assistant has no way to learn that the
//! epistemic labels are load-bearing — which is the case this framework is
//! specifically about.
//!
//! Paths are relative to the project root, not to `opendomain/`, and the two are
//! written by different code paths. `AGENTS.md` is portable and read by several
//! agents with no configuration; `.opencode/` wires the same discipline into one
//! specific tool with slash commands and subagents.
use super::Template;

/// `AGENTS.md` at the project root.
pub const AGENTS: Template = Template {
    path: "AGENTS.md",
    contents: include_str!("../../templates/agent/AGENTS.md"),
};

/// `.opencode/command/opendomain-status.md`.
pub const OD_STATUS: Template = Template {
    path: ".opencode/command/opendomain-status.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-status.md"),
};

/// `.opencode/command/opendomain-gate.md`.
pub const OD_GATE: Template = Template {
    path: ".opencode/command/opendomain-gate.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-gate.md"),
};

/// `.opencode/command/opendomain-trace.md`.
pub const OD_TRACE: Template = Template {
    path: ".opencode/command/opendomain-trace.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-trace.md"),
};

/// `.opencode/command/opendomain-impact.md`.
pub const OD_IMPACT: Template = Template {
    path: ".opencode/command/opendomain-impact.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-impact.md"),
};

/// `.opencode/command/opendomain-audit.md`.
pub const OD_AUDIT: Template = Template {
    path: ".opencode/command/opendomain-audit.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-audit.md"),
};

/// `.opencode/command/opendomain-record.md`.
pub const OD_RECORD: Template = Template {
    path: ".opencode/command/opendomain-record.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-record.md"),
};

/// `.opencode/command/opendomain-project.md`.
pub const OD_PROJECT: Template = Template {
    path: ".opencode/command/opendomain-project.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-project.md"),
};

/// `.opencode/command/opendomain-research.md`.
pub const OD_RESEARCH: Template = Template {
    path: ".opencode/command/opendomain-research.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-research.md"),
};

/// `.opencode/command/opendomain-domain.md`.
pub const OD_DOMAIN: Template = Template {
    path: ".opencode/command/opendomain-domain.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-domain.md"),
};

/// `.opencode/command/opendomain-requirements.md`.
pub const OD_REQUIREMENTS: Template = Template {
    path: ".opencode/command/opendomain-requirements.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-requirements.md"),
};

/// `.opencode/command/opendomain-opportunity.md`.
pub const OD_OPPORTUNITY: Template = Template {
    path: ".opencode/command/opendomain-opportunity.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-opportunity.md"),
};

/// `.opencode/command/opendomain-spec.md`.
pub const OD_SPEC: Template = Template {
    path: ".opencode/command/opendomain-spec.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-spec.md"),
};

/// `.opencode/command/opendomain-plan.md`.
pub const OD_PLAN: Template = Template {
    path: ".opencode/command/opendomain-plan.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-plan.md"),
};

/// `.opencode/command/opendomain-implement.md`.
pub const OD_IMPLEMENT: Template = Template {
    path: ".opencode/command/opendomain-implement.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-implement.md"),
};

/// `.opencode/command/opendomain-complete.md`.
pub const OD_COMPLETE: Template = Template {
    path: ".opencode/command/opendomain-complete.md",
    contents: include_str!("../../templates/agent/.opencode/command/opendomain-complete.md"),
};

/// `.opencode/agent/domain-researcher.md`.
pub const DOMAIN_RESEARCHER: Template = Template {
    path: ".opencode/agent/domain-researcher.md",
    contents: include_str!("../../templates/agent/.opencode/agent/domain-researcher.md"),
};

/// `.opencode/agent/context-completer.md`.
pub const CONTEXT_COMPLETER: Template = Template {
    path: ".opencode/agent/context-completer.md",
    contents: include_str!("../../templates/agent/.opencode/agent/context-completer.md"),
};

/// `.opencode/agent/opendomain-harnais.md` — l'architecte du harnais :
/// un `TASK-###` du registre devient un système d'agents vérifié. Subagent
/// borné (jamais persona principal), dans le moule du context-completer :
/// permissions deny-all + allows explicites, phases, handback. La doctrine
/// vit une fois, dans le skill `opendomain-harness`. Le spec-implementer
/// direct a été supprimé : TOUTE implémentation exécutée par des agents
/// passe par le harnais vérifié — pas de voie qui le contourne.
pub const HARNESS_ARCHITECT: Template = Template {
    path: ".opencode/agent/opendomain-harnais.md",
    contents: include_str!("../../templates/agent/.opencode/agent/opendomain-harnais.md"),
};

/// `.opencode/skills/opendomain/SKILL.md` — index.
pub const SKILL: Template = Template {
    path: ".opencode/skills/opendomain/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain/SKILL.md"),
};

/// `.opencode/skills/opendomain-epistemics/SKILL.md` — les 7 valeurs + promotions.
pub const SKILL_EPISTEMICS: Template = Template {
    path: ".opencode/skills/opendomain-epistemics/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain-epistemics/SKILL.md"),
};

/// `.opencode/skills/opendomain-artifacts/SKILL.md` — format, ids, relations.
pub const SKILL_ARTIFACTS: Template = Template {
    path: ".opencode/skills/opendomain-artifacts/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain-artifacts/SKILL.md"),
};

/// `.opencode/skills/opendomain-research/SKILL.md` — fan-out, triangulation.
pub const SKILL_RESEARCH: Template = Template {
    path: ".opencode/skills/opendomain-research/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain-research/SKILL.md"),
};

/// `.opencode/skills/opendomain-gates/SKILL.md` — résumé-first, slug → action.
pub const SKILL_GATES: Template = Template {
    path: ".opencode/skills/opendomain-gates/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain-gates/SKILL.md"),
};

/// `.opencode/skills/opendomain-harness/SKILL.md` — la doctrine d'exécution
/// du harnais, à UN seul endroit : les deux phases (construction, exécution
/// sous gates), l'attribution typée, les interdits. Les contrats de rôle par
/// plateforme ont été supprimés — le harnais se pilote par les commandes
/// (`/opendomain-implement` → rôles → skills), pas par un persona principal ;
/// une doctrine unique ne peut pas diverger d'avec elle-même.
pub const SKILL_HARNESS: Template = Template {
    path: ".opencode/skills/opendomain-harness/SKILL.md",
    contents: include_str!("../../templates/agent/.opencode/skills/opendomain-harness/SKILL.md"),
};

/// Every agent integration file.
pub fn all() -> Vec<Template> {
    vec![
        AGENTS,
        OD_STATUS,
        OD_GATE,
        OD_TRACE,
        OD_IMPACT,
        OD_AUDIT,
        OD_RECORD,
        OD_PROJECT,
        OD_RESEARCH,
        OD_DOMAIN,
        OD_REQUIREMENTS,
        OD_OPPORTUNITY,
        OD_SPEC,
        OD_PLAN,
        OD_IMPLEMENT,
        OD_COMPLETE,
        DOMAIN_RESEARCHER,
        CONTEXT_COMPLETER,
        HARNESS_ARCHITECT,
        SKILL,
        SKILL_EPISTEMICS,
        SKILL_ARTIFACTS,
        SKILL_RESEARCH,
        SKILL_GATES,
        SKILL_HARNESS,
    ]
}

/// `true` when this file is `AGENTS.md` at the project root.
///
/// It gets a gentler default than the rest. A project that already has an
/// `AGENTS.md` has instructions this tool did not write, and silently
/// replacing them with framework boilerplate would discard whatever the
/// project had already told its agents. The other files are namespaced under
/// `.opencode/`, so writing them cannot collide with
/// anything the project owns.
pub fn is_root_level(t: &Template) -> bool {
    !t.path.contains('/')
}
