//! Les vues d'ensemble du workspace : `status` (comptes par registre,
//! couverture, gate courante, prochaine étape) et `audit` (santé de la
//! traçabilité sur tout le workspace).

use anyhow::Result;
use opendomain_core::ids::IdKind;
use opendomain_core::report::{self, Style};
use opendomain_core::workspace::{Index, Workspace};

use super::args::Cli;
use super::context::to_json;

/// `opendomain status` — comptes par registre, couverture, gate courante.
pub fn cmd_status(ws: &Workspace, index: &Index, cli: &Cli, style: Style) -> Result<()> {
    let report = report::status("opendomain", &ws.config, index);
    if cli.json {
        println!("{}", to_json(&report)?);
    } else {
        println!("{}", report::render_status(&report, style));
        if let Some(step) = next_step(index, &report) {
            println!("\nnext smallest step: {step}");
        }
    }
    Ok(())
}

/// The smallest next step: the first blocked gate plus the registries that
/// are still empty for it. A gate blocked by five empty registries at once
/// tells the user where to look, not where to start; this names the files.
fn next_step(index: &Index, report: &report::StatusReport) -> Option<String> {
    let gate = report.blocking_gate?;
    let kinds: &[IdKind] = match gate {
        "context" => &[IdKind::Dom, IdKind::Stk],
        "research" => &[IdKind::Src, IdKind::Evd],
        "domain" => &[
            IdKind::Term,
            IdKind::Act,
            IdKind::Proc,
            IdKind::Ent,
            IdKind::Rule,
        ],
        "requirements" => &[IdKind::Req],
        "opportunity" => &[IdKind::Cpt, IdKind::Hyp],
        "specification" => &[IdKind::Spec, IdKind::Ac],
        "plan" => &[IdKind::Plan, IdKind::Task],
        "implementation" => &[IdKind::Test],
        "completion" => &[IdKind::Req, IdKind::Test],
        _ => &[],
    };
    let missing: Vec<String> = kinds
        .iter()
        .filter(|k| index.ids(**k).is_empty())
        .map(|k| format!("{}-### in {}", k.prefix(), k.default_path()))
        .collect();
    if missing.is_empty() {
        Some(format!(
            "gate `{gate}` is blocked but its registries are non-empty — run `opendomain gate {gate}` for the findings"
        ))
    } else {
        Some(format!(
            "gate `{gate}` is blocked — record {} (start with the first)",
            missing.join(", ")
        ))
    }
}

pub fn cmd_audit(index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let report = report::audit(index);
    if cli.json {
        println!("{}", to_json(&report)?);
    } else {
        println!("{}", report::render_audit(&report, style));
    }
    Ok(())
}
