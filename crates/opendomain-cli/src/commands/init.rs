//! `opendomain init` — crée `opendomain/` : registres + `opendomain.toml` +
//! contrats agent, puis le harnais. Idempotent : n'écrase jamais un fichier
//! existant sans `--force` (et `AGENTS.md` jamais).

use anyhow::Result;
use opendomain_core::scaffold;
use opendomain_core::report::Style;
use opendomain_core::vocab::RepositoryKind;

use super::args::{Cli, InitArgs};
use super::context::{start_dir, style_ok, to_json};

pub fn cmd_init(args: &InitArgs, cli: &Cli, style: Style) -> Result<()> {
    let start = start_dir(cli);
    let name = args
        .name
        .clone()
        .or_else(|| {
            start
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "opendomain-project".into());

    // `opendomain init` installs only: registries + opendomain.toml + agent contract.
    // Phase-0 context is never recorded here. Run `/opendomain-project`
    // (context-completer agent) afterwards to fill `project.md`,
    // `opendomain.toml [project]`, `DOM-001`/`STK-001` from the repository,
    // external research, then user questions for the rest.
    let mut config = scaffold::default_config(&name, args.profile.into(), args.authority.into());
    config.repository = args.repository.into();

    let od_dir = start.join("opendomain");
    // A greenfield declaration next to an existing implementation is the
    // Thinkell-class mistake: the workspace then claims nothing exists while
    // the repo proves otherwise. Warn, do not silently switch — the caller
    // decides what the project is, `opendomain` only reports what it sees. An existing
    // workspace that already declares brownfield/hybrid is not a mistake, so
    // the recorded classification wins over the flag default.
    let signals = scaffold::brownfield_signals(&start);
    let looks_brownfield = !signals.is_empty();
    let disk = opendomain_core::config::Config::load(&od_dir).ok();
    let already_classified = disk
        .as_ref()
        .is_some_and(|c| c.repository != RepositoryKind::Greenfield);
    if already_classified && config.repository == RepositoryKind::Greenfield {
        // On a re-run the recorded file is the truth: flags only take effect
        // with --force (which rewrites the file), so without it the flag
        // default must not silently downgrade the workspace.
        if let Some(recorded) = disk {
            config.repository = recorded.repository;
        }
    }
    if config.repository == RepositoryKind::Greenfield && looks_brownfield && !already_classified {
        eprintln!(
            "warning: found existing implementation signals ({}) but --repository greenfield was requested; \
             prefer --repository brownfield so `opendomain` treats code as a clue, not a specification",
            signals.join(", ")
        );
    }
    if args.dry_run {
        let templates = scaffold::templates::for_profile(config.profile);
        // `opendomain.toml` is written by `scaffold` itself rather than by a
        // template, so it has to be counted and listed here explicitly. Without
        // this the preview under-reported by one file, and the count did not
        // match what a real run created.
        let config_exists = od_dir.join("opendomain.toml").exists();
        let total = templates.len() + usize::from(!config_exists);
        println!("would create {total} file(s) under {}", od_dir.display());
        if !config_exists {
            println!("  opendomain.toml");
        }
        for t in templates {
            println!("  {}", t.path);
        }
        // The agent files go to the project root and follow different rules from
        // the registries, so the preview has to follow them too. Listing the
        // twenty-four outright as "would create" would be wrong twice over: a
        // fresh run does not create them all, and an existing `AGENTS.md` is
        // never replaced even under `--force`.
        let root = start.clone();
        let mut would_write = 0usize;
        let mut would_keep = 0usize;
        for t in scaffold::agent_contract::all() {
            let exists = root.join(&t.path).exists();
            let keep = exists && t.path == "AGENTS.md";
            if keep {
                would_keep += 1;
            } else if !exists {
                would_write += 1;
            }
            let note = match (keep, exists) {
                (true, _) => "  (left alone; --force does not replace this)",
                (_, true) => "  (unchanged; --force would refresh it)",
                _ => "",
            };
            println!("  {}{note}", t.path);
        }
        // The execution scaffold sits inside the same workspace directory and
        // is idempotent without --force: nothing it wrote before is replaced,
        // so a preview that omits it would under-report what `opendomain init` does.
        println!("\nharness (under opendomain/harness/):");
        for p in opendomain_core::scaffold::harness::preview_targets() {
            let exists = start.join(p).exists();
            let note = if exists { "  (unchanged; never overwritten)" } else { "" };
            println!("  {p}{note}");
        }
        println!("\nagent instructions: {would_write} to write, {would_keep} left alone");
        return Ok(());
    }

    let report = scaffold::scaffold(&od_dir, &config, args.force)?;
    if cli.json {
        println!("{}", to_json(&report)?);
        return Ok(());
    }
    if report.is_noop() {
        println!(
            "{}",
            style_ok(
                style,
                "workspace already initialised; nothing to do (use --force to overwrite)"
            )
        );
    } else {
        println!(
            "{}",
            style_ok(
                style,
                &format!(
                    "initialised {} — {} created, {} already present, {} agent file(s)",
                    od_dir.display(),
                    report.created.len(),
                    report.skipped.len(),
                    report.agent_files.len()
                )
            )
        );
    }
    for p in &report.created {
        println!("  + {p}");
    }
    for p in &report.skipped {
        println!("  = {p} (unchanged)");
    }
    if !report.agent_files.is_empty() {
        println!("\nagent instructions:");
        for p in &report.agent_files {
            println!("  + {p}");
        }
    }

    // The execution harness scaffolds under the same workspace directory:
    // `opendomain/harness/` — spec/graph/runs/config.opendomain.toml and the
    // meta-agent contract, installed into the agent the `--executor` flag
    // names. Idempotent: it never overwrites an existing file.
    opendomain_core::scaffold::harness::run(&start.to_string_lossy())?;

    println!("\nnext: run /opendomain-project to complete context, then `opendomain status` + `opendomain gate context`");
    println!("next (harness): `opendomain spec new 001-my-task` → `opendomain ground` → `opendomain plan` → `opendomain run`");
    Ok(())
}
