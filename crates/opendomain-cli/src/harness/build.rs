//! `opendomain build` — code generation + construction-time
//! verification, partie statique.
//!
//! Génère le **kit d'exécution complet** de chaque agent dans
//! `opendomain/harness/agents/<slug>/` — pas seulement un prompt :
//!
//! - `AGENTS.md` : le contrat d'exécution partagé (les six piliers d'un
//!   runtime d'agent : boucle, outils, contexte/mémoire, sandbox, hooks,
//!   skills) ;
//! - `<id>.md` : la σ_i (rôle, contrat, critères, outils, place dans la
//!   boucle) ;
//! - `<id>.kit.json` : le manifeste machine du même équipement ;
//! - `report.md` : la vérification de construction (PASS/FAIL par agent).
//!
//! Chaque artefact est jugé avant d'être admissible ; échec → code de
//! sortie non nul.

use std::path::Path;

use anyhow::{Result, anyhow};
use opendomain_core::harness::{AgentSpec, Dag, GraphFile};

/// `opendomain build <spec>`.
pub fn run(spec_input: &str) -> Result<()> {
    let sp = crate::harness::common::resolve_spec(spec_input)?;

    // graphe de la spec (archive) sinon graphe actif
    let graph_path = {
        let archive = Path::new("opendomain/harness/graph").join(format!("{}.json", sp.slug));
        let active = Path::new("opendomain/harness/graph/graph.json");
        if archive.exists() { archive } else if active.exists() { active.to_path_buf() } else {
            return Err(anyhow!(
                "aucun graphe pour `{}` — lancer d'abord `opendomain plan {}`",
                sp.slug,
                sp.slug
            ));
        }
    };
    let raw = std::fs::read_to_string(&graph_path)?;
    let file: GraphFile = serde_json::from_str(&raw).map_err(|e| anyhow!("{graph_path:?} : {e}"))?;
    let dag = Dag::build(&file)?;

    // le kit d'exécution (6 piliers) est dérivé du graphe réel + de la config.
    let out_dir = crate::harness::adapters::agents_dir(&sp.slug);
    std::fs::create_dir_all(&out_dir)?;
    let lang = std::fs::read_to_string(&sp.spec)
        .map(|t| crate::harness::common::detect_language(&t).to_string())
        .unwrap_or_else(|_| "cible".to_string());
    let loop_cfg = loop_config();
    std::fs::write(out_dir.join("AGENTS.md"), render_agents_md(&dag, &lang, &loop_cfg))?;
    println!("  + {}", out_dir.join("AGENTS.md").display());
    for spec in dag.nodes.values() {
        let path = out_dir.join(format!("{}.md", spec.id));
        std::fs::write(&path, render_agent(spec))?;
        println!("  + {path:?}");
        let kit_path = out_dir.join(format!("{}.kit.json", spec.id));
        std::fs::write(
            &kit_path,
            serde_json::to_string_pretty(&kit_manifest(spec, &loop_cfg))?,
        )?;
        println!("  + {kit_path:?}");
    }

    // La construction DÉFINIT les agents de nœud (phase 1 de l'architecte) :
    // chaque nœud → un fichier dans `.opencode/agent/` (permissions projetées
    // + contrat de sortie), visible et reviewable AVANT le run — l'exécution
    // invoquera ces ids, elle ne réécrira pas.
    let defined = crate::harness::adapters::define_node_agents(&dag, &sp.slug)?;
    for id in &defined {
        println!("  agent défini : {id}");
    }

    // vérification de construction
    let (report, ok) = verify(&dag);
    let report_file = out_dir.join("report.md");
    std::fs::write(&report_file, &report)?;
    println!("  + {}", report_file.display());

    if ok {
        println!(
            "OK construction vérifiée : {} agent(s) admissibles — kit complet dans {}",
            dag.nodes.len(),
            out_dir.display()
        );
        println!("  suite : opendomain run {}", sp.slug);
        Ok(())
    } else {
        Err(anyhow!(
            "vérification de construction ÉCHOUÉE — voir {report_file:?} ;\n  → corriger graph.json (opendomain plan {}) et relancer build",
            sp.slug
        ))
    }
}

