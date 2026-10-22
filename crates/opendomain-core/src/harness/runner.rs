//! Coordinateur d'exécution (« a coordinator executes the agent graph »).

use std::collections::BTreeMap;
use std::result::Result as StdResult;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{validate_output, AgentSpec, Dag, Failure, FailureKind};

/// Tentatives locales par défaut, si la config n'en décide pas autrement
/// (`[gates] max_attempts`) — le coût doit rester proportionnel à la localité.
pub const MAX_ATTEMPTS: u32 = 3;

/// Réglages d'exécution lus dans `opendomain/harness/config.opendomain.toml` par la CLI
/// et branchés ici : la config **gouverne** le runner, elle ne le décore pas.
#[derive(Debug, Clone, Copy)]
pub struct RunConfig {
    /// Tentatives locales max avant escalade d'attribution.
    pub max_attempts: u32,
    /// `gates.strict` : fail-closed. La CLI refuse `false` en amont (aucun
    /// verdict consultatif) ; la valeur vit ici par complétude du réglage.
    pub strict: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self { max_attempts: MAX_ATTEMPTS, strict: true }
    }
}

/// Exécution d'une commande de critère, fournie par la couche CLI (sandbox,
/// cwd du dépôt cible). Le runner la détient : les exécuteurs ne pilotent
/// **jamais** leurs propres gates — mêmes gates partout, structurellement.
pub type CommandGate<'e> = &'e dyn Fn(&str) -> Result<()>;

/// Stockage en mémoire : `d[a_i] → y_i` (« in-memory context store »).
#[derive(Debug, Default)]
pub struct ContextStore {
    outputs: BTreeMap<String, serde_json::Value>,
}

impl ContextStore {
    /// Nouveau stockage vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sortie d'un agent déjà exécuté.
    pub fn get(&self, id: &str) -> Option<&serde_json::Value> {
        self.outputs.get(id)
    }

    /// Enregistre la sortie d'un agent après gate OK.
    pub fn insert(&mut self, id: &str, value: serde_json::Value) {
        self.outputs.insert(id.to_string(), value);
    }

    /// Nombre de sorties validées.
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Le stockage est-il vide ?
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// Snapshot complet (pour la trace de run).
    pub fn snapshot(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.outputs
    }
}

/// Interface d'exécution d'**un** agent. Les adaptateurs (OpenCode, Claude
/// Code, Aider, mock de test) l'implémentent — executor-agnostique.
/// `run` est une génération pure : analyse des réponses, commandes de
/// critères et rejeux appartiennent au coordinateur.
pub trait Executor {
    /// Exécute `spec` avec ses entrées et, en cas de refine, le feedback
    /// structuré de la tentative précédente.
    fn run(
        &mut self,
        spec: &AgentSpec,
        input: &serde_json::Value,
        feedback: Option<&Failure>,
    ) -> Result<serde_json::Value>;

    /// Classe une erreur d'exécution en échec **typé** — canal de parse,
    /// timeout, lancement — au lieu d'un re-étiquetage aveugle en
    /// `SpecMismatch`. Chaque tentative garde un feedback dirigé.
    fn classify_error(&self, _err: &anyhow::Error) -> FailureKind {
        FailureKind::SpecMismatch
    }
}

