//! Workspace configuration, read from `opendomain/opendomain.toml`.

use crate::error::{OdError, Result};
use crate::model::vocab::{AuthorityModel, Profile, RepositoryKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The parsed contents of `opendomain/opendomain.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config schema version, so future migrations are detectable.
    #[serde(default = "default_version")]
    pub version: u32,

    /// One-line description of the project.
    #[serde(default)]
    pub name: String,

    /// Which parts of the framework this project opts into.
    #[serde(default)]
    pub profile: Profile,

    /// How specifications relate to code.
    #[serde(default)]
    pub authority: AuthorityModel,

    /// Whether the project starts from nothing, from code, or from both.
    #[serde(default)]
    pub repository: RepositoryKind,

    /// Free-form project metadata surfaced by `opendomain context`.
    #[serde(default)]
    pub project: BTreeMap<String, String>,

    /// Paths, relative to the `opendomain/` directory, that the workspace scans.
    ///
    /// Defaults cover every registry; narrow this when a project deliberately
    /// keeps part of its knowledge outside the framework.
    #[serde(default = "default_scan_paths")]
    pub scan: Vec<String>,

    /// Path prefixes excluded from scanning.
    #[serde(default = "default_exclude_paths")]
    pub exclude: Vec<String>,

    /// Failing gate names that must not block. For example `["gate-opportunity"]`.
    #[serde(default)]
    pub advisory_gates: Vec<String>,

    /// Extensionless TOML files holding per-kind field requirements.
    ///
    /// Mostly an escape hatch for projects with unusual vocabularies; the
    /// built-in schema is used when empty.
    #[serde(default)]
    pub schemas: BTreeMap<String, String>,

    /// The verification subsystem's settings (Section 9/10 of the framework
    /// paper). Absent sections default to the paper's values; `provision`
    /// reads them, nothing in the gates depends on them (the §9/§10 checks
    /// are commands, not gate rules — a gate that fires on every requirement
    /// before the artifacts it checks can be generated is the defect F10
    /// explicitly warns about).
    #[serde(default)]
    pub verification: VerificationConfig,

    /// The harness's at-rest settings — `[gates]` (strictness, retry budget),
    /// `[sandbox]` (the criteria commands' allowlist) and `[harness]` (the
    /// node-agents' pinned model and step budget). One config file
    /// for the whole workspace: the harness reads these, there is no second
    /// config to diverge from this one.
    #[serde(default)]
    pub gates: GatesConfig,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub harness: HarnessConfig,
}

/// `[gates]` — the runner's propagation policy (defaults = the paper's).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GatesConfig {
    /// No `y_i` propagates without a green gate.
    #[serde(default = "default_true")]
    pub strict: bool,
    /// Local retries before escalation to typed attribution.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

/// `[sandbox]` — the logical sandbox for the criteria `commands = [...]`.
///
/// Honesty: allowlist + cwd bounded to the repository, not OS isolation — a
/// container remains the only real wall. Ships ON: a green node whose command
/// "ran somewhere" is a door left open.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sandbox_allow")]
    pub allow: Vec<String>,
}

/// `[harness]` — the node-agents' execution bindings, projected into every
/// `.opencode/agent/` frontmatter at `opendomain build` time, reviewable
/// before any run. `model` pins `provider/model` per node-agent, so a quota
/// death on the CLI's default model can no longer kill a whole run; when
/// absent, the `OPENCODE_MODEL` environment variable is read at build time.
/// `steps` bounds each node: on the final step OpenCode withdraws the tools
/// and asks the model to summarize — and the gate judges that summary like
/// any other output, so a node can no longer burn a run in silence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HarnessConfig {
    /// `provider/model`, optionally `provider/model#variant`, baked into each
    /// node-agent's frontmatter. Absent → `OPENCODE_MODEL` at build time,
    /// else OpenCode's default (a subagent inherits its parent's model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Positive step budget per node-agent (default 40). `steps = 0` is
    /// refused: a zero budget can never emit a summary, the failure would be
    /// manufactured by the config, not observed by the gate.
    #[serde(default = "default_harness_steps", skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
}

fn default_harness_steps() -> Option<u32> {
    Some(40)
}

