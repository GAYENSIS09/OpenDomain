//! `runs/` — l'archive append-only des runs et son vocabulaire de noms.
//!
//! Une seule implémentation pour tout ce qui **trouve, nomme et fusionne**
//! les traces et dossiers de run — consommée par `run`, `verify`,
//! `attribute`, `repatch` et le pont de provenance de core. Avant ce
//! module, chaque commande re-parsait les noms à sa façon : `run-NNN` à
//! trois endroits, `trace*.jsonl` à deux, avec couverture inégale
//! (`attribute` ne voyait pas les traces de reprise).

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use opendomain_core::harness::{Failure, TraceEvent};

// ------------------------------------------------------------ noms de run ---

/// Numéro de run dans un nom `run-NNN-…` (`None` si non-conforme).
pub fn parse_run_number(name: &str) -> Option<u32> {
    name.strip_prefix("run-")
        .and_then(|r| r.split('-').next())
        .and_then(|n| n.parse::<u32>().ok())
}

/// Le nom canonique d'un dossier de run : `run-{:03}-<slug>`.
pub fn run_dir_name(n: u32, slug: &str) -> String {
    format!("run-{:03}-{slug}", n)
}

/// Dossier du prochain run : `runs/run-NNN-<slug>` (NNN = max + 1).
pub fn next_run_dir(slug: &str) -> Result<PathBuf> {
    let base = Path::new(crate::harness::common::RUNS_DIR);
    std::fs::create_dir_all(base)?;
    let mut max = 0u32;
    if let Ok(iter) = std::fs::read_dir(base) {
        for e in iter.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(n) = parse_run_number(&name) {
                max = max.max(n);
            }
        }
    }
    Ok(base.join(run_dir_name(max + 1, slug)))
}

/// Numéro du prochain artefact de reprise (`resume-NN.*`) dans le dossier.
pub fn next_resume_index(run_dir: &Path) -> u32 {
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

/// Résout un run : dossier direct (avec `trace.jsonl`) ou nom nu/sous-chaîne
/// résolu sous `runs/`. Renvoie `(slug, dossier)`.
pub fn resolve_run(run_input: &str) -> Result<(String, PathBuf)> {
    let clean = run_input.trim().trim_matches('"');
    let direct = Path::new(clean);
    if direct.is_dir() && direct.join("trace.jsonl").exists() {
        return run_dir_to_slug(direct);
    }
    let basename = direct
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| clean.to_string());
    let archive = Path::new(crate::harness::common::RUNS_DIR).join(&basename);
    if archive.is_dir() && archive.join("trace.jsonl").exists() {
        return run_dir_to_slug(&archive);
    }
    bail!(
        "run introuvable : {clean} — chercher un dossier `opendomain/harness/runs/run-NNN-<slug>` (trace.jsonl présent)"
    )
}

/// Slug depuis un nom de dossier `run-NNN-<slug>` — non-conforme → `spec`
/// (fallback historique de `audit::run_dir_to_slug`, conservé).
fn run_dir_to_slug(dir: &Path) -> Result<(String, PathBuf)> {
    let name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let slug = name
        .strip_prefix("run-")
        .and_then(|r| r.split_once('-').map(|(_, s)| s.to_string()))
        .unwrap_or_else(|| "spec".into());
    Ok((slug, dir.to_path_buf()))
}

// ------------------------------------------------------------------ traces ---

/// Fichiers de trace ordonnés du plus ancien au plus récent :
/// `trace.jsonl` (0), `trace-resume-01.jsonl` (1), …
pub fn trace_files(run_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<(usize, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(run_dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let idx = if name == "trace.jsonl" {
                Some(0)
            } else if let Some(rest) = name.strip_prefix("trace-resume-") {
                rest.trim_end_matches(".jsonl").parse::<usize>().ok()
            } else {
                None
            };
            if let Some(n) = idx {
                files.push((n, run_dir.join(&name)));
            }
        }
    }
    files.sort_by_key(|(n, _)| *n);
    if files.is_empty() {
        bail!("aucune trace dans {}", run_dir.display());
    }
    Ok(files.into_iter().map(|(_, p)| p).collect())
}

/// Tous les événements de trace du run, segments de reprise compris, dans
/// l'ordre chronologique — une lecture, plusieurs consommateurs.
pub fn trace_events(run_dir: &Path) -> Result<Vec<TraceEvent>> {
    let files = trace_files(run_dir)?;
    let mut events: Vec<TraceEvent> = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let ev: TraceEvent = serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("trace illisible ({}) : {e}", file.display()))?;
            events.push(ev);
        }
    }
    Ok(events)
}

