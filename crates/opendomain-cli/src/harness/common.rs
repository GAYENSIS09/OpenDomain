//! Résolution des chemins de spec + petits utilitaires partagés.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context as _, Result};
use opendomain_core::harness::AgentSpec;

/// Une spec résolue : son slug (ex. `001-ma-spec`), son dossier, son `spec.md`.
pub struct SpecPaths {
    pub slug: String,
    pub dir: PathBuf,
    pub spec: PathBuf,
}

/// Accepte un identifiant `TASK-###` (le registre **est** la tâche : la spec
/// est générée en vue, une fois, avec sa provenance), un slug (`001-ma-spec`),
/// un dossier (`opendomain/harness/specs/001-ma-spec`) ou un fichier
/// (`.../spec.md`) — ce que tape naturellement un humain.
pub fn resolve_spec(input: &str) -> Result<SpecPaths> {
    resolve_spec_in(".", input)
}

pub fn resolve_spec_in(root: &str, input: &str) -> Result<SpecPaths> {
    let clean = input.trim().trim_matches('"');

    // 0) TASK-### : le harnais se déclenche sur le PLAN des registres —
    //    l'enregistrement de tâche est la description de tâche `T`.
    if let Ok(id) = opendomain_core::Id::parse(clean) {
        if id.kind() == opendomain_core::IdKind::Task {
            return task_spec_paths(root, &id, clean);
        }
    }

    let direct = Path::new(clean);

    // 1) chemin vers spec.md
    if direct.is_file() && direct.file_name().map(|f| f == "spec.md").unwrap_or(false) {
        let dir = direct.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let slug = dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "spec".into());
        return Ok(SpecPaths {
            slug,
            dir,
            spec: direct.to_path_buf(),
        });
    }

    // 2) chemin vers un dossier de spec
    if direct.is_dir() {
        let spec = direct.join("spec.md");
        if spec.exists() {
            let slug = direct
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .ok_or_else(|| anyhow!("dossier sans nom : {input}"))?;
            return Ok(SpecPaths {
                slug,
                dir: direct.to_path_buf(),
                spec,
            });
        }
    }

    // 3) slug simple : opendomain/harness/specs/<slug>/spec.md
    let guessed = Path::new("opendomain/harness/specs").join(clean);
    let spec = guessed.join("spec.md");
    if spec.exists() {
        return Ok(SpecPaths {
            slug: clean.to_string(),
            dir: guessed,
            spec,
        });
    }

    Err(anyhow!(
        "spec introuvable pour `{input}`\n  → essayé : {clean}/spec.md, opendomain/harness/specs/{clean}/spec.md\n  → créer : opendomain spec new {clean}"
    ))
}

/// La voie TASK : l'enregistrement du registre **est** la tâche `T`.
///
/// La spec est une *vue* générée une fois (titre + prose de l'enregistrement
/// + provenance) : le harnais se déclenche sur le PLAN, pas sur un
/// redoublement de spec.
///
/// Idempotent : une vue existante est utilisée telle quelle (les mains de
/// l'agent restent dessus) — la régénération est explicite (supprimer le
/// fichier).
fn task_spec_paths(
    root: &str,
    id: &opendomain_core::Id,
    input: &str,
) -> Result<SpecPaths> {
    let ws = opendomain_core::Workspace::discover(Path::new(root))
        .with_context(|| format!("TASK {input} : aucun workspace OpenDomain depuis {root}"))?;
    let index = ws.load().context("index du workspace illisible")?;
    let record = index
        .get(id)
        .ok_or_else(|| anyhow!("TASK {input} introuvable dans les registres — l'agent de code décompose des tâches enregistrées, jamais inventées"))?;

    let slug = id.to_string().to_lowercase().replace('_', "-");
    let dir = Path::new(SPEC_DIR).join(&slug);
    let spec_file = dir.join("spec.md");
    if !spec_file.exists() {
        let upstream: Vec<String> = record.upstream().iter().map(|i| i.to_string()).collect();
        let mut text = String::new();
        text.push_str(&format!(
            "# {id} — {}\n\n",
            record.title
        ));
        text.push_str(&format!(
            "<!-- generated from the registries: the task record {id} is the task description T. \
The harness plan compiles from this view; hand edits stay until a regen. -->\n\n"
        ));
        text.push_str(&format!(
            "## T (task description)\n\n{}{}\n\n",
            record.title,
            if record.body.trim().is_empty() {
                String::new()
            } else {
                format!("\n\n{}", record.body.trim())
            }
        ));
        if !upstream.is_empty() {
            text.push_str(&format!(
                "## Provenance\n\nderived-from : {}\n",
                upstream.join(", ")
            ));
        }
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&spec_file, text)?;
        println!("  + {} (vue générée depuis le registre — la tâche vit dans les registres)", spec_file.display());
    }
    Ok(SpecPaths {
        slug,
        dir,
        spec: spec_file,
    })
}

