//! `opendomain reference` — les catalogues : les neuf gates, les kinds
//! d'artefacts et le catalogue des règles (blocking/advisory).

use anyhow::Result;
use opendomain_core::gates::{self, Gate};
use opendomain_core::ids::IdKind;
use opendomain_core::report::Style;

use super::args::Cli;

pub fn cmd_reference(cli: &Cli, style: Style) -> Result<()> {
    if cli.json {
        let gates: Vec<serde_json::Value> = Gate::ALL
            .iter()
            .map(|g| {
                serde_json::json!({
                    "number": g.number,
                    "name": g.name,
                    "stage": g.stage.as_str(),
                    "question": g.question,
                })
            })
            .collect();
        let kinds: Vec<serde_json::Value> = IdKind::ALL
            .iter()
            .map(|k| {
                serde_json::json!({
                    "prefix": k.prefix(),
                    "label": k.label(),
                    "path": k.default_path(),
                })
            })
            .collect();
        let rules: Vec<serde_json::Value> = gates::RULES
            .iter()
            .map(|r| {
                serde_json::json!({
                    "slug": r.full_slug(),
                    "gate": r.gate,
                    "gate_name": r.gate().name,
                    "severity": if r.severity == gates::Severity::Blocking {
                        "blocking"
                    } else {
                        "advisory"
                    },
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "gates": gates,
                "artifact_kinds": kinds,
                "rules": rules,
            })
        );
        return Ok(());
    }
    println!("gates\n");
    for g in Gate::ALL {
        println!("  {}  {:<14} {}", g.number, g.name, g.question);
    }
    println!("\nartifact kinds\n");
    for k in IdKind::ALL {
        println!("  {:<6} {:<24} {}", k.prefix(), k.label(), k.default_path());
    }

    // The rule catalogue. An agent has to be able to find out what will be
    // checked before it writes anything, not only after it trips a rule: a rule
    // that is invisible until it fires is a rule you find out about by failing.
    let blocking = gates::RULES
        .iter()
        .filter(|r| r.severity == gates::Severity::Blocking)
        .count();
    println!(
        "\nrules ({} total: {} blocking, {} advisory)\n",
        gates::RULES.len(),
        blocking,
        gates::RULES.len() - blocking
    );
    for g in Gate::ALL {
        let rules: Vec<&gates::Rule> = gates::RULES.iter().filter(|r| r.gate == g.number).collect();
        if rules.is_empty() {
            continue;
        }
        println!("  gate {} — {}", g.number, g.name);
        for r in rules {
            let mark = if r.severity == gates::Severity::Blocking {
                "BLOCK"
            } else {
                "adv  "
            };
            println!("    {mark}  {}", r.full_slug());
        }
    }
    let _ = style;
    Ok(())
}
