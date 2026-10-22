//! `opendomain plan` — la spec `T` devient un DAG d'agents `σ_i`.
//!
//! Deux voies :
//! 1. **hors-ligne** (défaut, déterministe, testable) : décomposition
//!    canonique `analyste → strategie → synthese → verificateur`, spécialisée
//!    par la spec (langue, contraintes, interdits nommés).
//! 2. **`--llm`** : `opencode run` propose le graphe ; le harnais le VÉRIFIE
//!    et, en cas de défaut typé, relance l'étape amont (boucle de correction).
//!
//! Dans les deux cas, la sortie est validée par `opendomain-harness` avant écriture
//! (archive `graph/<slug>.json` + graphe actif `graph/graph.json`).

use anyhow::Result;
use opendomain_core::harness::{AgentSpec, Contract, Criteria, Dag, GraphFile};
use serde_json::json;

use crate::harness::common::{detect_language, extract_forbidden, resolve_spec, section_lines, title_of};

/// `opendomain plan <spec> [--llm] [--timeout S]`.
pub fn run(spec_input: &str, use_llm: bool, timeout_secs: u64) -> Result<()> {
    let sp = resolve_spec(spec_input)?;
    let text = std::fs::read_to_string(&sp.spec)?;

    let file = if use_llm {
        match crate::harness::adapters::plan_with_llm(&sp.slug, &text, detect_language(&text), timeout_secs) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("!  --llm a échoué : {e}");
                eprintln!("  ↳ repli sur le planificateur hors-ligne (déterministe)");
                build_heuristic_graph(&sp.slug, &text)
            }
        }
    } else {
        build_heuristic_graph(&sp.slug, &text)
    };

    // Vérification de construction (statique) — on n'écrit jamais un graphe
    // invalide ou incomplet : l'admission du plan exige la complétude de
    // chaque agent (le même contrôle que `build` — la Garantie 3 appartient
    // au plan, pas à l'étape suivante).
    let dag = Dag::build(&file)?;
    let order = dag.topological_order()?;
    admission_complete(&dag)?;

    let graph_dir = std::path::Path::new("opendomain/harness/graph");
    std::fs::create_dir_all(graph_dir)?;
    let archive = graph_dir.join(format!("{}.json", sp.slug));
    let active = graph_dir.join("graph.json");
    let payload = serde_json::to_string_pretty(&file)?;
    std::fs::write(&archive, &payload)?;
    std::fs::write(&active, &payload)?;

    println!(
        "OK plan ({} nœud(s)) → {} (+ graphe actif {})",
        dag.nodes.len(),
        archive.display(),
        active.display()
    );
    println!("  ordre topologique : {}", order.join(" -> "));
    println!("  prochain pas      : opendomain build {}", sp.slug);
    Ok(())
}

/// Admission du plan : **complétude** de chaque agent avant écriture du
/// graphe (Garantie 3 — « a plan is admitted only with complete contracts »).
/// Une incomplétude sort typée, nœud et champ nommés.
pub(crate) fn admission_complete(dag: &Dag) -> Result<()> {
    let mut incomplets: Vec<String> = Vec::new();
    for (id, spec) in &dag.nodes {
        let manquants: Vec<String> = crate::harness::common::agent_completeness(spec)
            .into_iter()
            .filter(|(_, ok, _)| !ok)
            .map(|(_, _, field)| field)
            .collect();
        if !manquants.is_empty() {
            incomplets.push(format!("{id} : {}", manquants.join(", ")));
        }
    }
    if incomplets.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "plan refusé — agents incomplets (contrat/critères) : {}\n  → corriger le graphe proposé, ou lancer `opendomain plan <slug>` hors --llm",
            incomplets.join(" ; ")
        )
    }
}

