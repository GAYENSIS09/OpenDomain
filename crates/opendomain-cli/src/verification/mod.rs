//! Les commandes du sous-système de vérification — la chaîne F10, chaque
//! étape gardée par la précédente :
//!
//! 1. `opendomain rigour` — la table §9.7, plus un audit des déclarations de
//!    rigueur que le workspace enregistre aujourd'hui.
//! 2. `opendomain provision` — détection de la stack (repository-only), les
//!    règles de sélection ordonnées R1-R4, la porte de refus §10.13, puis les
//!    *spécifications exécutables générées* : scaffolds `.feature` taggés,
//!    avec provenance, et l'artefact ProvisioningDecision (ou le refus).
//! 3. `opendomain bind` — l'audit de provenance des bindings (§10.10) avec la
//!    profondeur de vérification (§10.9). Les checks G8-B5/B6 sont rapportés
//!    comme *findings*, pas comme des règles de gate.
//! 4. `opendomain provenance` — le pont F14/L9 : les citations de runs
//!    vérifiées contre les archives.
//!
//! | module | commande | source |
//! |---|---|---|
//! | [`rigour`] | `opendomain rigour` | §9.7 |
//! | [`provision`] | `opendomain provision` | §10 |
//! | [`bind`] | `opendomain bind` | §10.9/§10.10 |
//! | [`provenance`] | `opendomain provenance` | F14/L9 |
//! | [`common`] | — | kinds exigence-side et dates, partagés |
//!
//! La doctrine vit dans `opendomain_core::verification` ; ce dossier ne fait
//! qu'orchestrer. Tout reste additif dans le modèle d'artefacts : `rigour`,
//! `binding` et `verification-depth` passent par le canal des champs extra et
//! sont optionnels partout.

pub mod bind;
pub mod common;
pub mod provenance;
pub mod provision;
pub mod rigour;

pub use bind::bind;
pub use provenance::provenance;
pub use provision::provision;
pub use rigour::rigour;
