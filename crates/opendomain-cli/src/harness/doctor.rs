//! `opendomain doctor` — exécuteur-agnosticisme vérifiable.

use std::path::PathBuf;

use anyhow::Result;

/// (binaire, rôle) — les exécuteurs sont des composants remplaçables.
const TOOLS: &[(&str, &str)] = &[
    ("opencode", "exécuteur OpenCode (adapter recommandé)"),
    ("claude", "exécuteur Claude Code (adapter recommandé)"),
    ("aider", "exécuteur Aider (adapter optionnel)"),
    ("git", "versionnement des artefacts opendomain/harness/"),
    ("cargo", "toolchain Rust (tests du harnais et des projets cibles)"),
    ("node", "toolchain Node (projets cibles web)"),
    ("python", "toolchain Python (projets cibles Python)"),
];

/// `opendomain doctor`.
pub fn run() -> Result<()> {
    println!("opendomain doctor — harnais v{}", env!("CARGO_PKG_VERSION"));
    println!("boucle canonique : {}", opendomain_core::harness::LOOP);
    let mut found_any = false;
    for (bin, role) in TOOLS {
        let status = match which(bin) {
            Some(p) => {
                found_any = true;
                format!("[ok]     {} ({})", p.display(), role)
            }
            None => format!("[absent] {bin} — {role}"),
        };
        println!("{status}");
    }
    if !found_any {
        println!("\nAucun exécuteur LLM détecté : OpenDomain reste utilisable en mode");
        println!("déterministe (init / validate / gates offline) mais la phase 1 (M1)");
        println!("nécessitera un exécuteur (opencode, claude, …).");
    }
    Ok(())
}

/// Résolution `which` portable, sans dépendance : cherche le binaire dans PATH,
/// avec les suffixes Windows utiles (`.exe`, `.cmd`, `.bat`).
pub fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        // Windows : préférer les images exécutables réelles (`.exe`, `.cmd`,
        // `.bat`) aux shims shells sans extension (`opencode` est un script
        // `sh` généré par npm — non exécutable sous CreateProcess).
        let mut names: Vec<String> = vec![bin.to_string()];
        #[cfg(windows)]
        {
            names.insert(0, format!("{bin}.bat"));
            names.insert(0, format!("{bin}.cmd"));
            names.insert(0, format!("{bin}.exe"));
        }
        for name in names {
            let candidate = dir.join(&name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
