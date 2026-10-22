//! Exécuteurs d'agent pour la phase 2 :
//! - [`LlmExecutor`] : délègue chaque nœud à `opencode run --agent` /
//!   `claude -p --allowedTools` (executor-agnostic) — les commandes de
//!   critères **ne passent pas** par l'exécuteur : le runner les exécute
//!   (mêmes gates pour tous, structurellement) ;
//! - [`MockGraphExecutor`] : sorties **déterministes** conformes au graphe
//!   canonique — sert à CI, démo et vérification du harnais sans LLM.

use anyhow::{anyhow, Result};
use opendomain_core::harness::{AgentSpec, Failure, FailureKind};
use serde_json::{json, Value};

use crate::harness::adapters::{extract_json, run_captured};

/// Exécuteur LLM existant (`opencode`, `claude`).
pub struct LlmExecutor {
    pub bin: String,
    pub timeout_secs: u64,
    /// Skills résolues du registre (`opendomain resolve`), attachées selon
    /// `applies-to` — pillar 6 du kit, branché dans la boucle réelle.
    pub skills: Vec<crate::harness::components::SkillAttach>,
}

impl LlmExecutor {
    /// Instancie et vérifie la présence du binaire via PATH.
    pub fn new(bin: &str, timeout_secs: u64) -> Result<Self> {
        if crate::harness::doctor::which(bin).is_none() {
            return Err(anyhow!(
                "`{bin}` introuvable sur PATH — voir `opendomain doctor`, ou lancer \
`--executor mock` pour un run déterministe sans LLM"
            ));
        }
        Ok(Self {
            bin: bin.to_string(),
            timeout_secs,
            skills: Vec::new(),
        })
    }

    fn args(&self, prompt: &str, spec: &AgentSpec) -> Vec<String> {
        match self.bin.as_str() {
            "claude" => {
                let mut a = vec!["-p".into(), prompt.into()];
                // Moindre privilège projeté : l'allowlist du nœud devient un
                // flag du CLI Claude (non défini = refusé), pas une phrase.
                if !spec.tools.is_empty() {
                    a.push("--allowedTools".into());
                    a.push(spec.tools.iter().map(|t| claude_tool(t)).collect::<Vec<_>>().join(","));
                }
                a
            }
            _ => {
                // opencode (défaut) : agent de nœud généré avec permissions
                // V2 (deny-all puis allow de la whitelist), invoqué `--agent`.
                let mut a = vec!["run".into(), prompt.into()];
                if let Ok(agent_id) = crate::harness::adapters::write_opencode_agent(spec) {
                    a.push("--agent".into());
                    a.push(agent_id);
                }
                a
            }
        }
    }
}

impl opendomain_core::harness::Executor for LlmExecutor {
    fn run(
        &mut self,
        spec: &AgentSpec,
        input: &Value,
        feedback: Option<&Failure>,
    ) -> Result<Value> {
        let prompt = node_prompt(spec, input, feedback, &self.skills);
        let out = run_captured(&self.bin, &self.args(&prompt, spec), self.timeout_secs)?;
        if !out.status_ok && out.stdout.trim().is_empty() {
            return Err(anyhow!(
                "`{}` a échoué : {}",
                self.bin,
                out.stderr.lines().last().unwrap_or("(cause inconnue)")
            ));
        }
        let Some(json) = extract_json(&out.stdout).map(|s| s.to_string()) else {
            return Err(anyhow!(
                "aucun objet JSON dans la sortie — renvoyer UNIQUEMENT l'objet conforme au contrat"
            ));
        };
        let parsed: Value = serde_json::from_str(&json)
            .map_err(|e| anyhow!("JSON non sérialisable : {e} — corriger le contrat"))?;
        // Génération pure : les commandes de critères tournent côté runner.
        Ok(parsed)
    }

    /// Canal d'erreurs typé : le parse d'enveloppe est une violation de
    /// contrat ; un dépassement de délai reste une faute locale dirigée
    /// (hint d'augmentation de timeout). Le runner pose les hints.
    fn classify_error(&self, err: &anyhow::Error) -> FailureKind {
        let msg = format!("{err:#}");
        if msg.contains("JSON") {
            FailureKind::ContractViolation
        } else {
            FailureKind::SpecMismatch
        }
    }
}

