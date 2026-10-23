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

/// `opendomain run <spec> [--executor opencode|claude|mock] [--timeout S]
///                  [--resume runs/run-NNN-…] [--graph <fichier>]
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
    let gate_cfg = crate::harness::build::loop_config();
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
            let dag = Dag::build(&file)?;
            if graph_override.is_some() {
                crate::harness::plan::admission_complete(&dag)?;
            }
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
            let dag = Dag::build(&file)?;
            if graph_override.is_some() {
                crate::harness::plan::admission_complete(&dag)?;
            }
            let dir = next_run_dir(&sp.slug)?;
            std::fs::create_dir_all(dir.join("context"))?;
            std::fs::write(dir.join("graph.json"), &raw)?;
            (dag, dir, None, format!("{graph_path:?}"))
        }
    };

    println!("run {run_dir:?} — graphe : {graph_source} — exécuteur : {executor_name}");

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
    let resume_n = resume_map.as_ref().map(|_| next_resume_index(&run_dir));
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
    let archive = Path::new("opendomain/harness/graph").join(format!("{slug}.json"));
    let active = Path::new("opendomain/harness/graph/graph.json");
    if archive.exists() {
        Ok(archive)
    } else if active.exists() {
        Ok(active.to_path_buf())
    } else {
        Err(anyhow!("aucun graphe — lancer `opendomain plan {slug}`"))
    }
}

/// Dossier du prochain run : `runs/run-NNN-<slug>` (NNN = max + 1).
fn next_run_dir(slug: &str) -> Result<PathBuf> {
    let base = Path::new("opendomain/harness/runs");
    std::fs::create_dir_all(base)?;
    let mut max = 0u32;
    if let Ok(iter) = std::fs::read_dir(base) {
        for e in iter.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let eungs = name.strip_prefix("run-").map(|r| r.split('-').next().unwrap_or(""));
            if let Some(n) = eungs.and_then(|s| s.parse::<u32>().ok()) {
                max = max.max(n);
            }
        }
    }
    Ok(base.join(format!("run-{:03}-{slug}", max + 1)))
}

/// Numéro du prochain artefact de reprise (`resume-NN.*`) dans le dossier.
fn next_resume_index(run_dir: &Path) -> u32 {
    let mut max = 0u32;
    if let Ok(iter) = std::fs::read_dir(run_dir) {
        for e in iter.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(rest) = name.strip_prefix("resume-") {
                let num = rest.split_once('.').map(|(a, _)| a).unwrap_or(rest);
                if let Ok(n) = num.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    max + 1
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
fn gates_log(summary: &RunSummary) -> String {
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
