//! Workspace discovery and artifact indexing.

use crate::artifact::{Artifact, Document};
use crate::config::Config;
use crate::error::{OdError, Result};
use crate::ids::{Id, IdKind};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A located `opendomain/` directory together with its configuration.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The `opendomain/` directory.
    pub root: PathBuf,
    /// The project root, i.e. the parent of `root`.
    pub project_root: PathBuf,
    /// The parsed `opendomain.toml`.
    pub config: Config,
}

impl Workspace {
    /// Walks up from `start` looking for an `opendomain/` directory.
    pub fn discover(start: &Path) -> Result<Workspace> {
        let mut current = start.to_path_buf();
        loop {
            let candidate = current.join("opendomain");
            if candidate.is_dir() {
                return Workspace::open(&candidate);
            }
            if !current.pop() {
                return Err(OdError::WorkspaceNotFound(start.to_path_buf()));
            }
        }
    }

    /// Opens a known `opendomain/` directory.
    pub fn open(opendomain_dir: &Path) -> Result<Workspace> {
        let config = Config::load(opendomain_dir)?;
        let project_root = opendomain_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| opendomain_dir.to_path_buf());
        Ok(Workspace {
            root: opendomain_dir.to_path_buf(),
            project_root,
            config,
        })
    }

    /// Creates a workspace around an in-memory configuration, used before
    /// `opendomain.toml` exists on disk.
    pub fn virtual_root(project_root: &Path, config: Config) -> Workspace {
        Workspace {
            root: project_root.join("opendomain"),
            project_root: project_root.to_path_buf(),
            config,
        }
    }

    /// Resolves a path relative to the `opendomain/` root.
    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    /// Resolves a path relative to the project root.
    pub fn project_path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.project_root.join(relative)
    }

    /// Strips the workspace root, yielding a `/`-separated relative path.
    pub fn relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/")
            .into()
    }

    /// Every Markdown file the configuration says to scan.
    pub fn registry_files(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for rel in &self.config.scan {
            collect_markdown(&self.root.join(rel), &mut out)?;
        }
        for rel in &self.config.exclude {
            prune(&self.root.join(rel), &mut out);
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Every artifact in the workspace, indexed by identifier.
    pub fn load(&self) -> Result<Index> {
        let mut index = Index::default();
        for file in self.registry_files()? {
            let text = std::fs::read_to_string(&file).map_err(|e| OdError::read(&file, e))?;
            let rel = self.relative(&file);
            let doc = crate::parse::parse(&file, rel, &text)?;
            for artifact in &doc.artifacts {
                if let Some(prev) = index.by_id.get(&artifact.id) {
                    return Err(OdError::artifact(
                        &artifact.file,
                        format!(
                            "`{}` is already defined at {}:{}",
                            artifact.id,
                            prev.file.display(),
                            prev.line
                        ),
                    ));
                }
                index.by_id.insert(artifact.id.clone(), artifact.clone());
                index
                    .by_kind
                    .entry(artifact.id.kind())
                    .or_default()
                    .push(artifact.id.clone());
            }
            index.documents.push(doc);
        }
        for ids in index.by_kind.values_mut() {
            ids.sort();
        }
        Ok(index)
    }
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    if dir.is_file() {
        if has_md_extension(dir) {
            out.push(dir.to_path_buf());
        }
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| OdError::read(dir, e))? {
        let entry = entry.map_err(|e| OdError::read(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out)?;
        } else if has_md_extension(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn has_md_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown")
    )
}

fn prune(path: &Path, out: &mut Vec<PathBuf>) {
    out.retain(|p| !p.starts_with(path));
}

/// Every artifact in a workspace, indexed for lookup.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Index {
    /// Artifacts by identifier. Serialised as a map keyed by the id's string
    /// form, because a JSON object cannot have `Id` struct keys.
    #[serde(serialize_with = "serialize_by_id")]
    pub by_id: BTreeMap<Id, Artifact>,
    /// Identifiers grouped by kind. Serialised with `IdKind` keys by name,
    /// because a JSON object cannot have enum keys.
    #[serde(serialize_with = "serialize_by_kind")]
    pub by_kind: BTreeMap<IdKind, Vec<Id>>,
    /// Every parsed document, for whole-file reporting.
    pub documents: Vec<Document>,
}

fn serialize_by_kind<S: serde::Serializer>(
    map: &BTreeMap<IdKind, Vec<Id>>,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut out = s.serialize_map(Some(map.len()))?;
    for (kind, ids) in map {
        out.serialize_entry(kind.prefix(), ids)?;
    }
    out.end()
}

fn serialize_by_id<S: serde::Serializer>(
    map: &BTreeMap<Id, Artifact>,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut out = s.serialize_map(Some(map.len()))?;
    for (id, artifact) in map {
        out.serialize_entry(&id.to_string(), artifact)?;
    }
    out.end()
}

impl Index {
    /// Looks an artifact up by identifier.
    pub fn get(&self, id: &Id) -> Option<&Artifact> {
        self.by_id.get(id)
    }

    /// All identifiers of a kind, sorted.
    pub fn ids(&self, kind: IdKind) -> &[Id] {
        self.by_kind.get(&kind).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// All artifacts of a kind, sorted by identifier.
    pub fn of_kind(&self, kind: IdKind) -> Vec<&Artifact> {
        self.ids(kind)
            .iter()
            .filter_map(|id| self.by_id.get(id))
            .collect()
    }

    /// Identifiers that no loaded artifact defines.
    ///
    /// A dangling link is a traceability hole: `od trace` would stop dead at
    /// it, so it is reported rather than ignored.
    pub fn dangling(&self) -> BTreeMap<Id, Vec<Id>> {
        let mut out: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
        for artifact in self.by_id.values() {
            let missing: Vec<Id> = artifact
                .edges()
                .map(|(_, id)| id)
                .filter(|id| !self.by_id.contains_key(id))
                .collect();
            if !missing.is_empty() {
                out.insert(artifact.id.clone(), missing);
            }
        }
        out
    }

    /// Artifacts that reference an identifier which does not exist.
    pub fn dangling_keys(&self) -> Vec<Id> {
        self.dangling().into_keys().collect()
    }

    /// Total number of artifacts.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// `true` when the workspace holds no artifacts.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("od-ws-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    /// A fresh project directory. `name` must be unique per test: the suite runs
    /// in parallel, and a shared directory makes one test's extra registry file
    /// look like another test's duplicate identifier.
    fn fixture(name: &str) -> PathBuf {
        let project = scratch(name);
        let od = project.join("opendomain");
        fs::create_dir_all(&od).unwrap();
        fs::write(od.join("opendomain.toml"), "version = 1\n").unwrap();
        write(
            &od,
            "domain/glossary.md",
            "# Glossary\n\n## TERM-001 — Purchase order\n\nCanonical term.\n",
        );
        write(
            &od,
            "requirements/requirements.md",
            "## REQ-001 — Do the thing\n\n```toml\nderived-from = [\"TERM-001\"]\n```\n\nbody\n",
        );
        project
    }

    #[test]
    fn discovers_workspace_from_nested_directory() {
        let project = fixture("discover");
        let deep = project.join("src").join("a").join("b");
        fs::create_dir_all(&deep).unwrap();
        let ws = Workspace::discover(&deep).unwrap();
        assert_eq!(ws.root, project.join("opendomain"));
        assert_eq!(ws.project_root, project);
    }

    #[test]
    fn missing_workspace_is_an_error() {
        let dir = scratch("empty");
        let err = Workspace::discover(&dir).unwrap_err();
        assert!(format!("{err}").contains("od init"), "{err}");
    }

    #[test]
    fn loads_and_indexes_artifacts() {
        let project = fixture("load");
        let ws = Workspace::discover(&project).unwrap();
        let index = ws.load().unwrap();
        assert_eq!(index.len(), 2);
        assert_eq!(index.ids(IdKind::Term).len(), 1);
        assert_eq!(index.ids(IdKind::Req).len(), 1);
        assert!(index.dangling().is_empty());
    }

    #[test]
    fn reports_dangling_links() {
        let project = fixture("dangling");
        let od = project.join("opendomain");
        write(
            &od,
            "requirements/requirements.md",
            "## REQ-001 — Do the thing\n\n```toml\nderived-from = [\"EVD-999\"]\n```\n",
        );
        let ws = Workspace::discover(&project).unwrap();
        let index = ws.load().unwrap();
        let dangling = index.dangling();
        assert_eq!(dangling.len(), 1);
        assert!(dangling[&Id::parse("REQ-001").unwrap()].contains(&Id::parse("EVD-999").unwrap()));
    }

    #[test]
    fn rejects_duplicate_ids_across_files() {
        let project = fixture("dupes");
        let od = project.join("opendomain");
        write(&od, "domain/entities.md", "## TERM-001 — Duplicate\n\nx\n");
        let ws = Workspace::discover(&project).unwrap();
        let err = ws.load().unwrap_err();
        assert!(format!("{err}").contains("already defined"), "{err}");
    }

    #[test]
    fn relative_uses_forward_slashes() {
        let project = fixture("relative");
        let ws = Workspace::discover(&project).unwrap();
        let p = ws.root.join("domain").join("glossary.md");
        assert_eq!(ws.relative(&p), PathBuf::from("domain/glossary.md"));
    }
}
