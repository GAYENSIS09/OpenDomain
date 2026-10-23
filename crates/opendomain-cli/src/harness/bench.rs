//! M5 — `opendomain bench` : suite de bancs *déterministes* pour le harnais
//! (offline, sans LLM). Contrairement aux benchmarks IA à reproduire
//! (HumanEval/MBPP…), ici on mesure les **invariants mécaniques** :
//! 1. refus des graphes invalides (cycle, arête pendante),
//! 2. gates : toute corruption de `y_i` est interceptée (requis, interdit),
//! 3. convergeance du retry avec feedback,
//! 4. attribution correcte par type d'échec,
//! 5. reprise re-vérifiée : les sorties admises ne ré-exécutent pas,
//! 6. sandbox : allowlist honorée AVANT exécution (parser général unique),
//! 7. résolveur : un fournisseur par besoin, sinon échec typé ;
//! 8. admission : un agent incomplet n'entre nulle part (parité offline
//!    avec plan / run / repatch — S7) ;
//! 9. preuve : le `gates.log` écrit passe le pont de provenance (round-trip
//!    écrivain → lecteur du contrat cross-crate — S7).
//!
//! Un banc échoué ⇒ code de sortie non nul (utilisable en CI).

use anyhow::{Result, anyhow};
use opendomain_core::harness::{AgentSpec, Contract, Dag, Failure, FailureKind, GraphFile};
use opendomain_core::harness::{Runner, TraceEvent};
use serde_json::{Value, json};

/// Score rendu : (tests réussis, total).
pub struct Bench {
    pub passed: usize,
    pub total: usize,
}

/// `opendomain bench` — exécute la suite et imprime les verdicts.
pub fn run() -> Result<()> {
    let mut b = Bench { passed: 0, total: 0 };
    check(&mut b, "cycle refusé", &|| {
        Dag::build(&cycle_graph()).is_err()
    });
    check(&mut b, "arête pendante refusée", &|| {
        Dag::build(&dangling_graph()).is_err()
    });
    check(&mut b, "gate: champ requis intercepté", &|| {
        gate_rejects(&missing_field_output())
    });
    check(&mut b, "gate: motif interdit intercepté", &|| {
        gate_rejects(&banned_word_output())
    });
    check(&mut b, "gate: sortie conforme acceptée", &|| {
        gate_accepts(&valid_output())
    });
    check(&mut b, "gate: aiguille bornée — écho hors champ refusé", &test_echo_scoped);
    check(&mut b, "retry avec feedback converge", &test_retry_converges);
    check(&mut b, "attribution typée (contrat→structurel)", &|| {
        Failure::new("x", FailureKind::ContractViolation, "m")
            .attribution()
            == opendomain_core::harness::Attribution::Structural
    });
    check(&mut b, "attribution typée (grounding→amont)", &|| {
        Failure::new("x", FailureKind::GroundingFailure, "m").attribution()
            == opendomain_core::harness::Attribution::Upstream
    });
    check(&mut b, "attribution typée (sandbox→structurel, directive dédiée)", &|| {
        let f = Failure::new("x", FailureKind::SandboxViolation, "m");
        f.attribution() == opendomain_core::harness::Attribution::Structural
            && f.directive().contains("allowlist")
    });
    check(&mut b, "reprise re-vérifiée saute les admises", &test_resume_skips);
    check(&mut b, "sandbox : allowlist honorée avant exécution", &test_sandbox_allowlist);
    check(
        &mut b,
        "resolver : un fournisseur par besoin, sinon échec typé",
        &test_resolver_per_need,
    );
    check(
        &mut b,
        "kit : une capacité sans fournisseur refuse la construction",
        &test_kit_typed,
    );
    check(
        &mut b,
        "admission : un agent incomplet n'entre nulle part (même loi offline)",
        &test_admission_completeness,
    );
    check(
        &mut b,
        "preuve : gates.log écrit → pont de provenance (round-trip)",
        &test_gates_log_provenance_contract,
    );

    println!(
        "bench : {}/{} invariants vérifiés (harnais offline)",
        b.passed, b.total
    );
    if b.passed == b.total {
        Ok(())
    } else {
        Err(anyhow!("bench en échec — zero : {}/{}", b.passed, b.total))
    }
}

fn check(b: &mut Bench, name: &str, f: &dyn Fn() -> bool) {
    b.total += 1;
    let ok = f();
    if ok {
        b.passed += 1;
    }
    println!("  [{}] {name}", if ok { "ok" } else { "KO" });
}

// ------------------------------------------------------------- fixtures ---

