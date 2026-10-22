//! Le cycle de change (layout OpenSpec) : `propose` crée la proposition,
//! `apply` la marque appliquée, `archive` la déplace. Append-only :
//! [`write_new`] refuse d'écraser un fichier existant.

use anyhow::{bail, Context, Result};
use opendomain_core::report::Style;
use opendomain_core::workspace::Workspace;
use std::path::Path;

use super::args::{ApplyArgs, ArchiveArgs, ProposeArgs};
use super::context::style_ok;

pub fn cmd_propose(args: &ProposeArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    if slug.is_empty() {
        bail!("a change needs a name");
    }
    let dir = ws.root.join("changes").join(&slug);
    if dir.exists() {
        bail!("change '{}' already exists at {}", slug, dir.display());
    }
    std::fs::create_dir_all(dir.join("specs"))
        .with_context(|| format!("cannot create {}", dir.display()))?;

    let summary = if args.summary.is_empty() {
        "TODO: describe the change in one sentence.".to_string()
    } else {
        args.summary.clone()
    };

    let proposal = format!(
        "# {slug}\n\n\
         > {summary}\n\n\
         ## Why\n\n\
         What is the problem, and what evidence shows it is real?\n\n\
         ## What changes\n\n\
         _Describe the delta against the current state._\n\n\
         ## Impact\n\n\
         _Which artifacts gain, lose, or change status._\n",
        slug = slug,
        summary = summary
    );
    write_new(&dir.join("proposal.md"), &proposal)?;

    let tasks = "# Tasks\n\n\
                  | ID | Task | Artifact | Status |\n\
                  |----|------|----------|--------|\n\n\
                  _Add one row per unit of work. Reference the artifact each task serves._\n";
    write_new(&dir.join("tasks.md"), tasks)?;

    let design = format!(
        "# {slug} design\n\n\
         ## Context\n\n\
         _The forces at play: constraints, prior decisions, related changes._\n\n\
         ## Decision\n\n\
         _What was decided._\n\n\
         ## Consequences\n\n\
         _What becomes easier, what becomes harder, what is now locked in._\n",
        slug = slug
    );
    write_new(&dir.join("design.md"), &design)?;

    let spec_readme = "# Specifications\n\n\
                      _One file per affected capability, holding the deltas rather than \
                      restating current behaviour._\n";
    write_new(&dir.join("specs").join("README.md"), spec_readme)?;

    let change = format!(
        "<!-- OpenDomain change record. The CLI reads `status` to place this change. -->\n\
         status = \"PROPOSED\"\n\
         created = \"{}\"\n",
        today()
    );
    write_new(&dir.join("change.toml"), &change)?;

    println!(
        "{}",
        style_ok(style, &format!("proposed change '{}'", slug))
    );
    for f in [
        "proposal.md",
        "tasks.md",
        "design.md",
        "specs/README.md",
        "change.toml",
    ] {
        println!("  + changes/{slug}/{f}", slug = slug);
    }
    println!("\nnext: fill in proposal.md, then `opendomain apply {}`", slug);
    Ok(())
}

pub fn cmd_apply(args: &ApplyArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    let dir = ws.root.join("changes").join(&slug);
    let change = dir.join("change.toml");
    if !change.is_file() {
        bail!(
            "no change '{}' at {}. Create it with `opendomain propose {}`.",
            slug,
            dir.display(),
            slug
        );
    }
    let text = std::fs::read_to_string(&change)
        .with_context(|| format!("cannot read {}", change.display()))?;
    if !text.contains("status = \"APPLIED\"") {
        let updated = text.replacen("status = \"PROPOSED\"", "status = \"APPLIED\"", 1);
        std::fs::write(&change, updated)
            .with_context(|| format!("cannot write {}", change.display()))?;
    }
    println!(
        "{}",
        style_ok(style, &format!("change '{}' marked applied", slug))
    );
    println!("  now update the registries, then run `opendomain gate specification`");
    Ok(())
}

pub fn cmd_archive(args: &ArchiveArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    let from = ws.root.join("changes").join(&slug);
    if !from.is_dir() {
        bail!("no change directory at {}", from.display());
    }
    let to = ws.root.join("changes").join("archive").join(&slug);
    if to.exists() {
        bail!("{} already exists", to.display());
    }
    std::fs::create_dir_all(to.parent().expect("archive has a parent"))
        .with_context(|| format!("cannot create {}", to.parent().unwrap().display()))?;
    std::fs::rename(&from, &to)
        .with_context(|| format!("cannot move {} to {}", from.display(), to.display()))?;
    println!(
        "{}",
        style_ok(
            style,
            &format!("archived '{}' to changes/archive/{}", slug, slug)
        )
    );
    Ok(())
}

fn write_new(path: &Path, body: &str) -> Result<()> {
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    std::fs::write(path, body).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
