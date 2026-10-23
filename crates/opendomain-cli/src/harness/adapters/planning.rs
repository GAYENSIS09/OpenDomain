//! La boucle de conception — `Generate → Verify → Attribute → Refine` appliqué au
//! graphe : le concepteur (LLM via l'éditeur) propose, le harnais
//! **vérifie** (sérialisation puis admission au même moteur que `plan` :
//! structure, complétude, expressivité de l'éditeur), et tout défaut typé
//! revient comme feedback au raffinement suivant.
//!
//! Le transport est une fonction injectable : en prod c'est `opencode_run`,
//! en test c'est un transport scripté — la boucle de conception devient
//! vérifiable sans réseau (S4).

use anyhow::{anyhow, Result};
use opendomain_core::harness::GraphFile;

use super::transport::{opencode_run, Captured};

/// Le transport de la boucle de conception : un prompt, un timeout, la
/// sortie processus.
pub type PlanTransport<'t> = dyn Fn(&str, u64) -> Result<Captured> + 't;

/// `plan_with_llm` — voie réelle : transport `opencode run`, modèle
/// épinglé par `[harness] model` (fallback `OPENCODE_MODEL`) — jamais le
/// défaut fragile du CLI (leçon du 2026-10-23).
pub fn plan_with_llm(
    spec_slug: &str,
    spec_text: &str,
    detected_language: &str,
    timeout_secs: u64,
) -> Result<GraphFile> {
    let model = crate::harness::common::load_harness_bindings()?.model;
    plan_with_llm_transport(
        spec_slug,
        spec_text,
        detected_language,
        timeout_secs,
        &move |prompt, t| opencode_run(prompt, t, model.as_deref()),
    )
}

/// La boucle, transport visible — le LLM propose un graphe, le harnais le
/// vérifie (sérialisation + DAG + admission), tout défaut typé est
/// renvoyé comme feedback du raffinement suivant (budget :
/// `[gates] max_attempts`).
pub fn plan_with_llm_transport(
    spec_slug: &str,
    spec_text: &str,
    detected_language: &str,
    timeout_secs: u64,
    transport: &PlanTransport,
) -> Result<GraphFile> {
    let mut feedback = String::from("(première tentative)");
    // Le budget de la boucle de conception est celui du harnais : la config
    // `[gates] max_attempts` gouverne — plus aucun littéral.
    let max = crate::harness::common::loop_config()?.max_attempts;
    for attempt in 1..=max {
        let prompt = build_plan_prompt(spec_slug, spec_text, detected_language, &feedback);
        let out = transport(&prompt, timeout_secs)?;
        if !out.status_ok && out.stdout.trim().is_empty() {
            return Err(anyhow!(
                "`opencode run` a échoué : {}",
                out.stderr.lines().last().unwrap_or("(cause inconnue)")
            ));
        }
        let Some(json) = extract_json(&out.stdout).map(|s| s.to_string()) else {
            feedback = "aucun objet JSON trouvé dans la sortie — renvoie UNIQUEMENT l'objet JSON du graphe".into();
            continue;
        };
        match serde_json::from_str::<GraphFile>(&json) {
            Ok(file) => match opendomain_core::harness::admit(&file, &super::capabilities::gaps) {
                Ok(_) => return Ok(file),
                Err(e) => {
                    // Le verdict typé du MÊME moteur que plan/run/build en
                    // feedback : structure, complétude ou expressivité — le
                    // concepteur corrige la faute nommée, pas à l'aveugle.
                    feedback =
                        format!("admission refusée ({e}) — corrige et ne renvoie que le JSON");
                }
            },
            Err(e) => {
                feedback =
                    format!("JSON non sérialisable ({e}) — corrige et ne renvoie que le JSON");
            }
        }
        eprintln!("  ↻ tentative {attempt}: raffinement déclenché — {feedback}");
    }
    Err(anyhow!(
        "plan via opencode : {max} tentative(s) épuisée(s) sans graphe valide"
    ))
}

