//! M3 — audit des runs : `verify` (gates re-jouables) et `attribute`
//! (classification local/amont/structurel + directive + mémoire des leçons).
//!
//! La vérification d'exécution est rejouable, l'attribution est *typée* et
//! la récupération suit la localité de l'erreur.

use std::path::Path;

use anyhow::{anyhow, Result};
use opendomain_core::harness::{Dag, GraphFile};
use opendomain_core::harness::{gate, TraceEvent};

use crate::harness::executors::run_gate_command;

// ---------------------------------------------------------------- verify ---
/// `opendomain verify <run-dir>` : rejoue `y_i ∈ C_i` sur les sorties archivées
/// (schéma de contrat, aiguilles bornées, chaînes requises/interdites,
/// commandes de critères). La preuve du run n'est pas touchée : le rapport
/// s'écrit dans `verify.log` — `gates.log` reste celui du run ayant réellement
/// exécuté les agents (append-only strict).
pub fn verify(run_input: &str) -> Result<()> {
    let (slug, run_dir) = resolve_run(run_input)?;
    let dag = dag_of_run(&run_dir)?;
    let ctx = run_dir.join("context");
    let mut ko = 0usize;

    let mut log = String::from("# verify — gates rejoués sur les sorties archivées\n");
    for spec in dag.nodes.values() {
        let id = spec.id.as_str();
        let file = ctx.join(format!("{id}.json"));
        if !file.exists() {
            log.push_str(&format!("[absent] {id} — pas de sortie archivée\n"));
            ko += 1;
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
        match gate(spec, &value) {
            Ok(()) => log.push_str(&format!("[ok] {id}\n")),
            Err(f) => {
                log.push_str(&format!(
                    "[ko] {id} — {} : {msg}\n",
                    f.kind.label(),
                    msg = f.message
                ));
                ko += 1;
            }
        }
        // commandes : rejouées chaque fois que `verify` s'applique.
        for cmd in &spec.criteria.commands {
            let cmd_ok = run_gate_command(cmd, 600)
                .map(|_| "[ok]".to_string())
                .unwrap_or_else(|e| format!("[ko] {e}"));
            log.push_str(&format!("[cmd] {id} : {cmd} → {cmd_ok}\n"));
            if cmd_ok.starts_with("[ko") {
                ko += 1;
            }
        }
    }

    let out = run_dir.join("verify.log");
    std::fs::write(&out, log)?;
    println!("verify {slug} : {}", if ko == 0 { "PASS" } else { "FAIL" });
    println!("  → {} (gates.log du run intact)", out.display());
    if ko == 0 {
        Ok(())
    } else {
        Err(anyhow!("{ko} gate(s) en échec"))
    }
}

// ------------------------------------------------------------- attribute ---
/// `opendomain attribute <run-dir>` : classification typée des échecs,
/// directive de récupération, inscription dans la mémoire des leçons.
pub fn attribute(run_input: &str) -> Result<()> {
    let (slug, run_dir) = resolve_run(run_input)?;
    let trace = std::fs::read_to_string(run_dir.join("trace.jsonl"))?;
    let events: Vec<TraceEvent> = trace
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if events.is_empty() {
        return Err(anyhow!("trace.jsonl vide ou illisible dans {run_dir:?}"));
    }

    let failures: Vec<&opendomain_core::harness::Failure> = events
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Failed { failure, .. } => Some(failure),
            _ => None,
        })
        .collect();

    let md = attribution_markdown(&events, failures.first().copied());
    std::fs::write(run_dir.join("attribution.md"), md)?;

    for f in &failures {
        let run_name = run_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        remember(&slug, &run_name, f)?;
    }

    if failures.is_empty() {
        println!("attribute {slug} : aucun échec terminal — rien à attribuer");
        Ok(())
    } else {
        let f = failures[0];
        println!(
            "attribute {slug} : {}— {}\n  → attribution : {}\n  → directive : {}",
            f.kind.label(),
            f.message,
            f.attribution().label(),
            f.directive()
        );
        Ok(())
    }
}