/// Traduction d'une whitelist vers les noms d'outils Claude Code
/// (canonical) — projection réelle : l'exécuteur lui-même refuse le reste.
fn claude_tool(tool: &str) -> String {
    match tool.trim().to_lowercase().as_str() {
        "read" => "Read".into(),
        "write" => "Write".into(),
        "edit" => "Edit".into(),
        "bash" => "Bash".into(),
        "grep" => "Grep".into(),
        "glob" => "Glob".into(),
        "task" | "subagent" => "Task".into(),
        "webfetch" => "WebFetch".into(),
        "websearch" => "WebSearch".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

/// Exécute une commande de critère (gate) dans le repo cible (cwd courant).
/// `npm`, `.bat`… sont wrappés via `cmd /C` sous Windows par `run_captured`.
/// La sandbox logique (allowlist de `opendomain/opendomain.toml`, section `[sandbox]`) bloque
/// tout binaire non autorisé AVANT exécution.
pub fn run_gate_command(cmd: &str, timeout_secs: u64) -> Result<()> {
    let parts: Vec<String> = cmd.split_whitespace().map(str::to_string).collect();
    let (bin, args) = parts.split_first().ok_or_else(|| anyhow!("commande vide"))?;
    crate::harness::common::load_sandbox()?.permits(bin)?;
    let out = run_captured(bin, &args.to_vec(), timeout_secs)?;
    if out.status_ok {
        Ok(())
    } else {
        let mut tail: Vec<&str> = out.stderr.lines().collect();
        if tail.len() > 6 {
            tail.drain(..tail.len() - 6);
        }
        Err(anyhow!("commande `{cmd}` a échoué : {}", tail.join(" | ")))
    }
}

/// Prompt d'un nœud : rôle + outils + skills applicables + contrats +
/// critères + entrées amont + feedback éventuel de retry.
pub fn node_prompt(
    spec: &AgentSpec,
    input: &Value,
    feedback: Option<&Failure>,
    skills: &[crate::harness::components::SkillAttach],
) -> String {
    let input_brief = brief(input);
    let contract_out = if spec.contract.output_schema.is_null() {
        "(non contraint)".into()
    } else {
        spec.contract.output_schema.to_string()
    };
    let contract_in = if spec.contract.input_schema.is_null() {
        "(non contraint)".into()
    } else {
        spec.contract.input_schema.to_string()
    };
    let criteria = format!(
        "aiguilles brute requises: {:?}\naiguilles bornées (champ => exigé dans CE champ): {:?}\ninterdits: {:?}\ncommandes (exécutées par le runner, pas par toi): {:?}",
        spec.criteria.contains,
        spec.criteria.contains_in,
        merge_forbidden(spec),
        spec.criteria.commands
    );
    let tools = tools_brief(spec);
    let skills_text = crate::harness::components::applicable(skills, &spec.id)
        .iter()
        .map(|s| format!("--- skill `{}` ({} ) ---\n{}", s.id, s.applies_to.join(","), s.text))
        .collect::<Vec<_>>()
        .join("\n\n");
    let skills_section = if skills_text.is_empty() {
        String::new()
    } else {
        format!("\nSKILLS APPLICABLES (procédures à suivre, pas des suggestions) :\n{skills_text}\n")
    };
    let retry = match feedback {
        Some(f) => format!(
            "\n⚠ TA TENTATIVE PRÉCÉDENTE A ÉTÉ REJETÉE PAR LE GATE ({}) :\n{}\nindices : {:?}\nCorrige et ne renvoie que le JSON corrigé.",
            f.kind.label(),
            f.message,
            f.hints
        ),
        None => String::new(),
    };
    format!(
        "Tu es l'agent `{id}` du harnais OpenDomain (phase 2).\n\
RÔLE : {role}\n\
OUTILS AUTORISÉS (moindre privilège — tout autre outil : violation) : {tools}\n\
CONTRAT D'ENTRÉE : {contract_in}\nENTRÉES AMONT (contexte) : {input_brief}\n\
CONTRAT DE SORTIE — RENVOI UNIQUEMENT l'objet JSON conforme, sans prose ni bloc de code :\n{contract_out}\n\
CRITÈRES DE GATE :\n{criteria}{skills_section}{retry}",
        id = spec.id,
        role = spec.role,
        tools = tools,
    )
}

/// Motifs interdits = critères `not_contains` ∪ contrat `forbidden`.
fn merge_forbidden(spec: &AgentSpec) -> Vec<&str> {
    let mut v: Vec<&str> = spec.criteria.not_contains.iter().map(String::as_str).collect();
    v.extend(spec.contract.forbidden.iter().map(String::as_str));
    v
}

/// Brief des outils (pillar « tools ») pour le prompt.
fn tools_brief(spec: &AgentSpec) -> String {
    if spec.tools.is_empty() {
        "(aucun — lecture seule)".into()
    } else {
        spec.tools.join(", ")
    }
}

/// Sérialisation bornée des entrées (évite de saturer un prompt).
fn brief(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 6000 {
        let cut: String = s.chars().take(6000).collect();
        format!("{cut}… (tronqué)")
    } else {
        s
    }
}

/// Exécuteur simulé : sorties conformes au graphe canonique, zéro réseau.
pub struct MockGraphExecutor;

impl opendomain_core::harness::Executor for MockGraphExecutor {
    fn run(
        &mut self,
        spec: &AgentSpec,
        _input: &Value,
        _feedback: Option<&Failure>,
    ) -> Result<Value> {
        let id = spec.id.to_lowercase();
        let canned = if id.contains("analys") {
            json!({
                "regles": ["regle: inegalité stricte (< seuil)"],
                "cas_limites": ["cas limite: liste vide", "cas limite: seuil zero"],
                "ambiguites": []
            })
        } else if id.contains("strat") {
            json!({ "algorithme": "comparaison par paires avec retour anticipé", "plan": ["plan: traiter cas limites d'abord"] })
        } else if id.contains("synth") || id.contains("code") {
            json!({ "fichier": "src/mock.rs", "code": "```rust\nfn mock() {}\n```" })
        } else if id.contains("verif") || id.contains("verdict") {
            json!({ "verdict": "PASS", "raison": "controles conformes au graphe canonique", "controles": ["contrat", "cas limites"] })
        } else {
            // Nœud non canonique : placeholder fragile → montre le retry + attribution.
            json!({ "note": "mock: pas de sortie conforme pour ce nœud" })
        };
        // Génération pure : la sandbox et les commandes appartiennent au runner.
        Ok(canned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_prompt_contient_contrat_et_feedback() {
        let spec = AgentSpec::default();
        // prompt minimal sans feedback
        let p = node_prompt(&spec, &json!({"a": 1}), None, &[]);
        assert!(p.contains("JSON"));
        // avec feedback typé
        let f = Failure::new("x", opendomain_core::harness::FailureKind::ContractViolation, "champ requis");
        let p2 = node_prompt(&spec, &json!({}), Some(&f), &[]);
        assert!(p2.contains("CONTRAT"), "p2.begin: {}", p2.chars().take(60).collect::<String>());
    }

    #[test]
    fn mock_conforme_au_canonique() {
        use opendomain_core::harness::Dag;
        use opendomain_core::harness::gate;
        use opendomain_core::harness::Executor as _;
        let g = crate::harness::plan::build_heuristic_graph("001-demo", "# Spec\n\n## Contraintes\n- stack: rust\n\n## Critères\n- [ ] ok\n");
        let dag = Dag::build(&g).unwrap();
        let mut mock = MockGraphExecutor;
        for node in dag.topological_order().unwrap() {
            let spec = &dag.nodes[&node];
            let out = mock.run(spec, &json!({}), None).unwrap();
            assert!(gate(spec, &out).is_ok(), "mock non conforme pour {node}");
        }
    }
}
