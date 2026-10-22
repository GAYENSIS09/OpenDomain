//! Le contexte commun aux commandes de registre : résolution du répertoire
//! projet (`-C`), découverte du workspace, parsing des identifiants, style
//! terminal et sérialisation JSON.
//!
//! Chaque commande passe par [`with_index`] pour que la découverte et le
//! rapport d'erreur soient identiques partout.

use anyhow::{anyhow, Context, Result};
use opendomain_core::ids::Id;
use opendomain_core::report::Style;
use opendomain_core::workspace::{Index, Workspace};
use std::path::PathBuf;

use super::args::Cli;

/// Resolves the project directory a command should operate on.
pub fn start_dir(cli: &Cli) -> PathBuf {
    cli.directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Discovers the workspace, loads the index, and hands both to `body`.
///
/// Every read-only command goes through here so that workspace discovery and
/// error reporting stay identical across the CLI.
pub fn with_index<F>(cli: &Cli, style: Style, body: F) -> Result<()>
where
    F: FnOnce(&Workspace, &Index, Style) -> Result<()>,
{
    let start = start_dir(cli);
    let ws = Workspace::discover(&start)
        .with_context(|| format!("no OpenDomain workspace found from {}", start.display()))?;
    let index = ws.load().context("failed to load the workspace index")?;
    body(&ws, &index, style)
}

pub fn parse_id(raw: &str) -> Result<Id> {
    // `BadId`'s Display already names the expected shape, so no extra wrapping.
    Id::parse(raw).map_err(|e| anyhow!("{}", e))
}

/// Tints a success message, respecting the global `--no-color` / `--json` flag.
pub fn style_ok(style: Style, msg: &str) -> String {
    style.paint(msg, opendomain_core::report::Colour::Green)
}

/// Serialises a value as pretty JSON, or fails with context.
pub fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value).context("cannot serialise output as JSON")
}
