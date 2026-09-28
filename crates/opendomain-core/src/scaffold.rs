//! Scaffold generation and the template set.
//!
//! Templates are embedded in the binary so `od init` works from a single
//! executable with no network access and no template checkout. The same
//! templates are exposed through [`templates::all`] so the docs site and the
//! tests can assert on them.

use crate::config::Config;
use crate::error::{OdError, Result};
use crate::ids::IdKind;
use crate::vocab::{AuthorityModel, Profile};
use std::path::Path;

/// One file in the scaffold.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    /// Path relative to the `opendomain/` directory.
    pub path: &'static str,
    /// The literal file contents.
    pub contents: &'static str,
}

/// Every scaffold template.
///
/// Note which registries are present. `od init` writes the ones matching the
/// active profile: a project that has opted out of Stage 2 should not be
/// handed seven empty opportunity files to feel bad about.
pub mod templates {
    use super::{Profile, Template};

    /// `opendomain/project.md` — Phase 0 output.
    pub const PROJECT: Template = Template {
        path: "project.md",
        contents: include_str!("../templates/project.md"),
    };

    /// `opendomain/README.md` — the workspace's own orientation.
    pub const README: Template = Template {
        path: "README.md",
        contents: include_str!("../templates/workspace-README.md"),
    };

    /// `opendomain/domain/glossary.md`.
    pub const GLOSSARY: Template = Template {
        path: "domain/glossary.md",
        contents: include_str!("../templates/registry-glossary.md"),
    };

    /// `opendomain/domain/domain-map.md`.
    pub const DOMAIN_MAP: Template = Template {
        path: "domain/domain-map.md",
        contents: include_str!("../templates/registry-domain-map.md"),
    };

    /// `opendomain/domain/stakeholders.md`.
    pub const STAKEHOLDERS: Template = Template {
        path: "domain/stakeholders.md",
        contents: include_str!("../templates/registry-stakeholders.md"),
    };

    /// `opendomain/domain/actors.md`.
    pub const ACTORS: Template = Template {
        path: "domain/actors.md",
        contents: include_str!("../templates/registry-actors.md"),
    };

    /// `opendomain/domain/processes.md`.
    pub const PROCESSES: Template = Template {
        path: "domain/processes.md",
        contents: include_str!("../templates/registry-processes.md"),
    };

    /// `opendomain/domain/entities.md`.
    pub const ENTITIES: Template = Template {
        path: "domain/entities.md",
        contents: include_str!("../templates/registry-entities.md"),
    };

    /// `opendomain/domain/business-rules.md`.
    pub const BUSINESS_RULES: Template = Template {
        path: "domain/business-rules.md",
        contents: include_str!("../templates/registry-business-rules.md"),
    };

    /// `opendomain/domain/events.md`.
    pub const EVENTS: Template = Template {
        path: "domain/events.md",
        contents: include_str!("../templates/registry-events.md"),
    };

    /// `opendomain/domain/integrations.md`.
    pub const INTEGRATIONS: Template = Template {
        path: "domain/integrations.md",
        contents: include_str!("../templates/registry-integrations.md"),
    };

    /// `opendomain/domain/uncertainties.md`.
    pub const UNCERTAINTIES: Template = Template {
        path: "domain/uncertainties.md",
        contents: include_str!("../templates/registry-uncertainties.md"),
    };

    /// `opendomain/domain/conflicts.md`.
    pub const CONFLICTS: Template = Template {
        path: "domain/conflicts.md",
        contents: include_str!("../templates/registry-conflicts.md"),
    };

    /// `opendomain/domain/decisions.md`.
    pub const DECISIONS: Template = Template {
        path: "domain/decisions.md",
        contents: include_str!("../templates/registry-decisions.md"),
    };

    /// `opendomain/domain/research/sources.md`.
    pub const SOURCES: Template = Template {
        path: "domain/research/sources.md",
        contents: include_str!("../templates/registry-sources.md"),
    };

    /// `opendomain/domain/research/evidence.md`.
    pub const EVIDENCE: Template = Template {
        path: "domain/research/evidence.md",
        contents: include_str!("../templates/registry-evidence.md"),
    };

    /// `opendomain/domain/research/protocol.md`.
    pub const PROTOCOL: Template = Template {
        path: "domain/research/protocol.md",
        contents: include_str!("../templates/research-protocol.md"),
    };

    /// `opendomain/opportunity/opportunities.md`.
    pub const OPPORTUNITIES: Template = Template {
        path: "opportunity/opportunities.md",
        contents: include_str!("../templates/registry-opportunities.md"),
    };

    /// `opendomain/opportunity/assumptions.md`.
    pub const ASSUMPTIONS: Template = Template {
        path: "opportunity/assumptions.md",
        contents: include_str!("../templates/registry-assumptions.md"),
    };

