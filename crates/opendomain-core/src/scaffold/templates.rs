//! Every scaffold template.
//!
//! Note which registries are present. `opendomain init` writes the ones matching the
//! active profile: a project that has opted out of Stage 2 should not be
//! handed seven empty opportunity files to feel bad about.
use super::{Profile, Template};

/// `opendomain/project.md` — Phase 0 output.
pub const PROJECT: Template = Template {
    path: "project.md",
    contents: include_str!("../../templates/project.md"),
};

/// `opendomain/README.md` — the workspace's own orientation.
pub const README: Template = Template {
    path: "README.md",
    contents: include_str!("../../templates/workspace-README.md"),
};

/// `opendomain/domain/glossary.md`.
pub const GLOSSARY: Template = Template {
    path: "domain/glossary.md",
    contents: include_str!("../../templates/registry-glossary.md"),
};

/// `opendomain/domain/domain-map.md`.
pub const DOMAIN_MAP: Template = Template {
    path: "domain/domain-map.md",
    contents: include_str!("../../templates/registry-domain-map.md"),
};

/// `opendomain/domain/stakeholders.md`.
pub const STAKEHOLDERS: Template = Template {
    path: "domain/stakeholders.md",
    contents: include_str!("../../templates/registry-stakeholders.md"),
};

/// `opendomain/domain/actors.md`.
pub const ACTORS: Template = Template {
    path: "domain/actors.md",
    contents: include_str!("../../templates/registry-actors.md"),
};

/// `opendomain/domain/processes.md`.
pub const PROCESSES: Template = Template {
    path: "domain/processes.md",
    contents: include_str!("../../templates/registry-processes.md"),
};

/// `opendomain/domain/entities.md`.
pub const ENTITIES: Template = Template {
    path: "domain/entities.md",
    contents: include_str!("../../templates/registry-entities.md"),
};

/// `opendomain/domain/business-rules.md`.
pub const BUSINESS_RULES: Template = Template {
    path: "domain/business-rules.md",
    contents: include_str!("../../templates/registry-business-rules.md"),
};

/// `opendomain/domain/events.md`.
pub const EVENTS: Template = Template {
    path: "domain/events.md",
    contents: include_str!("../../templates/registry-events.md"),
};

/// `opendomain/domain/integrations.md`.
pub const INTEGRATIONS: Template = Template {
    path: "domain/integrations.md",
    contents: include_str!("../../templates/registry-integrations.md"),
};

/// `opendomain/domain/uncertainties.md`.
pub const UNCERTAINTIES: Template = Template {
    path: "domain/uncertainties.md",
    contents: include_str!("../../templates/registry-uncertainties.md"),
};

/// `opendomain/domain/conflicts.md`.
pub const CONFLICTS: Template = Template {
    path: "domain/conflicts.md",
    contents: include_str!("../../templates/registry-conflicts.md"),
};

/// `opendomain/domain/decisions.md`.
pub const DECISIONS: Template = Template {
    path: "domain/decisions.md",
    contents: include_str!("../../templates/registry-decisions.md"),
};

/// `opendomain/domain/research/sources.md`.
pub const SOURCES: Template = Template {
    path: "domain/research/sources.md",
    contents: include_str!("../../templates/registry-sources.md"),
};

/// `opendomain/domain/research/evidence.md`.
pub const EVIDENCE: Template = Template {
    path: "domain/research/evidence.md",
    contents: include_str!("../../templates/registry-evidence.md"),
};

/// `opendomain/domain/research/protocol.md`.
pub const PROTOCOL: Template = Template {
    path: "domain/research/protocol.md",
    contents: include_str!("../../templates/research-protocol.md"),
};

/// `opendomain/opportunity/opportunities.md`.
pub const OPPORTUNITIES: Template = Template {
    path: "opportunity/opportunities.md",
    contents: include_str!("../../templates/registry-opportunities.md"),
};

/// `opendomain/opportunity/assumptions.md`.
pub const ASSUMPTIONS: Template = Template {
    path: "opportunity/assumptions.md",
    contents: include_str!("../../templates/registry-assumptions.md"),
};

/// `opendomain/opportunity/contradictions.md`.
pub const CONTRADICTIONS: Template = Template {
    path: "opportunity/contradictions.md",
    contents: include_str!("../../templates/registry-contradictions.md"),
};

/// `opendomain/opportunity/concepts.md`.
pub const CONCEPTS: Template = Template {
    path: "opportunity/concepts.md",
    contents: include_str!("../../templates/registry-concepts.md"),
};

/// `opendomain/opportunity/hypotheses.md`.
pub const HYPOTHESES: Template = Template {
    path: "opportunity/hypotheses.md",
    contents: include_str!("../../templates/registry-hypotheses.md"),
};

/// `opendomain/opportunity/experiments.md`.
pub const EXPERIMENTS: Template = Template {
    path: "opportunity/experiments.md",
    contents: include_str!("../../templates/registry-experiments.md"),
};

/// `opendomain/opportunity/decisions.md`.
pub const CONCEPT_DECISIONS: Template = Template {
    path: "opportunity/decisions.md",
    contents: include_str!("../../templates/registry-concept-decisions.md"),
};

/// `opendomain/requirements/requirements.md`.
pub const REQUIREMENTS: Template = Template {
    path: "requirements/requirements.md",
    contents: include_str!("../../templates/registry-requirements.md"),
};

