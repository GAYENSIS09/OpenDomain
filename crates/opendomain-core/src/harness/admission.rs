//! L'admission — le contrôle déterministe par Rust : le modèle propose, le
//! moteur décide.
//!
//! Un moteur, une seule loi : le graphe **proposé** (par le concepteur LLM ou
//! par l'heuristique `--offline`) ne devient un DAG exécutable que si la
//! structure **et** la complétude de chaque agent passent, plus tout contrôle
//! de capacité branché par l'appelant (registre de composants, expressivité
//! de l'éditeur cible). Les quatre commandes consommatoes — `plan`, `build`,
//! `run --graph` (graphe fourni) et `repatch` — appelle le même moteur :
//! aucune porte dérobée, jamais deux politiques qui divergent.
//!
//! Un refus est **typé** et nomme le nœud et le champ en faute ; le fichier
//! actif n'est jamais modifié par une proposition rejetée.

use std::fmt;

use super::dag::GraphFile;
use super::{AgentSpec, Dag};

/// Une admission refusée — la catégorie est typée, le message est précis.
///
/// Les trois catégories couvrent la taxonomie du contrat cible :
/// `Structure` (un graphe n'est pas un DAG), `Incomplete` (un agent n'est pas
/// admissible tel que proposé), `Capability` (une capacité requise manque ou
/// n'est pas exprimable par l'éditeur).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// Le graphe n'est pas structuralement admissible.
    Structure { message: String },
    /// Un ou plusieurs agents sont incomplets (contrat/critères).
    Incomplete { message: String },
    /// Une capacité requise manque — sans fournisseur dans le registre, ou
    /// non exprimable par la cible d'exécution.
    Capability { message: String },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            AdmissionError::Structure { message }
            | AdmissionError::Incomplete { message }
            | AdmissionError::Capability { message } => message,
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for AdmissionError {}

impl AdmissionError {
    /// Compositeur interne du message de structure — utilisé par `dag.rs`
    /// pour produire le même libellé « graphe invalide : … » que l'erreur
    /// `anyhow` historique.
    pub fn structure(detail: String) -> Self {
        AdmissionError::Structure {
            message: format!("graphe invalide : {detail}"),
        }
    }
}

/// Ancre de capacité du moteur : par nœud, la liste des capacités en faute
/// `(besoin, détail)`. La CLI branche ici le registre de composants et
/// l'expressivité de l'éditeur cible — ils participent à la même décision
/// typée que la structure et la complétude.
pub type CapacityCheck = dyn Fn(&AgentSpec) -> Vec<(String, String)>;

/// Le moteur n'exige aucune capacité externe (admission par complétude seule).
pub fn no_caps(_: &AgentSpec) -> Vec<(String, String)> {
    Vec::new()
}

/// Les quatre conditions de complétude d'un agent — `(libellé, verdict,
/// champ en faute)`. Portée par le moteur déterministe : la même liste est
/// celle que `plan`, `build`, `run --graph` et `repatch` appliquent.
pub fn completeness(spec: &AgentSpec) -> Vec<(String, bool, String)> {
    let schema_ok = spec
        .contract
        .output_schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .map(|r| !r.is_empty())
        .unwrap_or(false)
        || spec
            .contract
            .output_schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .map(|p| !p.is_empty())
            .unwrap_or(false);
    let criteria_ok = spec.criteria.schema_check
        || !spec.criteria.contains.is_empty()
        || !spec.criteria.contains_in.is_empty()
        || !spec.criteria.not_contains.is_empty()
        || !spec.criteria.commands.is_empty();
    vec![
        (
            "rôle non vide".into(),
            !spec.role.trim().is_empty(),
            "role".into(),
        ),
        (
            "liste d'outils présente".into(),
            !spec.tools.is_empty(),
            "tools".into(),
        ),
        (
            "contrat de sortie (required/properties)".into(),
            schema_ok,
            "contract".into(),
        ),
        (
            "critères C_i non vides".into(),
            criteria_ok,
            "criteria".into(),
        ),
    ]
}