    /// `opendomain/opportunity/contradictions.md`.
    pub const CONTRADICTIONS: Template = Template {
        path: "opportunity/contradictions.md",
        contents: include_str!("../templates/registry-contradictions.md"),
    };

    /// `opendomain/opportunity/concepts.md`.
    pub const CONCEPTS: Template = Template {
        path: "opportunity/concepts.md",
        contents: include_str!("../templates/registry-concepts.md"),
    };

    /// `opendomain/opportunity/hypotheses.md`.
    pub const HYPOTHESES: Template = Template {
        path: "opportunity/hypotheses.md",
        contents: include_str!("../templates/registry-hypotheses.md"),
    };

    /// `opendomain/opportunity/experiments.md`.
    pub const EXPERIMENTS: Template = Template {
        path: "opportunity/experiments.md",
        contents: include_str!("../templates/registry-experiments.md"),
    };

    /// `opendomain/opportunity/decisions.md`.
    pub const CONCEPT_DECISIONS: Template = Template {
        path: "opportunity/decisions.md",
        contents: include_str!("../templates/registry-concept-decisions.md"),
    };

    /// `opendomain/requirements/requirements.md`.
    pub const REQUIREMENTS: Template = Template {
        path: "requirements/requirements.md",
        contents: include_str!("../templates/registry-requirements.md"),
    };

    /// `opendomain/requirements/non-functional.md`.
    pub const NON_FUNCTIONAL: Template = Template {
        path: "requirements/non-functional.md",
        contents: include_str!("../templates/registry-non-functional.md"),
    };

    /// `opendomain/requirements/constraints.md`.
    pub const CONSTRAINTS: Template = Template {
        path: "requirements/constraints.md",
        contents: include_str!("../templates/registry-constraints.md"),
    };

    /// `opendomain/requirements/baselines.md`.
    pub const BASELINES: Template = Template {
        path: "requirements/baselines.md",
        contents: include_str!("../templates/registry-baselines.md"),
    };

    /// `opendomain/use-cases/use-case-model.md`.
    pub const USE_CASES: Template = Template {
        path: "use-cases/use-case-model.md",
        contents: include_str!("../templates/registry-use-cases.md"),
    };

    /// `opendomain/specs/README.md`.
    pub const SPECS_README: Template = Template {
        path: "specs/README.md",
        contents: include_str!("../templates/specs-README.md"),
    };

    /// `opendomain/plans/plans.md`.
    pub const PLANS: Template = Template {
        path: "plans/plans.md",
        contents: include_str!("../templates/registry-plans.md"),
    };

    /// `opendomain/tasks/tasks.md`.
    pub const TASKS: Template = Template {
        path: "tasks/tasks.md",
        contents: include_str!("../templates/registry-tasks.md"),
    };

    /// `opendomain/contracts/contracts.md`.
    pub const CONTRACTS: Template = Template {
        path: "contracts/contracts.md",
        contents: include_str!("../templates/registry-contracts.md"),
    };

    /// `opendomain/architecture/architecture.md`.
    pub const ARCHITECTURE: Template = Template {
        path: "architecture/architecture.md",
        contents: include_str!("../templates/architecture.md"),
    };

    /// `opendomain/architecture/decisions/records.md`.
    pub const ADRS: Template = Template {
        path: "architecture/decisions/records.md",
        contents: include_str!("../templates/registry-adrs.md"),
    };

    /// `opendomain/validation/scenarios.md`.
    pub const SCENARIOS: Template = Template {
        path: "validation/scenarios.md",
        contents: include_str!("../templates/validation-scenarios.md"),
    };

    /// `opendomain/validation/acceptance-tests.md`.
    pub const ACCEPTANCE_TESTS: Template = Template {
        path: "validation/acceptance-tests.md",
        contents: include_str!("../templates/validation-acceptance-tests.md"),
    };

    /// `opendomain/validation/drift.md`.
    pub const DRIFT: Template = Template {
        path: "validation/drift.md",
        contents: include_str!("../templates/validation-drift.md"),
    };

    /// `opendomain/validation/audit.md`.
    pub const AUDIT: Template = Template {
        path: "validation/audit.md",
        contents: include_str!("../templates/validation-audit.md"),
    };

    /// `opendomain/changes/.gitkeep` note explaining the change layout.
    pub const CHANGES_README: Template = Template {
        path: "changes/README.md",
        contents: include_str!("../templates/changes-README.md"),
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
}

/// What `od init` wrote.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct InitReport {
    /// Files created.
    pub created: Vec<String>,
    /// Files that already existed and were left alone.
    pub skipped: Vec<String>,
    /// Agent instruction files created outside `opendomain/`.
    pub agent_files: Vec<String>,
}

