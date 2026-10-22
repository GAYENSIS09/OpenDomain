//! Le sous-système de vérification (Sections 9/10 du papier + le pont F14) :
//!
//! - [`mechanism`] — la table de rigueur (§9.7), la provenance de binding
//!   (§10.10), la profondeur de vérification (§10.9) ;
//! - [`provisioning`] — le provisionneur : détection repo-only, règles
//!   ordonnées R1–R4, la porte de refus §10.13, les scaffolds `.feature` ;
//! - [`provenance`] — le pont F14 : chaque `run = "…"` vérifié contre
//!   l'archive qu'il nomme, et les runs que personne ne réclame.

pub mod mechanism;
pub mod provenance;
pub mod provisioning;

// La surface §9.7/§10.9/§10.10 est portée par `mechanism` — re-exportée ici
// pour que `opendomain_core::verification::declared_rigour` reste le chemin
// public.
pub use mechanism::*;