/// Racine des specs (utilisée par `spec new` et les workflows).
pub const SPEC_DIR: &str = "opendomain/harness/specs";

// ------------------------------------------------------------- complétude ---
/// Complétude d'un agent. La politique vit dans le **moteur d'admission** de
/// core (`harness::admission`) — un seul contrôle, partagé par `plan`,
/// `build`, `run --graph` et `repatch` ; ce wrapper historique reste pour les
 /// appels CLI qui ne portent pas la décision d'admission (rapport de build).
pub fn agent_completeness(spec: &AgentSpec) -> Vec<(String, bool, String)> {
    opendomain_core::harness::completeness(spec)
}

/// Aiguilles d'un agent, toutes provenances confondues (`contains` brutes +
/// bornées). Utilisé pour signaler les critères auto-satisfaisables.
pub fn criteria_needles(spec: &AgentSpec) -> Vec<String> {
    let mut out: Vec<String> = spec.criteria.contains.clone();
    for needles in spec.criteria.contains_in.values() {
        out.extend(needles.iter().cloned());
    }
    out
}

// ------------------------------------------------------------------ sandbox -
/// Politique d'isolation des exécutions (pillar « sandboxes » d'un runtime
/// d'agent) : une allowlist de binaires pour les commandes de critères.
///
/// Chargée depuis la config **générale** du workspace (`opendomain.toml`) —
/// une seule config, pas de fichier harness séparé à faire diverger :
///
/// ```toml
/// [sandbox]
/// enabled = true
/// allow  = ["cargo", "npm", "pytest"]
/// ```
///
/// Honnêteté : c'est un bac à sable **logique** (allowlist + cwd borné au
/// dépôt), pas une isolation OS — un conteneur reste le seul vrai muret.
pub struct Sandbox {
    pub enabled: bool,
    pub allow: Vec<String>,
}

impl Sandbox {
    /// Le binaire est-il autorisé par l'allowlist ?
    /// Les sous-commandes sont déjà séparées avant l'appel (`cargo test` →
    /// binaire `cargo`).
    pub fn permits(&self, bin: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let bare = Path::new(bin)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| bin.to_string());
        if self.allow.iter().any(|a| a == &bare) {
            Ok(())
        } else {
            Err(anyhow!(
                "SandboxViolation : `{bare}` n'est pas dans l'allowlist {:?} — \
ajouter au [sandbox] allow de opendomain/opendomain.toml si légitime",
                self.allow
            ))
        }
    }

    /// Depuis une config parsée par le parser **général** de core — un seul
    /// parser du format (`Config::parse`/`Config::load`), jamais deux
    /// lectures divergentes du même fichier.
    pub fn from_config(cfg: &opendomain_core::config::Config) -> Self {
        Sandbox { enabled: cfg.sandbox.enabled, allow: cfg.sandbox.allow.clone() }
    }
}

/// Lit la sandbox du projet courant, depuis `opendomain.toml`.
///
/// Fail-loud conservé : si le workspace existe mais refuse de parser,
/// l'erreur se propage — une sandbox *silencieusement* désactivée ressemble
/// à un contournement. Hors workspace (usage hors projet, tests) : désactivée.
pub fn load_sandbox() -> Result<Sandbox> {
    if Path::new(opendomain_core::harness::WORKSPACE_CONFIG).exists() {
        let cfg = opendomain_core::config::Config::load(Path::new("opendomain"))?;
        return Ok(Sandbox { enabled: cfg.sandbox.enabled, allow: cfg.sandbox.allow });
    }
    Ok(Sandbox { enabled: false, allow: Vec::new() })
}

