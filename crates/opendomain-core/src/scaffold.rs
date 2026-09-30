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

/// Marker files or directories whose presence suggests the project already
/// has an implementation, so `--repository greenfield` is probably wrong.
///
/// Checked by name only, never by content: `od init` must stay fast and
/// must not parse user code to decide what the project is.
const BROWNFIELD_MARKERS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "pom.xml",
    "build.gradle",
    "docker-compose.yml",
    "docker-compose.yaml",
    "prisma/schema.prisma",
    "services",
    "src",
    "docs",
];

/// Names of brownfield markers found under `root`.
///
/// Empty means no implementation signal was detected. Non-empty does not
/// prove the project is brownfield — it only means `od init --repository
/// greenfield` deserves a warning pointing at `--repository brownfield`.
/// The caller decides; this function only reports what exists.
pub fn brownfield_signals(root: &Path) -> Vec<String> {
    BROWNFIELD_MARKERS
        .iter()
        .filter(|m| root.join(m).exists())
        .map(|m| m.to_string())
        .collect()
}

/// Strips `<!-- ... -->` sections, mirroring the parser: template examples
/// live inside comments and must not read as declared artifacts. An
/// unterminated comment discards the tail, exactly as the parser does.
///
/// Used by tests to distinguish a live `## DOM-` heading from the template's
/// commented example.
#[allow(dead_code)]
fn strip_html_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `true` when `text` declares a live (non-comment) heading with `prefix`,
/// e.g. an actual `## DOM-001` rather than the template's commented example.
#[allow(dead_code)]
fn has_live_heading(text: &str, prefix: &str) -> bool {
    strip_html_comments(text)
        .lines()
        .any(|l| l.trim_start().starts_with(prefix))
}

/// One file in the scaffold.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    /// Path relative to the `opendomain/` directory.
    pub path: &'static str,
    /// The literal file contents.
    pub contents: &'static str,
}

/// Agent integration files, written to the project root rather than into
/// `opendomain/`.
///
/// These exist because a workspace nobody's agent knows about is a workspace the
/// discipline does not reach. `od init` creates forty-one registries and, without
/// these, produces a project where an assistant has no way to learn that the
/// epistemic labels are load-bearing — which is the case this framework is
/// specifically about.
///
/// Paths are relative to the project root, not to `opendomain/`, and the two are
/// written by different code paths. `AGENTS.md` is portable and read by several
/// agents with no configuration; `.opencode/` wires the same discipline into one
/// specific tool with slash commands and subagents.
pub mod agent_contract {
    use super::Template;

    /// `AGENTS.md` at the project root.
    pub const AGENTS: Template = Template {
        path: "AGENTS.md",
        contents: include_str!("../templates/agent/AGENTS.md"),
    };

    /// `.opencode/command/od-status.md`.
    pub const OD_STATUS: Template = Template {
        path: ".opencode/command/od-status.md",
        contents: include_str!("../templates/agent/.opencode/command/od-status.md"),
    };

    /// `.opencode/command/od-gate.md`.
    pub const OD_GATE: Template = Template {
        path: ".opencode/command/od-gate.md",
        contents: include_str!("../templates/agent/.opencode/command/od-gate.md"),
    };

    /// `.opencode/command/od-trace.md`.
    pub const OD_TRACE: Template = Template {
        path: ".opencode/command/od-trace.md",
        contents: include_str!("../templates/agent/.opencode/command/od-trace.md"),
    };

    /// `.opencode/command/od-impact.md`.
    pub const OD_IMPACT: Template = Template {
        path: ".opencode/command/od-impact.md",
        contents: include_str!("../templates/agent/.opencode/command/od-impact.md"),
    };

    /// `.opencode/command/od-audit.md`.
    pub const OD_AUDIT: Template = Template {
        path: ".opencode/command/od-audit.md",
        contents: include_str!("../templates/agent/.opencode/command/od-audit.md"),
    };

    /// `.opencode/command/od-record.md`.
    pub const OD_RECORD: Template = Template {
        path: ".opencode/command/od-record.md",
        contents: include_str!("../templates/agent/.opencode/command/od-record.md"),
    };

    /// `.opencode/command/od-project.md`.
    pub const OD_PROJECT: Template = Template {
        path: ".opencode/command/od-project.md",
        contents: include_str!("../templates/agent/.opencode/command/od-project.md"),
    };