/// Gate d'exécution : `y_i ∈ C_i` ?
///
/// Vérifie dans l'ordre : schéma de sortie (contrat), aiguilles bornées aux
/// champs du contrat (`contains_in`), chaînes requises, chaînes/motifs
/// interdits (critères + contrat). Les commandes (`cargo test`…) sont
/// exécutées par le runner — dans la sandbox du projet cible, via la
/// [`CommandGate`] fournie par la CLI ; le verdict de chacune est tracé.
///
/// Aucun `y_i` n'est propagé en aval sans gate OK — c'est ce qui empêche la
/// propagation des erreurs intermédiaires.
pub fn gate(spec: &AgentSpec, output: &serde_json::Value) -> StdResult<(), Failure> {
    if spec.criteria.schema_check && !spec.contract.output_schema.is_null() {
        if let Err(errs) = validate_output(output, &spec.contract.output_schema) {
            return Err(Failure::new(
                &spec.id,
                FailureKind::ContractViolation,
                errs.join(" ; "),
            )
            .with_hints(vec!["faire correspondre exactement contract.output_schema".into()]));
        }
    }

    // Aiguilles bornées : exercées sur le champ déclaré du contrat, pas sur
    // la sérialisation brute — un écho placé ailleurs ne satisfait plus.
    for (field, needles) in &spec.criteria.contains_in {
        let Some(field_value) = output.get(field) else {
            return Err(Failure::new(
                &spec.id,
                FailureKind::CriteriaFailure,
                format!("critère `contains[{field}]` : champ `{field}` absent de la sortie"),
            )
            .with_hints(vec![format!(
                "remplir le champ `{field}` du contrat de sortie, aiguilles : {needles:?}"
            )]));
        };
        let text = field_value.to_string();
        for needle in needles {
            if !text.contains(needle.as_str()) {
                return Err(Failure::new(
                    &spec.id,
                    FailureKind::CriteriaFailure,
                    format!("critère `contains[{field}]: {needle}` non satisfait dans `{field}`"),
                )
                .with_hints(vec![format!(
                    "cherché `{needle}` dans le champ `{field}` — corriger ce champ"
                )]));
            }
        }
    }

    let text = output.to_string();
    for needle in &spec.criteria.contains {
        if !text.contains(needle.as_str()) {
            return Err(Failure::new(
                &spec.id,
                FailureKind::CriteriaFailure,
                format!("critère `contient: {needle}` non satisfait"),
            )
            .with_hints(vec![format!("chaîne requise absente : `{needle}`")]));
        }
    }
    for banned in spec
        .criteria
        .not_contains
        .iter()
        .chain(spec.contract.forbidden.iter())
    {
        if text.contains(banned.as_str()) {
            return Err(Failure::new(
                &spec.id,
                FailureKind::CriteriaFailure,
                format!("motif interdit détecté : `{banned}`"),
            )
            .with_hints(vec![format!("retirer le motif `{banned}`")]));
        }
    }
    Ok(())
}

/// Événement horodaté logiquement de la trace d'exécution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceEvent {
    /// Le nœud commence (entrées agrégées disponibles).
    Start { node: String },
    /// Une commande de critère a tourné (sandbox côté projet cible) — verdict
    /// traceable, colonne du `gates.log`.
    Command { node: String, cmd: String, ok: bool, message: String },
    /// Gate OK après `attempts` tentative(s) — `attempts = 0` signale un nœud
    /// **repris** d'un run antérieur après re-passage du gate (aucune
    /// exécution nouvelle, valeur réindexée dans le contexte).
    Passed { node: String, attempts: u32 },
    /// Gate KO → refine local avec feedback, tentative `attempt`.
    Retry {
        node: String,
        attempt: u32,
        kind: FailureKind,
        message: String,
    },
    /// Échec non rattrapé localement : attribution + directive.
    Failed { failure: Failure, directive: String },
}

/// Bilan d'un run : nœuds passés, échec terminal éventuel, trace complète.
#[derive(Debug, Default)]
pub struct RunSummary {
    /// Nœuds dont la sortie a passé le gate.
    pub passed: Vec<String>,
    /// Premier échec non rattrapé (le run s'arrête là).
    pub failed: Option<Failure>,
    /// Trace ordonnée des événements.
    pub events: Vec<TraceEvent>,
}

/// Le coordinateur : dispatche les agents dans l'ordre topologique et
/// filtre chaque sortie par un gate avant propagation.
pub struct Runner<'e> {
    executor: &'e mut dyn Executor,
    commands: Option<CommandGate<'e>>,
    config: RunConfig,
    store: ContextStore,
    summary: RunSummary,
}

impl std::fmt::Debug for Runner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // L'exécuteur est un trait objet sans Debug : seul ce que la config
        // gouverne est rendu, la partie dynamique reste non exhaustive.
        f.debug_struct("Runner")
            .field("config", &self.config)
            .field("store", &self.store)
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}

