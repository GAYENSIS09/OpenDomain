//! # OpenDomain core
//!
//! The engine behind the `od` command-line tool. Everything here is a pure
//! function of a workspace on disk, which is what makes `od audit` and
//! `od gate` trustworthy: they never mutate state, so running them twice gives
//! the same answer.
//!
//! ## The idea
//!
//! A software project accumulates knowledge — what the domain is, why a rule
//! exists, which requirement a test covers — and that knowledge is normally
//! trapped in chat transcripts and people's heads. OpenDomain makes it a set of
//! addressable, linkable artifacts in plain Markdown, then enforces the links.
//!
//! ## Layout
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`vocab`] | Closed value sets: knowledge types, statuses, friction, operators |
//! | [`ids`] | `REQ-001`, `REQ-INVENTORY-012`, `AC-INVENTORY-012-01` |
//! | [`artifact`] | The artifact model and the relation vocabulary |
//! | [`parse`] | Markdown registry parsing |
//! | [`workspace`] | Discovery and indexing |
//! | [`graph`] | Traceability, impact, rendering |
//! | [`gates`] | The nine quality gates |
//! | [`scaffold`] | `od init` |
//! | [`report`] | Terminal and JSON rendering |
//!
//! ## Example
//!
//! Every line here is prefixed with `//! `, which leaves a leading space, so the
//! registry text is built from escapes rather than a literal block.
//!
//! ```
//! use opendomain_core::{gates::Gate, parse, workspace::Index};
//! use std::path::{Path, PathBuf};
//!
//! let text = concat!(
//!     "## REQ-001 — Reject an over-receipt\n",
//!     "\n",
//!     "```toml\n",
//!     "type = \"FUNCTIONAL\"\n",
//!     "status = \"VALIDATED\"\n",
//!     "epistemic = \"EXPLICIT\"\n",
//!     "derived-from = [\"RULE-001\"]\n",
//!     "```\n",
//!     "\n",
//!     "The system SHALL reject a receipt exceeding the remaining quantity.\n",
//! );
//!
//! let doc = parse::parse(Path::new("requirements.md"), PathBuf::from("requirements.md"), text)
//!     .unwrap();
//! let mut index = Index::default();
//! for artifact in doc.artifacts {
//!     index.by_id.insert(artifact.id.clone(), artifact);
//! }
//!
//! // The requirement rests on RULE-001, which nobody ever defined. Gate 8 treats
//! // a link with no target as a traceability hole rather than ignoring it.
//! let report = opendomain_core::gates::evaluate(Gate::by_name("completion").unwrap(), &index);
//! assert!(!report.passed());
//! assert!(report.findings.iter().any(|f| f.rule == "gate-8/dangling-reference"));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod artifact;
pub mod config;
pub mod error;
pub mod gates;
pub mod graph;
pub mod ids;
pub mod parse;
pub mod report;
pub mod scaffold;
pub mod vocab;
pub mod workspace;

pub use artifact::{Artifact, Document, Relation};
pub use config::Config;
pub use error::{OdError, Result};
pub use gates::{evaluate, Gate, GateReport, Severity};
pub use graph::{Direction, Graph, ImpactReport};
pub use ids::{Id, IdKind};
pub use vocab::{ArtifactStatus, AuthorityModel, Confidence, KnowledgeType, Profile, Stage};
pub use workspace::{Index, Workspace};

/// The framework version, taken from the crate manifest at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
