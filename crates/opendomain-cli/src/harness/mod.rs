//! Le moteur d'exécution d'OpenDomain, branché sous forme de
//! sous-commandes de `opendomain`.
//!
//! Chaque module est une étape de la boucle canonique
//! *Generate → Verify → Attribute → Refine* (voir
//! [`opendomain_core::harness::LOOP`]) :
//!
//! | module | commande | étape |
//! |---|---|---|
//! | [`ground`] | `opendomain ground` | grounding à la conception |
//! | [`plan`] | `opendomain plan` | `T` → DAG d'agents `σ_i` |
//! | [`build`] | `opendomain build` | kit d'exécution + vérification statique |
//! | [`runflow`] | `opendomain run` | exécution sous gates `y_i ∈ C_i` |
//! | [`audit`] | `opendomain verify` / `opendomain attribute` | gates rejouables + attribution |
//! | [`repatch`] | `opendomain repatch` | re-planification partielle (structurel) |
//! | [`components`] | `opendomain resolve` / `opendomain skill` | registre de composants |
//! | [`doctor`] | `opendomain doctor` | exécuteur-agnosticisme vérifiable |
//! | [`bench`] | `opendomain bench` | invariants mécaniques, offline |
//! | [`commands`] | `opendomain validate` | contrôle statique d'un graph.json |
//!
//! Le scaffold du harnais (`opendomain init`, `spec new`) vit désormais dans
//! `opendomain-core::scaffold_harness` — un seul arbre de templates, un seul
//! écrivain, contrats OpenCode et Claude générés d'un bloc.

pub mod adapters;
pub mod audit;
pub mod bench;
pub mod build;
pub mod commands;
pub mod common;
pub mod components;
pub mod doctor;
pub mod executors;
pub mod ground;
pub mod plan;
pub mod repatch;
pub mod runflow;
pub mod runs;
pub mod workflows;