/// Le dernier échec terminal du run, évidence comprise, « healed » inclus.
pub struct TerminalFailure {
    /// L'échec terminal (dernier `Failed` tout segment confondu).
    pub failure: Failure,
    /// L'évidence du refus — tous les `Retry` du nœud, chronologie complète
    /// (le terminal inclus).
    pub evidence: Vec<String>,
    /// Le nœud a-t-il cicatrisé (un `Passed(attempts ≥ 1)` plus tard —
    /// tentative ≥ 1 ; un `Passed(0)` n'est qu'une reprise ré-indexée).
    pub healed: bool,
}

/// Fusion chronologique de TOUS les segments de trace — le dernier échec
/// terminal, son évidence, et son état de cicatrisation.
pub fn gather_terminal(run_dir: &Path) -> Result<TerminalFailure> {
    let events = trace_events(run_dir)?;

    let Some(idx) = events
        .iter()
        .rposition(|e| matches!(e, TraceEvent::Failed { .. }))
    else {
        bail!(
            "aucun échec terminal dans les traces de {} — rien à re-planifier \
(si l'échec antérieur est déjà réglé, lancer un run neuf)",
            run_dir.display()
        );
    };
    let failure = match &events[idx] {
        TraceEvent::Failed { failure, .. } => failure.clone(),
        _ => unreachable!(),
    };
    let healed = events[idx + 1..].iter().any(|e| {
        matches!(
            e,
            TraceEvent::Passed { node, attempts } if node == &failure.node && *attempts >= 1
        )
    });
    let evidence: Vec<String> = {
        let mut ev: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                TraceEvent::Retry { node, message, .. } if node == &failure.node => {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect();
        // L'évidence inclut le terminal — chronologie complète des refus.
        ev.push(failure.message.clone());
        ev
    };

    Ok(TerminalFailure {
        failure,
        evidence,
        healed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier temporaire façon run.
    fn fake_run(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opendomain-runs-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn les_noms_de_run_se_parsent_et_se_forment_en_aller_retour() {
        assert_eq!(parse_run_number("run-007-migrer"), Some(7));
        assert_eq!(parse_run_number("run-42-a-b"), Some(42));
        assert_eq!(parse_run_number("not-a-run"), None);
        assert_eq!(parse_run_number("run-x-a"), None);
        assert_eq!(run_dir_name(7, "migrer"), "run-007-migrer");
        assert_eq!(parse_run_number(&run_dir_name(123, "s")), Some(123));
    }

    #[test]
    fn les_traces_sont_alors_et_incluent_les_reprises() {
        let dir = fake_run("traces");
        std::fs::write(dir.join("trace.jsonl"), "1\n2\n").unwrap();
        std::fs::write(dir.join("trace-resume-02.jsonl"), "5\n").unwrap();
        std::fs::write(dir.join("trace-resume-01.jsonl"), "3\n4\n").unwrap();
        std::fs::write(dir.join("gates.jsonl"), "borde\n").unwrap();
        let files = trace_files(&dir).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "trace.jsonl",
                "trace-resume-01.jsonl",
                "trace-resume-02.jsonl"
            ],
            "ordre chronologique, gâchis non-trace ignoré"
        );
    }

    #[test]
    fn le_dernier_echec_termine_voyage_declare_tous_les_segments() {
        use opendomain_core::harness::{Failure, FailureKind};
        let dir = fake_run("terminal");
        let f1 = Failure::new("verificateur", FailureKind::CriteriaFailure, "raison 1");
        let f2 = Failure::new("synthese", FailureKind::SpecMismatch, "raison 2");
        std::fs::write(
            dir.join("trace.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&TraceEvent::Failed {
                    failure: f1,
                    directive: "d1".into()
                })
                .unwrap(),
                serde_json::to_string(&TraceEvent::Retry {
                    node: "synthese".into(),
                    attempt: 1,
                    kind: FailureKind::SpecMismatch,
                    message: "e1".into()
                })
                .unwrap()
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("trace-resume-01.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&TraceEvent::Failed {
                    failure: f2,
                    directive: "d2".into()
                })
                .unwrap(),
                serde_json::to_string(&TraceEvent::Passed {
                    node: "synthese".into(),
                    attempts: 2
                })
                .unwrap()
            ),
        )
        .unwrap();

        // 1. le terminal vient du DEUXIÈME segment (fusion chronologique) ;
        let t = gather_terminal(&dir).unwrap();
        assert_eq!(t.failure.node, "synthese");
        assert_eq!(t.failure.message, "raison 2");
        assert!(t.healed, "un Passed(2 ≥ 1) suit le terminal — cicatrisé");
        assert_eq!(t.evidence, vec!["e1".to_string(), "raison 2".to_string()]);

        // 2. tous les `Failed` des deux segments sont visibles — l'attribut
        //    tardif ne perd plus la reprise.
        let events = trace_events(&dir).unwrap();
        let failed: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                TraceEvent::Failed { failure, .. } => Some(failure.node.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            failed,
            vec!["verificateur".to_string(), "synthese".to_string()]
        );
    }
}