/// Markdown d'attribution (réutilisé par `runflow` à la fin d'un run).
pub fn attribution_markdown(
    events: &[TraceEvent],
    terminal: Option<&opendomain_core::harness::Failure>,
) -> String {
    let mut md = String::from("# Attribution d'erreurs\n\n");
    let retries = events.iter().filter(|e| matches!(e, TraceEvent::Retry { .. })).count();
    md.push_str(&format!(
        "- tentatives locales cumulées : {retries}\n- échec terminal : {}\n",
        match terminal {
            Some(f) => format!("`{}` — {} → {}", f.node, f.kind.label(), f.message),
            None => "aucun".into(),
        }
    ));
    if let Some(f) = terminal {
        md.push_str(&format!(
            "\n## Attribution : {}\n\n**Directive** : {}\n\n",
            f.attribution().label(),
            f.directive()
        ));
        md.push_str(&format!("## Type d'échec : {}\n\n", f.kind.label()));
        md.push_str(&format!("## Message du gate\n`{}`\n", f.message));
        if !f.hints.is_empty() {
            md.push_str("\n## Hints\n");
            for h in &f.hints {
                md.push_str(&format!("- {h}\n"));
            }
        }
    }
    md
}

// ---------------------------------------------------------------- memory ---
/// M4 — la mémoire des leçons : un échec typé y laisse une trace permanente
/// (`opendomain/harness/memory/registre.md`) — « ne payer deux fois les mêmes erreurs ».
pub fn remember(slug: &str, run_name: &str, f: &opendomain_core::harness::Failure) -> Result<()> {
    let dir = Path::new("opendomain/harness/memory");
    std::fs::create_dir_all(dir)?;
    let path = dir.join("registre.md");
    let header = if !path.exists() {
        String::from(
            "# mémoire des leçons — échecs typés\n\nFormat : date | spec | run | nœud | type | message\n",
        )
    } else {
        String::new()
    };
    let line = format!(
        "{} | {slug} | {run_name} | {} | {} | `{}`\n",
        chrono_stamp(),
        f.node,
        f.kind.label(),
        f.message.replace('\n', " ")
    );
    // Dédoublonnage : `run` et `attribute` rapportent tous deux l'échec ;
    // le registre n'inscrit un même (run, nœud, type) qu'une fois — sans
    // dépendre de l'horodatage à la minute près.
    if path.exists() {
        let key = format!("| {run_name} | {} | {} |", f.node, f.kind.label());
        let existing = std::fs::read_to_string(&path)?;
        if existing.lines().any(|l| l.contains(&key)) {
            return Ok(());
        }
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    if !header.is_empty() {
        file.write_all(header.as_bytes())?;
    }
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// Estamp locale `YYYY-MM-DD HH:MM` (sans chrono : std fait l'affaire).
pub(crate) fn chrono_stamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| secs_to_stamp(d.as_secs()))
        .unwrap_or_default()
}

fn secs_to_stamp(secs: u64) -> String {
    // math minimaliste — pas de tz : UTC suffisant pour un registre.
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}")
}

/// Algorithme des jours civils (Howard Hinnant, domaine public).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------------------------------------------------------------- helpers ---
/// Accepte `runs/run-002-spec` (dossier direct) ou le seul nom du run ;
/// renvoie (slug, dir). Le repli cherche le nom sous
/// `opendomain/harness/runs/` — l'endroit unique où les runs sont archivés.
pub(crate) fn resolve_run(run_input: &str) -> Result<(String, std::path::PathBuf)> {
    let clean = run_input.trim().trim_matches('"');
    let direct = Path::new(clean);
    if direct.is_dir() && direct.join("trace.jsonl").exists() {
        return run_dir_to_slug(direct);
    }
    let basename = direct
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| clean.to_string());
    let archive = Path::new("opendomain/harness/runs").join(&basename);
    if archive.is_dir() && archive.join("trace.jsonl").exists() {
        return run_dir_to_slug(&archive);
    }
    Err(anyhow!(
        "run introuvable : {clean} — chercher un dossier `opendomain/harness/runs/run-NNN-<slug>` (trace.jsonl présent)"
    ))
}

fn run_dir_to_slug(dir: &Path) -> Result<(String, std::path::PathBuf)> {
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

/// Graphe figé dans le dossier de run (copie conservée lors du run).
fn dag_of_run(run_dir: &Path) -> Result<Dag> {
    let graph = run_dir.join("graph.json");
    let raw = std::fs::read_to_string(&graph).map_err(|e| anyhow!("{graph:?} illisible : {e}"))?;
    let file: GraphFile = serde_json::from_str(&raw)?;
    Dag::build(&file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estamp_et_jours_civils() {
        // 2026-01-01 00:00 UTC = 1767225600
        assert!(secs_to_stamp(1_767_225_600).starts_with("2026-01-01"));
        assert!(secs_to_stamp(0).starts_with("1970-01-01"));
    }
}
