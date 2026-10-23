//! ======================================================================
//! Le scaffold du harnais — l'écrivain unique de `opendomain/harness/` et
//! des deux contrats du méta-agent.
//!
//! Un seul arbre de templates (`templates/harness-*`), une seule liste plate,
//! un seul passage : les deux contrats (OpenCode **et** Claude Code) sont
//! générés d'un bloc, sans commutateur — les dossiers `.claude/` et
//! `.opencode/` sont des espaces de noms, pas des choix ; chaque assistant
//! lit le sien et ignore l'autre. Idempotent : rien d'existant n'est jamais
//! réécrit.
//! ======================================================================
use anyhow::Result;

type Template = (&'static str, &'static str);

/// Tout ce que le harnais génère, en une liste plate, un seul pattern de
/// nommage (`harness-*`) :
///
/// 1. l'arborescence `opendomain/harness/` (README, config fermée, specs,
///    contrats, agents, grounding, graphe, runs, mémoire) ;
/// 2. le contrat méta-agent **pour OpenCode** (`.opencode/agent/`) ;
/// 3. le contrat méta-agent **pour Claude Code** (`.claude/agents/`).
pub const FILES: &[Template] = &[
    (
        "opendomain/harness/README.md",
        include_str!("../../templates/harness-README.md"),
    ),
    (
        "opendomain/harness/specs/README.md",
        include_str!("../../templates/harness-specs-README.md"),
    ),
    (
        "opendomain/harness/specs/000-exemple/spec.md",
        include_str!("../../templates/harness-spec-exemple.md"),
    ),
    (
        "opendomain/harness/contracts/README.md",
        include_str!("../../templates/harness-contracts-README.md"),
    ),
    (
        "opendomain/harness/agents/README.md",
        include_str!("../../templates/harness-agents-README.md"),
    ),
    (
        "opendomain/harness/grounding/README.md",
        include_str!("../../templates/harness-grounding-README.md"),
    ),
    (
        "opendomain/harness/graph/graph.json",
        include_str!("../../templates/harness-graph.json"),
    ),
    (
        "opendomain/harness/runs/README.md",
        include_str!("../../templates/harness-runs-README.md"),
    ),
    (
        "opendomain/harness/memory/README.md",
        include_str!("../../templates/harness-memory-README.md"),
    ),
    // Pas de .gitignore pour le harnais lui-même : la politique de versioning
    // est portée PAR les arbres générés — `opendomain/.gitignore` (régistries,
    // harnais, runs) et `.opencode/.gitignore` (contrat d'agent), écrits par
    // le scaffold de `init` comme les autres fichiers. Le .gitignore du
    // PROJET n'est jamais touché.
    // Pas de contrat méta-agent : le harnais se pilote par les commandes
    // (/opendomain-implement → rôles → skills). La doctrine d'exécution vit
    // UNE fois, dans le skill `opendomain-harness` — aucune copie par
    // plateforme, donc aucune divergence possible.
];

/// Gabarit d'une nouvelle spec (`opendomain spec new <slug>`).
const SPEC_SAMPLE: &str = include_str!("../../templates/harness-spec-sample.md");

/// Génère **tout** le harnais, d'un bloc : les 10 fichiers d'arborescence
/// et les deux contrats du méta-agent. Les critères `C_i` n'ont pas de
/// fichier à part : ils vivent dans `graph.json`, à côté du contrat de
/// chaque nœud — une seule source de vérité. Idempotent : ne réécrit
/// jamais un fichier existant.
pub fn run(root: &str) -> Result<()> {
    let base = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&base)?;

    for (rel, content) in FILES {
        write_tpl(&base, rel, content)?;
    }

    println!("\nOK OpenDomain initialisé dans `{}` (harnais complet - pilotage par les commandes, doctrine dans le skill)", base.display());
    println!("  1. lire  opendomain/harness/graph/graph.json   — l'exemple exécuté (README)");
    println!("  2. tester `opendomain validate`        — DAG + contrats vérifiés");
    println!("  3. créer une spec : `opendomain spec new 001-ma-spec`");
    Ok(())
}

