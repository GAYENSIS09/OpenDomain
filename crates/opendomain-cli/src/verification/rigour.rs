//! `opendomain rigour` — la table §9.7 toujours ; l'audit des déclarations
//! de rigueur quand un workspace en enregistre.

use anyhow::Result;
use opendomain_core::ids::IdKind;
use opendomain_core::verification;
use serde::Serialize;

use super::common::REQUIREMENT_KINDS;

/// `opendomain rigour` — the table always; the workspace audit when inside a
/// workspace that has records.
pub fn rigour(index: &opendomain_core::workspace::Index, json: bool) -> Result<()> {
    #[derive(Serialize)]
    struct Audit {
        records: usize,
        declared: usize,
        missing: Vec<String>,
        unknown: Vec<serde_json::Value>,
    }

    let table: Vec<serde_json::Value> = verification::RIGOUR_TABLE
        .iter()
        .map(|row| {
            serde_json::json!({
                "character": row.character,
                "primary": row.primary,
                "secondary": row.secondary,
            })
        })
        .collect();

    let mut records: Vec<&opendomain_core::Artifact> = REQUIREMENT_KINDS
        .iter()
        .flat_map(|k| index.of_kind(*k))
        .collect();
    records.extend(index.of_kind(IdKind::Ac));
    records.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

    let mut audit = Audit {
        records: records.len(),
        declared: 0,
        missing: Vec::new(),
        unknown: Vec::new(),
    };
    for a in &records {
        match verification::declared_rigour(a) {
            Some(_) => audit.declared += 1,
            None => {
                if let Some(bad) = verification::unknown_rigour(a) {
                    audit.unknown.push(serde_json::json!({
                        "id": a.id.to_string(),
                        "declared": bad,
                    }));
                } else {
                    audit.missing.push(a.id.to_string());
                }
            }
        }
    }
    let unknown = std::mem::take(&mut audit.unknown);
    let missing = std::mem::take(&mut audit.missing);

    if json {
        let report = serde_json::json!({ "table": table, "workspace": {
            "records": audit.records,
            "declared": audit.declared,
            "missing": missing,
            "unknown": unknown,
        }});
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!(
        "Verification rigour selection (Section 9.7) — an engineering decision, never inferred:\n"
    );
    println!(
        "{:<28} {:<24} {}",
        "Requirement character", "Primary mechanism", "Secondary"
    );
    for row in verification::RIGOUR_TABLE {
        println!(
            "{:<28} {:<24} {}",
            row.character, row.primary, row.secondary
        );
    }
    println!(
        "\nDeclare per requirement in its TOML block: rigour = \"Business behaviour, acceptance\".\nThe requirement character and the primary mechanism both resolve. The last row\nis deliberate: a test would silently assert a contested rule — a contested rule\ngets a scenario, not a test.\n"
    );
    println!(
        "Workspace: {} of {} requirement/criterion records declare a rigour.",
        audit.declared, audit.records
    );
    if !unknown.is_empty() {
        println!("\nunresolved declarations (no table row matches — fix the value):");
        for u in &unknown {
            println!(
                "  - {}: rigour = \"{}\"",
                u["id"].as_str().unwrap_or(""),
                u["declared"].as_str().unwrap_or("")
            );
        }
    }
    if !missing.is_empty() {
        let shown: Vec<String> = missing.iter().take(5).cloned().collect();
        println!(
            "\nmissing: {}{} — declare `rigour` so the provisioner can bind the right mechanism.",
            shown.join(", "),
            if missing.len() > 5 {
                format!(" … +{}", missing.len() - 5)
            } else {
                String::new()
            }
        );
        println!("\nnext: declare the rigour per record, then run `opendomain provision`.");
    } else if audit.declared > 0 {
        println!("\nnext: run `opendomain provision` to generate the scaffolds the declared rigour names.");
    }
    Ok(())
}