/// La σ_i sous forme d'artefact lisible et réutilisable tel quel comme prompt.
fn render_agent(spec: &AgentSpec) -> String {
    let deps = if spec.depends_on.is_empty() {
        "-".to_string()
    } else {
        spec.depends_on.join(", ")
    };
    let tools = if spec.tools.is_empty() {
        "(aucun)".to_string()
    } else {
        spec.tools.join(", ")
    };
    let schema_in = pretty_null(&spec.contract.input_schema);
    let schema_out = pretty_null(&spec.contract.output_schema);
    let contains_in = if spec.criteria.contains_in.is_empty() {
        "-".to_string()
    } else {
        let mut out = String::new();
        for (field, needles) in &spec.criteria.contains_in {
            out.push_str(&format!("- champ `{field}` contient : {}\n", needles.join(", ")));
        }
        out
    };
    let header = format!(
        "# σ_i — {}\n\nAgent généré par OpenDomain. Artefact validé\nà la construction — ne pas éditer à la main ; reconstituer via `opendomain plan`.\n",
        spec.id
    );
    let loop_section = "## Ta place dans la boucle (Agent Loop)\n\
Tu tournes **sous le harnais** : appelé par le coordinateur avec le contexte amont,\ntu réponds par l'objet JSON du contrat de sortie, **rien d'autre**. Le runner\nexécute tes commandes de critères et vérifie ta sortie ; en cas de refus tu\nreçois un feedback typé et tu corriges (≤ max tentatives). Au-delà, l'échec est\nattribué (local / amont / structurel) — ta sortie n'est jamais propagée sans\nvérification.\n";
    let memory_section = format!(
        "## Contexte & mémoire\n- entrées : les sorties validées (`y_j`) de tes dépendances — {deps} —\n  injectées par le harnais (bornées, ne réclame jamais plus) ;\n- leçons du passé : `opendomain/harness/memory/registre.md` — à consulter avant\nd'improviser.\n"
    );

    format!(
        "{header}\n\
## Rôle (directive système)\n{role}\n\n\
{loop_section}\n\
## Outils autorisés (moindre privilège)\n{tools}\n\n\
## Contrat d'entrée\n```json\n{schema_in}\n```\n\n\
## Contrat de sortie (gate : schema_check = {schema_check})\n```json\n{schema_out}\n```\n\n\
## Critères C_i\n- contient obligatoire : {contains}\n{contains_in}- interdits : {not_contains}\n- commandes de gate (sandbox) : {commands}\n\n\
## Dépend amont de\n{deps}\n\n\
{memory_section}\n\
## Rappel de gate\nUne sortie qui ne satisfait pas son contrat/critères n'est\nPAS propagée : retries avec feedback typé, sinon attribution (local / amont /\nstructurel). Contrat d'exécution complet : voir `AGENTS.md` du kit.\n",
        role = spec.role.trim(),
        schema_check = spec.criteria.schema_check,
        contains = join_or_dash(&spec.criteria.contains),
        contains_in = contains_in,
        not_contains = join_or_dash(&spec.criteria.not_contains),
        commands = join_or_dash(&spec.criteria.commands),
    )
}

/// Vérification statique de construction : la liste des verdicts + santé,
/// plus les *avertissements* — les critères auto-satisfaisables (l'aiguille
/// figure déjà dans le rôle que l'agent va lire) ne bloquent pas la
/// construction mais lèvent le drapeau : les appuyer d'une commande ou d'un
/// nœud vérificateur avant le run.
fn verify(dag: &Dag) -> (String, bool) {
    let mut lines = String::from("# Rapport de construction\n\n");
    let mut ok_all = true;

    for id in dag.topological_order().unwrap_or_default() {
        let spec = &dag.nodes[&id];
        let checks = crate::harness::common::agent_completeness(spec);
        let node_ok = checks.iter().all(|(_, ok, _)| *ok);
        ok_all &= node_ok;
        lines.push_str(&format!(
            "## {id} — {}\n",
            if node_ok { "PASS" } else { "FAIL" }
        ));
        for (label, ok, field) in &checks {
            lines.push_str(&format!(
                "- [{}] {label} `{}` : {}\n",
                if *ok { "x" } else { " " },
                field,
                if *ok { "ok" } else { "MANQUANT" }
            ));
        }
        // Critères auto-satisfaisables : l'aiguille demandée est déjà dans
        // la directive système du nœud — l'agent la satisfait en répétant
        // ce qu'il a lu. Avertissement, pas refus.
        for needle in crate::harness::common::criteria_needles(spec) {
            if !needle.trim().is_empty() && spec.role.contains(needle.as_str()) {
                lines.push_str(&format!(
                    "- ⚠ critère auto-satisfaisable : l'aiguille `{needle}` figure dans le \
rôle `{id}` — l'appuyer d'une commande (`criteria.commands`) ou d'un nœud \
verificateur, sinon le gate mesure l'obéissance au prompt, pas le travail\n"
                ));
            }
        }
        lines.push('\n');
    }

    (lines, ok_all)
}

fn join_or_dash(items: &[String]) -> String {
    if items.is_empty() {
        "-".into()
    } else {
        items.join(", ")
    }
}