// ---------------------------------------------------------------- bindings -
/// Liaisons d'exécution projetées dans le frontmatter de chaque agent de
/// nœud à la construction (`opendomain build`) : le modèle épinglé
/// (`model: provider/model#variant`) et le budget de pas (`steps:` — au
/// dernier pas, OpenCode retire les outils et demande un résumé, que le
/// gate juge comme toute sortie). Source unique : la section `[harness]` de
/// la config générale ; sans `model` configuré, la variable `OPENCODE_MODEL`
/// est lue à la construction — l'épinglage par exécution, sans éditer la
/// config. Hors workspace : les défauts (aucun modèle épinglé, 40 pas).
pub struct NodeBindings {
    pub model: Option<String>,
    pub steps: Option<u32>,
}

/// Lit les liaisons `[harness]` du projet courant. Fail-loud comme la
/// sandbox : une config qui refuse de parser se propage — jamais de lecture
/// silencieuse qui ressemble à un contournement.
pub fn load_harness_bindings() -> Result<NodeBindings> {
    let harness = if Path::new("opendomain/opendomain.toml").exists() {
        opendomain_core::config::Config::load(Path::new("opendomain"))?.harness
    } else {
        <opendomain_core::config::Config as Default>::default().harness
    };
    let model = harness
        .model
        .or_else(|| std::env::var("OPENCODE_MODEL").ok())
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    Ok(NodeBindings { model, steps: harness.steps })
}

// --------------------------------------------------------------- chemins ---
/// DAG : archive `<slug>.json` + graphe actif `graph.json`.
pub const GRAPH_DIR: &str = "opendomain/harness/graph";
/// Archive des runs — append-only, `runs/run-NNN-<slug>/`.
pub const RUNS_DIR: &str = "opendomain/harness/runs";
/// Mémoire des leçons typées (M4) — dossier ; `registre.md` vit dedans.
pub const MEMORY_DIR: &str = "opendomain/harness/memory";

// ------------------------------------------------------- gouvernance de boucle -
/// `[gates]` de la config générale : `strict` + `max_attempts` — la config
/// **gouverne** la boucle du harnais (run, plan LLM, révision `--llm`) ; il
/// n'y a pas de second fichier de réglages.
pub struct LoopCfg {
    pub strict: bool,
    pub max_attempts: u32,
}

impl Default for LoopCfg {
    fn default() -> Self {
        Self { strict: true, max_attempts: opendomain_core::harness::MAX_ATTEMPTS }
    }
}

/// Lit la config `[gates]` du projet — l'appelant la branche dans le runner.
/// Fail-loud conservé : un fichier présent qui refuse de parser se propage
/// (un harnais ne devine pas ses réglages) ; hors workspace : les défauts du
/// moteur (`MAX_ATTEMPTS`, strict).
pub fn loop_config() -> Result<LoopCfg> {
    if std::path::Path::new(opendomain_core::harness::WORKSPACE_CONFIG).exists() {
        let cfg = opendomain_core::config::Config::load(std::path::Path::new("opendomain"))?;
        return Ok(LoopCfg { strict: cfg.gates.strict, max_attempts: cfg.gates.max_attempts });
    }
    Ok(LoopCfg::default())
}

/// Premier titre `# Titre` d'un Markdown.
pub fn title_of(text: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim_start().starts_with("# ") && !l.starts_with("## "))
        .map(|l| l.trim_start().trim_start_matches('#').trim().to_string())
}

/// Lignes à puces d'une section `## <motclé>` (jusqu'au prochain titre),
/// dépliées (`- x` → `x`), sans lignes vides, plafonnées à `max`.
pub fn section_lines(text: &str, keyword: &str, max: usize) -> Vec<String> {
    let wanted = keyword.to_lowercase();
    let mut out = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            in_section = t.to_lowercase().contains(&wanted);
            continue;
        }
        if in_section && !t.is_empty() {
            let item = t.trim_start_matches("- ").trim_start_matches("* ").to_string();
            if !item.is_empty() {
                out.push(item);
                if out.len() >= max {
                    break;
                }
            }
        }
    }
    out
}

