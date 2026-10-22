//! `opendomain ground` — grounding à la conception.
//!
//! Collecte **avant** la génération : marqueurs de projet, manifestes,
//! README, toolchain — stockés comme notes datées dans
//! `opendomain/harness/grounding/<slug>/repo-scan.md`. Un échec plus tard de ce
//! type sera `grounding_failure` (attribution: amont → re-exécuter ceci).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::harness::common::{resolve_spec, title_of};
use crate::harness::doctor::which;

const MANIFESTS: &[(&str, &str)] = &[
    ("Cargo.toml", "Rust"),
    ("package.json", "Node"),
    ("pyproject.toml", "Python"),
    ("requirements.txt", "Python (pip)"),
    ("go.mod", "Go"),
    ("pom.xml", "Java (Maven)"),
];

/// `opendomain ground <spec>`.
pub fn run(spec_input: &str) -> Result<()> {
    let sp = resolve_spec(spec_input)?;
    let spec_text = std::fs::read_to_string(&sp.spec)?;

    let mut md = String::new();
    md.push_str(&format!(
        "# Grounding — {}\n\nCollecte à la conception. Source : repo cible + toolchains.\n\n",
        sp.slug
    ));

    // 1) projet détecté
    let title = title_of(&spec_text).unwrap_or_else(|| sp.slug.clone());
    md.push_str(&format!(
        "## Spec\n- titre : {title}\n- dossier : {}\n\n",
        sp.dir.display()
    ));

    // 2) manifestes
    md.push_str("## Projet\n");
    let mut found = Vec::new();
    for (file, lang) in MANIFESTS {
        if Path::new(file).exists() {
            found.push((file, lang));
            md.push_str(&format!("- `{file}` — projet {lang}\n"));
        }
    }
    if found.is_empty() {
        md.push_str("- aucun manifeste reconnu à la racine (repo cible ?)\n");
    }

    // 3) arborescence (2 niveaux)
    md.push_str("\n## Arborescence (profil)\n");
    md.push_str(&walk_outline(Path::new("."), 0, 2));

    // 4) README (extrait)
    for readme in ["Readme.md", "README.md", "README.txt"] {
        if let Ok(content) = std::fs::read_to_string(readme) {
            md.push_str(&format!(
                "\n## {readme} (extrait)\n```\n{}```\n",
                head(&content, 60)
            ));
            break;
        }
    }

    // 5) toolchain
    md.push_str("\n## Toolchain\n");
    for bin in ["cargo", "node", "npm", "python", "git"] {
        md.push_str(&match which(bin) {
            Some(p) => format!("- {bin} : {}\n", p.display()),
            None => format!("- {bin} : absent\n"),
        });
    }
    md.push_str("\n> Notes produites automatiquement — sources datées à l'écriture ;\n> relancer `opendomain ground` pour rafraîchir.\n");

    // écriture
    let out_dir = PathBuf::from("opendomain/harness/grounding").join(&sp.slug);
    std::fs::create_dir_all(&out_dir)?;
    let out_file = out_dir.join("repo-scan.md");
    std::fs::write(&out_file, &md)?;

    // empreinte du dépôt au moment du design — rejouée (partie bon marché :
    // les manifestes) avant tout `run`. Une dérive du dépôt sort typée
    // `grounding_failure` AVANT tout appel LLM : « failures typed before any
    // spend » a besoin d'un instantané à comparer.
    let fingerprint = serde_json::json!({
        "manifests": manifest_hashes(),
        "tree": tree_names(Path::new("."), 1),
        "toolchain": toolchain_present(),
    });
    let fp_file = out_dir.join("fingerprint.json");
    std::fs::write(&fp_file, serde_json::to_string_pretty(&fingerprint)?)?;

    println!("OK grounding → {}", out_file.display());
    println!("  empreinte   → {}", fp_file.display());
    if found.is_empty() {
        println!("  signalement : aucun manifeste détecté — vérifier le repo cible");
    }
    Ok(())
}

// ------------------------------------------------------------- empreinte ---
/// Hashes déterministes (FNV-1a 64, std-only) des manifestes présents — la
/// partie **bon marché** de l'empreinte, rejouée à chaque `run`.
pub fn manifest_hashes() -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for (file, _) in MANIFESTS {
        if let Ok(bytes) = std::fs::read(file) {
            out.insert((*file).to_string(), fnv1a(&bytes));
        }
    }
    out
}

