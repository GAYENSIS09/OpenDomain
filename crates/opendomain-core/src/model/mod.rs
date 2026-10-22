//! Le modèle de données : les artefacts, les identifiants, les vocabulaires.
//!
//! Trois modules, une seule idée : tout ce que les registres Markdown
//! portent devient une donnée typée, fermée et indexable.

pub mod artifact;
pub mod ids;
pub mod vocab;

pub use artifact::{Artifact, Document, Promotion, Relation};
pub use ids::{Id, IdKind};
pub use vocab::{
    ArtifactStatus, AuthorityModel, Confidence, KnowledgeType, Profile, RepositoryKind, Stage,
};