impl InitReport {
    /// `true` when nothing was written because the workspace already existed.
    pub fn is_noop(&self) -> bool {
        self.created.is_empty() && self.agent_files.is_empty()
    }
}

/// Writes the scaffold into `opendomain_dir`.
///
/// Existing files are never overwritten: a project that has spent a week
/// curating `domain/glossary.md` must not lose it to a re-run of `od init`.
pub fn scaffold(opendomain_dir: &Path, config: &Config, force: bool) -> Result<InitReport> {
    let mut report = InitReport::default();
    std::fs::create_dir_all(opendomain_dir).map_err(|e| OdError::write(opendomain_dir, e))?;

    let rendered = render_config(config);
    let config_path = opendomain_dir.join("opendomain.toml");
    if force || !config_path.exists() {
        write(&config_path, &rendered)?;
        report.created.push("opendomain.toml".into());
    } else {
        report.skipped.push("opendomain.toml".into());
    }

    for template in templates::for_profile(config.profile) {
        let target = opendomain_dir.join(template.path);
        if target.exists() && !force {
            report.skipped.push(template.path.to_string());
            continue;
        }
        write(&target, template.contents)?;
        report.created.push(template.path.to_string());
    }

    Ok(report)
}

/// Renders `opendomain.toml`, header comments included.
///
/// The body comes from the serializer rather than from string formatting, so
/// the file the CLI writes is guaranteed to be the file the parser accepts.
fn render_config(config: &Config) -> String {
    let mut text = String::new();
    text.push_str("# OpenDomain workspace configuration.\n");
    text.push_str("# Every key is optional; the values below are the defaults.\n");
    text.push_str("# Docs: https://opendomain.dev/reference/configuration\n\n");
    text.push_str(&config.render());
    text
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| OdError::write(parent, e))?;
    }
    std::fs::write(path, contents).map_err(|e| OdError::write(path, e))
}

/// The default configuration for a new project, given a name and a choice of
/// profile.
pub fn default_config(name: &str, profile: Profile, authority: AuthorityModel) -> Config {
    Config {
        name: name.to_string(),
        profile,
        authority,
        ..Config::default()
    }
}

/// The registry file a new artifact of `kind` should be appended to.
///
/// Returns the path relative to the `opendomain/` directory.
pub fn registry_path_for(kind: IdKind) -> &'static str {
    kind.default_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("od-scaffold-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn full_profile_writes_every_template() {
        let dir = scratch("full");
        let od = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Full, AuthorityModel::SpecAnchored);
        let report = scaffold(&od, &cfg, false).unwrap();
        assert!(!report.created.is_empty());
        for t in templates::all() {
            assert!(od.join(t.path).exists(), "{} was not written", t.path);
        }
    }

    #[test]
    fn core_profile_omits_opportunity_registries() {
        let dir = scratch("core");
        let od = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        scaffold(&od, &cfg, false).unwrap();
        assert!(od.join("domain/glossary.md").exists());
        assert!(!od.join("opportunity/concepts.md").exists());
    }

    #[test]
    fn lean_profile_omits_domain_registries() {
        let set = templates::for_profile(Profile::Lean);
        assert!(!set.iter().any(|t| t.path == "domain/glossary.md"));
        assert!(set.iter().any(|t| t.path == "specs/README.md"));
    }

    #[test]
    fn never_overwrites_without_force() {
        let dir = scratch("no-overwrite");
        let od = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        scaffold(&od, &cfg, false).unwrap();
        fs::write(od.join("domain/glossary.md"), "MY CURATED GLOSSARY").unwrap();

        let report = scaffold(&od, &cfg, false).unwrap();
        assert_eq!(
            fs::read_to_string(od.join("domain/glossary.md")).unwrap(),
            "MY CURATED GLOSSARY"
        );
        assert!(report.skipped.contains(&"domain/glossary.md".to_string()));
    }

    #[test]
    fn every_template_is_valid_utf8_and_non_empty() {
        for t in templates::all() {
            assert!(!t.contents.trim().is_empty(), "{} is empty", t.path);
            assert!(t.path.ends_with(".md"), "{} is not markdown", t.path);
        }
    }

    #[test]
    fn rendered_config_round_trips() {
        let mut cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        cfg.project.insert("region".into(), "Senegal".into());
        let text = render_config(&cfg);
        let back = Config::parse(&text).expect("rendered config must parse");
        assert_eq!(back.name, "Acme");
        assert_eq!(back.profile, Profile::Full);
        assert_eq!(back.authority, AuthorityModel::SpecAsSource);
        assert_eq!(
            back.project.get("region").map(String::as_str),
            Some("Senegal")
        );
    }
}