/// `opendomain/requirements/non-functional.md`.
pub const NON_FUNCTIONAL: Template = Template {
    path: "requirements/non-functional.md",
    contents: include_str!("../../templates/registry-non-functional.md"),
};

/// `opendomain/requirements/constraints.md`.
pub const CONSTRAINTS: Template = Template {
    path: "requirements/constraints.md",
    contents: include_str!("../../templates/registry-constraints.md"),
};

/// `opendomain/requirements/baselines.md`.
pub const BASELINES: Template = Template {
    path: "requirements/baselines.md",
    contents: include_str!("../../templates/registry-baselines.md"),
};

/// `opendomain/use-cases/use-case-model.md`.
pub const USE_CASES: Template = Template {
    path: "use-cases/use-case-model.md",
    contents: include_str!("../../templates/registry-use-cases.md"),
};

/// `opendomain/specs/README.md`.
pub const SPECS_README: Template = Template {
    path: "specs/README.md",
    contents: include_str!("../../templates/specs-README.md"),
};

/// `opendomain/plans/plans.md`.
pub const PLANS: Template = Template {
    path: "plans/plans.md",
    contents: include_str!("../../templates/registry-plans.md"),
};

/// `opendomain/tasks/tasks.md`.
pub const TASKS: Template = Template {
    path: "tasks/tasks.md",
    contents: include_str!("../../templates/registry-tasks.md"),
};

/// `opendomain/contracts/contracts.md`.
pub const CONTRACTS: Template = Template {
    path: "contracts/contracts.md",
    contents: include_str!("../../templates/registry-contracts.md"),
};

/// `opendomain/architecture/architecture.md`.
pub const ARCHITECTURE: Template = Template {
    path: "architecture/architecture.md",
    contents: include_str!("../../templates/architecture.md"),
};

/// `opendomain/architecture/decisions/records.md`.
pub const ADRS: Template = Template {
    path: "architecture/decisions/records.md",
    contents: include_str!("../../templates/registry-adrs.md"),
};

/// `opendomain/validation/scenarios.md`.
pub const SCENARIOS: Template = Template {
    path: "validation/scenarios.md",
    contents: include_str!("../../templates/validation-scenarios.md"),
};

/// `opendomain/validation/acceptance-tests.md`.
pub const ACCEPTANCE_TESTS: Template = Template {
    path: "validation/acceptance-tests.md",
    contents: include_str!("../../templates/validation-acceptance-tests.md"),
};

/// `opendomain/validation/drift.md`.
pub const DRIFT: Template = Template {
    path: "validation/drift.md",
    contents: include_str!("../../templates/validation-drift.md"),
};

/// `opendomain/validation/audit.md`.
pub const AUDIT: Template = Template {
    path: "validation/audit.md",
    contents: include_str!("../../templates/validation-audit.md"),
};

/// `opendomain/changes/.gitkeep` note explaining the change layout.
pub const CHANGES_README: Template = Template {
    path: "changes/README.md",
    contents: include_str!("../../templates/changes-README.md"),
};

/// Templates written for every profile.
pub fn core() -> Vec<Template> {
    vec![
        README,
        PROJECT,
        GLOSSARY,
        DOMAIN_MAP,
        STAKEHOLDERS,
        ACTORS,
        PROCESSES,
        ENTITIES,
        BUSINESS_RULES,
        EVENTS,
        INTEGRATIONS,
        UNCERTAINTIES,
        CONFLICTS,
        DECISIONS,
        SOURCES,
        EVIDENCE,
        PROTOCOL,
        REQUIREMENTS,
        NON_FUNCTIONAL,
        CONSTRAINTS,
        BASELINES,
        USE_CASES,
        SPECS_README,
        PLANS,
        TASKS,
        CONTRACTS,
        ARCHITECTURE,
        ADRS,
        SCENARIOS,
        ACCEPTANCE_TESTS,
        DRIFT,
        AUDIT,
        CHANGES_README,
    ]
}

/// Templates added only by the `full` profile.
pub fn opportunity() -> Vec<Template> {
    vec![
        OPPORTUNITIES,
        ASSUMPTIONS,
        CONTRADICTIONS,
        CONCEPTS,
        HYPOTHESES,
        EXPERIMENTS,
        CONCEPT_DECISIONS,
    ]
}

/// The template set for a profile.
pub fn for_profile(profile: Profile) -> Vec<Template> {
    let mut set = core();
    if profile.has_opportunity() {
        set.extend(opportunity());
    }
    if !profile.has_domain() {
        // LEAN assumes an external domain model; do not scaffold empty
        // registries that would then read as "we have no domain".
        set.retain(|t| {
            !matches!(
                t.path,
                "domain/glossary.md"
                    | "domain/domain-map.md"
                    | "domain/stakeholders.md"
                    | "domain/actors.md"
                    | "domain/processes.md"
                    | "domain/entities.md"
                    | "domain/business-rules.md"
                    | "domain/events.md"
                    | "domain/integrations.md"
                    | "domain/uncertainties.md"
                    | "domain/conflicts.md"
                    | "domain/decisions.md"
                    | "domain/research/sources.md"
                    | "domain/research/evidence.md"
                    | "domain/research/protocol.md"
            )
        });
    }
    set
}

/// Every template, regardless of profile. Used by the docs generator.
pub fn all() -> Vec<Template> {
    let mut set = core();
    set.extend(opportunity());
    set
}