/// Décomposition canonique, spécialisée par la spec. Déterministe : renvoie
/// toujours un graphe que `Dag::build` accepte.
pub fn build_heuristic_graph(slug: &str, spec_text: &str) -> GraphFile {    let title = title_of(spec_text).unwrap_or_else(|| slug.to_string());
    let lang = detect_language(spec_text);
    let constraints = section_lines(spec_text, "Contraintes", 12);
    let forbidden = extract_forbidden(&constraints);
    let constraints_bits = {
        let joined = dedupe(constraints.iter().take(4).cloned()).join(" ; ");
        if joined.is_empty() {
            String::new()
        } else {
            format!(" (contraintes : {joined})")
        }
    };

    let analyste = AgentSpec {
        id: "analyste".into(),
        role: format!(
            "Analyste de spécification pour « {title} » : extraire règles comportementales, \
cas limites explicites et implicites, ambiguïtés résolues (citer la source dans la spec). \
Ne produit aucun code ; alimente la stratégie et le vérificateur."
        ),
        tools: vec!["read".into(), "grep".into(), "glob".into()],
        contract: Contract {
            input_schema: json!({
                "type": "object", "required": ["spec"],
                "properties": { "spec": { "type": "string" } }
            }),
            output_schema: json!({
                "type": "object", "required": ["regles", "cas_limites", "ambiguites"],
                "properties": {
                    "regles": { "type": "array", "items": { "type": "string" } },
                    "cas_limites": { "type": "array", "items": { "type": "string" } },
                    "ambiguites": { "type": "array", "items": { "type": "string" } }
                }
            }),
            forbidden: vec!["```".into()],
        },
        criteria: Criteria {
            schema_check: true,
            contains: Vec::new(),
            not_contains: Vec::new(),
            commands: Vec::new(),
            contains_in: {
                let mut m = std::collections::BTreeMap::new();
                // aiguille bornée au champ réel — un écho ailleurs ne compte plus
                m.insert("cas_limites".into(), vec!["cas limite".into()]);
                m
            },
        },
        depends_on: Vec::new(),
        ..Default::default()
    };

    let strategie = AgentSpec {
        id: "strategie".into(),
        role: format!(
            "Stratège pour « {title} » : choisir l'algorithme et les structures de données \
({lang}), planifier le traitement de chaque cas limite de l'analyste, avec retour \
anticipé quand pertinent. Aucun code métier final."
        ),
        tools: vec!["read".into(), "grep".into()],
        contract: Contract {
            input_schema: json!({ "type": "object", "required": ["analyste"] }),
            output_schema: json!({
                "type": "object", "required": ["algorithme", "plan"],
                "properties": {
                    "algorithme": { "type": "string" },
                    "plan": { "type": "array", "items": { "type": "string" } }
                }
            }),
            forbidden: Vec::new(),
        },
        criteria: Criteria {
            schema_check: true,
            contains: Vec::new(),
            not_contains: Vec::new(),
            commands: Vec::new(),
            contains_in: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("plan".into(), vec!["plan:".into()]);
                m
            },
        },
        depends_on: vec!["analyste".into()],
        ..Default::default()
    };

    let synthese = AgentSpec {
        id: "synthese".into(),
        role: format!(
            "Synthétiseur pour « {title} » : émettre l'implémentation {lang} dans un bloc de \
code unique ; respecter les contraintes{constraints_bits} ; un traitement par cas \
limite planifié ; docstring/commentaire de la spec conservé verbatim."
        ),
        tools: vec!["read".into(), "write".into(), "edit".into()],
        contract: Contract {
            input_schema: json!({ "type": "object", "required": ["strategie"] }),
            output_schema: json!({
                "type": "object", "required": ["fichier", "code"],
                "properties": {
                    "fichier": { "type": "string" },
                    "code": { "type": "string", "minLength": 10 }
                }
            }),
            forbidden,
        },
        criteria: Criteria {
            schema_check: true,
            contains: Vec::new(),
            not_contains: Vec::new(),
            commands: Vec::new(),
            contains_in: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("code".into(), vec!["```".into()]);
                m
            },
        },
        depends_on: vec!["strategie".into()],
        ..Default::default()
    };

    let verificateur = AgentSpec {
        id: "verificateur".into(),
        role: format!(
            "Vérificateur final pour « {title} » : contrôler chaque critère de la spec \
(conformité, cas limites couverts, motifs interdits absents) et émettre un \
verdict PASS/FAIL motivé — jamais ambigu, jamais optimiste."
        ),
        tools: vec!["read".into(), "grep".into(), "bash".into()],
        contract: Contract {
            input_schema: json!({ "type": "object", "required": ["synthese"] }),
            output_schema: json!({
                "type": "object", "required": ["verdict", "raison", "controles"],
                "properties": {
                    "verdict": { "type": "string", "minLength": 4 },
                    "raison": { "type": "string", "minLength": 10 },
                    "controles": { "type": "array", "minItems": 1, "items": { "type": "string" } }
                }
            }),
            forbidden: Vec::new(),
        },
        criteria: Criteria {
            schema_check: true,
            // Pas d'aiguille sur `verdict` : un verdict FAIL motivé est une
            // sortie légitime du vérificateur, pas une faute du vérificateur.
            // Le gate l'exige articulé : verdict + raison + contrôles cités.
            contains: Vec::new(),
            not_contains: Vec::new(),
            commands: Vec::new(),
            contains_in: Default::default(),
        },
        depends_on: vec!["synthese".into()],
        ..Default::default()
    };

    GraphFile {
        meta: json!({
            "name": slug,
            "generator": "opendomain plan (hors-ligne)",
            "langue": lang,
            "source": "plan hors-ligne ou --llm"
        }),
        nodes: vec![analyste, strategie, synthese, verificateur],
    }
}

