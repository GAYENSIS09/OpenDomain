//! Échecs typés `f ∈ F`, attribution à trois niveaux et la boucle canonique.
//!
//! Le harnais ne renvoie pas un simple oui/non : chaque verification produit un
//! échec **typé** qui pointe vers l'étape amont responsable. C'est ce routage
//! typé qui permet de corriger localement sans recalculer tout le pipeline.

use serde::{Deserialize, Serialize};

/// Boucle unifiée de vérification.
pub const LOOP: &str = "Generate → Verify → Attribute → Refine";

/// `f ∈ F` : catégorie d'échec (spec, grounding, contrat…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// La sortie viole la spécification (statique ou comportementale).
    SpecMismatch,
    /// Connaissance externe manquante ou incorrecte (grounding / recherche API).
    GroundingFailure,
    /// Contrat d'E/S violé (schéma de sortie, champ manquant, motif interdit).
    ContractViolation,
    /// Un critère `C_i` d'exécution n'est pas satisfait (tests, texte requis…).
    CriteriaFailure,
    /// Un binaire hors allowlist a été refusé AVANT exécution — faute de
    /// configuration (ou de résolution de besoin), jamais de l'agent : la
    /// directive est ailleurs que la relance d'agent.
    SandboxViolation,
    /// L'exécuteur LLM est indisponible (quota épuisé, authentification,
    /// endpoint en panne) : rien à relancer ni à réviser — reprendre plus tard.
    UpstreamFailure,
}

impl FailureKind {
    /// Libellé lisible.
    pub fn label(self) -> &'static str {
        match self {
            Self::SpecMismatch => "violation de spécification",
            Self::GroundingFailure => "échec de grounding",
            Self::ContractViolation => "violation de contrat",
            Self::CriteriaFailure => "critère non satisfait",
            Self::SandboxViolation => "violation de sandbox",
            Self::UpstreamFailure => "panne amont (exécuteur)",
        }
    }

    /// Directive qui prime sur `Attribution::action()` quand le type d'échec
    /// connaît une remédiation plus précise que le niveau d'attribution.
    pub fn directive(self) -> Option<&'static str> {
        match self {
            // La faute est dans le réglage, pas dans l'agent : relancer le
            // même agent serait un retry aveugle. L'inscription de la
            // décision est un `repatch` explicite, jamais automatique.
            Self::SandboxViolation => Some(
                "corriger la sandbox — allowlist `[sandbox]` de \
`opendomain/harness/config.opendomain.toml` ; l'inscrire explicitement : \
`opendomain repatch <run> --allow <binaire>` — puis relancer le nœud",
            ),
            // Panne amont : relancer le même agent trois fois ne change rien
            // (leçon du 2026-10-23) — la reprise est différée, ciblée.
            Self::UpstreamFailure => Some(
                "amont indisponible (quota, auth, endpoint) — reprendre plus tard : \
`opendomain run <spec> --resume <run>` ; une panne amont se re-planifie, \
elle ne se retente pas à l'identique trois fois",
            ),
            _ => None,
        }
    }
}

/// Attribution à trois niveaux — le coût de la récupération est
/// proportionnel à la localité de l'erreur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    /// `a_i` échoue malgré des entrées correctes → **retry local** avec feedback.
    Local,
    /// La faute vient d'une dépendance → **ré-exécution amont**, puis `a_i`.
    Upstream,
    /// La décomposition elle-même est fautive → **re-planification** du
    /// sous-graphe (remonte à la phase de construction).
    Structural,
}

impl Attribution {
    /// Libellé lisible.
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "erreur locale",
            Self::Upstream => "erreur amont",
            Self::Structural => "erreur structurelle",
        }
    }

    /// Directive de récupération associée.
    pub fn action(self) -> &'static str {
        match self {
            Self::Local => "relancer le même agent avec le feedback du vérificateur",
            Self::Upstream => "ré-exécuter l'agent amont responsable, puis relancer",
            Self::Structural => {
                "re-planifier le sous-graphe concerné — révision partielle \
`opendomain repatch <run>` (nœud fautif révisé, amonts vérifiés ancrés) \
ou regénération `opendomain plan <slug>` (via opencode ; `--offline` en repli explicite)"
            }
        }
    }
}

/// Routage typé : chaque échec pointe vers l'étape amont responsable.
///
/// - *contract violation* → révision de la décomposition ;
/// - *grounding failure* → ré-exécution de l'étape de récupération de
///   connaissance.
pub fn attribute(kind: FailureKind) -> Attribution {
    match kind {
        FailureKind::SpecMismatch | FailureKind::CriteriaFailure => Attribution::Local,
        FailureKind::GroundingFailure | FailureKind::UpstreamFailure => Attribution::Upstream,
        // Le réglage (contrat, sandbox) est fautive de façon structurelle :
        // la remédiation passe par la configuration, pas par l'agent.
        FailureKind::ContractViolation | FailureKind::SandboxViolation => Attribution::Structural,
    }
}

/// Feedback structuré transmis à l'agent lors du *refine* — pas un simple non,
/// mais un échec typé, des indices et le rang de la tentative
/// (« verification-driven refinement »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Failure {
    /// Agent en échec.
    pub node: String,
    /// Type d'échec `f`.
    pub kind: FailureKind,
    /// Message précis produit par le gate.
    pub message: String,
    /// Indices pour la régénération ciblée.
    #[serde(default)]
    pub hints: Vec<String>,
    /// Rang de la tentative (1 = première).
    pub attempt: u32,
}

impl Failure {
    /// Nouvelle occurrence d'échec (tentative 1).
    pub fn new(node: impl Into<String>, kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            node: node.into(),
            kind,
            message: message.into(),
            hints: Vec::new(),
            attempt: 1,
        }
    }

    /// Définir les indices de régénération.
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = String>) -> Self {
        self.hints = hints.into_iter().collect();
        self
    }

    /// Détailler le message du gate (contexte de commande typé, etc.).
    pub fn with_message_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.message = format!("{} — {}", self.message, suffix.into());
        self
    }

    /// Définir le rang de tentative.
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    /// Attribution dérivée du type d'échec.
    pub fn attribution(&self) -> Attribution {
        attribute(self.kind)
    }

    /// Directive de récupération : la directive typée du kind quand il en
    /// connaît une, sinon celle du niveau d'attribution.
    pub fn directive(&self) -> String {
        self.kind
            .directive()
            .map(String::from)
            .unwrap_or_else(|| self.attribution().action().to_string())
    }
}
