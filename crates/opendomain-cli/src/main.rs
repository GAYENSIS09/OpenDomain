//! L'interface en ligne de commande `opendomain` — le point d'entrée mince.
//!
//! La définition clap vit dans [`commands::args`] ; les commandes de registre
//! dans [`commands`] ; le harnais d'exécution dans [`harness`] ; le
//! sous-système de vérification (§9/§10) dans [`verification`] — chaque
//! dossier une responsabilité, à l'image de `opendomain_core::harness`.
//!
//! Toute commande est une coquille mince sur `opendomain_core` : découvrir un
//! workspace, charger l'index, confier à une fonction pure, rendre le
//! résultat. Rien ici ne réécrit un registre sauf `opendomain init`,
//! `opendomain link` et les commandes de cycle de vie — et ceux-là
//! n'ajoutent qu'append-only.

use anyhow::{Context, Result};
use clap::Parser;
use commands::args::{Cli, Command, HarnessSpecOp, SkillOp, WorkflowOp};
use commands::changes::{cmd_apply, cmd_archive, cmd_propose};
use commands::context::with_index;
use commands::gates::cmd_gate;
use commands::init::cmd_init;
use commands::inspect::{cmd_graph, cmd_impact, cmd_show, cmd_trace};
use commands::reference::cmd_reference;
use commands::registry::{cmd_advance, cmd_link, cmd_new};
use commands::status::{cmd_audit, cmd_status};
use opendomain_core::report::Style;

mod commands;
mod harness;
mod verification;

fn main() {
    let cli = Cli::parse();
    let style = Style::detect(cli.no_color || cli.json);
    if let Err(err) = run(&cli, style) {
        eprintln!("opendomain: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli, style: Style) -> Result<()> {
    match &cli.command {
        Command::Init(args) => cmd_init(args, cli, style),
        Command::Reference => cmd_reference(cli, style),
        Command::Status => with_index(cli, style, |ws, index, _| {
            cmd_status(ws, index, cli, style)
        }),
        Command::Trace(args) => with_index(cli, style, |_ws, index, _| {
            cmd_trace(args, index, style, cli)
        }),
        Command::Impact(args) => with_index(cli, style, |_ws, index, _| {
            cmd_impact(args, index, style, cli)
        }),
        Command::Graph(args) => with_index(cli, style, |_ws, index, _| cmd_graph(args, index)),
        Command::Gate(args) => with_index(cli, style, |_ws, index, _| {
            cmd_gate(args, index, style, cli)
        }),
        Command::Audit => with_index(cli, style, |_ws, index, _| cmd_audit(index, style, cli)),
        Command::Link(args) => {
            with_index(cli, style, |ws, index, _| cmd_link(args, ws, index, style))
        }
        Command::Show(args) => with_index(cli, style, |_ws, index, _| {
            cmd_show(args, index, style, cli)
        }),
        Command::New(args) => with_index(cli, style, |_ws, index, _| {
            cmd_new(args, index, cli)
        }),
        Command::Advance(args) => with_index(cli, style, |ws, index, _| {
            cmd_advance(args, ws, index, style)
        }),
        Command::Propose(args) => {
            with_index(cli, style, |ws, _index, _| cmd_propose(args, ws, style))
        }
        Command::Apply(args) => with_index(cli, style, |ws, _index, _| cmd_apply(args, ws, style)),
        Command::Archive(args) => {
            with_index(cli, style, |ws, _index, _| cmd_archive(args, ws, style))
        }
        // Execution harness. These commands address the workspace from the
        // current directory, so the global `-C` flag is honoured by entering
        // it once — the registries commands resolve the same way.
        Command::Spec { op } => {
            enter(cli)?;
            match op {
                HarnessSpecOp::New { slug } => opendomain_core::scaffold::harness::spec_new(slug),
            }
        }
        Command::Validate { path } => {
            enter(cli)?;
            harness::commands::validate(path)
        }
        Command::Doctor => {
            enter(cli)?;
            harness::doctor::run()
        }
        Command::Plan { spec_path, llm, timeout } => {
            enter(cli)?;
            harness::plan::run(spec_path, *llm, *timeout)
        }
        Command::Ground { spec_path } => {
            enter(cli)?;
            harness::ground::run(spec_path)
        }
        Command::Build { spec_path } => {
            enter(cli)?;
            harness::build::run(spec_path)
        }
        Command::Run { spec_path, executor, timeout, resume, graph } => {
            enter(cli)?;
            harness::runflow::run(spec_path, executor, *timeout, resume.clone(), graph.clone())
        }
        Command::Verify { run_path } => {
            enter(cli)?;
            harness::audit::verify(run_path)
        }
        Command::Attribute { run_path } => {
            enter(cli)?;
            harness::audit::attribute(run_path)
        }
        Command::Repatch { run_path, llm, allow, timeout } => {
            enter(cli)?;
            harness::repatch::run(run_path, *llm, allow.as_deref(), *timeout)
        }
        Command::Bench => {
            enter(cli)?;
            harness::bench::run()
        }
        Command::Workflow { op } => {
            enter(cli)?;
            match op {
                WorkflowOp::New { kind, slug } => harness::workflows::run(kind, slug),
            }
        }
        Command::Skill { op } => {
            enter(cli)?;
            match op {
                // an explicit `--provides …`? else: frontmatter or typed error
                SkillOp::Add { source, id, provides } => {
                    let forced = if provides.is_empty() { None } else { Some(provides.clone()) };
                    harness::components::skill_add(source, id.clone(), forced)
                }
                SkillOp::New { id } => harness::components::skill_new(id),
            }
        }
        Command::Resolve { spec_path } => {
            enter(cli)?;
            harness::components::resolve_cmd(spec_path)
        }
        // Verification subsystem (Sections 9/10): rigour table, provisioner,
        // binding audit. Workspace commands — they read registries and write
        // generated scaffolds, so they honour `-C` through workspace discovery.
        Command::Rigour => with_index(cli, style, |_ws, index, _| {
            verification::rigour(index, cli.json)
        }),
        Command::Provision { dry_run, .. } => with_index(cli, style, |ws, index, _| {
            verification::provision(ws, index, *dry_run)
        }),
        Command::Bind => with_index(cli, style, |_ws, index, _| {
            verification::bind(index, cli.json)
        }),
        Command::Provenance => with_index(cli, style, |ws, index, _| {
            verification::provenance(ws, index, cli.json)
        }),
    }
}

/// Enters `-C <dir>` for the harness commands, which work relative to the
/// project root the same way the registry commands resolve it.
fn enter(cli: &Cli) -> Result<()> {
    if let Some(dir) = &cli.directory {
        std::env::set_current_dir(dir)
            .with_context(|| format!("cannot enter {}", dir.display()))?;
    }
    Ok(())
}