fn node(id: &str, deps: &[&str]) -> AgentSpec {
    AgentSpec {
        id: id.into(),
        role: "rôle".into(),
        contract: Contract {
            output_schema: json!({
                "type": "object", "required": ["value", "somme"],
                "properties": { "value": {"type":"number"}, "somme": {"type":"string"} }
            }),
            ..Default::default()
        },
        criteria: opendomain_core::harness::Criteria {
            schema_check: true,
            contains: Vec::new(),
            not_contains: vec!["interdit".into()],
            commands: Vec::new(),
            contains_in: Default::default(),
        },
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
        ..Default::default()
    }
}

fn cycle_graph() -> GraphFile {
    GraphFile {
        meta: Value::Null,
        nodes: vec![node("a", &["b"]), node("b", &["a"])],
    }
}

fn dangling_graph() -> GraphFile {
    GraphFile {
        meta: Value::Null,
        nodes: vec![node("a", &["fantome"])],
    }
}

fn valid_output() -> Value {
    json!({ "value": 1, "somme": "ok" })
}

fn missing_field_output() -> Value {
    json!({ "value": 1 })
}

fn banned_word_output() -> Value {
    json!({ "value": 1, "somme": "c'est interdit ici" })
}

/// gate() KO si le banc exige un rejet.
fn gate_rejects(v: &Value) -> bool {
    opendomain_core::harness::gate(&node("a", &[]), v).is_err()
}

fn gate_accepts(v: &Value) -> bool {
    opendomain_core::harness::gate(&node("a", &[]), v).is_ok()
}

/// Aiguille bornée : l'écho dans un autre champ ne satisfait plus ; la vraie
/// valeur dans le champ déclaré passe. (echo gaming = décoratif, banni.)
fn test_echo_scoped() -> bool {
    use std::collections::BTreeMap;
    let mut spec = node("a", &[]);
    spec.criteria.contains_in = {
        let mut m = BTreeMap::new();
        m.insert("somme".into(), vec!["ok".into()]);
        m
    };
    let echo = json!({ "value": 1, "somme": "-", "note": "ok" });
    let vrai = json!({ "value": 1, "somme": "ok" });
    opendomain_core::harness::gate(&spec, &echo).is_err() && opendomain_core::harness::gate(&spec, &vrai).is_ok()
}

/// Un exécuteur qui échoue d'abord puis passe : le Runner doit converger.
struct ConvExecutor {
    calls: u32,
}

impl opendomain_core::harness::Executor for ConvExecutor {
    fn run(
        &mut self,
        _spec: &AgentSpec,
        _input: &Value,
        feedback: Option<&Failure>,
    ) -> Result<Value, anyhow::Error> {
        self.calls += 1;
        match self.calls {
            1 if feedback.is_none() => Ok(missing_field_output()),
            _ => Ok(valid_output()),
        }
    }
}

fn test_retry_converges() -> bool {
    let file = GraphFile {
        meta: Value::Null,
        nodes: vec![node("a", &[])],
    };
    let dag = Dag::build(&file).unwrap();
    let mut ex = ConvExecutor { calls: 0 };
    let s = Runner::new(&mut ex).run(&dag);
    s.failed.is_none() && s.passed == vec!["a".to_string()]
}

/// Reprise : une sortie archivée admise ne déclenche aucun appel exécuteur —
/// et sa valeur est réinjectée dans le contexte pour les dépendants.
struct CountingExecutor {
    calls: u32,
}

impl opendomain_core::harness::Executor for CountingExecutor {
    fn run(
        &mut self,
        _spec: &AgentSpec,
        _input: &Value,
        _f: Option<&Failure>,
    ) -> Result<Value, anyhow::Error> {
        self.calls += 1;
        Ok(valid_output())
    }
}

fn test_resume_skips() -> bool {
    let file = GraphFile {
        meta: Value::Null,
        nodes: vec![node("a", &[]), node("b", &["a"])],
    };
    let dag = Dag::build(&file).unwrap();
    let mut archive = std::collections::BTreeMap::new();
    archive.insert("a".into(), valid_output());
    let mut ex = CountingExecutor { calls: 0 };
    let (s, store) = Runner::new(&mut ex).run_resuming(&dag, &archive);
    ex.calls == 1
        && s.passed.len() == 2
        && s.events.iter().any(|e| matches!(e, TraceEvent::Passed { attempts: 0, .. }))
        && store.get("a").is_some()
}

/// La sandbox refuse l'inconnu AVANT l'exécution — et reflète `enabled`.
/// Passe désormais par le **seul** parser du format (`Config::parse` de core).
fn test_sandbox_allowlist() -> bool {
    use opendomain_core::config::Config;
    let from_cfg = |text: &str| {
        Config::parse(text).map(|c| crate::harness::common::Sandbox::from_config(&c))
    };
    let on = from_cfg("version = 1\n\n[sandbox]\nenabled = true\nallow = [\"cargo\", \"rustc\"]\n").unwrap();
    // Désactivée = décision écrite explicite (`enabled = false`) — le
    // réglage réel ; plus de « section absente faussement désactivée ».
    let off = from_cfg("[sandbox]\nenabled = false\n").unwrap();
    on.enabled
        && off.permits("curl").is_ok() // désactivée = fail-open assumé, explicite
        && on.permits("cargo").is_ok()
        && on.permits("curl").is_err()
        && on.permits("curl").err().unwrap().to_string().contains("SandboxViolation")
}