    /// `.opencode/command/od-research.md`.
    pub const OD_RESEARCH: Template = Template {
        path: ".opencode/command/od-research.md",
        contents: include_str!("../templates/agent/.opencode/command/od-research.md"),
    };

    /// `.opencode/command/od-domain.md`.
    pub const OD_DOMAIN: Template = Template {
        path: ".opencode/command/od-domain.md",
        contents: include_str!("../templates/agent/.opencode/command/od-domain.md"),
    };

    /// `.opencode/command/od-requirements.md`.
    pub const OD_REQUIREMENTS: Template = Template {
        path: ".opencode/command/od-requirements.md",
        contents: include_str!("../templates/agent/.opencode/command/od-requirements.md"),
    };

    /// `.opencode/command/od-opportunity.md`.
    pub const OD_OPPORTUNITY: Template = Template {
        path: ".opencode/command/od-opportunity.md",
        contents: include_str!("../templates/agent/.opencode/command/od-opportunity.md"),
    };

    /// `.opencode/command/od-spec.md`.
    pub const OD_SPEC: Template = Template {
        path: ".opencode/command/od-spec.md",
        contents: include_str!("../templates/agent/.opencode/command/od-spec.md"),
    };

    /// `.opencode/command/od-plan.md`.
    pub const OD_PLAN: Template = Template {
        path: ".opencode/command/od-plan.md",
        contents: include_str!("../templates/agent/.opencode/command/od-plan.md"),
    };

    /// `.opencode/command/od-implement.md`.
    pub const OD_IMPLEMENT: Template = Template {
        path: ".opencode/command/od-implement.md",
        contents: include_str!("../templates/agent/.opencode/command/od-implement.md"),
    };

    /// `.opencode/command/od-complete.md`.
    pub const OD_COMPLETE: Template = Template {
        path: ".opencode/command/od-complete.md",
        contents: include_str!("../templates/agent/.opencode/command/od-complete.md"),
    };

    /// `.opencode/agent/domain-researcher.md`.
    pub const DOMAIN_RESEARCHER: Template = Template {
        path: ".opencode/agent/domain-researcher.md",
        contents: include_str!("../templates/agent/.opencode/agent/domain-researcher.md"),
    };

    /// `.opencode/agent/spec-implementer.md`.
    pub const SPEC_IMPLEMENTER: Template = Template {
        path: ".opencode/agent/spec-implementer.md",
        contents: include_str!("../templates/agent/.opencode/agent/spec-implementer.md"),
    };

    /// `.opencode/agent/context-completer.md`.
    pub const CONTEXT_COMPLETER: Template = Template {
        path: ".opencode/agent/context-completer.md",
        contents: include_str!("../templates/agent/.opencode/agent/context-completer.md"),
    };

    /// `.opencode/skills/opendomain/SKILL.md` — index.
    pub const SKILL: Template = Template {
        path: ".opencode/skills/opendomain/SKILL.md",
        contents: include_str!("../templates/agent/.opencode/skills/opendomain/SKILL.md"),
    };

    /// `.opencode/skills/opendomain-epistemics/SKILL.md` — les 7 valeurs + promotions.
    pub const SKILL_EPISTEMICS: Template = Template {
        path: ".opencode/skills/opendomain-epistemics/SKILL.md",
        contents: include_str!("../templates/agent/.opencode/skills/opendomain-epistemics/SKILL.md"),
    };

    /// `.opencode/skills/opendomain-artifacts/SKILL.md` — format, ids, relations.
    pub const SKILL_ARTIFACTS: Template = Template {
        path: ".opencode/skills/opendomain-artifacts/SKILL.md",
        contents: include_str!("../templates/agent/.opencode/skills/opendomain-artifacts/SKILL.md"),
    };

    /// `.opencode/skills/opendomain-research/SKILL.md` — fan-out, triangulation.
    pub const SKILL_RESEARCH: Template = Template {
        path: ".opencode/skills/opendomain-research/SKILL.md",
        contents: include_str!("../templates/agent/.opencode/skills/opendomain-research/SKILL.md"),
    };