impl Default for HarnessConfig {
    fn default() -> Self {
        HarnessConfig { model: None, steps: default_harness_steps() }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_attempts() -> u32 {
    3
}

fn default_sandbox_allow() -> Vec<String> {
    ["cargo", "npm", "pytest", "python", "git", "rustc"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for GatesConfig {
    fn default() -> Self {
        GatesConfig { strict: true, max_attempts: default_max_attempts() }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        SandboxConfig { enabled: true, allow: default_sandbox_allow() }
    }
}

/// `[verification]` — how the subsystem behaves for this workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct VerificationConfig {
    /// §10.13: provisioning is refused when fewer behavioural acceptance
    /// criteria than this exist ("BDD for one scenario is indirection with
    /// no payoff").
    #[serde(default = "default_min_acceptance_criteria")]
    pub minimal_acceptance_criteria: u32,
    /// §10.5: the BDD-off profile — a first-class, declared mode in which the
    /// team accepts the indirection cost and prefers acceptance-criterion
    /// identifiers carried in test names. Declaring it is a decision record;
    /// using it is not a violation. Under this profile, provisioning is
    /// refused and the alternative mechanism goes on record.
    #[serde(default)]
    pub bdd_off: bool,
    /// The change-set is experimental and non-production: BDD is refused for
    /// it (§10.13).
    #[serde(default)]
    pub experimental: bool,
}

fn default_min_acceptance_criteria() -> u32 {
    3
}

impl Default for VerificationConfig {
    fn default() -> Self {
        VerificationConfig {
            minimal_acceptance_criteria: default_min_acceptance_criteria(),
            bdd_off: false,
            experimental: false,
        }
    }
}

fn default_version() -> u32 {
    1
}

fn default_scan_paths() -> Vec<String> {
    [
        "domain",
        "opportunity",
        "requirements",
        "use-cases",
        "specs",
        "plans",
        "tasks",
        "contracts",
        "architecture",
        "validation",
        "changes",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_exclude_paths() -> Vec<String> {
    ["changes/archive", ".opendomain-cache"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: default_version(),
            name: String::new(),
            profile: Profile::Core,
            authority: AuthorityModel::SpecAnchored,
            repository: RepositoryKind::Greenfield,
            project: BTreeMap::new(),
            scan: default_scan_paths(),
            exclude: default_exclude_paths(),
            advisory_gates: Vec::new(),
            schemas: BTreeMap::new(),
            verification: VerificationConfig::default(),
            gates: GatesConfig::default(),
            sandbox: SandboxConfig::default(),
            harness: HarnessConfig::default(),
        }
    }
}

impl Config {
    /// Parses a configuration from TOML text.
    pub fn parse(text: &str) -> Result<Config> {
        let cfg: Config = toml::from_str(text).map_err(OdError::Config)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Reads a configuration from disk.
    pub fn load(opendomain_dir: &Path) -> Result<Config> {
        let path = opendomain_dir.join("opendomain.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OdError::MissingConfig(opendomain_dir.to_path_buf())
            } else {
                OdError::read(path, e)
            }
        })?;
        Config::parse(&text)
    }

    /// Renders the configuration as TOML.
    pub fn render(&self) -> String {
        toml::to_string_pretty(self).expect("Config is always serializable")
    }

    fn validate(&self) -> Result<()> {
        if self.version > 1 {
            return Err(OdError::ConfigInvalid(format!(
                "config `version` is {} but this build understands at most 1 — upgrade the CLI",
                self.version
            )));
        }
        for p in &self.scan {
            if p.trim().is_empty() {
                return Err(OdError::ConfigInvalid(
                    "`scan` contains an empty path".into(),
                ));
            }
        }
        if let Some(model) = self.harness.model.as_deref() {
            if model.trim().is_empty() {
                return Err(OdError::ConfigInvalid(
                    "`harness.model` is empty — set a `provider/model` or remove the key \
                     (the node-agents then follow `OPENCODE_MODEL` or OpenCode's default)"
                        .into(),
                ));
            }
        }
        if self.harness.steps == Some(0) {
            return Err(OdError::ConfigInvalid(
                "`harness.steps` must be at least 1 — a zero budget can never emit a summary, \
                 the failure would be manufactured by the config instead of observed by the gate"
                    .into(),
            ));
        }
        Ok(())
    }

    /// `true` when the path (relative to `opendomain/`) should be scanned.
    pub fn scans(&self, relative: &Path) -> bool {
        let rel = relative.to_string_lossy().replace('\\', "/");
        if self
            .exclude
            .iter()
            .any(|e| rel == *e || rel.starts_with(&format!("{e}/")))
        {
            return false;
        }
        self.scan
            .iter()
            .any(|s| rel == *s || rel.starts_with(&format!("{s}/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.profile, Profile::Core);
        assert_eq!(cfg.authority, AuthorityModel::SpecAnchored);
        assert!(cfg.scans(Path::new("domain/glossary.md")));
        assert!(cfg.scans(Path::new("specs/inventory/spec.md")));
        assert!(!cfg.scans(Path::new("README.md")));
        assert!(!cfg.scans(Path::new("changes/archive/2026-01-01-x/proposal.md")));
    }

    #[test]
    fn round_trips_through_toml() {
        let mut cfg = Config::default();
        cfg.name = "Acme Warehouse".into();
        cfg.profile = Profile::Full;
        cfg.project.insert("region".into(), "Senegal".into());
        let text = cfg.render();
        let back = Config::parse(&text).unwrap();
        assert_eq!(back.name, "Acme Warehouse");
        assert_eq!(back.profile, Profile::Full);
        assert_eq!(
            back.project.get("region").map(String::as_str),
            Some("Senegal")
        );
    }

    #[test]
    fn rejects_future_versions() {
        let err = Config::parse("version = 99").unwrap_err();
        assert!(format!("{err}").contains("upgrade the CLI"), "{err}");
    }

    #[test]
    fn rejects_unknown_keys() {
        let err = Config::parse("nonsense = true").unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn verification_section_defaults_when_absent() {
        let cfg = Config::parse("").unwrap();
        assert_eq!(cfg.verification.minimal_acceptance_criteria, 3);
        assert!(!cfg.verification.bdd_off);
        assert!(!cfg.verification.experimental);
    }

    #[test]
    fn verification_section_round_trips_and_stays_strict() {
        let cfg = Config::parse(
            "[verification]\nminimal-acceptance-criteria = 5\nbdd-off = true\nexperimental = true\n",
        )
        .unwrap();
        assert_eq!(cfg.verification.minimal_acceptance_criteria, 5);
        assert!(cfg.verification.bdd_off);
        assert!(cfg.verification.experimental);
        // deny_unknown_fields still applies inside the section.
        let err = Config::parse(
            "[verification]\nminimal-acceptance-criteria = 3\nnonsense = true\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn gates_and_sandbox_sections_round_trip_with_defaults() {
        // Absentes → les défauts du papier, dans UNE seule config.
        let cfg = Config::parse("").unwrap();
        assert!(cfg.gates.strict);
        assert_eq!(cfg.gates.max_attempts, 3);
        assert!(cfg.sandbox.enabled);
        assert!(cfg.sandbox.allow.contains(&"cargo".to_string()));
        // Présentes → lues, et strictes sur les clés inconnues.
        let cfg = Config::parse(
            "[gates]\nstrict = false\nmax_attempts = 5\n\n[sandbox]\nenabled = true\nallow = [\"cargo\"]\n",
        )
        .unwrap();
        assert!(!cfg.gates.strict);
        assert_eq!(cfg.gates.max_attempts, 5);
        assert_eq!(cfg.sandbox.allow, vec!["cargo"]);
        let err = Config::parse("[gates]\nnonsense = true\n").unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn harness_section_round_trips_with_defaults() {
        // Absente → le budget de pas du papier de soin V2, aucun modèle
        // épinglé (le nœud suit alors OPENCODE_MODEL ou le défaut OpenCode).
        let cfg = Config::parse("").unwrap();
        assert_eq!(cfg.harness.steps, Some(40));
        assert_eq!(cfg.harness.model, None);
        // Présente → lue, et stricte sur les clés inconnues.
        let cfg = Config::parse(
            "[harness]\nmodel = \"opencode-go/glm-5.3-flash\"\nsteps = 60\n",
        )
        .unwrap();
        assert_eq!(cfg.harness.model.as_deref(), Some("opencode-go/glm-5.3-flash"));
        assert_eq!(cfg.harness.steps, Some(60));
        let err = Config::parse("[harness]\nnonsense = true\n").unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
        // Le rendu (init, repatch --allow) réécrit la section telle quelle —
        // un modèle absent ne se sérialise pas en chaîne vide.
        let text = Config::default().render();
        assert!(text.contains("steps = 40"), "{text}");
        assert!(!text.contains("model"), "{text}");
    }

    #[test]
    fn harness_section_refuses_les_valeurs_fabriquees() {
        // steps = 0 : un budget nul ne peut jamais émettre de résumé — la
        // panne serait fabriquée par la config, pas observée par le gate.
        let err = Config::parse("[harness]\nsteps = 0\n").unwrap_err();
        assert!(err.to_string().contains("at least 1"), "{err}");
        // model vide : épingler du vide est une faute de frappe, pas un choix.
        let err = Config::parse("[harness]\nmodel = \"\"\n").unwrap_err();
        assert!(err.to_string().contains("empty"), "{err}");
    }
}
