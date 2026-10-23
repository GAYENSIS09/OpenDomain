//! `opendomain run` — phase 2 : coordinateur branché sur un
//! exécuteur, exécution en ordre topologique, gates obligatoires, archivage
//! complet du run (append-only) et reprise **re-vérifiée** (récupération
//! amont : une sortie archivée ne rentre que si elle repasse son gate).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use opendomain_core::harness::{Dag, GraphFile};
use opendomain_core::harness::{gate, Executor, RunConfig, Runner, RunSummary, TraceEvent};

use crate::harness::common::resolve_spec;
use crate::harness::executors::LlmExecutor;

/// `opendomain run <spec> [--timeout S] [--resume runs/run-NNN-…]
///                  [--graph <fichier>]` — l'exécuteur est opencode (unique).
///
/// `--graph` : graphe alternatif (ex. révisé par `repatch`) — admis comme
/// un graphe de plan (complétude re-vérifiée), figé dans le dossier de run.
pub fn run(
    spec_input: &str,
    executor_name: &str,
    timeout_secs: u64,
    resume_dir: Option<String>,
    graph_override: Option<String>,
) -> Result<()> {
    let sp = resolve_spec(spec_input)?;
    let spec_text = std::fs::read_to_string(&sp.spec).unwrap_or_default();

    // Config `[gates]` : elle **gouverne** le runner (max_attempts branché
    // au dessous) ; `strict = false` est refusé bruyamment — aucune
    // sémantique consultative inventée, la doctrine est fail-closed.
    let gate_cfg = crate::harness::common::loop_config()?;
    if !gate_cfg.strict {
        return Err(anyhow!(
            "gates.strict=false refusé : verdict consultatif interdit (fail-closed) — \
remettre `strict = true` dans opendomain/opendomain.toml, ou supprimer le réglage"
        ));
    }

    // Garde-fou de fraîcheur : empreinte des manifestes rejouée AVANT tout
    // appel LLM. Dérive du dépôt → `grounding_failure` typé (attribution
    // amont), pas un échec LLM mal attribué trois agents plus loin.
    crate::harness::ground::check_freshness(&sp.slug)?;

    println!(
        "gates : strict, max_attempts = {} (opendomain.toml) — exécuteur : {executor_name}",
        gate_cfg.max_attempts
    );

    // Graphe : en reprise, celui **figé dans le dossier de run** (le graphe
    // courant a pu être re-planifié) ; sinon archive de la spec ou actif.
    // `--graph` : un graphe alternatif (ex. révisé par `repatch`) désigné
    // explicitement — passé à l'ADMISSION comme un graphe de plan, les
    // contrats incomplets ne prennent pas la porte dérobée.
    let (dag, run_dir, resume_map, graph_source) = match resume_dir {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            if !dir.exists() {
                return Err(anyhow!("reprise : dossier inexistant {}", dir.display()));
            }
            let graph_path = match &graph_override {
                Some(g) => PathBuf::from(g),
                None => {
                    if !dir.join("graph.json").exists() {
                        return Err(anyhow!(
                            "reprise : {} ne contient pas graph.json",
                            dir.display()
                        ));
                    }
                    dir.join("graph.json")
                }
            };
            let raw = std::fs::read_to_string(&graph_path)?;
            let file: GraphFile = serde_json::from_str(&raw)?;
            // Graphe avec `--graph` explicite : admission COMPLÈTE au même
            // moteur que le plan — pas de porte dérobée. Graphe figé d'un
            // run : structure seule (déjà admis au moment du run).
            let dag = if graph_override.is_some() {
                opendomain_core::harness::admit(&file, &crate::harness::adapters::capabilities::gaps)?
            } else {
                Dag::build(&file)?
            };
            let verified = re_verify_archive(&dir, &dag)?;
            let source = format!("{graph_path:?}");
            (dag, dir, Some(verified), source)
        }
        None => {
            let graph_path = match &graph_override {
                Some(g) => PathBuf::from(g),
                None => pick_graph(&sp.slug)?,
            };
            let raw = std::fs::read_to_string(&graph_path)?;
            let file: GraphFile = serde_json::from_str(&raw)?;
            // Graphe avec `--graph` explicite : admission COMPLÈTE au même
            // moteur que le plan — pas de porte dérobée. Graphe figé d'un
            // run : structure seule (déjà admis au moment du run).
            let dag = if graph_override.is_some() {
                opendomain_core::harness::admit(&file, &crate::harness::adapters::capabilities::gaps)?
            } else {
                Dag::build(&file)?
            };
            let dir = crate::harness::runs::next_run_dir(&sp.slug)?;
            std::fs::create_dir_all(dir.join("context"))?;
            std::fs::write(dir.join("graph.json"), &raw)?;
            (dag, dir, None, format!("{graph_path:?}"))
        }
    };

    println!("run {run_dir:?} — graphe : {graph_source} — exécuteur : {executor_name}");

    // Read-back AU RUN (REQ-003 : avant toute exécution) — la construction
    // a relu à l'écriture ; entre `build` et `run`, un fichier peut être
    // édité à la main ou périmé. La relecture rejoue les mêmes contrôles
    // sur le fichier réel : deny universel en tête, allows == whitelist
    // projetée, épinglage conforme. Divergence → échec typé, jamais un run
    // sur une matérialisation non prouvée.
    {
        let bindings = crate::harness::common::load_harness_bindings()?;
        let mut relus = 0usize;
        for id in dag.topological_order()? {
            let spec = &dag.nodes[&id];
            let agent_id = crate::harness::adapters::node_agent_id(spec, &sp.slug)?;
            let path = Path::new(".opencode/agent").join(format!("{agent_id}.md"));
            if !path.is_file() {
                return Err(anyhow!(
                    "MaterializationStale : agent `{agent_id}` absent du frontmatter de \
l'éditeur — lancer `opendomain build {}` (la matérialisation précède le run, \
toujours)",
                    sp.slug
                ));
            }
            let actions = crate::harness::adapters::capabilities::projected_actions(&spec.tools)?;
            crate::harness::adapters::verify_materialized(&actions, &bindings, &path)
                .map_err(|e| {
                    anyhow!(
                        "MaterializationStale : {e} — relancer `opendomain build {}` \
(la matérialisation ne suit plus le graphe admis)",
                        sp.slug
                    )
                })?;
            relus += 1;
        }
        println!(
            "read-back du run : {relus} agent(s) re-vérifié(s) contre le graphe admis"
        );
    }

    // Sonde d'amont (leçon du 2026-10-23 : 3 tentatives du nœud `analyste`
    // tuées par la même panne `opencode` — endpoint indisponible) : un
    // one-shot AVANT le run. Panne amont = échec typé immédiat, pas trois
    // tentatives à l'identique.
    if executor_name == "opencode" {
        crate::harness::executors::LlmExecutor::probe_upstream(timeout_secs)?;
    }

    // Exécuteur unique du run : `opencode` — la décomposition réelle passe
    // par lui. Le mock déterministe reste le moteur d'auto-vérification
    // INTERNE (`opendomain bench`, tests) et n'est pas un mode de run ;
    // claude/aider restent derrière le trait, sans surface de run.
    if executor_name != "opencode" {
        return Err(anyhow!(
            "exécuteur unique du run : `opencode` — la voie réelle, visible, archivée. \
Le mock déterministe reste l'auto-vérification interne (`opendomain bench`), \
pas un mode de run."
        ));
    }

    let (summary, store) = {
        let mut ex = LlmExecutor::new(executor_name, timeout_secs)?;
        ex.slug = Some(sp.slug.clone());
        // Réponses BRUTES archivées à côté des sorties validées : ce qui se
        // passe est lisible (fichiers, jamais la console — les accents y
        // sont corrompus, leçon du 2026-10-23).
        ex.raw_dir = Some(run_dir.join("context"));
        // skills du registre → prompts des nœuds ciblés (pillar 6)
        ex.skills = crate::harness::components::attach_for_run(&spec_text, false);
        // kit résolu de chaque nœud (fetch interdit au run : la construction
        // a déjà fourni ; une source distante ici est une faute de construction)
        let kit_résolu = crate::harness::components::resolve_kit(
            &dag,
            &crate::harness::components::registry_or_empty(),
            false,
        )?;
        for nk in &kit_résolu {
            ex.skills.extend(nk.skills.clone());
        }
        if !ex.skills.is_empty() {
            let ids: Vec<&str> = ex.skills.iter().map(|s| s.id.as_str()).collect();
            println!("skills chargées : {}", ids.join(", "));
        }
        let mut boxed: Box<dyn Executor> = Box::new(ex);
        // Commandes de critères : exécutées ICI (sandbox du projet cible),
        // pas par les exécuteurs — mêmes gates pour tous, structurellement.
        let cmd_gate = |cmd: &str| crate::harness::executors::run_gate_command(cmd, timeout_secs);
        let runner = Runner::new(boxed.as_mut())
            .with_config(RunConfig {
                max_attempts: gate_cfg.max_attempts,
                strict: true,
            })
            .commands(&cmd_gate);
        runner.run_all(&dag, resume_map.as_ref().map(|(map, _)| map))
    };

    // ---- archivage (append-only) -------------------------------------------
    // En reprise, chaque artefact est un **fichier nouveau** (`*-resume-NN`)
    // : la preuve du run d'origine n'est jamais écrasée. Le `context/` est
    // l'état de travail — rafraîchi ; l'historique viv dans trace/gates/resume.
    let resume_n = resume_map.as_ref().map(|_| crate::harness::runs::next_resume_index(&run_dir));
    let rs = resume_n
        .map(|n| format!("-resume-{n:02}"))
        .unwrap_or_default();
    if let Some((_, log)) = &resume_map {
        std::fs::write(
            run_dir.join(format!("resume-{:02}.log", resume_n.unwrap())),
            log,
        )?;
    }
    for (id, value) in store.snapshot() {
        std::fs::write(
            run_dir.join("context").join(format!("{id}.json")),
            serde_json::to_string_pretty(value)?,
        )?;
    }
    let trace_lines: Vec<String> = summary
        .events
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect();
    std::fs::write(
        run_dir.join(format!("trace{rs}.jsonl")),
        trace_lines.join("\n") + "\n",
    )?;
    std::fs::write(run_dir.join(format!("gates{rs}.log")), gates_log(&summary))?;

    // Bilan + attribution (M3) + mémoire des leçons (M4).
    let failure_note = match &summary.failed {
        Some(f) => {
            let md = crate::harness::audit::attribution_markdown(&summary.events, Some(f));
            std::fs::write(run_dir.join(format!("attribution{rs}.md")), md)?;
            let run_name = run_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            crate::harness::audit::remember(&sp.slug, &run_name, f)?;
            format!(
                "\n!  échec terminal : {} — {}\n   → {}",
                f.kind.label(),
                f.message,
                f.directive()
            )
        }
        None => String::new(),
    };

    println!(
        "OK run : {} nœud(s) validé(s), dossier {}{}",
        summary.passed.len(),
        run_dir.display(),
        failure_note
    );
    match &summary.failed {
        None => Ok(()),
        Some(_) => Err(anyhow!(
        "run terminé avec échec — reprise ciblée : opendomain run {slug} --resume {dir}",
        slug = sp.slug,
        dir = run_dir.display()
    )),
    }
}