impl<'e> Runner<'e> {
    /// Nouveau runner sur un exécuteur fourni.
    pub fn new(executor: &'e mut dyn Executor) -> Self {
        Self {
            executor,
            commands: None,
            config: RunConfig::default(),
            store: ContextStore::new(),
            summary: RunSummary::default(),
        }
    }

    /// Branche l'exécution des commandes de critères (sandbox de la CLI).
    /// Sans elle, des `commands` déclarées ne tournent jamais et les nœuds
    /// restent verts à tort — le fail-closed l'interdit.
    pub fn commands(mut self, gate: CommandGate<'e>) -> Self {
        self.commands = Some(gate);
        self
    }

    /// Branche la config `[gates]` lue par la CLI : la valeur écrite dans
    /// `config.opendomain.toml` décide du nombre de tentatives.
    pub fn with_config(mut self, config: RunConfig) -> Self {
        self.config = config;
        self
    }

    /// Exécute le DAG jusqu'au premier échec non rattrapé. `resume` :
    /// sorties archivées **re-jouées au gate** avant réinsertion dans le
    /// contexte (« vérifié à la reprise ») ; un défaut de re-check
    /// ré-exécute le nœud au lieu de faire confiance à l'archive. Retourne
    /// le bilan **et** le contexte (les `y_i` validés) pour l'archivage.
    pub fn run_all(
        mut self,
        dag: &Dag,
        resume: Option<&BTreeMap<String, serde_json::Value>>,
    ) -> (RunSummary, ContextStore) {
        self.go(dag, resume);
        (self.summary, self.store)
    }

    /// Exécution complète — signature stable pour les tests et l'usage simple.
    pub fn run(self, dag: &Dag) -> RunSummary {
        self.run_all(dag, None).0
    }

    /// Reprise vérifiée (récupération amont) : les nœuds dont la sortie
    /// archivée repasse le gate sont sautés (attempts: 0) et leurs valeurs
    /// nourrissent les dépendants ; les autres tournent à nouveau.
    pub fn run_resuming(
        self,
        dag: &Dag,
        resume: &BTreeMap<String, serde_json::Value>,
    ) -> (RunSummary, ContextStore) {
        self.run_all(dag, Some(resume))
    }

    fn go(&mut self, dag: &Dag, resume: Option<&BTreeMap<String, serde_json::Value>>) {
        match dag.topological_order() {
            Ok(order) => {
                for id in order {
                    let Some(spec) = dag.nodes.get(&id) else { continue };
                    if let Some(archive) = resume {
                        if let Some(value) = archive.get(&id) {
                            if gate(spec, value).is_ok() {
                                self.store.insert(&id, value.clone());
                                self.summary.passed.push(id.clone());
                                self.summary.events.push(TraceEvent::Passed {
                                    node: id.clone(),
                                    attempts: 0,
                                });
                                continue;
                            }
                        }
                    }
                    self.execute_node(spec);
                    if self.summary.failed.is_some() {
                        break;
                    }
                }
            }
            Err(e) => {
                self.summary.failed = Some(Failure::new(
                    "<graphe>",
                    FailureKind::ContractViolation,
                    e.to_string(),
                ));
            }
        }
    }

