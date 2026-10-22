//! Modèles de données « σ_i » — les agents du système, y décrits au complet.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `T` : description de tâche compilée en intention structurée
/// (« prompt analysis » : goal, contraintes, domaine, exemples, grounding).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Intent {
    pub goal: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub grounding_notes: Vec<String>,
}

/// `σ_i` : la spécification complète d'un agent `a_i`.
///
/// Le harnais n'admet un agent dans le système que si cette spec est
/// complète ( rôle, whitelist d'outils, contrat, critères ) — la
/// « construction-time verification » avant tout passage en aval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Identifiant unique dans le DAG.
    pub id: String,
    /// Rôle en langue naturelle (devient la directive système du prompt).
    pub role: String,
    /// Prompt système généré par la phase `build` (code generation).
    #[serde(default)]
    pub prompt: String,
    /// Whitelist d'outils (moindre privilège) : `read`, `bash`, …
    #[serde(default)]
    pub tools: Vec<String>,
    /// Contrat entrée/sortie explicite (« explicit input/output contract »).
    #[serde(default)]
    pub contract: Contract,
    /// `C_i` : critères de vérification machine-lisibles.
    #[serde(default)]
    pub criteria: Criteria,
    /// Arêtes entrantes : les agents dont la sortie est requise avant exécution.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Contrat E/S : schémas JSON + motifs interdits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contract {
    /// JSON Schema de l'attendu en entrée (peut rester null).
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// JSON Schema de la sortie — gate obligatoire si présent.
    #[serde(default)]
    pub output_schema: serde_json::Value,
    /// Motifs interdits dans la sortie (« forbidden patterns »).
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// `C_i` : critères de vérification d'exécution — vérifiés avant que `y_i`
/// ne soit propagé aux agents situés en aval (`y_i ∈ C_i`).
///
/// Note : `schema_check` vaut **true** par défaut (aussi hors sérialisation)
/// — un gate ne désarme que sur décision explicite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criteria {
    /// Valider la sortie contre `contract.output_schema` (défaut : oui).
    #[serde(default = "bool_true")]
    pub schema_check: bool,
    /// Chaînes requises dans la sortie.
    #[serde(default)]
    pub contains: Vec<String>,
    /// Chaînes interdites dans la sortie.
    #[serde(default)]
    pub not_contains: Vec<String>,
    /// Commandes qui doivent retourner 0 (ex. `cargo test`) — exécutées par la couche CLI dans le bac à sable du projet cible.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Aiguilles **bornées à un champ du contrat** : champ → aiguilles requises
    /// dans ce champ (et lui seul). Vérifie le contenu utile, pas l'écho :
    /// une aiguille placée n'importe où dans la sérialisation brute ne
    /// satisfait plus le gate.
    #[serde(default)]
    pub contains_in: BTreeMap<String, Vec<String>>,
}

impl Default for Criteria {
    fn default() -> Self {
        Self {
            schema_check: true,
            contains: Vec::new(),
            not_contains: Vec::new(),
            commands: Vec::new(),
            contains_in: BTreeMap::new(),
        }
    }
}

fn bool_true() -> bool {
    true
}