/// Tokens entre backticks apparaissant dans des lignes marquées « interdit »
/// (les contraintes nourrissent directement `criteria.not_contains`).
pub fn extract_forbidden(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for l in lines {
        // « interdi* » couvre interdit / interdiction / interdire / interdis.
        if !l.to_lowercase().contains("interdi") {
            continue;
        }
        for (i, seg) in l.split('`').enumerate() {
            if i % 2 == 1 && !seg.trim().is_empty() {
                out.push(seg.trim().to_string());
            }
        }
    }
    out
}

/// Détection de la langue cible, sans dépendance : marqueurs forts (fences,
/// manifestes), puis mots-clés faibles (« stack: rust » dans les contraintes).
pub fn detect_language(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("```python") || t.contains("pyproject.toml") || t.contains("requirements.txt") {
        "python"
    } else if t.contains("```rust") || t.contains("cargo.toml") {
        "rust"
    } else if t.contains("```typescript") || t.contains("```javascript") || t.contains("package.json")
    {
        "javascript"
    } else if t.contains("```go") || t.contains("golang") {
        "go"
    } else if t.contains("python") {
        "python"
    } else if t.contains("rust") || t.contains("cargo") {
        "rust"
    } else if t.contains("typescript") || t.contains("javascript") {
        "javascript"
    } else {
        "cible"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_et_interdits() {
        let text = "# T\n\n## Contraintes\n- stack: python\n- interdiction des imports tiers `numpy` et pas de `eval(`\n\n## Critères\n- [ ] tests verts\n";
        assert_eq!(title_of(text).as_deref(), Some("T"));
        let lines = section_lines(text, "Contraintes", 10);
        assert_eq!(lines.len(), 2);
        let forbidden = extract_forbidden(&lines);
        assert_eq!(forbidden, vec!["numpy", "eval("]);
        assert_eq!(section_lines(text, "Critères", 10), vec!["[ ] tests verts"]);
        assert_eq!(detect_language(text), "python");
    }

    #[test]
    fn section_absente() {
        assert!(section_lines("# x\nblabla\n", "Critères", 5).is_empty());
    }

    #[test]
    fn sandbox_allowlist() {
        let cfg = Sandbox {
            enabled: true,
            allow: vec!["cargo".into(), "npm".into()],
        };
        assert!(cfg.permits("cargo").is_ok());
        assert!(cfg.permits("npm").is_ok());
        assert!(cfg.permits("curl http://x").is_err(), "binaire hors liste");
        // désactivée → tout passe (décision explicite, jamais accidentelle)
        let off = Sandbox { enabled: false, allow: Vec::new() };
        assert!(off.permits("curl http://x").is_ok());
    }

    #[test]
    fn sandbox_depuis_le_parser_general_de_la_config() {
        // Le seul parser est celui de core (`Config::parse`) — le parser
        // manuscrit de common est supprimé (S2) : une seule loi du format.
        use opendomain_core::config::Config;
        let on = Config::parse("version = 1\n\n[sandbox]\nenabled = true\nallow = [\"cargo\", \"rustc\"]\n")
            .map(|c| Sandbox::from_config(&c))
            .expect("config simple doit parser");
        assert!(on.enabled);
        assert_eq!(on.allow, vec!["cargo".to_string(), "rustc".to_string()]);
        let absent = Config::parse("version = 1\n")
            .map(|c| Sandbox::from_config(&c))
            .expect("config sans sandbox doit parser");
        // Défaut du **moteur** : une section `[sandbox]` absente hérite du
        // réglage par défaut de la config générale (enabled = true + sa
        // allowlist). L'ancien parser manuscrit « section absente →
        // désactivée » était une divergence silencieuse; S2 l'élimine.
        assert!(absent.enabled);
        assert_eq!(absent.allow, opendomain_core::config::Config::default().sandbox.allow);
    }
}