fn fnv1a(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Empreinte réelle = celle des manifestes **au moment du run**.
fn manifests_now() -> std::collections::BTreeMap<String, String> {
    manifest_hashes()
}

/// Vérifie la fraîcheur du grounding d'une spec avant exécution :
///
/// - `fingerprint.json` absent → avertissement (projets antérieurs à
///   l'empreinte), le run continue ;
/// - manifestes dérivés (contenu, présence, disparition) → échec typé
///   `grounding_failure` — attribution amont : re-jouer le grounding de
///   l'étape — AVANT tout appel LLM.
pub fn check_freshness(slug: &str) -> Result<()> {
    let fp_path = Path::new("opendomain/harness/grounding").join(slug).join("fingerprint.json");
    let Ok(raw) = std::fs::read_to_string(&fp_path) else {
        eprintln!(
            "! [grounding] empreinte absente pour {slug} ({}) — relancer `opendomain ground {slug}` ; le run continue sans garde-fou de fraîcheur",
            fp_path.display()
        );
        return Ok(());
    };
    let now = manifests_now();
    let stored: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("empreinte illisible ({}) : {e}", fp_path.display()))?;
    let old: std::collections::BTreeMap<String, String> = stored
        .get("manifests")
        .and_then(serde_json::Value::as_object)
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|h| (k.clone(), h.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let mut derive: Vec<String> = Vec::new();
    for (file, hash) in &now {
        if old.get(file).map(|h| h != hash).unwrap_or(true) {
            derive.push(format!("{file} modifié"));
        }
    }
    for file in old.keys() {
        if !now.contains_key(file) {
            derive.push(format!("{file} disparu"));
        }
    }
    for file in now.keys() {
        if !old.contains_key(file) {
            derive.push(format!("{file} apparu"));
        }
    }
    if derive.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "grounding_failure : le dépôt a dérivé depuis `ground {slug}` — {}\n\
  → attribution : amont — re-jouer le grounding de l'étape : `opendomain ground {slug}`, puis relancer",
            derive.join(" ; ")
        ))
    }
}

/// Noms de premier niveau de l'arborescence (profil fixe de l'empreinte).
fn tree_names(dir: &Path, max_depth: usize) -> Vec<String> {
    tree_inner(dir, 0, max_depth)
}

/// Dossiers jamais décrits par l'empreinte : artefacts de build, caches,
/// métadonnées de VCS. Une seule définition, deux usages — la skip-list
/// dupliquée en deux constantes locales finissait par diverger.
const SKIP: &[&str] = &["target", "node_modules", ".git", "dist", ".venv", "__pycache__"];

fn tree_inner(dir: &Path, depth: usize, max_depth: usize) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        out.push(format!("{}{}", "  ".repeat(depth), name));
        if entry.path().is_dir() && depth + 1 <= max_depth {
            out.extend(tree_inner(&entry.path(), depth + 1, max_depth));
        }
    }
    out
}

/// Présence de la toolchain, partie descriptive de l'empreinte.
fn toolchain_present() -> std::collections::BTreeMap<String, bool> {
    ["cargo", "node", "npm", "python", "git"]
        .iter()
        .map(|bin| ((*bin).to_string(), which(bin).is_some()))
        .collect()
}

/// Profil d'arborescence jusqu'à `max_depth`.
fn walk_outline(dir: &Path, depth: usize, max_depth: usize) -> String {
    let mut out = String::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        let is_dir = entry.path().is_dir();
        out.push_str(&format!("{}- {}\n", "  ".repeat(depth), name));
        if is_dir && depth + 1 <= max_depth {
            out.push_str(&walk_outline(&entry.path(), depth + 1, max_depth));
        }
    }
    out
}

fn head(text: &str, lines: usize) -> String {
    let mut out: Vec<&str> = text.lines().take(lines).collect();
    if out.len() == lines {
        out.push("…");
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_deterministe_et_sensible_au_contenu() {
        assert_eq!(fnv1a(b"abc"), fnv1a(b"abc"), "FNV-1a stable");
        assert_ne!(fnv1a(b"abc"), fnv1a(b"abd"), "1 bit de contenu → autre empreinte");
        assert_eq!(fnv1a(b"").len(), 16, "empreinte hexadécimale 64 bits");
    }

    #[test]
    fn hashmanifestes_reflechit_cargo_toml_du_projet() {
        let hashes = manifest_hashes();
        // ce dépôt porte un Cargo.toml à sa racine — il doit figurer.
        assert!(hashes.contains_key("Cargo.toml"), "manifestes : {hashes:?}");
    }
}
