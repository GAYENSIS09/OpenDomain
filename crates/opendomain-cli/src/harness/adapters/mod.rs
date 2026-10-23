//! Adaptateurs éditeur — la couche qui parle au logiciel concret, découpée
//! par responsabilité (S4 de la refonte architecturale) :
//!
//! - [`transport`] : exécuter un binaire et capturer ses flux (modèle de la
//!   machine hôte : Windows `cmd /C`, POSIX direct) ;
//! - [`planning`] : la boucle de conception (proposer → vérifier →
//!   raffiner) — testable sans réseau via un transport injectable ;
//! - [`materializer`] : matérialiser les agents de nœud (frontmatter V2,
//!   moindre privilège) — visible et reviewable avant le run ;
//! - [`capabilities`] : ce que l'éditeur cible peut **exprimer** — un outil
//!   hors vocabulaire produit une erreur explicite, jamais une permission
//!   fictive.
//!
//! La surface publique reste `adapters::*` : la découpe ne casse aucun
//! appel ; c'est le contrait qui s'éclaire, pas les incompatibilités.

pub mod capabilities;
#[cfg(test)] // sonde de conformité (REQ-005) : infrastructure de tests
             // aujourd'hui ; deviendra pub avec une commande dédiée.
mod conformite;
mod materializer;
mod planning;
mod transport;

pub use materializer::*;
pub use planning::*;
pub use transport::*;
