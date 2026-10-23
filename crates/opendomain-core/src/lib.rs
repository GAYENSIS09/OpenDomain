//! # OpenDomain core
//!
//! The engine behind the `opendomain` command-line tool. Everything here is a pure
//! function of a workspace on disk, which is what makes `opendomain audit` and
//! `opendomain gate` trustworthy: they never mutate state, so running them twice gives
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
//! | [`model`] | The closed data model: `vocab`, `ids`, `artifact` |
//! | [`registries`] | Reading the workspace from disk: `config`, `parse`, `workspace` |
//! | [`analysis`] | Traceability (`graph`) and the nine quality gates (`gates`) |
//! | [`report`] | Terminal and JSON rendering |
//! | [`scaffold`] | `opendomain init` and its template set |
//! | [`verification`] | The §9/§10 verification subsystem: rigour, provisioning, provenance |
//! | [`harness`] | The execution harness: agent DAGs under verified gates |
//!
//! The flat module names are kept as aliases, so `opendomain_core::parse`,
//! `opendomain_core::gates`, … resolve exactly as they always did.
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

pub mod analysis;
pub mod docs_fidelity;
pub mod error;
pub mod harness;
pub mod model;
pub mod registries;
pub mod report;
pub mod scaffold;
pub mod verification;

// Alias de compatibilité : les noms plats d'origine continuent de résoudre
// (`opendomain_core::parse`, `opendomain_core::gates`, …) après le
// regroupement par responsabilité. `mechanism` ne prend PAS d'alias : le nom
// `verification` est déjà porté par le module, et sa surface §9.7/§10.9/§10.10
// est re-exportée par `verification/mod.rs` (`pub use mechanism::*`).
pub use analysis::{gates, graph};
pub use model::{artifact, ids, vocab};
pub use registries::{config, parse, workspace};
pub use verification::{provenance, provisioning};

pub use analysis::graph::{Direction, Graph, ImpactReport};
pub use analysis::gates::{evaluate, Gate, GateReport, Severity};
pub use error::{OdError, Result};
pub use model::artifact::{Artifact, Document, Relation};
pub use model::ids::{Id, IdKind};
pub use model::vocab::{ArtifactStatus, AuthorityModel, Confidence, KnowledgeType, Profile, Stage};
pub use registries::config::Config;
pub use registries::workspace::{Index, Workspace};

/// The framework version, taken from the crate manifest at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