    /// Entrée agrégée d'un nœud = sorties de ses dépendances (contexte).
    fn aggregate_input(&self, spec: &AgentSpec) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for d in &spec.depends_on {
            let v = self
                .store
                .get(d.trim())
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            map.insert(d.trim().to_string(), v);
        }
        serde_json::Value::Object(map)
    }

    /// Génère → commande → vérifie → raffine pour un nœud (boucle locale).
    fn execute_node(&mut self, spec: &AgentSpec) {
        let input = self.aggregate_input(spec);
        self.summary.events.push(TraceEvent::Start {
            node: spec.id.clone(),
        });
        let mut feedback: Option<Failure> = None;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.executor.run(spec, &input, feedback.as_ref()) {
                Ok(output) => {
                    // Commandes de critères : partie de la production de
                    // l'agent, exécutées par le runner (un agent n'a pas
                    // « fini » tant que ses commandes ne passent pas).
                    if let Some(gate_ko) = self.run_commands(spec) {
                        match self.admit_gate_ko(spec, gate_ko, attempt) {
                            Some(next) => feedback = Some(next),
                            None => return, // échec terminal déclaré
                        }
                        continue;
                    }
                    match gate(spec, &output) {
                        Ok(()) => {
                            self.store.insert(&spec.id, output);
                            self.summary.passed.push(spec.id.clone());
                            self.summary.events.push(TraceEvent::Passed {
                                node: spec.id.clone(),
                                attempts: attempt,
                            });
                            return;
                        }
                        Err(f) => match self.admit_gate_ko(spec, f, attempt) {
                            Some(next) => feedback = Some(next),
                            None => return,
                        },
                    }
                }
                Err(e) => {
                    let kind = self.executor.classify_error(&e);
                    let f = Failure::new(&spec.id, kind, e.to_string())
                        .with_hints(error_hints(kind, &e))
                        .with_attempt(attempt);
                    match self.admit_gate_ko(spec, f, attempt) {
                        Some(next) => feedback = Some(next),
                        None => return,
                    }
                }
            }
        }
    }

    /// Exécute les commandes de critères via le gate de commandes fourni.
    /// Verdict de chacune tracé ; première commande en défaut → échec typé
    /// (`SandboxViolation` pour un refus d'allowlist, `CriteriaFailure`
    /// pour une commande qui retourne non-zéro). Fail-closed : sans gate de
    /// commandes branché, une `commands` déclarée refuse le nœud plutôt que
    /// de tourner « où elle peut ».
    fn run_commands(&mut self, spec: &AgentSpec) -> Option<Failure> {
        if self.config.strict {
            self.exec_commands_blocking(spec)
        } else {
            self.exec_commands_advisory(spec)
        }
    }

    fn exec_commands_blocking(&mut self, spec: &AgentSpec) -> Option<Failure> {
        for cmd in &spec.criteria.commands {
            let outcome = match self.commands {
                Some(gate) => gate(cmd).map(|_| String::new()).unwrap_or_else(|e| e.to_string()),
                None => "aucun gate de commandes fourni par le runner".to_string(),
            };
            if outcome.is_empty() {
                self.summary.events.push(TraceEvent::Command {
                    node: spec.id.clone(),
                    cmd: cmd.clone(),
                    ok: true,
                    message: String::new(),
                });
                continue;
            }
            let kind = if outcome.contains("SandboxViolation") {
                FailureKind::SandboxViolation
            } else {
                FailureKind::CriteriaFailure
            };
            self.summary.events.push(TraceEvent::Command {
                node: spec.id.clone(),
                cmd: cmd.clone(),
                ok: false,
                message: outcome.clone(),
            });
            return Some(
                Failure::new(
                    &spec.id,
                    kind,
                    format!("commande `{cmd}`"),
                )
                .with_hints(command_hints(&outcome))
                .with_message_suffix(outcome),
            );
        }
        None
    }

    fn exec_commands_advisory(&mut self, spec: &AgentSpec) -> Option<Failure> {
        for cmd in &spec.criteria.commands {
            let gate = self.commands;
            let outcome = match gate {
                Some(gate) => gate(cmd).map(|_| String::new()).unwrap_or_else(|e| e.to_string()),
                None => "aucun gate de commandes fourni par le runner".to_string(),
            };
            self.summary.events.push(TraceEvent::Command {
                node: spec.id.clone(),
                cmd: cmd.clone(),
                ok: outcome.is_empty(),
                message: outcome,
            });
        }
        None
    }

    /// Consigne un gate KO : escalade terminale au-delà de `max_attempts`,
    /// sinon Retry + feedback typé pour la tentative suivante.
    fn admit_gate_ko(&mut self, spec: &AgentSpec, mut f: Failure, attempt: u32) -> Option<Failure> {
        f.attempt = attempt;
        if attempt >= self.config.max_attempts {
            self.declare_failed(f);
            return None;
        }
        self.summary.events.push(TraceEvent::Retry {
            node: spec.id.clone(),
            attempt,
            kind: f.kind,
            message: f.message.clone(),
        });
        Some(f)
    }

    /// Échec terminal : type l'attribution et produit la directive de
    /// récupération (directive typée du kind quand il en connaît une).
    fn declare_failed(&mut self, f: Failure) {
        let directive = f.directive();
        self.summary
            .events
            .push(TraceEvent::Failed { directive, failure: f.clone() });
        self.summary.failed = Some(f);
    }
}