/// Fichier graphe : archive de la spec si présente, sinon graphe actif.
fn pick_graph(slug: &str) -> Result<PathBuf> {
    let archive = Path::new(crate::harness::common::GRAPH_DIR).join(format!("{slug}.json"));
    let active = Path::new(crate::harness::common::GRAPH_DIR).join("graph.json");
    if archive.exists() {
        Ok(archive)
    } else if active.exists() {
        Ok(active.to_path_buf())
    } else {
        Err(anyhow!("aucun graphe — lancer `opendomain plan {slug}`"))
    }
}

/// Reprise **re-vérifiée** (« verified at resumption », le miroir de « verified
/// at construction ») : chaque sortie archivée repasse son gate AVANT
/// réinsertion dans le contexte. Une sortie en défaut n'est pas admise — le
/// nœud ré-exécute au lieu de faire confiance à l'archive. Renvoie les valeurs
/// admises et le journal des verdicts.
fn re_verify_archive(
    run_dir: &Path,
    dag: &Dag,
) -> Result<(BTreeMap<String, serde_json::Value>, String)> {
    let values = load_context_values(run_dir)?;
    let mut verified = BTreeMap::new();
    let mut log = String::from("# reprise — re-vérification des sorties archivées\n");

    let order = dag.topological_order().unwrap_or_default();
    let mut ok = 0usize;
    let mut ko = 0usize;
    for id in &order {
        let Some(spec) = dag.nodes.get(id) else { continue };
        let Some(value) = values.get(id) else {
            log.push_str(&format!("[absent] {id} — pas de sortie archivée → ré-exécution\n"));
            ko += 1;
            continue;
        };
        match gate(spec, value) {
            Ok(()) => {
                log.push_str(&format!("[ok] {id} — admise, ré-indexée (aucune exécution)\n"));
                verified.insert(id.clone(), value.clone());
                ok += 1;
            }
            Err(f) => {
                log.push_str(&format!(
                    "[ko] {id} — {} : {} → ré-exécution\n",
                    f.kind.label(),
                    f.message
                ));
                ko += 1;
            }
        }
    }
    // Sorties archivées hors du graphe figé — signalées, jamais admises.
    for id in values.keys() {
        if !dag.nodes.contains_key(id) {
            log.push_str(&format!("[hors-graphe] {id} — ignorée\n"));
        }
    }
    log.push_str(&format!(
        "\nadmises : {ok} — ré-exécutées : {ko} (attempts: 0 = repris, pas acts de foi)\n"
    ));
    println!("reprise : {ok} re-vérifiée(s) admise(s), {ko} ré-exécutée(s)");
    Ok((verified, log))
}

