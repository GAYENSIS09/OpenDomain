//! The artifact model.
//!
//! An *artifact* is the smallest addressable unit of engineering knowledge in
//! OpenDomain: one source, one rule, one requirement, one test. Artifacts live
//! inside plain Markdown registry files so they diff well, review well, and can
//! be written by an AI agent without a special editor.
//!
//! # File format
//!
//! Each artifact is an `##`-level heading followed by a fenced TOML block and
//! free-form prose. The TOML block holds everything the machine checks; the
//! prose holds everything a human reads.
//!
//! ````markdown
//! ## REQ-012 — Consult the movement history of an item
//!
//! ```toml
//! type = "FUNCTIONAL"
//! status = "DRAFT"
//! epistemic = "INFERRED"
//! confidence = "MEDIUM"
//! actor = ["ACT-002"]
//! evidence = ["EVD-014"]
//! rules = ["RULE-007"]
//! ```
//!
//! **Statement**
//!
//! The system SHALL allow authorized users to consult the chronological
//! movements of an item.
//! ````
//!
//! Free-form prose is never machine-read, so an agent can write a rich
//! explanation without risking a parse failure. Only the TOML block is
//! load-bearing, and a malformed block is reported rather than guessed at.

use crate::ids::Id;
use crate::vocab::{ArtifactStatus, Confidence, KnowledgeType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A relation between two artifacts.
///
/// The vocabulary is deliberately small. Every edge in the traceability graph
/// must answer "why does this depend on that", which rules out catch-all edges
/// like `related-to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relation {
    /// A rests on evidential support from B.
    Supports,
    /// A was extracted from B.
    DerivedFrom,
    /// A realises B.
    Implements,
    /// A cannot exist without B.
    DependsOn,
    /// A narrows the permitted implementations of B.
    Constrains,
    /// A checks that B holds.
    Verifies,
    /// A replaces B.
    Supersedes,
    /// A and B cannot both hold as written.
    ConflictsWith,
    /// A is downstream of B and must be revisited when B changes.
    AffectedBy,
    /// A is the single owning domain of B's concept.
    OwnedBy,
    /// A subsumes B into its own lifecycle.
    Contains,
    /// A answers B's question.
    Answers,
    /// A tests B's falsifiable claim.
    Tests,
}

impl Relation {
    /// All relations, in canonical order.
    pub const ALL: &'static [Relation] = &[
        Relation::Supports,
        Relation::DerivedFrom,
        Relation::Implements,
        Relation::DependsOn,
        Relation::Constrains,
        Relation::Verifies,
        Relation::Supersedes,
        Relation::ConflictsWith,
        Relation::AffectedBy,
        Relation::OwnedBy,
        Relation::Contains,
        Relation::Answers,
        Relation::Tests,
    ];

    /// The relation as it appears in artifact TOML.
    pub fn as_str(self) -> &'static str {
        match self {
            Relation::Supports => "supports",
            Relation::DerivedFrom => "derived-from",
            Relation::Implements => "implements",
            Relation::DependsOn => "depends-on",
            Relation::Constrains => "constrains",
            Relation::Verifies => "verifies",
            Relation::Supersedes => "supersedes",
            Relation::ConflictsWith => "conflicts-with",
            Relation::AffectedBy => "affected-by",
            Relation::OwnedBy => "owned-by",
            Relation::Contains => "contains",
            Relation::Answers => "answers",
            Relation::Tests => "tests",
        }
    }

    /// Parses a relation, accepting `snake_case` as well as `kebab-case`.
    pub fn parse(raw: &str) -> Option<Relation> {
        let norm = raw.trim().to_ascii_lowercase().replace('_', "-");
        Relation::ALL.iter().copied().find(|r| r.as_str() == norm)
    }

    /// True when following this edge **upstream** moves toward the reason an
    /// artifact exists. Used by `od trace --direction backward`.
    ///
    /// Upstream relations are *declared by the artifact that depends on
    /// something* and point at it: `REQ-004 derived-from RULE-001`,
    /// `TEST-021 verifies AC-004-02`. Walking these edges forward, from `to`,
    /// is therefore the "why does this exist?" walk.
    pub fn is_upstream(self) -> bool {
        matches!(
            self,
            Relation::DerivedFrom
                | Relation::Supports
                | Relation::DependsOn
                | Relation::OwnedBy
                | Relation::Constrains
                | Relation::Answers
                | Relation::Verifies
                | Relation::Tests
        )
    }

    /// True when following this edge **downstream** moves toward delivery.
    /// Used by `od trace --direction forward`.
    ///
    /// These relations name something this artifact produces or contains, so
    /// they point away from the reason and towards the work.
    pub fn is_downstream(self) -> bool {
        matches!(
            self,
            Relation::Implements | Relation::Contains | Relation::AffectedBy
        )
    }
}