// -------------------------------------------------------------- kit complet –

/// `[gates]` de `opendomain.toml` : strict + max_attempts — la config
/// **générale** du workspace, il n'y a pas de second fichier.
pub struct LoopCfg {
    pub strict: bool,
    pub max_attempts: u32,
}

impl Default for LoopCfg {
    fn default() -> Self {
        Self { strict: true, max_attempts: 3 }
    }
}

/// Lit la config `[gates]` du projet — la CLI la branchera dans le runner
/// (`[gates]` gouverne, il ne décore pas). La config ne parse pas : l'erreur
/// se propage ; un harnais ne devine pas ses réglages.
pub(crate) fn loop_config() -> LoopCfg {
    let cfg = opendomain_core::config::Config::load(std::path::Path::new("opendomain"))
        .unwrap_or_default();
    LoopCfg { strict: cfg.gates.strict, max_attempts: cfg.gates.max_attempts }
}

/// `AGENTS.md` : le contrat d'exécution **partagé** des agents de la spec —
/// les 6 piliers d'un runtime d'agent, rendus depuis le graphe réel.
fn render_agents_md(dag: &Dag, lang: &str, cfg: &LoopCfg) -> String {
    let mut agents_tbl = String::new();
    for id in dag.topological_order().unwrap_or_default() {
        let s = &dag.nodes[&id];
        agents_tbl.push_str(&format!(
            "| `{id}` | {} | {} | {} |\n",
            short_role(&s.role),
            s.tools.join(", "),
            brief_gate(s)
        ));
    }
    let mut md = String::new();
    md.push_str("# AGENTS.md — contrat d'exécution des agents de ce harnais\n\n");
    md.push_str("Généré par le méta-agent OpenDomain. Chaque agent de ce\n");
    md.push_str("dossier reçoit les six capacités d'un runtime d'agent complet — pas un\n");
    md.push_str("prompt vague : un équipement.\n\n");
    md.push_str("## 1. La boucle d'agent (Agent Loop)\n");
    md.push_str("Le programme central est le coordinateur de `opendomain-harness` (commande\n");
    md.push_str("`opendomain run`) : il appelle chaque agent en ordre topologique, analyse\n");
    md.push_str("sa réponse, applique les gates, puis rejoue les tentatives ciblées.\n");
    md.push_str(&format!(
        "Réglage : {} tentatives locales, propagation stricte : {} (voir\n`opendomain/harness/config.opendomain.toml` `[gates]`).\n\n",
        cfg.max_attempts,
        if cfg.strict { "oui" } else { "non" }
    ));
    md.push_str("## 2. Outils (Tools) — moindre privilège\n");
    md.push_str("Chaque agent reçoit sa whitelist dans sa σ_i et dans son prompt ; utiliser\n");
    md.push_str("un outil hors liste est une violation. Les commandes de critères partagent\n");
    md.push_str("la même sandbox.\n\n");
    md.push_str("## 3. Contexte & mémoire\n");
    md.push_str("- **contexte** : les sorties validées des dépendances amont, injectées en\n");
    md.push_str("  JSON borné (~6000 caractères) ; le grounding du projet vit dans\n");
    md.push_str("  `opendomain/harness/grounding/`.\n");
    md.push_str("- **mémoire** : `opendomain/harness/memory/registre.md` consigne les échecs typés du\n");
    md.push_str("  passé ; à consulter avant tout travail sensible.\n\n");
    md.push_str("## 4. Sandbox (isolation)\n");
    md.push_str("Les commandes de critères tournent dans la sandbox logique de\n");
    md.push_str("`opendomain/harness/config.opendomain.toml` (`[sandbox]`) : allowlist de binaires et\n");
    md.push_str("dossier courant borné au dépôt. Un binaire hors liste produit une erreur\n");
    md.push_str("typée `SandboxViolation` — jamais de surprise.\n\n");
    md.push_str("## 5. Hooks & contrôles (gardes-fous)\n");
    md.push_str("- un gate automatique à chaque sortie (`y_i ∈ C_i`) : schéma, motifs,\n");
    md.push_str("  commandes de tests/lint ;\n");
    md.push_str("- une revue croisée : l'agent `verificateur` relit le travail amont avant\n");
    md.push_str("  propagation ;\n");
    md.push_str("- `opendomain verify <run>` rejoue tous les gates sur un run archivé.\n\n");
    md.push_str("## 6. Skills & instructions\n");
    md.push_str("Les procédures réutilisables vivent dans `opendomain/harness/specs/` (gabarits métier\n");
    md.push_str("`migration`, `refactor`, `tests`, `doc` via `opendomain workflow new`) et\n");
    md.push_str(&format!(
        "dans ce fichier. Langue du dépôt détectée : **{lang}** — les sorties\nrédigées (docs, messages) s'y tiennent.\n\n"
    ));
    md.push_str("## Annuaire des agents\n");
    md.push_str("| id | rôle court | outils | gate |\n|---|---|---|---|\n");
    md.push_str(&agents_tbl);
    md.push_str("\n---\n");
    md.push_str("Refaire ce kit : `opendomain build <slug>` — l'artisan est le méta-agent,\n");
    md.push_str("pas une main humaine qui écrirait des prompts de travers.\n");
    md
}

