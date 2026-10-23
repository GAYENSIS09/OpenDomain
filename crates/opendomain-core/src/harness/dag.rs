//! `G = (V, E)` — le DAG des dépendances d'agents.
//!
//! Le graphe doit être **orienté et acyclique** : chaque nœud porte une
//! spécification `σ_i` complète. `opendomain validate` refuse tout ce qui
//! n'en est pas un (boucle sur soi, arête pendante, cycle).

use std::collections::{BTreeMap, VecDeque};

use anyhow::bail;
use serde::{Deserialize, Serialize};

use super::admission::AdmissionError;
use super::model::AgentSpec;

/// Fichier `graph.json` tel que stocké dans `opendomain/graph/`.
///
/// Tolérance racine assumée : des clés additionnelles au niveau du fichier
/// (la `_note` du modèle d'exemple) restent ignorées — de la prose. C'est à
/// l'intérieur d'un nœud que la refus doit être précis : [`AgentSpec`],
/// `Contract`, `Criteria`, `Kit` refusent tout champ inconnu (une typo y
/// dépouillerait un gate en silence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFile {
    /// Métadonnées libres (nom, source, date…).
    #[serde(default)]
    pub meta: serde_json::Value,
    /// Nœuds `V`, chacun portant ses arêtes via `depends_on`.
    pub nodes: Vec<AgentSpec>,
}

/// Le DAG validé.
#[derive(Debug, Clone)]
pub struct Dag {
    /// Nœuds indexés par identifiant.
    pub nodes: BTreeMap<String, AgentSpec>,
}

impl Dag {
    /// Construit un DAG depuis un [`GraphFile`] et le valide.
    ///
    /// Les refus sont typés ([`AdmissionError::Structure`]) et précisent
    /// nœud et cause — une typo ne passe jamais tout seul.
    pub fn build(file: &GraphFile) -> Result<Self, AdmissionError> {
        let mut nodes = BTreeMap::new();
        for n in &file.nodes {
            let id = n.id.trim();
            if id.is_empty() {
                return Err(AdmissionError::structure("nœud sans `id`".into()));
            }
            if nodes.contains_key(id) {
                return Err(AdmissionError::structure(format!("id de nœud dupliqué : `{id}`")));
            }
            let mut spec = n.clone();
            spec.id = id.to_string();
            nodes.insert(id.to_string(), spec);
        }
        let dag = Self { nodes };
        dag.validate_typed()?;
        Ok(dag)
    }

    /// Contrôles statiques, typés : boucle sur soi, arêtes pendantes,
    /// acyclicité.
    fn validate_typed(&self) -> Result<(), AdmissionError> {
        for n in self.nodes.values() {
            if n.depends_on.iter().any(|d| d.trim() == n.id) {
                return Err(AdmissionError::structure(format!("`{}` dépend de lui-même", n.id)));
            }
            for d in &n.depends_on {
                if !self.nodes.contains_key(d.trim()) {
                    return Err(AdmissionError::structure(format!(
                        "arête pendante : `{}` dépend de `{}` (nœud inexistant)",
                        n.id,
                        d.trim()
                    )));
                }
            }
        }
        if let Err(e) = self.topological_order() {
            return Err(AdmissionError::structure(format!(
                "cycle détecté — le graphe n'est pas un DAG ({e})"
            )));
        }
        Ok(())
    }

    /// Contrôles statiques, à l'error `anyhow` historique.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.validate_typed().map_err(anyhow::Error::from)
    }

    /// Tri topologique (Kahn). En cas de cycle, liste les nœuds piégés.
    pub fn topological_order(&self) -> anyhow::Result<Vec<String>> {
        let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for id in self.nodes.keys() {
            indegree.entry(id.clone()).or_insert(0);
            dependents.insert(id.clone(), Vec::new());
        }
        for n in self.nodes.values() {
            for d in &n.depends_on {
                *indegree.get_mut(&n.id).expect("nœud connu") += 1;
                dependents
                    .entry(d.trim().to_string())
                    .or_default()
                    .push(n.id.clone());
            }
        }
        let mut ready: VecDeque<String> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut order: Vec<String> = Vec::new();
        while let Some(u) = ready.pop_front() {
            order.push(u.clone());
            let ups = dependents.get(&u).cloned().unwrap_or_default();
            for v in ups {
                let e = indegree.get_mut(&v).expect("nœud connu");
                *e -= 1;
                if *e == 0 {
                    ready.push_back(v);
                }
            }
        }
        if order.len() != self.nodes.len() {
            let stuck: Vec<String> = indegree
                .iter()
                .filter(|(_, d)| **d > 0)
                .map(|(k, _)| k.clone())
                .collect();
            bail!("cycle impliquant : {}", stuck.join(", "));
        }
        Ok(order)
    }

    /// Spécs des dépendances directes d'un nœud (dans l'ordre déclaré).
    pub fn dependencies(&self, id: &str) -> Vec<&AgentSpec> {
        self.nodes
            .get(id)
            .map(|n| {
                n.depends_on
                    .iter()
                    .filter_map(|d| self.nodes.get(d.trim()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
