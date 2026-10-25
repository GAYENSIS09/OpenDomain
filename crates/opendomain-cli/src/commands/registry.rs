//! Les écritures de registre, append-only comme `init` : `link` (ajouter
//! une relation dans le bloc `toml`), `new` (le prochain identifiant libre —
//! lecture seule, ne réserve rien), `advance` (déplacer le status, jamais en
//! arrière).

use anyhow::{anyhow, bail, Context, Result};
use opendomain_core::ids::{Id, IdKind};
use opendomain_core::report::{self, Style};
use opendomain_core::vocab::ArtifactStatus;
use opendomain_core::workspace::{Index, Workspace};

use super::args::{AdvanceArgs, Cli, LinkArgs, NewArgs};
use super::context::{parse_id, style_ok, to_json};

pub fn cmd_link(args: &LinkArgs, ws: &Workspace, index: &Index, style: Style) -> Result<()> {
    if args.check {
        let dangling = index.dangling();
        if dangling.is_empty() {
            println!("{}", style_ok(style, "every link resolves"));
            return Ok(());
        }
        let total: usize = dangling.values().map(|v| v.len()).sum();
        for (id, missing) in &dangling {
            for m in missing {
                println!(
                    "{}",
                    report::render_finding_line(
                        "link/dangling",
                        &format!("{} references {}, which does not exist", id, m),
                        style,
                    )
                );
            }
        }
        bail!("{} dangling reference(s)", total);
    }

    // Anything other than `--check` needs a complete id/relation/target triple.
    let (raw_id, relation, raw_target) = match (&args.id, &args.relation, &args.target) {
        (Some(i), Some(r), Some(t)) => (i.as_str(), r.as_str(), t.as_str()),
        _ => bail!(
            "`opendomain link` needs either --check, or an id plus a relation plus a target.\n\
             examples:\n  \
               opendomain link --check\n  \
               opendomain link REQ-001 derived-from EVD-014"
        ),
    };

    let id = parse_id(raw_id)?;
    let target = parse_id(raw_target)?;
    let rel = opendomain_core::Relation::parse(relation)
        .ok_or_else(|| anyhow!("unknown relation '{}'", relation))?;

    if index.get(&id).is_none() {
        bail!("{} is not defined in this workspace", id);
    }
    if index.get(&target).is_none() {
        bail!(
            "{} is not defined in this workspace, so the link would dangle",
            target
        );
    }

    let file = index
        .get(&id)
        .map(|a| a.file.clone())
        .ok_or_else(|| anyhow!("{} has no file on disk", id))?;
    let path = ws.root.join(&file);
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;

    let key = rel.as_str();
    let line = format!("{} = [\"{}\"]", key, target);
    if let Some(added) = append_link(&mut text, &id, &line) {
        std::fs::write(&path, &text).with_context(|| format!("cannot write {}", path.display()))?;
        println!(
            "{}",
            style_ok(
                style,
                &format!("{}: added {} = [{}]", file.display(), key, target)
            )
        );
        if added {
            println!("  (created a `toml` block for this artifact)");
        }
    } else {
        println!(
            "{}",
            format!(
                "{}: {} already links to {} - nothing to do",
                file.display(),
                key,
                target
            )
        );
    }
    Ok(())
}