/// `context/*.json` d'un run existant, **avec leur valeur** — la reprise
/// re-gate les valeurs, pas seulement les ids.
pub fn load_context_values(run_dir: &Path) -> Result<BTreeMap<String, serde_json::Value>> {
    let mut out = BTreeMap::new();
    let dir = run_dir.join("context");
    if let Ok(iter) = std::fs::read_dir(&dir) {
        for e in iter.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".json") {
                let raw = std::fs::read_to_string(dir.join(&name))?;
                match serde_json::from_str(&raw) {
                    Ok(v) => {
                        out.insert(id.to_string(), v);
                    }
                    Err(_) => eprintln!("! contexte illisible : {name} → ré-exécution du nœud"),
                }
            }
        }
    }
    Ok(out)
}

/// Journal lisible des gates — commandes incluses (verdict par commande).
/// `pub(crate)` : le banc (M5) vérifie le round-trip écrivain → lecteur du
/// contrat de provenance de core — le format n'est pas un détail local.
pub(crate) fn gates_log(summary: &RunSummary) -> String {
    let mut out = String::from("# gates y_i ∈ C_i (par ordre d'exécution)\n");
    for e in &summary.events {
        match e {
            TraceEvent::Start { node } => out.push_str(&format!("[ ] start  {node}\n")),
            TraceEvent::Command { node, cmd, ok, message } => {
                let verdict = if *ok { "ok" } else { "ko" };
                let tail = if message.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", message.lines().last().unwrap_or(""))
                };
                out.push_str(&format!("[{verdict}] run-cmd {node} : {cmd}{tail}\n"));
            }
            TraceEvent::Passed { node, attempts } => {
                if *attempts == 0 {
                    out.push_str(&format!(
                        "[ok] gate   {node} (repris — sortie archivée re-vérifiée)\n"
                    ));
                } else {
                    out.push_str(&format!("[ok] gate   {node} (attempts: {attempts})\n"));
                }
            }
            TraceEvent::Retry { node, attempt, kind, message } => {
                out.push_str(&format!(
                    "[ko] gate   {node} (attempt {attempt}) — {} : {message}\n",
                    kind.label()
                ))
            }
            TraceEvent::Failed { failure, directive } => {
                out.push_str(&format!(
                    "[!!] final  {} — {} → {}\n",
                    failure.node,
                    failure.kind.label(),
                    directive
                ))
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendomain_core::harness::{Contract, Failure, FailureKind, RunSummary};
    use serde_json::json;

    fn node_complet(id: &str, deps: &[&str]) -> opendomain_core::harness::AgentSpec {
        opendomain_core::harness::AgentSpec {
            id: id.into(),
            role: "rôle".into(),
            tools: vec!["read".into()],
            contract: Contract {
                output_schema: json!({
                    "type": "object", "required": ["value"],
                    "properties": { "value": {"type": "number"} }
                }),
                ..Default::default()
            },
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    /// La grammaire de `gates.log` est un **contrat cross-crate** : le pont
    /// de provenance de core (`verification::provenance::parse_log`) lit
    /// exactement `[ok|ko]`, `gate <nœud>`, `(repris` et `[!!] final`. Ce
    /// test éping le l'écrivain sur la grammaire du lecteur.
    #[test]
    fn le_journal_gates_porte_la_grammaire_device_par_la_provenance() {
        let summary = RunSummary {
            passed: vec!["a".into()],
            failed: None,
            events: vec![
                TraceEvent::Start { node: "a".into() },
                TraceEvent::Retry {
                    node: "a".into(),
                    attempt: 1,
                    kind: FailureKind::CriteriaFailure,
                    message: "champ requis manquant".into(),
                },
                TraceEvent::Passed { node: "a".into(), attempts: 2 },
                TraceEvent::Passed { node: "b".into(), attempts: 0 },
                TraceEvent::Failed {
                    failure: Failure::new("c", FailureKind::SpecMismatch, "raté"),
                    directive: "reprendre".into(),
                },
            ],
        };
        let log = gates_log(&summary);
        assert!(log.contains("[ ] start  a"), "start : {log}");
        assert!(log.contains("[ko] gate   a (attempt 1)"), "retry : {log}");
        assert!(log.contains("[ok] gate   a (attempts: 2)"), "passed : {log}");
        assert!(log.contains("(repris"), "reprise ré-indexée marquée : {log}");
        assert!(log.contains("[!!] final  c"), "terminal : {log}");
    }

    /// La reprise re-vérifiée : une sortie conforme est ré-indexée, une
    /// sortie en défaut n'est PAS admise, et ce qui sort du graphe figé est
    /// signalé sans jamais entrer dans le contexte.
    #[test]
    fn la_reprise_re_verify_archive_admet_et_refuse() {
        use opendomain_core::harness::GraphFile;
        let dir = std::env::temp_dir().join("opendomain-runflow-reverify");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("context")).unwrap();
        std::fs::write(dir.join("context").join("a.json"), json!({ "value": 1 }).to_string()).unwrap();
        std::fs::write(dir.join("context").join("b.json"), json!({ "value": "faux type" }).to_string()).unwrap();
        std::fs::write(dir.join("context").join("zombie.json"), json!({"x": 1}).to_string()).unwrap();

        let file = GraphFile { meta: json!({}), nodes: vec![node_complet("a", &[]), node_complet("b", &["a"])] };
        let dag = Dag::build(&file).unwrap();

        let (verified, log) = re_verify_archive(&dir, &dag).expect("re-vérification rejouable");
        assert!(verified.contains_key("a"), "conforme admise : {verified:?}");
        assert!(!verified.contains_key("b"), "défaut jamais admise");
        assert!(log.contains("[ok] a — admise, ré-indexée"), "admis : {log}");
        assert!(log.contains("[ko] b —"), "défaut → ré-exécution : {log}");
        assert!(log.contains("[hors-graphe] zombie"), "hors graphe signalé : {log}");
    }
}
