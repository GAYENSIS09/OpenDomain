//! L'analyse : les neuf gates (au repos, sur les registres) et le graphe de
//! traçabilité (Q-Why / Q-Blast).
//!
//! Discipline du module : l'analyse ne sait pas exécuter — les gates lisent
//! l'index, le graphe le reconstruit à la volée ; aucun module ici
//! n'importe `crate::harness`.

pub mod gates;
pub mod graph;