    /// `.opencode/skills/opendomain-gates/SKILL.md` — résumé-first, slug → action.
    pub const SKILL_GATES: Template = Template {
        path: ".opencode/skills/opendomain-gates/SKILL.md",
        contents: include_str!("../templates/agent/.opencode/skills/opendomain-gates/SKILL.md"),
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
            SPEC_IMPLEMENTER,
            CONTEXT_COMPLETER,
            SKILL,
            SKILL_EPISTEMICS,
            SKILL_ARTIFACTS,
            SKILL_RESEARCH,
            SKILL_GATES,
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
///
/// `od init` installs only: registries + `opendomain.toml` + agent contract.
/// Context is completed afterwards by `/od-project` (context-completer agent),
/// never at init time.
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

    // No context recording here by design: `od init` installs the workspace
    // and the agent contract, nothing more. `/od-project` (context-completer)
    // fills `project.md`, `[project]`, `DOM-001`/`STK-001` from the repository,
    // external research, then user answers.
    write_agent_contract(opendomain_dir, force, &mut report)?;

    Ok(report)
}

/// Writes the agent instruction files, which live at the project root rather
/// than inside `opendomain/`.
///
/// The root is taken as the parent of the workspace directory, which is where
/// `od init` puts it. If the workspace directory has no parent — a bare
/// relative name — the files are written relative to the current directory
/// instead, and this is reported rather than guessed at, because an agent
/// contract written to the wrong place is worse than one not written.
///
/// `AGENTS.md` is treated differently from the rest: it is skipped when it
/// already exists, even under `--force`. The namespaced files cannot collide
/// with anything the project owns; a root-level `AGENTS.md` can, and
/// overwriting instructions this tool did not write is not a decision `od init`
/// should make on the project's behalf. `--force` overwrites the `.opencode/`
/// files, which the tool owns outright.
fn write_agent_contract(opendomain_dir: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    let root = opendomain_dir.parent().unwrap_or_else(|| Path::new("."));

    for template in agent_contract::all() {
        let target = root.join(template.path);
        let exists = target.exists();

        // An existing AGENTS.md belongs to the project. Skipping it is the safe
        // reading of "never overwrite", and the report says so, so a user who
        // expected the framework instructions there learns they were not written
        // rather than assuming they were.
        if exists && (agent_contract::is_root_level(&template) || !force) {
            report.skipped.push(template.path.to_string());
            continue;
        }

        write(&target, template.contents)?;
        report.agent_files.push(template.path.to_string());
    }

    Ok(())
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
    fn template_comments_are_balanced() {
        // An unterminated `<!--` makes the parser discard everything after it,
        // so a registry whose example is never closed also swallows every
        // artifact a user adds. Thirty templates shipped in that state: the
        // example was hidden as intended and so was the entire registry. This
        // test exists so that cannot come back silently.
        for t in templates::all() {
            assert_eq!(
                t.contents.matches("<!--").count(),
                t.contents.matches("-->").count(),
                "{} has unbalanced HTML comments, so its guidance section and any \
                 artifact a user appends will be stripped as comment text",
                t.path
            );
        }
    }

    #[test]
    fn template_guidance_sections_are_not_swallowed() {
        // The complement of the balance test: a balanced comment that is closed
        // in the wrong place still hides the guidance. Confirm that the closing
        // marker precedes the first non-example heading in every template that
        // opens a comment at all.
        for t in templates::all() {
            let Some(open) = t.contents.find("<!--") else {
                continue;
            };
            let body = &t.contents[open..];
            let Some(close) = body.find("-->") else {
                continue; // balance test reports this
            };
            let after = &body[close + 3..];
            let guidance = after
                .lines()
                .find(|l| l.starts_with("## "))
                .unwrap_or_default();
            assert!(
                !guidance.is_empty(),
                "{} closes its comment at the very end, hiding all guidance",
                t.path
            );
        }
    }

    #[test]
    fn agent_files_land_at_the_project_root_not_inside_the_workspace() {
        let dir = scratch("agent-root");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        let report = scaffold(&od_dir, &cfg, false).unwrap();

        assert!(od_dir.join("AGENTS.md").exists() == false);
        assert!(dir.join("AGENTS.md").exists());
        assert!(dir.join(".opencode/skills/opendomain/SKILL.md").exists());
        assert!(report.agent_files.iter().any(|p| p == "AGENTS.md"));

        // The namespaced files must not land inside the workspace, or `od`'s own
        // scanner would read them as registries.
        assert!(!od_dir.join(".opencode").exists());
    }

    #[test]
    fn an_existing_agents_md_is_never_replaced() {
        // The project's own instructions are not the tool's to overwrite, and
        // `--force` does not change that: the flag exists to refresh files `od`
        // wrote, not to claim a file it did not.
        let dir = scratch("agents-existing");
        std::fs::write(dir.join("AGENTS.md"), "# our own instructions\n").unwrap();
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);

        for force in [false, true] {
            scaffold(&od_dir, &cfg, force).unwrap();
            assert_eq!(
                std::fs::read_to_string(dir.join("AGENTS.md")).unwrap(),
                "# our own instructions\n",
                "AGENTS.md was overwritten with force={force}"
            );
        }
    }

