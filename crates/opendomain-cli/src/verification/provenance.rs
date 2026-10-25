//! `opendomain provenance` — le pont F14 : chaque enregistrement qui cite un
//! run (`run = "…"`) est vérifié contre l'archive qu'il nomme, et les runs
//! qu'aucun enregistrement ne réclame sont listés.

use anyhow::Result;
use opendomain_core::provenance as run_provenance;
use opendomain_core::Workspace;

/// `opendomain provenance` — le pont F14 : chaque enregistrement qui cite un
/// run (`run = "…"`) est vérifié contre l'archive qu'il nomme, et les runs
/// qu'aucun enregistrement ne réclame sont listés.
pub fn provenance(
    ws: &Workspace,
    index: &opendomain_core::workspace::Index,
    json: bool,
) -> Result<()> {
    let report = run_provenance::audit(index, &ws.root);

    if json {
        let rows: Vec<serde_json::Value> = report
            .rows
            .iter()
            .map(|r| {
                let v = &r.verification;
                serde_json::json!({
                    "artifact": r.artifact,
                    "run": v.run_name,
                    "verdict": run_provenance::verdict_label(v),
                    "exists": v.exists,
                    "frozen_graph": v.frozen_graph,
                    "gates_log": v.gates_log,
                    "outputs": v.outputs,
                    "final_failure": v.final_failure,
                    "node": r.citation.node,
                    "node_verdict": v.node_verdict.as_ref().map(|nv| match nv {
                        run_provenance::NodeVerdict::Verified(line) => serde_json::json!({"ok": true, "line": line}),
                        run_provenance::NodeVerdict::Failed(line) => serde_json::json!({"ok": false, "line": line}),
                    }),
                })
            })
            .collect();
        let out = serde_json::json!({
            "records_citing": report.rows.len(),
            "verified": report.verified_count(),
            "total_runs": report.total_runs,
            "orphan_runs": report.orphan_runs,
            "runs_dir_exists": report.runs_dir_exists,
            "rows": rows,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("Run provenance (F14/L9) — le pont des archives de run vers les registres :\n");
    if !report.runs_dir_exists {
        if report.rows.is_empty() {
            println!("aucune archive de run (`opendomain/harness/runs/` n'existe) et aucun\nenregistrement n'en cite : quand un travail exécuté hors des registres a\nvérifié une étape, archivez-le là, puis `run = \"…\"` cite l'archive qui\na porté la vérification.");
            return Ok(());
        }
        println!(
            "warning: `opendomain/harness/runs/` n'existe pas alors que {} enregistrement(s) citent un run :",
            report.rows.len()
        );
    }
    for r in &report.rows {
        let v = &r.verification;
        println!(
            "  {} ({}) → {} : {}",
            r.artifact,
            r.artifact_prefix,
            v.run_name,
            run_provenance::verdict_label(v)
        );
        if !v.exists {
            println!("      l'archive n'existe pas — la citation est un appel à vérifier, pas une preuve");
        } else if v.final_failure.is_some() {
            println!("      {}", v.final_failure.clone().unwrap_or_default());
        } else if let Some(nv) = &v.node_verdict {
            match nv {
                run_provenance::NodeVerdict::Verified(line) => println!("      {line}"),
                run_provenance::NodeVerdict::Failed(line) => println!("      {line}"),
            }
        }
    }
    let total = report.rows.len();
    if total > 0 {
        println!(
            "\ntotal : {total} citation(s) — {} vérifiée(s), {} à corriger",
            report.verified_count(),
            report.broken().len()
        );
    }
    if !report.orphan_runs.is_empty() {
        println!(
            "\nruns jamais cités ({} / {}) — une preuve archivée que rien ne réclame :",
            report.orphan_runs.len(),
            report.total_runs
        );
        for r in report.orphan_runs.iter().take(6) {
            println!("  - {r}");
        }
        if report.orphan_runs.len() > 6 {
            println!("  … +{} autres", report.orphan_runs.len() - 6);
        }
        println!(
            "attachez `run = \"{}\"` à l'enregistrement qu'ils ont produit — un run non cité\nest un rapport que personne n'a signé.",
            report.orphan_runs.first().unwrap_or(&String::new())
        );
    }
    if total == 0 && report.runs_dir_exists {
        println!(
            "l'archive existe ({} run(s)) mais aucun enregistrement n'en cite : ajoutez\n`run = \"runs/<run-NNN-…>\"` au TEST/DRIFT produit par ce travail.",
            report.total_runs
        );
    }
    Ok(())
}