/// Manifeste machine du kit par agent (pour les exécuteurs et adaptateurs).
fn kit_manifest(spec: &AgentSpec, cfg: &LoopCfg) -> serde_json::Value {
    serde_json::json!({
        "agent": spec.id,
        "standard": "opendomain-kit",
        "loop": {
            "runner": "opendomain_core::harness::Runner",
            "max_attempts": cfg.max_attempts,
            "strict_gates": cfg.strict,
            "gate": "y_i ∈ C_i avant propagation",
        },
        "tools": spec.tools,
        "hooks": {
            "schema_check": spec.criteria.schema_check,
            "contains": spec.criteria.contains,
            "contains_in": spec.criteria.contains_in,
            "not_contains": spec.criteria.not_contains,
            "forbidden_patterns": spec.contract.forbidden,
            "run_gate_commands": spec.criteria.commands,
        },
        "context": {
            "deps": spec.depends_on,
            "compaction": "borné à ~6000 caractères par le harnais",
        },
        "memory": { "lessons": "opendomain/harness/memory/registre.md", "runs": "opendomain/harness/runs/" },
        "sandbox": { "config": "opendomain/harness/config.opendomain.toml [sandbox]" },
    })
}

fn short_role(role: &str) -> String {
    let first = role.lines().next().unwrap_or("").trim();
    first.chars().take(60).collect()
}

fn brief_gate(spec: &AgentSpec) -> String {
    if spec.criteria.commands.is_empty() {
        format!("schema={} + motifs", spec.criteria.schema_check)
    } else {
        format!("schema={} + cmds", spec.criteria.schema_check)
    }
}

fn pretty_null(v: &serde_json::Value) -> String {
    if v.is_null() {
        "(non contraint)".into()
    } else {
        serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_des_gates_lit_la_config_generale() {
        // La section vit dans opendomain.toml ; les défauts sont ceux du
        // papier (strict, 3 tentatives) et une valeur explicite gouverne.
        let cfg = opendomain_core::config::Config::parse(
            "[gates]\nstrict = false\nmax_attempts = 5\n",
        )
        .unwrap();
        assert!(!cfg.gates.strict);
        assert_eq!(cfg.gates.max_attempts, 5);
        // Défauts sur section absente.
        let d = opendomain_core::config::Config::parse("").unwrap();
        assert!(d.gates.strict);
        assert_eq!(d.gates.max_attempts, 3);
    }

    #[test]
    fn agents_md_sans_mots_colles() {
        let file = GraphFile {
            meta: json!(null),
            nodes: vec![AgentSpec {
                id: "a".into(),
                role: "rôle de test".into(),
                tools: vec!["read".into()],
                ..Default::default()
            }],
        };
        let dag = Dag::build(&file).unwrap();
        let md = render_agents_md(&dag, "rust", &LoopCfg::default());
        assert!(md.contains("dossier reçoit"));
        assert!(md.contains("sa σ_i et dans son prompt"));
        assert!(md.contains("## 6. Skills & instructions"));
        assert!(!md.contains("decette"), "pas de mots collés");
        for bad in ["decette", "promptvagabond", "sa s_i", "lesagents"] {
            assert!(!md.contains(bad), "mot collé détecté : {bad}");
        }
    }

    #[test]
    fn manifeste_kit_complet() {
        let spec = AgentSpec {
            id: "x".into(),
            role: "r".into(),
            tools: vec!["bash".into()],
            contract: Default::default(),
            criteria: Default::default(),
            depends_on: vec!["y".into()],
            prompt: Default::default(),
        };
        let kit = kit_manifest(&spec, &LoopCfg::default());
        assert_eq!(kit["agent"], "x");
        assert_eq!(kit["loop"]["max_attempts"], 3);
        assert_eq!(kit["tools"], serde_json::json!(["bash"]));
        assert_eq!(kit["context"]["deps"], serde_json::json!(["y"]));
        assert_eq!(kit["memory"]["lessons"], "opendomain/harness/memory/registre.md");
    }
}