/// Extrait le premier objet JSON complet d'une sortie prose.
pub fn extract_json(text: &str) -> Option<&str> {
    let begin = text.find('{')?;
    let end = text.rfind('}')?;
    if end > begin {
        Some(&text[begin..=end])
    } else {
        None
    }
}

fn build_plan_prompt(
    spec_slug: &str,
    spec_text: &str,
    detected_language: &str,
    feedback: &str,
) -> String {
    let spec_head: String = spec_text.lines().take(220).collect::<Vec<_>>().join("\n");
    format!(
        "Tu es le PLANIFICATEUR du harnais OpenDomain (construction).\n\
Retourne EXCLUSIVEMENT un objet JSON, sans prose, sans bloc de code, avec exactement ces règles :\n\
- clés: \"meta\" (objet libre) et \"nodes\" (tableau de 2 à 6 spécifications d'agents σ_i)\n\
- chaque node: {{\"id\": identifiant ascii sans accents, \"role\": directive complète, \"tools\": whitelist stricte (read/grep/glob/write/edit/bash), \"contract\": {{\"input_schema\": objet JSON Schema, \"output_schema\": objet JSON Schema avec \"required\" non vide, \"forbidden\": motifs interdits}}, \"criteria\": {{\"schema_check\": true, \"contains\": [...], \"not_contains\": [...], \"commands\": [...] }}, \"depends_on\": [ids des nœuds requis]}}\n\
- DÉCOMPOSE depuis la tâche, pas d'un moule : dérive les rôles, leur nombre et leur ordre des besoins réels de la SPEC (une correction de parseur n'a pas besoin d'un synthétiseur ; une tâche de recherche n'a pas besoin d'un éditeur). Le moule analyse→stratégie→synthèse→vérification est un cas particulier, jamais un réflexe ; 2 à 6 nœuds, graphe acyclique.\n\
- crée des CRITÈRES observables pour chaque nœud : ce que la sortie DOIT contenir (contains), ne doit pas contenir (not_contains), et — quand le travail est vérifiable par commande (tests, lint, compilation) — la commande qui le prouve dans \"commands\" (elle tournera sous sandbox, hors de l'agent).\n\
- NE PRODUIS AUCUN CODE MÉTIER ; le graphe décrit des agents, pas la solution.\n\
- langue cible du projet: {detected_language}.\n\
Retour du contrôle précédent (à corriger si nécessaire): {feedback}\n\
--- SPEC ({spec_slug}) ---\n{spec_head}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn extraction_json_prose() {
        let s = "Voici la sortie:\n{\"meta\": {}, \"nodes\": []}\nBonne journée.";
        assert_eq!(extract_json(s).unwrap(), "{\"meta\": {}, \"nodes\": []}");
        assert!(extract_json("pas de json").is_none());
    }

    #[test]
    fn prompt_contient_regles_de_gates() {
        let p = build_plan_prompt("001-x", "spec", "python", "ok");
        assert!(p.contains("nodes"));
        assert!(p.contains("output_schema"));
        assert!(p.contains("python"));
        assert!(p.contains("PLANIFICATEUR"), "rôle : {p}");
    }

    const GRAPHE_OK: &str = r#"{"meta": {}, "nodes": []}"#;

    #[test]
    fn la_boucle_converge_avec_un_transport_scripte_sans_reseau() {
        // Appel 1 : prose sans JSON → feedback demandant l'objet seul ;
        // appel 2 : graphe valide → Ok. Le transport injecté est réel
        // (aucun `opencode run` lancé) et le feedback du refus revient dans
        // le prompt suivant.
        let prompts: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let pas = RefCell::new(0u32);
        let transport = |prompt: &str, _t: u64| -> Result<Captured> {
            let premier = *pas.borrow() == 0;
            *pas.borrow_mut() += 1;
            prompts.borrow_mut().push(prompt.to_string());
            Ok(Captured {
                status_ok: true,
                stdout: if premier {
                    String::from("bonne journée, sans objet")
                } else {
                    GRAPHE_OK.into()
                },
                stderr: String::new(),
            })
        };
        let file = plan_with_llm_transport("spec-bench", "texte", "rust", 60, &transport)
            .expect("arrive en deux tentatives");
        assert!(file.nodes.is_empty());
        assert_eq!(*pas.borrow(), 2, "deux appels, pas un de plus");
        assert!(
            prompts.borrow()[1].contains("aucun objet JSON"),
            "le feedback du premier refus revient dans le prompt suivant"
        );
    }

    #[test]
    fn la_boucle_epuise_le_budget_puis_refuse_typed() {
        let motif = self_cycle_graph();
        let transport = |_prompt: &str, _t: u64| -> Result<Captured> {
            Ok(Captured {
                status_ok: true,
                stdout: motif.clone(),
                stderr: String::new(),
            })
        };
        let err = plan_with_llm_transport("spec-bench", "texte", "rust", 60, &transport)
            .expect_err("graphe auto-invalide à chaque tentative");
        let msg = format!("{err}");
        assert!(msg.contains("tentative(s) épuisée(s)"), "message : {msg}");
    }

    /// Un graphe toujours self-cycle : le feedback porte bien la cause
    /// typée du refus (`graphe invalide (...)`) à chaque tentative.
    fn self_cycle_graph() -> String {
        r#"{"meta": {}, "nodes": [{
            "id": "x", "role": "r", "tools": ["read"],
            "contract": {"input_schema": null, "output_schema": null, "forbidden": []},
            "criteria": {"schema_check": true, "contains": [], "not_contains": [], "commands": [], "contains_in": {}},
            "depends_on": ["x"]
        }]}"#
        .to_string()
    }

    #[test]
    fn la_boucle_raffine_sur_un_refus_de_capacite() {
        // Appel 1 : un graphe structuralement valide MAIS portant un outil
        // hors vocabulaire (`oracle`) — le refus vient de l'ANCRE de
        // capacité, au même moteur, et revient nommée dans le prompt 2 ;
        // appel 2 : le graphe corrigé, complet et exprimable → Ok.
        let prompts: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let pas = RefCell::new(0u32);
        let transport = |prompt: &str, _t: u64| -> Result<Captured> {
            let n = *pas.borrow();
            *pas.borrow_mut() += 1;
            prompts.borrow_mut().push(prompt.to_string());
            Ok(Captured {
                status_ok: true,
                stdout: if n == 0 {
                    orchestr_graph()
                } else {
                    ORCHESTION_CORRIGEE.into()
                },
                stderr: String::new(),
            })
        };
        let file = plan_with_llm_transport("spec-bench", "texte", "rust", 60, &transport)
            .expect("le raffinement corrige l'outil non exprimable");
        assert_eq!(file.nodes.len(), 1);
        assert_eq!(*pas.borrow(), 2, "deux appels, pas un de plus");
        let p2 = &prompts.borrow()[1];
        assert!(
            p2.contains("capacités non satisfaites") && p2.contains("oracle"),
            "la cause typée (capacité) revient dans le prompt suivant : {p2}"
        );
    }

    /// Graphe structuralement valide mais un outil hors vocabulaire.
    fn orchestr_graph() -> String {
        r#"{"meta": {}, "nodes": [{
            "id": "x", "role": "r", "tools": ["oracle", "read"],
            "contract": {"input_schema": null, "output_schema": null, "forbidden": []},
            "criteria": {"schema_check": true, "contains": [], "not_contains": [], "commands": [], "contains_in": {}},
            "depends_on": []
        }]}"#
        .to_string()
    }

    /// Le même graphe corrigé : whitelist exprimable, contrat et critères
    /// non vides — le raffinement est vérifié, pas supposé.
    const ORCHESTION_CORRIGEE: &str = r#"{"meta": {}, "nodes": [{
            "id": "x", "role": "rôle", "tools": ["read"],
            "contract": {"input_schema": null, "output_schema": {"type": "object", "required": ["ok"]}, "forbidden": []},
            "criteria": {"schema_check": true, "contains": [], "not_contains": [], "commands": [], "contains_in": {}},
            "depends_on": []
        }]}"#;
}