/// A single loaded artifact: its identity, its machine-checked metadata, and
/// the prose that accompanies it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// The parsed identifier.
    pub id: Id,
    /// The heading text after the identifier.
    pub title: String,
    /// The registry file it was loaded from, relative to `opendomain/`.
    pub file: PathBuf,
    /// The 1-based heading line in that file, for editor integration.
    pub line: usize,
    /// `type`, `priority`, `scope`, `class`, `outcome`, `authority`, …
    pub kind_fields: BTreeMap<String, String>,
    /// Lifecycle status, defaulted to `DRAFT` when absent.
    pub status: ArtifactStatus,
    /// Provenance of the claim, when the artifact makes one.
    pub epistemic: Option<KnowledgeType>,
    /// Support strength, when recorded.
    pub confidence: Option<Confidence>,
    /// Outgoing edges to other artifacts.
    pub links: BTreeMap<Relation, Vec<Id>>,
    /// Keys present in the TOML block that are not recognised.
    pub extra: BTreeMap<String, toml::Value>,
    /// The free-form body below the TOML block.
    pub body: String,
}

impl Artifact {
    /// Reads a recognised string field, falling back to the raw map.
    pub fn field(&self, key: &str) -> Option<&str> {
        self.kind_fields.get(key).map(|s| s.as_str())
    }

    /// Every outgoing edge as `(relation, target)` pairs.
    pub fn edges(&self) -> impl Iterator<Item = (Relation, Id)> + '_ {
        self.links
            .iter()
            .flat_map(|(rel, ids)| ids.iter().map(move |id| (*rel, id.clone())))
    }

    /// Direct upstream neighbours: the artifacts this one was derived from,
    /// depends on, or that it verifies.
    pub fn upstream(&self) -> Vec<Id> {
        self.edges()
            .filter(|(rel, _)| rel.is_upstream())
            .map(|(_, id)| id)
            .collect()
    }

    /// Direct downstream neighbours: what this artifact produces, implements or
    /// contains, as declared by *this* artifact.
    ///
    /// This is the outgoing, downstream-relation view. The complementary
    /// question — "what depends on this artifact" — needs the graph, because it
    /// lives on the *incoming* edges of this artifact. See
    /// [`crate::graph::Graph::dependents`].
    pub fn downstream(&self) -> Vec<Id> {
        self.edges()
            .filter(|(rel, _)| rel.is_downstream())
            .map(|(_, id)| id)
            .collect()
    }

    /// `true` when the artifact is not retired.
    pub fn is_live(&self) -> bool {
        !matches!(
            self.status,
            ArtifactStatus::Superseded | ArtifactStatus::Deprecated
        )
    }

    /// `true` when an open question has been closed.
    ///
    /// This is deliberately *not* `is_live`: an uncertainty and a drift stay
    /// live well past the point they stop blocking. Only an explicit resolution
    /// closes them, because the whole value of tracking unknowns is that they
    /// cannot be closed by accident.
    pub fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            ArtifactStatus::Verified | ArtifactStatus::Deprecated | ArtifactStatus::Superseded
        )
    }

    /// `true` when the artifact carries a claim whose provenance is not solid.
    ///
    /// This is the query behind `od status` warnings and Gate 1: an
    /// `ASSUMED` or `UNKNOWN` artifact that reached a downstream stage without
    /// being resolved is the single most common way a framework like this gets
    /// quietly abused.
    pub fn has_unresolved_provenance(&self) -> bool {
        matches!(
            self.epistemic,
            Some(KnowledgeType::Assumed)
                | Some(KnowledgeType::Unknown)
                | Some(KnowledgeType::Conflicting)
        )
    }
}