/// Le kit d'un nœud se résout à la construction : une capacité (skill,
/// commande) sans fournisseur est une erreur typée qui nomme le pas.
fn test_kit_typed() -> bool {
    use crate::harness::components::{resolve_kit, Registry};
    let spec = AgentSpec {
        id: "deployeur".into(),
        role: "rôle".into(),
        kit: opendomain_core::harness::Kit {
            skills: vec!["deploy.inconnu".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    let file = GraphFile { meta: Value::Null, nodes: vec![spec] };
    let Ok(dag) = Dag::build(&file) else { return false };
    let err = match resolve_kit(&dag, &Registry::default(), false) {
        Ok(_) => return false,
        Err(e) => e.to_string(),
    };
    err.contains("sans fournisseur")
        && err.contains("deploy.inconnu")
        && err.contains("kit add skill")
}

/// Parité offline de l'admission (S7) : un agent incomplet est refusé par le
/// même moteur que plan / run --graph / repatch — le mode hors-ligne skewte
/// la même loi, pas une affiche.
fn test_admission_completeness() -> bool {
    let fainéant = AgentSpec {
        id: "fainéant".into(),
        role: "rôle présent".into(),
        ..Default::default()
    };
    let file = GraphFile { meta: Value::Null, nodes: vec![fainéant] };
    let err = format!(
        "{:#}",
        opendomain_core::harness::admit(&file, &opendomain_core::harness::no_caps)
            .expect_err("l'incomplet n'entre nulle part")
    );
    err.contains("incomplets") && err.contains("contract") && err.contains("fainéant")
}

/// Le round-trip du contrat `gates.log` (S7) : ce que `runflow::gates_log`
/// écrit dans une archive et que `provenance::verify` lit — l'écrivain et
/// le lecteur du format passent au banc, comme en CI, dans un tempdir.
fn test_gates_log_provenance_contract() -> bool {
    use crate::harness::runflow::gates_log;
    use opendomain_core::harness::RunSummary;
    use opendomain_core::provenance::{verify, RunCitation};

    let base = std::env::temp_dir().join("opendomain-bench-provenance");
    let _ = std::fs::remove_dir_all(&base);
    let run = base.join("run-001-bench");
    if std::fs::create_dir_all(run.join("context")).is_err() {
        return false;
    }
    if std::fs::write(run.join("graph.json"), r#"{"meta":null,"nodes":[]}"#).is_err() {
        return false;
    }
    let summary = RunSummary {
        passed: vec!["probeur".into()],
        failed: None,
        events: vec![
            opendomain_core::harness::TraceEvent::Start { node: "probeur".into() },
            opendomain_core::harness::TraceEvent::Passed { node: "probeur".into(), attempts: 1 },
        ],
    };
    if std::fs::write(run.join("gates.log"), gates_log(&summary)).is_err() {
        return false;
    }
    if std::fs::write(run.join("context").join("probeur.json"), r#"{"ok":true}"#).is_err() {
        return false;
    }
    let rv = verify(
        &base,
        &RunCitation { run: "run-001-bench".into(), node: Some("probeur".into()) },
    );
    rv.exists
        && rv.frozen_graph
        && rv.gates_log
        && rv.outputs == 1
        && rv.node_verdict.as_ref().map(|v| v.is_ok()).unwrap_or(false)
}

/// Un besoin = un fournisseur ; un besoin sans fournisseur est un `miss`
/// typé (jamais un composant inventé ou un silence).
fn test_resolver_per_need() -> bool {
    use crate::harness::components::{Component, Registry, default_state, resolve};
    let mut registry = Registry::default();
    registry.components.push(Component {
        id: "revue".into(),
        kind: "skill".into(),
        provides: vec!["design.revue".into()],
        // pas de Default sur Component : la provenance est explicite
        requires: Default::default(),
        source_kind: "local".into(),
        source: String::new(),
        source_url: None,
        version: None,
        installed_at: None,
        state: default_state(),
        applies_to: Vec::new(),
        preferred: false,
    });
    let r = resolve(&registry, &["design.revue".into()]);
    let r2 = resolve(&registry, &["fusion.manifestes".into()]);
    r.misses.is_empty()
        && r.hits.len() == 1
        && r.hits[0].0 == "design.revue"
        && r.chosen.len() == 1
        && r2.chosen.is_empty()
        && r2.misses == vec!["fusion.manifestes".to_string()]
        && r2.hits.is_empty()
}
