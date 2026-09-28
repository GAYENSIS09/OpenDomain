//! Workspace configuration, read from `opendomain/opendomain.toml`.

use crate::error::{OdError, Result};
use crate::vocab::{AuthorityModel, Profile, RepositoryKind};
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

    /// Free-form project metadata surfaced by `od context`.
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
    ["changes/archive", ".od-cache"]
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
}
