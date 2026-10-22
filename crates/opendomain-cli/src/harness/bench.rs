//! M5 — `opendomain bench` : suite de bancs *déterministes* pour le harnais
//! (offline, sans LLM). Contrairement aux benchmarks IA à reproduire
//! (HumanEval/MBPP…), ici on mesure les **invariants mécaniques** :
//! 1. refus des graphes invalides (cycle, arête pendante),
//! 2. gates : toute corruption de `y_i` est interceptée (requis, interdit),
//! 3. convergeance du retry avec feedback,
//! 4. attribution correcte par type d'échec,
//! 5. reprise re-vérifiée : les sorties admises ne ré-exécutent pas,
//! 6. sandbox : allowlist honorée AVANT exécution,
//! 7. résolveur : un fournisseur par besoin, sinon échec typé.
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
fn test_sandbox_allowlist() -> bool {
    let on = crate::harness::common::parse_sandbox_config(
        "[opendomain]\nversion = 1\n\n[sandbox]\nenabled = true\nallow = [\"cargo\", \"rustc\"]\n",
    );
    let off = crate::harness::common::parse_sandbox_config("[opendomain]\nversion = 1\n");
    on.enabled
        && off.permits("curl").is_ok() // désactivée = fail-open assumé, explicite
        && on.permits("cargo").is_ok()
        && on.permits("curl").is_err()
        && on.permits("curl").err().unwrap().to_string().contains("SandboxViolation")
}

/// Un besoin = un fournisseur ; un besoin sans fournisseur est un `miss`
/// typé (jamais un composant inventé ou un silence).
fn test_resolver_per_need() -> bool {
    use crate::harness::components::{Component, Registry, resolve};
    let mut registry = Registry::default();
    registry.components.push(Component {
        id: "revue".into(),
        kind: "skill".into(),
        provides: vec!["design.revue".into()],
        ..Default::default()
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
