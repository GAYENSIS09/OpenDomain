//! `opendomain gate` — exécuter une gate par nom (`context`…) ou numéro
//! (0-8), seule ou `--through`, et rendre les findings.

use anyhow::{anyhow, bail, Result};
use opendomain_core::gates::{self, Gate};
use opendomain_core::report::{self, Style};
use opendomain_core::workspace::Index;

use super::args::{Cli, GateArgs};
use super::context::{style_ok, to_json};

fn parse_gate(raw: &str) -> Result<Gate> {
    if let Ok(n) = raw.parse::<u8>() {
        if let Some(g) = Gate::by_number(n) {
            return Ok(g);
        }
        bail!("no gate numbered {}; gates run 0-8", n);
    }
    Gate::by_name(&raw.to_ascii_lowercase()).ok_or_else(|| {
        anyhow!(
            "unknown gate '{}'; run `opendomain reference` for the list",
            raw
        )
    })
}

pub fn cmd_gate(args: &GateArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let reports = match (&args.gate, args.through) {
        (Some(name), _) => vec![gates::evaluate(parse_gate(name)?, index)],
        (None, Some(through)) => gates::evaluate_up_to(index, through),
        (None, None) => gates::evaluate_up_to(index, 8),
    };

    let failed: Vec<&opendomain_core::gates::GateReport> =
        reports.iter().filter(|r| !r.passed()).collect();

    if cli.json {
        println!("{}", to_json(&reports)?);
    } else {
        for r in &reports {
            println!("{}", report::render_gate(r, style));
        }
        if failed.is_empty() {
            println!("{}", style_ok(style, "all requested gates passed"));
        } else {
            println!(
                "{}",
                format!("{} of {} gate(s) failed", failed.len(), reports.len())
            );
        }
    }

    if !failed.is_empty() {
        // `--strict` is redundant today: a blocking finding always fails, and an
        // advisory never does. It is accepted so scripts can be explicit.
        let _ = args.strict;
        bail!("quality gates reported blocking findings");
    }
    Ok(())
}