/// Dédoublonnage de chaînes (utilitaire local).
fn dedupe(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for i in items {
        if !i.trim().is_empty() && seen.insert(i.clone()) {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Spec 001 — migrer les logs\n\n## Contraintes\n- stack: rust\n- interdiction: appels `unwrap()` dans le parsing\n\n## Critères\n- [ ] tests verts\n";

    #[test]
    fn le_graphe_canonique_est_valide() {
        let file = build_heuristic_graph("001-migrer", SAMPLE);
        let dag = Dag::build(&file).expect("graphe canonique doit passer Dag::build");
        assert_eq!(
            dag.topological_order().unwrap(),
            vec!["analyste", "strategie", "synthese", "verificateur"]
        );
    }

    #[test]
    fn interdits_de_la_spec_propages() {
        let file = build_heuristic_graph("001-migrer", SAMPLE);
        let synthese = file.nodes.iter().find(|n| n.id == "synthese").unwrap();
        assert_eq!(synthese.contract.forbidden, vec!["unwrap()".to_string()]);
    }

    #[test]
    fn le_planificateur_refuse_le_cycle() {
        let bad = GraphFile {
            meta: json!({}),
            nodes: vec![
                AgentSpec {
                    id: "a".into(),
                    role: "r".into(),
                    depends_on: vec!["b".into()],
                    ..Default::default()
                },
                AgentSpec {
                    id: "b".into(),
                    role: "r".into(),
                    depends_on: vec!["a".into()],
                    ..Default::default()
                },
            ],
        };
        let err = format!("{:#}", Dag::build(&bad).unwrap_err());
        assert!(err.contains("cycle"), "message: {err}");
    }

    #[test]
    fn le_plan_refuse_les_agents_incomplets_avant_ecriture() {
        let bad = GraphFile {
            meta: json!({}),
            nodes: vec![
                AgentSpec {
                    id: "a".into(),
                    role: "rôle présent".into(),
                    ..Default::default()
                },
            ],
        };
        let dag = Dag::build(&bad).expect("structure OK, complétude en faute");
        let err = admission_complete(&dag).expect_err("admission complète refuse");
        let text = err.to_string();
        assert!(text.contains("incomplets"), "message typé : {text}");
        assert!(text.contains("tools"), "champ fautif nommé : {text}");
        assert!(text.contains("contract"), "contrat nommé : {text}");
    }

    #[test]
    fn les_criteres_canoniques_sont_bornes_aux_champs() {
        let file = build_heuristic_graph("001-migrer", SAMPLE);
        let synthese = file.nodes.iter().find(|n| n.id == "synthese").unwrap();
        assert!(
            synthese.criteria.contains.is_empty(),
            "plus d'aiguille brute, écho-satisfaisable"
        );
        assert_eq!(
            synthese.criteria.contains_in.get("code").cloned(),
            Some(vec!["```".to_string()]),
            "l'aiguille du code est bornée au champ `code`"
        );
        let analyste = file.nodes.iter().find(|n| n.id == "analyste").unwrap();
        assert_eq!(
            analyste.criteria.contains_in.get("cas_limites").cloned(),
            Some(vec!["cas limite".to_string()])
        );
    }

    #[test]
    fn dedupe_fonctionne() {
        let v = dedupe(
            ["a".to_string(), "a".to_string(), String::new(), "b".to_string()].into_iter(),
        );
        assert_eq!(v, vec!["a".to_string(), "b".to_string()]);
    }
}