/// Hints par catégorie d'échec d'exécution — un retry n'est pas simplement
/// « réessayé » mais dirigé.
fn error_hints(kind: FailureKind, e: &anyhow::Error) -> Vec<String> {
    let msg = format!("{e:#}");
    match kind {
        FailureKind::ContractViolation => {
            vec!["renvoyer UNIQUEMENT l'objet JSON conforme au contrat, sans prose".into()]
        }
        _ if msg.contains("dépassé") => vec![
            "l'appel a dépassé le délai — augmenter --timeout ou réduire le prompt".into(),
        ],
        _ if msg.contains("lancement") => {
            vec!["vérifier `opendomain doctor` et l'exécuteur".into()]
        }
        _ => vec![format!("corriger selon le message du gate : {msg}")],
    }
}

/// Hints pour une commande de critère en défaut — typée avant tout retry.
fn command_hints(outcome: &str) -> Vec<String> {
    if outcome.contains("SandboxViolation") {
        vec![
            "ajouter le binaire au `[sandbox] allow` si légitime, ou re-résoudre \
le besoin qui l'exige (`opendomain resolve <slug>`)"
                .into(),
        ]
    } else {
        vec!["corriger la production pour que la commande retourne 0".into()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{Attribution, Contract, GraphFile};
    use serde_json::json;

    /// Exécuteur « capricieux » : échoue au premier appel, passe ensuite.
    struct FlakyExecutor {
        calls: u32,
    }

    impl Executor for FlakyExecutor {
        fn run(
            &mut self,
            _spec: &AgentSpec,
            _input: &serde_json::Value,
            _feedback: Option<&Failure>,
        ) -> Result<serde_json::Value> {
            self.calls += 1;
            if self.calls == 1 {
                Ok(json!({ "value": 42 }))
            } else {
                Ok(json!({ "value": 42, "somme": "ok" }))
            }
        }
    }

    /// Exécuteur toujours en défaut : gate KO à chaque tentative.
    struct BrokenExecutor;

    impl Executor for BrokenExecutor {
        fn run(
            &mut self,
            _spec: &AgentSpec,
            _input: &serde_json::Value,
            _feedback: Option<&Failure>,
        ) -> Result<serde_json::Value> {
            Ok(json!({ "value": 42 }))
        }
    }

    fn spec_somme() -> AgentSpec {
        AgentSpec {
            id: "somme".into(),
            role: "calcule une somme".into(),
            contract: Contract {
                output_schema: json!({
                    "type": "object",
                    "required": ["value", "somme"],
                    "properties": {
                        "value": { "type": "number" },
                        "somme": { "type": "string" }
                    }
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn gate_relance_avec_feedback_puis_passe() {
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec_somme()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = FlakyExecutor { calls: 0 };
        let summary = Runner::new(&mut ex).run(&dag);
        assert_eq!(summary.passed, vec!["somme".to_string()]);
        assert!(summary.failed.is_none());
        assert!(summary.events.iter().any(|e| matches!(
            e,
            TraceEvent::Retry { kind: FailureKind::ContractViolation, .. }
        )));
        assert_eq!(ex.calls, 2);
    }

    #[test]
    fn echec_terminal_est_type_et_attribue() {
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec_somme()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = BrokenExecutor;
        let summary = Runner::new(&mut ex).run(&dag);

        let f = summary.failed.expect("doit échouer");
        assert_eq!(f.kind, FailureKind::ContractViolation);
        assert_eq!(f.attribution(), Attribution::Structural);
        assert!(summary.events.iter().any(|e| matches!(
            e,
            TraceEvent::Failed { directive, .. }
                if directive.contains("re-planifier")
        )));
    }

    #[test]
    fn la_reprise_re_verifie_le_gate_puis_saute() {
        let downstream = AgentSpec {
            id: "apres".into(),
            role: "consomme la sortie".into(),
            depends_on: vec!["somme".into()],
            ..Default::default()
        };
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec_somme(), downstream],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = FlakyExecutor { calls: 0 };

        // sortie archivée CONFORME : sautée (attempts: 0), valeur réinjectée.
        let mut resume = BTreeMap::new();
        resume.insert("somme".to_string(), json!({ "value": 42, "somme": "ok" }));
        let (summary, store) = Runner::new(&mut ex).run_resuming(&dag, &resume);
        assert_eq!(summary.passed, vec!["somme".to_string(), "apres".to_string()]);
        assert_eq!(ex.calls, 1, "seul `apres` tourne");
        assert!(summary
            .events
            .iter()
            .any(|e| matches!(e, TraceEvent::Passed { node: _, attempts: 0 })));
        assert!(store.get("somme").is_some(), "la valeur reprise nourrit `apres`");
    }

    #[test]
    fn la_reprise_re_execute_une_sortie_corrompue() {
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec_somme()],
        };
        let dag = Dag::build(&file).unwrap();

        // sortie archivée NON conforme au contrat : ré-exécutée, jamais admise.
        let mut resume = BTreeMap::new();
        resume.insert("somme".to_string(), json!({ "value": 42 }));
        let mut ex = FlakyExecutor { calls: 0 };
        let (summary, _store) = Runner::new(&mut ex).run_resuming(&dag, &resume);
        assert_eq!(ex.calls, 2, "ré-exécution jusqu'à une sortie conforme");
        assert!(summary.passed.contains(&"somme".to_string()));
    }

    #[test]
    fn les_aiguilles_bornees_refusent_lecho_dans_autre_champ() {
        use std::collections::BTreeMap as Map;
        // l'aiguille est exigée dans `cas_limites` — l'écho dans `note`
        // ne satisfait plus le gate ; l'aiguille au vrai champ passe.
        let mut criteria = crate::harness::Criteria::default();
        criteria
            .contains_in
            .insert("cas_limites".into(), vec!["cas limite".into()]);
        let spec = AgentSpec { id: "a".into(), criteria, ..Default::default() };

        let echo = json!({ "note": "cas limite", "cas_limites": [] });
        let err = gate(&spec, &echo).expect_err("l'écho ne suffit plus");
        assert_eq!(err.kind, FailureKind::CriteriaFailure);
        assert!(err.message.contains("cas_limites"));

        let conforme = json!({ "note": "-", "cas_limites": ["cas limite: liste vide"] });
        assert!(gate(&spec, &conforme).is_ok());

        let mut absent = Map::<String, Vec<String>>::new();
        absent.insert("x".into(), vec!["-".into()]);
        assert!(!absent.is_empty(), "les map bornées restent typées");
    }

    #[test]
    fn les_commandes_tournent_dans_le_runner_et_sont_typees() {
        // contrat-vert mais commande rouge : retry → échec terminal typé
        // critère non satisfait, commande tracée à chaque tentative.
        struct GreenExecutor;
        impl Executor for GreenExecutor {
            fn run(
                &mut self,
                _spec: &AgentSpec,
                _input: &serde_json::Value,
                _f: Option<&Failure>,
            ) -> Result<serde_json::Value> {
                Ok(json!({ "value": 42, "somme": "ok" }))
            }
        }
        let mut spec = spec_somme();
        spec.criteria.commands = vec!["cargo test".into()];

        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec.clone()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = GreenExecutor;
        let rouge =
            |_cmd: &str| Result::<()>::Err(anyhow::anyhow!("commande `cargo test` a échoué : exit 1"));
        let summary = Runner::new(&mut ex).commands(&rouge).run(&dag);

        let f = summary.failed.expect("commande rouge → échec terminal");
        assert_eq!(f.kind, FailureKind::CriteriaFailure);
        assert!(f.message.contains("cargo test"));
        assert!(f.hints.iter().any(|h| h.contains("retourne 0")));
        let cmd_events: Vec<&TraceEvent> = summary
            .events
            .iter()
            .filter(|e| matches!(e, TraceEvent::Command { .. }))
            .collect();
        assert_eq!(cmd_events.len(), 3, "une trace de commande par tentative");
    }

    #[test]
    fn sans_gate_de_commandes_le_fail_closed_refuse_le_noeud() {
        // `commands` déclarée mais aucun gate branché : le nœud est refusé,
        // jamais « verts à tort » — l'inverse serait un contournement.
        struct GreenExecutor;
        impl Executor for GreenExecutor {
            fn run(
                &mut self,
                _spec: &AgentSpec,
                _input: &serde_json::Value,
                _f: Option<&Failure>,
            ) -> Result<serde_json::Value> {
                Ok(json!({ "value": 42, "somme": "ok" }))
            }
        }
        let mut spec = spec_somme();
        spec.criteria.commands = vec!["cargo test".into()];
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec.clone()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = GreenExecutor;
        let summary = Runner::new(&mut ex).run(&dag);
        let f = summary.failed.expect("commands non exécutable → échec typé");
        assert!(f.message.contains("aucun gate de commandes"));
    }

    #[test]
    fn sandbox_violation_est_typee_avant_execution() {
        struct GreenExecutor;
        impl Executor for GreenExecutor {
            fn run(
                &mut self,
                _spec: &AgentSpec,
                _input: &serde_json::Value,
                _f: Option<&Failure>,
            ) -> Result<serde_json::Value> {
                Ok(json!({ "value": 42, "somme": "ok" }))
            }
        }
        let mut spec = spec_somme();
        spec.criteria.commands = vec!["curl http://x".into()];
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec.clone()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = GreenExecutor;
        let refuse =
            |_cmd: &str| Result::<()>::Err(anyhow::anyhow!("SandboxViolation : `curl` hors allowlist"));
        let summary = Runner::new(&mut ex).commands(&refuse).run(&dag);

        let f = summary.failed.expect("sandbox KO → échec terminal");
        assert_eq!(f.kind, FailureKind::SandboxViolation);
        assert_eq!(f.attribution(), Attribution::Structural);
        assert!(f.directive().contains("allowlist"), "directive spécialisée");
        assert!(summary.events.iter().any(|e| matches!(
            e,
            TraceEvent::Failed { directive, .. }
                if directive.contains("allowlist")
        )));
    }

    #[test]
    fn la_config_gouverne_les_tentatives() {
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![spec_somme()],
        };
        let dag = Dag::build(&file).unwrap();
        let mut ex = BrokenExecutor;
        let config = RunConfig { max_attempts: 1, strict: true };
        let summary = Runner::new(&mut ex).with_config(config).run(&dag);
        assert!(summary.failed.is_some(), "max_attempts=1 → échec immédiat");
        let retries = summary
            .events
            .iter()
            .filter(|e| matches!(e, TraceEvent::Retry { .. }))
            .count();
        assert_eq!(retries, 0, "aucune relance avec max_attempts=1");
    }

    #[test]
    fn une_reprise_sans_archive_fait_le_meme_travail() {
        let file = GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![
                spec_somme(),
                AgentSpec {
                    id: "apres".into(),
                    depends_on: vec!["somme".into()],
                    ..Default::default()
                },
            ],
        };
        let dag = Dag::build(&file).unwrap();
        let empty = BTreeMap::new();
        let mut ex = FlakyExecutor { calls: 0 };
        let (summary, _) = Runner::new(&mut ex).run_resuming(&dag, &empty);
        assert_eq!(summary.passed.len(), 2);
        // somme : 1 KO + 1 OK ; apres : 1 appel. Reprise sans archive = run neuf.
        assert_eq!(ex.calls, 3);
    }
}