/// Adds `line` to the artifact's `toml` block, creating that block if needed.
///
/// Returns `true` when a new fenced block had to be created, which the caller
/// reports because inserting one into prose deserves a visible mention.
fn append_link(text: &mut String, id: &Id, line: &str) -> Option<bool> {
    let id_text = id.to_string();
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();

    // Find this artifact's heading. The parser accepts `## ID — Title` and a
    // bare `## ID`; accept either so a hand-written heading is not a hard error.
    let start = lines.iter().position(|l| {
        let t = l.trim();
        t == format!("## {}", id_text) || t.starts_with(&format!("## {} ", id_text))
    })?;

    // Find this artifact's fenced ```toml block, if it has one. Stop at the
    // next `##` heading so a block belonging to a later artifact is never
    // mistaken for this one's.
    let mut toml_open: Option<usize> = None;
    let mut in_fence = false;
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        if l.starts_with("## ") {
            break;
        }
        if l.trim_start().starts_with("```") {
            if in_fence {
                in_fence = false;
            } else if l.to_ascii_lowercase().contains("toml") && toml_open.is_none() {
                toml_open = Some(i);
            } else {
                in_fence = !l.to_ascii_lowercase().contains("toml");
            }
        }
    }

    let mut updated = lines.clone();
    match toml_open {
        Some(open) => {
            // Insert just before the closing fence.
            let mut close = open + 1;
            while close < updated.len() && !updated[close].trim_start().starts_with("```") {
                if updated[close].trim() == line {
                    return None; // already linked
                }
                close += 1;
            }
            if close >= updated.len() {
                return None; // unterminated fence: refuse to guess
            }
            updated.insert(close, line.to_string());
            *text = updated.join("\n");
            Some(false)
        }
        None => {
            // No metadata block yet. Create one directly under the heading.
            let at = start + 1;
            updated.insert(at, String::new());
            updated.insert(at + 1, "```toml".into());
            updated.insert(at + 2, line.to_string());
            updated.insert(at + 3, "```".into());
            *text = updated.join("\n");
            Some(true)
        }
    }
}

/// Prints the next free identifier of a kind, so two writers never claim the
/// same id by hand-counting headings. Read-only: it reserves nothing, and a
/// registry edited since still wins — re-run before writing, not after.
pub fn cmd_new(args: &NewArgs, index: &Index, cli: &Cli) -> Result<()> {
    let kind = IdKind::from_prefix(&args.kind)
        .map_err(|e| anyhow!("{e}; run `opendomain reference` for the known prefixes"))?;
    let next = match &args.scope {
        Some(scope) => {
            let norm = scope.trim().to_ascii_uppercase();
            Id::next_scoped(kind, Some(norm.as_str()), index.ids(kind))
        }
        None => Id::next_flat(kind, index.ids(kind)),
    };
    if cli.json {
        println!(
            "{}",
            to_json(&serde_json::json!({ "id": next.to_string() }))?
        );
    } else {
        println!("{next}");
    }
    Ok(())
}

pub fn cmd_advance(args: &AdvanceArgs, ws: &Workspace, index: &Index, style: Style) -> Result<()> {
    let id = parse_id(&args.id)?;
    let target_status: ArtifactStatus = args
        .status
        .parse()
        .map_err(|e: String| anyhow!("unknown status '{}': {}", args.status, e))?;

    let artifact = index
        .get(&id)
        .ok_or_else(|| anyhow!("{} is not defined in this workspace", id))?;

    if let Some(current) = artifact.status.rank() {
        if let Some(next) = target_status.rank() {
            if next < current {
                bail!(
                    "{} is {}; moving it back to {} loses recorded progress. Edit the file by hand \
                     if that is really what you want.",
                    id,
                    artifact.status.as_str(),
                    target_status.as_str()
                );
            }
        }
    }

    let file = artifact.file.clone();
    let path = ws.root.join(&file);
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;

    let old = format!("status = \"{}\"", artifact.status.as_str());
    let new = format!("status = \"{}\"", target_status.as_str());
    if !text.contains(&old) {
        bail!(
            "{}: expected a `{}` line for {}. Add it by hand, then re-run.",
            file.display(),
            old,
            id
        );
    }
    let occurrences = text.matches(&old).count();
    if occurrences > 1 {
        bail!(
            "{}: found {} occurrences of `{}`; refusing to guess which one is {}",
            file.display(),
            occurrences,
            old,
            id
        );
    }
    text = text.replacen(&old, &new, 1);
    std::fs::write(&path, &text).with_context(|| format!("cannot write {}", path.display()))?;
    println!(
        "{}",
        style_ok(
            style,
            &format!(
                "{}: {} -> {}",
                id,
                artifact.status.as_str(),
                target_status.as_str()
            )
        )
    );
    Ok(())
}