/// A parsed Markdown file: its front-matter-free preamble plus its artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Path relative to the workspace `opendomain/` directory.
    pub path: PathBuf,
    /// Text appearing before the first artifact heading.
    pub preamble: String,
    /// Artifacts in document order.
    pub artifacts: Vec<Artifact>,
}

impl Document {
    /// The `##` heading of the document, if present.
    pub fn title(&self) -> Option<&str> {
        self.preamble
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").trim())
    }
}

/// Locates an artifact inside the workspace for editor jump-to-file.
pub fn relative_file(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).unwrap_or(file).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::IdKind;

    #[test]
    fn relations_parse_both_cases() {
        assert_eq!(Relation::parse("derived-from"), Some(Relation::DerivedFrom));
        assert_eq!(Relation::parse("derived_from"), Some(Relation::DerivedFrom));
        assert_eq!(Relation::parse("DERIVED-FROM"), Some(Relation::DerivedFrom));
        assert_eq!(Relation::parse("nonsense"), None);
    }

    #[test]
    fn upstream_and_downstream_are_disjoint_and_total() {
        let id = Id::new(IdKind::Req, 1);
        let mut doc = Document {
            path: PathBuf::from("x.md"),
            preamble: String::new(),
            artifacts: vec![],
        };
        let mut links: BTreeMap<Relation, Vec<Id>> = BTreeMap::new();
        links.insert(Relation::DerivedFrom, vec![Id::new(IdKind::Src, 1)]);
        links.insert(Relation::Implements, vec![Id::new(IdKind::Task, 1)]);
        links.insert(Relation::Supersedes, vec![Id::new(IdKind::Spec, 1)]);
        doc.artifacts.push(Artifact {
            id: id.clone(),
            title: "t".into(),
            file: PathBuf::from("x.md"),
            line: 1,
            kind_fields: BTreeMap::new(),
            status: ArtifactStatus::Draft,
            epistemic: None,
            confidence: None,
            links,
            extra: BTreeMap::new(),
            body: String::new(),
        });
        let a = &doc.artifacts[0];
        assert_eq!(a.upstream(), vec![Id::new(IdKind::Src, 1)]);
        assert_eq!(a.downstream(), vec![Id::new(IdKind::Task, 1)]);
    }

    #[test]
    fn unresolved_provenance_is_detected() {
        let base = Artifact {
            id: Id::new(IdKind::Req, 1),
            title: "t".into(),
            file: PathBuf::from("x.md"),
            line: 1,
            kind_fields: BTreeMap::new(),
            status: ArtifactStatus::Draft,
            epistemic: None,
            confidence: None,
            links: BTreeMap::new(),
            extra: BTreeMap::new(),
            body: String::new(),
        };
        assert!(!base.has_unresolved_provenance());
        for k in [
            KnowledgeType::Assumed,
            KnowledgeType::Unknown,
            KnowledgeType::Conflicting,
        ] {
            let mut a = base.clone();
            a.epistemic = Some(k);
            assert!(a.has_unresolved_provenance(), "{k} should be unresolved");
        }
        for k in [
            KnowledgeType::Explicit,
            KnowledgeType::Inferred,
            KnowledgeType::Hybrid,
            KnowledgeType::Tacit,
        ] {
            let mut a = base.clone();
            a.epistemic = Some(k);
            assert!(!a.has_unresolved_provenance(), "{k} should be resolved");
        }
    }

    #[test]
    fn terminal_statuses_are_not_live() {
        let mut a = Artifact {
            id: Id::new(IdKind::Req, 1),
            title: "t".into(),
            file: PathBuf::from("x.md"),
            line: 1,
            kind_fields: BTreeMap::new(),
            status: ArtifactStatus::Draft,
            epistemic: None,
            confidence: None,
            links: BTreeMap::new(),
            extra: BTreeMap::new(),
            body: String::new(),
        };
        assert!(a.is_live());
        for s in [ArtifactStatus::Superseded, ArtifactStatus::Deprecated] {
            a.status = s;
            assert!(!a.is_live(), "{s} should not be live");
        }
    }
}
