//! `opendomain plan` — la spec `T` devient un DAG d'agents `σ_i`.
//!
//! Deux voies :
//! 1. **via opencode** (défaut, la voie réelle) : `opencode run` propose le
//!    graphe ; le harnais le VÉRIFIE et, en cas de défaut typé, relance
//!    l'étape amont (boucle de correction). La panne amont sort typée —
//!    jamais un repli silencieux vers le déterministe.
//! 2. **`--offline`** (demandé explicitement) : décomposition canonique
//!    déterministe `analyste → strategie → synthese → verificateur`,
//!    spécialisée par la spec (langue, contraintes, interdits nommés) —
//!    CI, pas de réseau.
//!
//! Dans les deux cas, la sortie est validée par `opendomain-harness` avant écriture
//! (archive `graph/<slug>.json` + graphe actif `graph/graph.json`).

use anyhow::{anyhow, Result};
use opendomain_core::harness::{AgentSpec, Contract, Criteria, GraphFile};
use serde_json::json;

use crate::harness::common::{detect_language, extract_forbidden, resolve_spec, section_lines, title_of};

/// `opendomain plan <spec> [--offline] [--timeout S]`.
///
/// La voie par défaut est la décomposition **réelle** : opencode propose le
/// graphe, le harnais le vérifie (Generate → Verify → Refine) — la panne
/// amont sort typée, jamais un repli silencieux vers le déterministe.
/// `--offline` : repli explicite (CI, pas de réseau) — le graphe
/// déterministe reste disponible, mais comme un choix nommé, pas un
/// camouflet automatique.
pub fn run(spec_input: &str, offline: bool, timeout_secs: u64) -> Result<()> {
    let sp = resolve_spec(spec_input)?;
    let text = std::fs::read_to_string(&sp.spec)?;

    let file = if offline {
        println!("plan : mode hors-ligne explicite — graphe déterministe (repli nommé, pas la voie réelle)");
        build_heuristic_graph(&sp.slug, &text)
    } else {
        // Sonde d'amont AVANT le plan (leçon du 2026-10-23 / REQ-002) : le
        // message d'erreur recommandait la sonde one-shot — le plan l'exécute
        // désormais lui-même. Échec typé amont immédiat : aucune décomposition
        // tentée sur un endpoint mort.
        crate::harness::executors::LlmExecutor::probe_upstream(timeout_secs)?;
        crate::harness::adapters::plan_with_llm(&sp.slug, &text, detect_language(&text), timeout_secs)
            .map_err(|e| {
                anyhow!(
                    "plan via opencode : {e} — \
amont indisponible ? (sonde : `opencode run \"Réponds OK.\"`) ; \
le repli déterministe existe et se demande explicitement : `opendomain plan {slug} --offline`",
                    slug = sp.slug
                )
            })?
    };

    // Vérification de construction (statique) — on n'écrit jamais un graphe
    // invalide ou incomplet : l'admission du plan exige la complétude de
    // chaque agent (le même contrôle que `build` — la Garantie 3 appartient
    // au plan, pas à l'étape suivante). Un seul moteur, dans core : la
    // structure **et** la complétude y sont jugées en un appel.
    let dag = opendomain_core::harness::admit(&file, &crate::harness::adapters::capabilities::gaps)?;
    let order = dag.topological_order()?;

    let graph_dir = std::path::Path::new(crate::harness::common::GRAPH_DIR);
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

/// Décomposition canonique, spécialisée par la spec. Déterministe : renvoie
/// toujours un graphe que le moteur d'admission accepte.
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
            "generator": "opendomain plan (déterministe --offline)",
            "langue": lang,
            "source": "plan via opencode (défaut), ou --offline demandé explicitement"
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
    use opendomain_core::harness::Dag;

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
        let err = format!(
            "{:#}",
            opendomain_core::harness::admit(&bad, &opendomain_core::harness::no_caps).expect_err("admission refuse l'incomplet")
        );
        let text = err;
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