    #[test]
    fn re_running_init_adds_missing_agent_files_without_touching_registries() {
        let dir = scratch("agent-rerun");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);

        scaffold(&od_dir, &cfg, false).unwrap();
        std::fs::remove_dir_all(dir.join(".opencode")).unwrap();
        std::fs::write(od_dir.join("project.md"), "edited by a person\n").unwrap();

        let report = scaffold(&od_dir, &cfg, false).unwrap();
        assert!(dir.join(".opencode/command/od-gate.md").exists());
        assert_eq!(
            std::fs::read_to_string(od_dir.join("project.md")).unwrap(),
            "edited by a person\n"
        );
        assert!(!report.is_noop());
    }

    #[test]
    fn every_agent_template_is_non_empty() {
        for t in agent_contract::all() {
            assert!(!t.contents.trim().is_empty(), "{} is empty", t.path);
        }
    }

    #[test]
    fn the_agent_contract_names_only_commands_that_exist() {
        // The contract tells an agent which commands to reach for. A command
        // named here that the CLI does not have is the contract lying, which is
        // worse than the contract being silent: an agent that runs it and finds
        // nothing has been told, authoritatively, that the workspace is
        // self-describing when it is not.
        //
        // The source is scraped rather than linked, because the CLI is a
        // separate crate that depends on this one, not the reverse. The check
        // is deliberately narrow: a clap variant of that name in the `Command`
        // enum.
        for cmd in contract_commands() {
            assert!(
                cli_declares(&cmd),
                "the agent contract names `od {cmd}`, which the CLI does not declare; \
                 the contract's command table must be checked against the `Command` enum"
            );
        }
    }

    #[test]
    fn the_agent_contract_documents_every_command() {
        // The converse of the check above, and the one that was missing. A
        // contract that lists some of the tool's commands and silently omits the
        // rest is the more likely defect, because omission produces no error: a
        // half-documented tool reads as a fully documented one. An agent never
        // learns that `od graph` or `od propose` exist.
        //
        // `help` is clap's own, not a command of ours.
        let documented = contract_commands();
        for cmd in cli_commands() {
            if cmd == "Help" {
                continue;
            }
            assert!(
                documented.contains(&cmd.to_lowercase()),
                "`od {}` exists but the agent contract does not mention it; an agent \
                 reading AGENTS.md cannot discover it",
                cmd.to_lowercase()
            );
        }
    }

    #[test]
    fn the_agent_contract_names_every_gate_and_its_question() {
        // The gate table in the contract answers "what does this stage want?",
        // which is the question an agent has before it runs anything. If the
        // table drifts from `Gate::ALL`, an agent is being told to satisfy a
        // stage by a question the tool no longer asks.
        let contract = agent_contract::AGENTS.contents;
        for g in crate::gates::Gate::ALL {
            let row = format!("| {} | `{}` | {} |", g.number, g.name, g.question);
            assert!(
                contract.contains(&row),
                "the agent contract's gate table does not carry gate {} `{}` with its \
                 question; the tool's own text is \"{}\"",
                g.number,
                g.name,
                g.question
            );
        }
    }

    #[test]
    fn the_agent_contract_states_the_real_rule_counts() {
        // The contract quotes a number so an agent knows whether `od reference`
        // is truncated. A wrong count makes the reference look incomplete when
        // it is not, which is the same failure as an undocumented command.
        let contract = agent_contract::AGENTS.contents;
        let blocking = crate::gates::RULES
            .iter()
            .filter(|r| r.severity == crate::gates::Severity::Blocking)
            .count();
        let total = crate::gates::RULES.len();
        let expected = format!("**{total} rules**");
        assert!(
            contract.contains(&expected),
            "the agent contract does not state the rule count as {expected}"
        );
        let blocking_expected = format!("{blocking} blocking");
        assert!(
            contract.contains(&blocking_expected),
            "the agent contract does not state the blocking count as {blocking_expected}"
        );
    }

    /// Every command name the shipped agent contract mentions, lowercased.
    ///
    /// Deliberately narrow. A run of text is read as a command only if the
    /// first word after `od` is a plain word: `--help` and `--json` are flags,
    /// and `od init --profile lean` names the profile `lean`, not a command.
    /// Treating those as commands is how a completeness check starts crying
    /// wolf, and a check that cries wolf gets ignored.
    fn contract_commands() -> Vec<String> {
        let contract = agent_contract::AGENTS.contents;
        let mut found: Vec<String> = contract
            .lines()
            .filter_map(|line| {
                line.split("`od ")
                    .skip(1)
                    .filter_map(|rest| rest.split(['`', ' ', '|', ',']).next())
                    .filter(|w| {
                        !w.is_empty()
                            && !w.starts_with('-')
                            && w.chars().all(|c| c.is_ascii_alphanumeric())
                    })
                    .map(str::to_lowercase)
                    .next()
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// The command variants the CLI's `Command` enum declares, in source order.
    ///
    /// The enum body is bounded by its closing brace at the start of a line.
    /// Without that bound the search runs on into the rest of the file and
    /// picks up every capitalised `Args` struct after it, which then have to be
    /// explained away as "commands" — the kind of noise that makes a check
    /// useless.
    fn cli_commands() -> Vec<String> {
        let src = fs::read_to_string("../opendomain-cli/src/main.rs")
            .expect("the CLI source is part of the workspace");
        let body = src
            .split("enum Command {")
            .nth(1)
            .expect("the CLI declares a `Command` enum");
        let end = body
            .lines()
            .position(|l| l.trim_start().starts_with('}'))
            .expect("the `Command` enum is closed");
        body.lines()
            .take(end)
            .map(str::trim)
            .filter_map(|l| l.split('(').next())
            .map(str::trim)
            .filter(|l| {
                !l.is_empty()
                    && !l.starts_with('#')
                    && l.chars().next().is_some_and(char::is_uppercase)
            })
            .map(|l| l.trim_end_matches(',').to_string())
            .collect()
    }

    fn cli_declares(cmd: &str) -> bool {
        let want = capitalise(cmd);
        cli_commands().iter().any(|v| *v == want)
    }

    fn capitalise(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }

    #[test]
    fn emitted_agent_contract_matches_templates() {
        // What `od init` writes must be byte-for-byte the shipped templates:
        // otherwise the framework's own contract drifts from what every other
        // project receives — the exact defect this file set exists to prevent.
        //
        // (An earlier version of this test compared against copies at the
        // repository root, which were never committed, so it failed on every
        // fresh clone. Comparing the emission round-trip instead keeps the
        // invariant without requiring the repo to dogfood its own workspace.)
        let dir = scratch("contract-roundtrip");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        scaffold(&od_dir, &cfg, false).unwrap();
        for t in agent_contract::all() {
            if agent_contract::is_root_level(&t) {
                continue; // `AGENTS.md` belongs to the project, not the tool.
            }
            let emitted = fs::read_to_string(dir.join(t.path)).unwrap_or_else(|e| {
                panic!("`od init` did not emit {} ({e})", t.path);
            });
            assert_eq!(
                emitted, t.contents,
                "{} as emitted differs from its shipped template",
                t.path
            );
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

    #[test]
    fn brownfield_signals_detect_existing_implementation() {
        let dir = scratch("signals-empty");
        assert!(brownfield_signals(&dir).is_empty());

        let dir = scratch("signals-code");
        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("services")).unwrap();
        let signals = brownfield_signals(&dir);
        assert!(signals.contains(&"package.json".to_string()));
        assert!(signals.contains(&"services".to_string()));
        assert!(!signals.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn init_installs_only_and_records_no_context() {
        // `od init` installs the workspace + agent contract and records
        // nothing: context is completed by `/od-project` (context-completer),
        // never at init time. Even with `[project]` keys present, no DOM/STK
        // artifact is invented.
        let dir = scratch("init-no-context");
        let od = dir.join("opendomain");
        let mut cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        cfg.project.insert("purpose".into(), "Coach sellers".into());
        scaffold(&od, &cfg, false).unwrap();
        let map = fs::read_to_string(od.join("domain/domain-map.md")).unwrap();
        assert!(!has_live_heading(&map, "## DOM-"));
        let stk = fs::read_to_string(od.join("domain/stakeholders.md")).unwrap();
        assert!(!has_live_heading(&stk, "## STK-"));
        let project = fs::read_to_string(od.join("project.md")).unwrap();
        assert!(!project.contains("<!-- od-inventory -->"));
    }

    #[test]
    fn context_completer_is_report_bound_and_writes_context() {
        // The context-completer fills project.md + [project] + DOM/STK and
        // asks the user for the rest. Its body must bind it to repo reading,
        // keyword-driven external research, and human-only gaps — otherwise it
        // is a second researcher with no completion contract.
        let body = agent_contract::CONTEXT_COMPLETER.contents;
        assert!(body.contains("Phase 1"), "must read the repository first");
        assert!(body.contains("Phase 2"), "must run external research from keywords");
        assert!(body.contains("Phase 4"), "must ask the user for the rest");
        assert!(body.contains("**Human-only gaps**"), "report contract: gaps section");
        assert!(body.contains("INFERRED"), "code findings are INFERRED at best");
        assert!(
            agent_contract::all().iter().any(|t| t.path.ends_with("context-completer.md")),
            "context-completer must be emitted by `od init`"
        );
        assert!(
            !agent_contract::all().iter().any(|t| t.path.ends_with("brownfield-discoverer.md")),
            "brownfield-discoverer must be gone"
        );
    }

    #[test]
    fn gate_commands_cover_all_nine_gates() {
        // One useful command per gate stage: project(0), research(1),
        // domain(2), requirements(3), opportunity(4), spec(5), plan(6),
        // implement(7), complete(8). A missing stage command means the
        // framework's full potential is undiscoverable from slash commands.
        for path in [
            ".opencode/command/od-project.md",
            ".opencode/command/od-research.md",
            ".opencode/command/od-domain.md",
            ".opencode/command/od-requirements.md",
            ".opencode/command/od-opportunity.md",
            ".opencode/command/od-spec.md",
            ".opencode/command/od-plan.md",
            ".opencode/command/od-implement.md",
            ".opencode/command/od-complete.md",
        ] {
            assert!(
                agent_contract::all().iter().any(|t| t.path == path),
                "{path} must be emitted by `od init`"
            );
        }
    }

    #[test]
    fn domain_researcher_fans_out_by_keyword_cluster() {
        // The researcher must delegate in parallel by keyword cluster and
        // triangulate: single-source restatement is not corroboration.
        let body = agent_contract::DOMAIN_RESEARCHER.contents;
        assert!(body.contains("Fan out"), "must fan out in parallel");
        assert!(body.contains("keyword"), "must work from keyword clusters");
        assert!(body.contains("single-source") || body.contains("Single-source"));
        assert!(body.contains("Not claimed"));
    }

    #[test]
    fn focused_skills_exist_and_are_called() {
        // The framework's strength lives in callable skills, not in one
        // generic file nobody loads: epistemics, artifacts, research, gates.
        // Each command/agent states what it calls, so the call graph is
        // reviewable rather than aspirational.
        for path in [
            ".opencode/skills/opendomain-epistemics/SKILL.md",
            ".opencode/skills/opendomain-artifacts/SKILL.md",
            ".opencode/skills/opendomain-research/SKILL.md",
            ".opencode/skills/opendomain-gates/SKILL.md",
        ] {
            assert!(
                agent_contract::all().iter().any(|t| t.path == path),
                "{path} must be emitted by `od init`"
            );
        }
        let index = agent_contract::SKILL.contents;
        assert!(index.contains("opendomain-epistemics"));
        assert!(index.contains("opendomain-artifacts"));
        assert!(index.contains("opendomain-research"));
        assert!(index.contains("opendomain-gates"));

        // Inter-agent calls are declared, not implied.
        assert!(agent_contract::CONTEXT_COMPLETER.contents.contains("domain-researcher"));
        assert!(agent_contract::SPEC_IMPLEMENTER.contents.contains("Called by"));

        // Summary-first kills the verbatim-noise: every command states it.
        for t in agent_contract::all() {
            if t.path.starts_with(".opencode/command/") {
                assert!(
                    t.contents.contains("résumé-first") || t.contents.contains("Verdict"),
                    "{} must render summary-first, not verbatim-noise",
                    t.path
                );
            }
        }

        // od-complete routes to od-audit instead of re-rendering the chain.
        assert!(agent_contract::OD_COMPLETE.contents.contains("/od-audit"));
        assert!(agent_contract::OD_AUDIT.contents.contains("PAS"));
    }
}