/// Le moteur d'admission : une proposition devient un DAG seulement si la
/// structure et la complétude passent et si aucun contrôle de capacité
/// branché ne nomme de faute. **Toute** commande qui écrit un graphe actif
/// passe par ici — une proposition rejetée ne modifie jamais les fichiers.
pub fn admit(file: &GraphFile, capacities: &CapacityCheck) -> Result<Dag, AdmissionError> {
    let dag = Dag::build(file)?;
    let mut incomplets: Vec<(String, Vec<String>)> = Vec::new();
    let mut capacites: Vec<String> = Vec::new();
    for (id, spec) in &dag.nodes {
        let manquants: Vec<String> = completeness(spec)
            .into_iter()
            .filter(|(_, ok, _)| !ok)
            .map(|(_, _, field)| field)
            .collect();
        let caps = capacities(spec);
        if !caps.is_empty() {
            for (need, detail) in caps {
                capacites.push(format!("{id} : {need} ({detail})"));
            }
        } else if !manquants.is_empty() {
            incomplets.push((id.clone(), manquants));
        }
    }
    // Triage : une capacité nommée par l'ancre est un refus de capacité ; la
    // complétude garde le message historique du plan (même texte qu'avant
    // l'unification — les consommateurs ne voient pas la couture).
    if !capacites.is_empty() {
        return Err(AdmissionError::Capability {
            message: format!(
                "plan refusé — capacités non satisfaites : {}",
                capacites.join(" ; ")
            ),
        });
    }
    if !incomplets.is_empty() {
        let details = incomplets
            .iter()
            .map(|(id, manquants)| format!("{id} : {}", manquants.join(", ")))
            .collect::<Vec<_>>()
            .join(" ; ");
        return Err(AdmissionError::Incomplete {
            message: format!(
                "plan refusé — agents incomplets (contrat/critères) : {details}\n  → corriger le graphe proposé, ou demander le repli déterministe : `opendomain plan <slug> --offline`"
            ),
        });
    }
    Ok(dag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(id: &str, deps: &[&str]) -> AgentSpec {
        AgentSpec {
            id: id.into(),
            role: "rôle de test".into(),
            tools: vec!["read".into()],
            contract: crate::harness::Contract {
                output_schema: json!({ "type": "object", "required": ["code"], "properties": { "code": { "type": "string" } } }),
                ..Default::default()
            },
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    fn file(nodes: Vec<AgentSpec>) -> GraphFile {
        GraphFile {
            meta: serde_json::Value::Null,
            nodes,
        }
    }

    #[test]
    fn structure_refuse_le_cycle_avec_message_precis() {
        let err = admit(&file(vec![node("a", &["b"]), node("b", &["a"])]), &no_caps).unwrap_err();
        let AdmissionError::Structure { message } = err else {
            panic!("variante structure attendue : {err:?}")
        };
        assert!(message.contains("cycle"), "message : {message}");
    }

    #[test]
    fn structure_refuse_une_arete_pendante_avec_les_deux_bouts() {
        let err = admit(&file(vec![node("a", &["fantome"])]), &no_caps).unwrap_err();
        let AdmissionError::Structure { message } = err else {
            panic!("variante structure attendue : {err:?}")
        };
        assert!(
            message.contains("arête pendante") && message.contains("fantome"),
            "message : {message}"
        );
    }

    #[test]
    fn incomplet_nomme_le_noeud_et_les_champs_en_faute() {
        let agent = AgentSpec {
            id: "a".into(),
            role: "rôle présent".into(),
            ..Default::default()
        };
        let err = admit(&file(vec![agent]), &no_caps).unwrap_err();
        let AdmissionError::Incomplete { message } = err else {
            panic!("variante incomplet attendue : {err:?}")
        };
        assert!(message.contains("incomplets"), "message : {message}");
        assert!(message.contains("tools"), "champ fautif nommé : {message}");
        assert!(message.contains("contract"), "contrat nommé : {message}");
        assert!(message.contains("a :"), "nœud nommé : {message}");
    }

    #[test]
    fn une_capacite_en_faute_est_type_cache_et_precedente_sur_la_completude() {
        let agent = node("a", &[]);
        let hook = |_spec: &AgentSpec| {
            vec![("carnet.design".to_string(), "aucun fournisseur".to_string())]
        };
        let hook: &CapacityCheck = &hook;
        let err = admit(&file(vec![agent]), hook).unwrap_err();
        let AdmissionError::Capability { message } = err else {
            panic!("variante capacité attendue : {err:?}")
        };
        assert!(
            message.contains("capacités non satisfaites"),
            "message : {message}"
        );
        assert!(
            message.contains("carnet.design"),
            "besoin nommé : {message}"
        );
    }
}
