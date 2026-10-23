//! Le moteur d'exécution d'OpenDomain — en sous-module discipliné de
//! `opendomain-core`.
//!
//! `opendomain-core::gates` (et le reste du crate hors de ce dossier) garde ce
//! que le projet *sait* : des registres d'artefacts liés, vérifiés par nine
//! gates d'arrêt. Ce module garde ce que les agents *font* : il exécute une
//! tâche comme un DAG d'agents spécialisés et n'admet une sortie `y_i` dans
//! le contexte aval qu'après son gate (`y_i ∈ C_i`) — aucune erreur
//! intermédiaire ne se propage.
//!
//! **La règle de discipline du module** (l'ancien garde-fou de compilation,
//! devenu une frontière interne) : en dehors de ce dossier, aucun module du
//! crate n'importe `crate::harness::…` ; à l'intérieur, aucun module
//! n'importe les registres. Le savoir et le faire se rencontrent dans la CLI
//! seule.
//!
//! Les notions suivent **Meta-Agent: From Task Descriptions to Verified
//! Multi-Agent Systems** :
//!
//! | notion | code |
//! |---|---|
//! | `T` (description de tâche)     | [`model::Intent`] |
//! | `s_i` (spécification d'agent)  | [`model::AgentSpec`] |
//! | contrat entrée/sortie          | [`model::Contract`] |
//! | critères `C_i`                 | [`model::Criteria`] |
//! | `G = (V, E)` (DAG)             | [`dag::Dag`] / [`dag::GraphFile`] |
//! | échecs typés `f ∈ F`           | [`failure::FailureKind`] |
//! | attribution 3 niveaux          | [`failure::Attribution`] |
//! | *y_i ∈ C_i* (gates)            | [`validate::validate_output`] |
//!
//! - un [`runner::Executor`] abstrait exécute *un* agent (opencode,
//!   claude, mock de test…) — c'est ce qui rend le système *executor-agnostic* ;
//! - un [`runner::Runner`] parcourt le DAG en ordre topologique ;
//! - chaque sortie `y_i` passe par [`runner::gate`] **avant** d'être
//!   propagée en aval (`y_i ∈ C_i`) ;
//! - en cas d'échec, [`failure::attribute`] désigne la stratégie :
//!   retry local = [`runner::MAX_ATTEMPTS`], ré-exécution amont, ou
//!   re-planification (remontée à la construction).
//!
//! La boucle canonique — [`failure::LOOP`] — est
//! *Generate → Verify → Attribute → Refine*.

pub mod admission;
pub mod dag;
pub mod failure;
pub mod model;
pub mod runner;
pub mod validate;

/// Le chemin de la config unique, depuis la racine du dépôt.
///
/// Une seule config (`opendomain/opendomain.toml`, sections `[gates]`,
/// `[sandbox]`, `[harness]`) — pas de second fichier de réglages. La
/// constante vit ici parce que les directives d'échec du runner (core) la
/// citent et que le harnais n'importe pas les registres (frontière de
/// discipline) ; la CLI et le scaffold l'utilisent telle quelle. Au commit
/// historique la directive de sandbox nommait encore un
/// `opendomain/harness/config.opendomain.toml` — fichier que le scaffold
/// refuse d'écrire : c'est une directive sur un chemin fantôme, réglée au
/// S3 de la refonte.
pub const WORKSPACE_CONFIG: &str = "opendomain/opendomain.toml";

pub use admission::{admit, completeness, no_caps, AdmissionError};
pub use dag::{Dag, GraphFile};
pub use failure::{attribute, Attribution, Failure, FailureKind, LOOP};
pub use model::{AgentSpec, Contract, Criteria, Intent, Kit};
pub use runner::{
    gate, CommandGate, ContextStore, Executor, RunConfig, RunSummary, Runner, TraceEvent,
    MAX_ATTEMPTS,
};
pub use validate::validate_output;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn node(id: &str, deps: &[&str]) -> AgentSpec {
        AgentSpec {
            id: id.into(),
            role: "rôle de test".into(),
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn refuse_un_cycle() {
        let file = GraphFile {
            meta: Value::Null,
            nodes: vec![node("a", &["b"]), node("b", &["a"])],
        };
        assert!(Dag::build(&file).is_err());
    }

    #[test]
    fn refuse_une_arete_pendante() {
        let file = GraphFile {
            meta: Value::Null,
            nodes: vec![node("a", &["fantome"])],
        };
        assert!(Dag::build(&file).is_err());
    }

    #[test]
    fn ordre_topologique_kahn() {
        let file = GraphFile {
            meta: Value::Null,
            nodes: vec![node("c", &["b"]), node("b", &["a"]), node("a", &[])],
        };
        let dag = Dag::build(&file).unwrap();
        assert_eq!(dag.topological_order().unwrap(), vec!["a", "b", "c"]);
        assert_eq!(dag.dependencies("b")[0].id, "a");
    }

    #[test]
    fn schema_de_sortie_basique() {
        let schema = json!({
            "type": "object",
            "required": ["code"],
            "additionalProperties": false,
            "properties": { "code": { "type": "string", "minLength": 2 } }
        });
        assert!(validate_output(&json!({ "code": "ok" }), &schema).is_ok());
        assert!(validate_output(&json!({ "code": "" }), &schema).is_err());
        assert!(validate_output(&json!({ "code": "x", "extra": 1 }), &schema).is_err());
    }

    #[test]
    fn attribution_typee() {
        assert_eq!(attribute(FailureKind::SpecMismatch), Attribution::Local);
        assert_eq!(attribute(FailureKind::GroundingFailure), Attribution::Upstream);
        assert_eq!(attribute(FailureKind::UpstreamFailure), Attribution::Upstream);
        assert_eq!(attribute(FailureKind::ContractViolation), Attribution::Structural);
    }

    #[test]
    fn panne_amont_est_typee_et_dirigee_vers_la_reprise_differee() {
        let f = Failure::new("x", FailureKind::UpstreamFailure, "quota épuisé");
        assert_eq!(f.attribution(), Attribution::Upstream);
        let directive = f.directive();
        assert!(directive.contains("reprendre plus tard"));
        assert!(directive.contains("--resume"));
    }

    #[test]
    fn un_graphe_sans_kit_se_lit_tel_que_kit_vide() {
        // rétrocompatibilité : les graphes écrits avant le kit restent valides
        let raw = r#"{
            "meta": null,
            "nodes": [{
                "id": "analyste",
                "role": "analyse",
                "tools": ["read"],
                "contract": {"input_schema": null, "output_schema": null, "forbidden": []},
                "criteria": {"schema_check": true, "contains": [], "not_contains": [],
                             "commands": [], "contains_in": {}},
                "depends_on": []
            }]
        }"#;
        let file: GraphFile = serde_json::from_str(raw).unwrap();
        let dag = Dag::build(&file).unwrap();
        let spec = &dag.nodes[&"analyste".to_string()];
        assert!(spec.kit.is_empty(), "kit par défaut : vide");
    }
}