/// `opendomain spec new <slug>` — une spec = un dossier (traceable, append-only).
pub fn spec_new(slug: &str) -> Result<()> {
    spec_new_in(".", slug)
}

/// Variante ancrée sur une racine explicite (testable sans
/// `set_current_dir`, qui est un état de processus et casserait les tests
/// en parallèle).
pub fn spec_new_in(root: &str, slug: &str) -> Result<()> {
    anyhow::ensure!(
        !slug.is_empty(),
        "slug requis : `opendomain spec new 001-ma-spec`"
    );
    let dir = std::path::PathBuf::from(root)
        .join("opendomain/harness/specs")
        .join(slug);
    let spec_file = dir.join("spec.md");
    if spec_file.exists() {
        anyhow::bail!("existe déjà : {}", spec_file.display());
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&spec_file, SPEC_SAMPLE)?;
    println!(
        "OK {} créé\n  → remplir Objectif / Contraintes / Critères, puis `opendomain plan {slug}` (M1)",
        spec_file.display()
    );
    Ok(())
}

/// Écrit un gabarit sans jamais écraser un fichier existant (idempotent).
fn write_tpl(base: &std::path::Path, rel: &str, content: &str) -> Result<()> {
    let dest = base.join(rel);
    if dest.exists() {
        eprintln!("  = {rel} (sauté : existe déjà)");
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dest, content)?;
    println!("  + {rel}");
    Ok(())
}

/// Tout ce que `opendomain init` écrit côté harnais — la liste complète,
/// contrats compris — pour le `--dry-run`. Le dry-run prédit exactement
/// ce que le passage réel écrira : prédire moins, c'est mentir.
pub fn preview_targets() -> Vec<&'static str> {
    FILES.iter().map(|(rel, _)| *rel).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dépôt cible jetable, unique par étiquette et par processus.
    fn temp(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("opd-hscaf-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn tous_les_gabarits_sont_non_vides() {
        for (rel, content) in FILES {
            assert!(!content.trim().is_empty(), "gabarit vide : {rel}");
            assert!(!rel.contains('\\'), "séparateur POSIX requis : {rel}");
        }
        assert!(!SPEC_SAMPLE.trim().is_empty());
    }

    #[test]
    fn tout_est_genere_dun_bloc_sans_persona_ni_doublon() {
        let dir = temp("all");
        run(dir.to_str().unwrap()).unwrap();
        assert!(dir.join("opendomain/harness/README.md").exists());
        // Pas de config harness séparée : les réglages vivent dans la
        // config générale (`opendomain/opendomain.toml`, sections
        // `[gates]`/`[sandbox]`), une seule source de vérité.
        assert!(!dir.join("opendomain/harness/config.opendomain.toml").exists());
        assert!(dir.join("opendomain/harness/graph/graph.json").exists());
        // Pas de contrat méta-agent, dans AUCUN espace de noms : le harnais
        // se pilote par les commandes → rôles → skills ; la doctrine vit une
        // fois, dans le skill `opendomain-harness`.
        assert!(!dir.join(".opencode/agent/opendomain-harness.md").exists());
        assert!(!dir.join(".claude/agents/opendomain-harness.md").exists());
        // Idempotence : un second passage ne réécrit rien et n'échoue pas.
        run(dir.to_str().unwrap()).unwrap();
        assert!(dir.join("opendomain/harness/README.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn le_dry_run_predit_exactement_le_passage_reel() {
        let targets = preview_targets();
        assert_eq!(targets.len(), FILES.len());
        // Tout chemin prédit est un chemin réel du harnais — et rien ne
        // prédit de persona méta-agent.
        for t in &targets {
            assert!(t.starts_with("opendomain/harness/"), "{t}");
            assert!(!t.contains("harnais"), "{t}");
        }
    }

    #[test]
    fn spec_new_ecrit_une_spec_et_refuse_les_doublons() {
        let dir = temp("spec");
        spec_new_in(dir.to_str().unwrap(), "001-demo").unwrap();
        assert!(dir.join("opendomain/harness/specs/001-demo/spec.md").exists());
        assert!(
            spec_new_in(dir.to_str().unwrap(), "001-demo").is_err(),
            "un doublon est refusé"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

