//! Modularité du harnais : registre de composants (skills, tools, serveurs
//! MCP, exécuteurs) + résolution minimale **sans redondance**.
//!
//! Un composant déclare ce qu'il **fournit** (`provides` — des ids de
//! capacité comme `design.revue`), ce qu'il **exige** (`requires` — bins,
//! env, fichiers) et où il vit (`source`). La spec exprime des **besoins**
//! (section `## Besoins` de `spec.md`). `opendomain resolve <slug>` associe à
//! chaque besoin exactement un fournisseur — préférence : `preferred` →
//! source locale → distante (le principe de localité) — puis vérifie
//! la fermeture des exigences contre la machine.
//!
//! Serveurs MCP / tools : déclarés dans `components.json` à la main (le
//! pont d'entrée modulaire) ; l'exécuteur réel qui les Aureva naît en M6.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use opendomain_core::harness::AgentSpec;
use serde_json::Value;

/// Chemin du registre dans le projet cible.
pub const REGISTRY_PATH: &str = "opendomain/harness/components.json";
/// Dossier d'installation des skills locales.
pub const SKILLS_DIR: &str = "opendomain/harness/skills";

// -------------------------------------------------------------- registre ---

/// Un composant du harnais — l'unité de modularité.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Component {
    pub id: String,
    /// skill | tool | mcp | executor | validator
    pub kind: String,
    /// Capacités rendues (ids exacts, ex. `design.revue`).
    pub provides: Vec<String>,
    /// Exigences vérifiées sur la machine.
    pub requires: Requires,
    /// `local` (fichier dispo) ou `remote` (url / branchement externe).
    pub source_kind: String,
    /// Fichier local (skill) ou url / désignateur.
    pub source: String,
    /// L'ORIGINE d'installation (url ou chemin déclaré) — jamais écrasée par
    /// le chemin local : une capacité installée du web reste traçable.
    pub source_url: Option<String>,
    /// Version déclarée par le composant, si elle l'est.
    pub version: Option<String>,
    /// Date d'installation (YYYY-MM-DD) — la provenance a une date.
    pub installed_at: Option<String>,
    /// Cycle de vie : `discovered` (trouvé) → `validated` (checks passés)
    /// → `activated` (autorisé à l'usage). Défaut : `activated` — le
    /// comportement historique d'un registre sans cycle de vie.
    pub state: String,
    /// Nœuds auxquels la skill s'attache (ids du graphe, ou `*`).
    pub applies_to: Vec<String>,
    /// Retenu en priorité par le résolveur.
    pub preferred: bool,
}

/// Les trois états du cycle de vie d'un composant.
pub const STATES: &[&str] = &["discovered", "validated", "activated"];

/// Le défaut d'état : `activated` — un registre écrit avant la mécanique de
/// cycle de vie reste utilisable tel quel.
pub fn default_state() -> String {
    "activated".to_string()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Requires {
    pub bins: Vec<String>,
    pub env: Vec<String>,
    pub files: Vec<String>,
}

impl Requires {
    fn from_json(v: Option<&Value>) -> Requires {
        let Some(v) = v else { return Requires::default() };
        Requires {
            bins: str_list(v.get("bins")),
            env: str_list(v.get("env")),
            files: str_list(v.get("files")),
        }
    }

    /// Texte lisible des exigences.
    pub fn brief(&self) -> String {
        let mut out = Vec::new();
        if !self.bins.is_empty() {
            out.push(format!("bins: {:?}", self.bins));
        }
        if !self.env.is_empty() {
            out.push(format!("env: {:?}", self.env));
        }
        if !self.files.is_empty() {
            out.push(format!("files: {:?}", self.files));
        }
        if out.is_empty() { "aucune".into() } else { out.join(", ") }
    }
}

/// Le registre entier (miroir de `components.json`).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    pub components: Vec<Component>,
}

impl Registry {
    pub fn load(path: &str) -> Result<Registry> {
        let raw = std::fs::read_to_string(path).with_context(|| {
            format!(
                "registre `{path}` introuvable — créer via `opendomain skill new <id>` ou `opendomain skill add <url|chemin>`"
            )
        })?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| anyhow!("{path} illisible : {e}"))?;
        let mut out = Registry::default();
        let Some(arr) = v.get("components").and_then(Value::as_array) else {
            return Ok(out);
        };
        for c in arr {
            out.components.push(Component {
                id: c.get("id").and_then(Value::as_str).unwrap_or("").into(),
                kind: c.get("kind").and_then(Value::as_str).unwrap_or("skill").into(),
                provides: str_list(c.get("provides")),
                requires: Requires::from_json(c.get("requires")),
                source_kind: c
                    .get("source")
                    .and_then(|s| s.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("local")
                    .into(),
                source: c
                    .get("source")
                    .and_then(|s| s.get("path_or_url"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                applies_to: str_list(c.get("applies_to")),
                preferred: c.get("preferred").and_then(Value::as_bool).unwrap_or(false),
                source_url: c
                    .get("source_url")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                version: c
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                installed_at: c
                    .get("installed_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                state: c
                    .get("state")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(default_state)
                    .into(),
            });
        }
        Ok(out)
    }

    /// Insère (écrase l'`id` existant : un index reste petit et réécrit).
    /// L'état est validé ici — une faute d'orthographe d'état est un refus
    /// typé, jamais un état fantôme dans le registre.
    pub fn upsert(&mut self, c: Component) -> Result<()> {
        anyhow::ensure!(
            STATES.contains(&c.state.as_str()),
            "état de composant inconnu : `{}` (attendu : discovered | validated | activated) — {}",
            c.state, c.id
        );
        self.components.retain(|x| x.id != c.id);
        self.components.push(c);
        Ok(())
    }

    /// Sérialisation du registre courant.
    pub fn to_json(&self) -> Result<String> {
        let comps: Vec<Value> = self
            .components
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id, "kind": c.kind,
                    "provides": c.provides,
                    "requires": {
                        "bins": c.requires.bins,
                        "env": c.requires.env,
                        "files": c.requires.files,
                    },
                    "source": { "type": c.source_kind, "path_or_url": c.source },
                    "source_url": c.source_url,
                    "version": c.version,
                    "installed_at": c.installed_at,
                    "state": c.state,
                    "applies_to": c.applies_to,
                    "preferred": c.preferred,
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "components": comps }))
            .map_err(|e| anyhow!("{e}"))
    }
}

fn str_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

// --------------------------------------------------------------- skills ---

/// Analyse le frontmatter d'une skill : (`id`, `provides`, `applies-to`).
pub fn parse_skill_frontmatter(text: &str) -> (String, Vec<String>, Vec<String>) {
    let mut id = String::new();
    let mut provides = Vec::new();
    let mut applies = Vec::new();
    let mut in_fm = false;
    let mut seen_open = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "---" {
            if seen_open {
                break;
            }
            seen_open = true;
            in_fm = true;
            continue;
        }
        if !in_fm {
            continue;
        }
        if let Some((k, v)) = t.split_once(':') {
            let val = v.trim();
            match k.trim() {
                "id" => id = val.to_string(),
                "provides" => provides = split_list(val),
                "applies-to" | "applies_to" => applies = split_list(val),
                _ => {}
            }
        }
    }
    (id, provides, applies)
}

fn split_list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Installe une skill (contenu déjà rapatrié) dans `opendomain/harness/skills/<id>.md`
/// puis met à jour le registre — l'alias historique du composant général.
pub fn install_skill(
    content: &str,
    original_name: &str,
    force_id: Option<&str>,
    force_provides: Option<Vec<String>>,
) -> Result<Component> {
    install_component(content, "skill", original_name, force_id, force_provides, None)
}

/// Retire le frontmatter d'un contenu markdown.
fn strip_frontmatter(text: &str) -> String {
    let mut lines = text.lines().peekable();
    if lines.peek().map(|l| l.trim() == "---").unwrap_or(false) {
        for l in lines.by_ref().skip(1) {
            if l.trim() == "---" {
                break;
            }
        }
    }
    lines.collect::<Vec<_>>().join("\n")
}

fn sanitize_name(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(48)
        .collect()
}

/// Gabarit d'une skill écrite par le méta-agent (`skill new`).
const SKILL_TEMPLATE: &str = "---\nid: {id}\nprovides: {id}.basique\napplies-to: *\n---\n\n\
# Skill — {id}

> Écrite par le méta-agent. Remplacer ce gabarit par la vraie procédure :
> selon le modèle, une skill est **lue comme une procédure** — claire,
> numérotée, avec ses garde-fous et ses critères de sortie.

## Objectif
<ce que la skill rend, un verbe, une attente mesurable>

## Étapes
1. <entrée : ce dont l'agent a besoin avant de démarrer>
2. <action : une étape, un résultat>
3. <vérification : comment savoir que c'est bon — un critère, une commande>

## Garde-fous
- interdiction : `X` — connaître la frontière, pas la repousser
- en cas d'ambiguïté : demander, ne pas improviser
";

/// `opendomain skill new <id>` : le méta-agent démarre une nouvelle skill.
pub fn skill_new(id_raw: &str) -> Result<()> {
    let id = sanitize_name(id_raw);
    anyhow::ensure!(!id.is_empty(), "id de skill requis (lettres/chiffres -_)");
    let path = Path::new(SKILLS_DIR).join(format!("{id}.md"));
    anyhow::ensure!(!path.exists(), "existe déjà : {}", path.display());
    if let Err(e) = Registry::load(REGISTRY_PATH) {
        // registre absent → on le crée vide ; registre présent mais cassé →
        // ne pas écraser silencieusement : l'erreur remonte.
        if registry_exists() {
            return Err(e);
        }
        std::fs::create_dir_all(Path::new(REGISTRY_PATH).parent().unwrap())?;
    }
    std::fs::create_dir_all(SKILLS_DIR)?;
    std::fs::write(&path, SKILL_TEMPLATE.replace("{id}", &id))?;
    println!("  + {}", path.display());
    let content = std::fs::read_to_string(&path)?;
    install_skill(&content, &id, Some(&id), None)?;
    println!(
        "OK skill {id} installée — éditer {} (provides, étapes), puis re-resoudre : opendomain resolve <slug>",
        path.display()
    );
    Ok(())
}

/// Le projet cible a-t-il un registre ?
fn registry_exists() -> bool {
    Path::new(REGISTRY_PATH).exists()
}

/// Charge le registre, ou en crée un vide si le fichier est absent.
pub fn registry_or_empty() -> Registry {
    Registry::load(REGISTRY_PATH).unwrap_or_default()
}

/// `opendomain resolve <slug>` : besoins de la spec → composants choisis
/// (un par besoin, aucune redondance) + vérification des exigences.
/// Écrit `opendomain/harness/agents/<slug>/resolve.md` et échoue typé si manque.
pub fn resolve_cmd(spec_input: &str) -> Result<()> {
    let sp = crate::harness::common::resolve_spec(spec_input)?;
    let spec_text = std::fs::read_to_string(&sp.spec)?;

    // des commandes de gate ? (besoin `terminal` automatique)
    let has_commands = std::fs::read_to_string(
        Path::new("opendomain/harness/graph").join(format!("{}.json", sp.slug)),
    )
    .or_else(|_| std::fs::read_to_string("opendomain/harness/graph/graph.json"))
    .ok()
    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    .and_then(|v| {
        v.get("nodes").and_then(Value::as_array).map(|nodes| {
            nodes.iter().any(|n| {
                n.get("criteria")
                    .and_then(|c| c.get("commands"))
                    .and_then(Value::as_array)
                    .map(|a| !a.is_empty())
                    .unwrap_or(false)
            })
        })
    })
    .unwrap_or(false);

    anyhow::ensure!(
        registry_exists(),
        "aucun registre de composants — créer : `opendomain skill new <id>` ou `opendomain skill add <url|chemin>`"
    );

    let needs = needs_of_spec(&spec_text, has_commands);
    let mut report = format!(
        "# Résolution des composants — {slug}\n\nRegistre : `opendomain/harness/components.json`\n\n",
        slug = sp.slug
    );
    if needs.is_empty() {
        report.push_str("Aucun besoin déclaré (section `## Besoins` de la spec).\n");
        let dir = crate::harness::adapters::agents_dir(&sp.slug);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("resolve.md"), &report)?;
        println!("OK resolve {} : aucun besoin", sp.slug);
        return Ok(());
    }
    let reg = registry_or_empty();
    let res = resolve(&reg, &needs);
    let mut any_failed = false;
    report.push_str("| besoin | composant retenu | exigences |\n|---|---|---|\n");
    for (need, comp_id) in &res.hits {
        let Some(c) = reg.components.iter().find(|c| &c.id == comp_id) else {
            continue;
        };
        let verdicts = check_requirements(c);
        let ok = verdicts.iter().all(|r| r.is_ok());
        any_failed |= !ok;
        report.push_str(&format!(
            "| {need} | `{}` ({}, {}) | {} |\n",
            c.id,
            c.kind,
            if c.source_kind == "local" {
                c.source.clone()
            } else {
                format!("branchement exécuteur : {}", c.source)
            },
            c.requires.brief()
        ));
        for r in verdicts.iter().filter(|r| r.is_err()) {
            report.push_str(&format!("- ⚠ **{need}** : {}\n", r.as_ref().unwrap_err()));
        }
    }
    report.push_str("\n| besoin sans fournisseur |\n|---|\n");
    for m in &res.misses {
        report.push_str(&format!(
            "| **{m}** — déclarer dans le registre via `opendomain skill add`/`skill new`, ou un composant manuel |\n"
        ));
    }
    report.push_str("\nResoudre autrement équivaut à improviser ; le harnais préfère l'inventaire.\n");

    let dir = crate::harness::adapters::agents_dir(&sp.slug);
    std::fs::create_dir_all(&dir)?;
    let out = dir.join("resolve.md");
    std::fs::write(&out, &report)?;
    let state = if res.misses.is_empty() && !any_failed { "PASS" } else { "À COMPLÉTER" };
    println!(
        "resolve {} : {} → {} composant(s), {} sans fournisseur ({})",
        sp.slug,
        needs.len(),
        res.chosen.len(),
        res.misses.len(),
        state
    );
    println!("  → {}", out.display());
    if res.misses.is_empty() && !any_failed {
        Ok(())
    } else {
        Err(anyhow!(
            "besoins sans fournisseur ou exigences non satisfaites — voir {out:?}"
        ))
    }
}

fn upsert_registry(c: Component) -> Result<()> {
    if !registry_exists() {
        std::fs::write(REGISTRY_PATH, "{\"components\": []}")?;
    }
    let mut reg = Registry::load(REGISTRY_PATH)?;
    reg.upsert(c)?;
    std::fs::write(REGISTRY_PATH, reg.to_json()?)?;
    Ok(())
}

// ------------------------------------------------------------- résolution ---

/// Besoins d'une spec : section `## Besoins` + `terminal` automatique quand
/// des nœuds portent des commandes de gate.
pub fn needs_of_spec(spec_text: &str, has_gate_commands: bool) -> Vec<String> {
    let mut needs = crate::harness::common::section_lines(spec_text, "Besoins", 24);
    for n in &mut needs {
        *n = n.trim_start_matches("- ").trim().to_string();
    }
    if has_gate_commands && !needs.iter().any(|n| n == "terminal") {
        needs.push("terminal".into());
    }
    needs
}

/// Le résultat de la résolution (le cœur « aucune redondance »).
pub struct Resolution {
    /// Un composant peut couvrir plusieurs besoins — il ne figure qu'ici.
    pub chosen: Vec<Component>,
    /// (besoin → composant) dans l'ordre des besoins.
    pub hits: Vec<(String, String)>,
    /// Besoins sans fournisseur.
    pub misses: Vec<String>,
}

/// Le résolveur : le moins coûteux qui couvre — un besoin = un fournisseur,
/// sauf réutilisation volontaire (le composant partagé entre plusieurs
/// besoins n'apparaît qu'une fois dans `chosen`).
pub fn resolve(registry: &Registry, needs: &[String]) -> Resolution {
    let mut chosen_idx: Vec<usize> = Vec::new();
    let mut hits: Vec<(String, String)> = Vec::new();
    let mut misses: Vec<String> = Vec::new();
    for need in needs {
        let mut cands: Vec<usize> = registry
            .components
            .iter()
            .enumerate()
            .filter(|(_, c)| c.provides.iter().any(|p| p == need))
            .map(|(i, _)| i)
            .collect();
        // préférence : preferred → local → premier déclaré
        cands.sort_by_key(|&i| {
            let c = &registry.components[i];
            (if c.preferred { 0 } else { 1 }, {
                if c.source_kind == "local" { 0 } else { 1 }
            })
        });
        match cands.first() {
            Some(&i) => {
                if !chosen_idx.contains(&i) {
                    chosen_idx.push(i);
                }
                hits.push((
                    need.clone(),
                    registry.components[i].id.clone(),
                ));
            }
            None => misses.push(need.clone()),
        }
    }
    Resolution {
        chosen: chosen_idx.into_iter().map(|i| registry.components[i].clone()).collect(),
        hits,
        misses,
    }
}

/// Vérifie les exigences d'un composant contre la machine (bins/env/fichiers).
pub fn check_requirements(c: &Component) -> Vec<Result<()>> {
    let mut out = Vec::new();
    for b in &c.requires.bins {
        out.push(if crate::harness::doctor::which(b).is_some() {
            Ok(())
        } else {
            Err(anyhow!(
                "composant `{}` : binaire `{b}` introuvable sur PATH — \
installer, ou retirer de ses `requires.bins`",
                c.id
            ))
        });
    }
    for e in &c.requires.env {
        out.push(match std::env::var(e) {
            Ok(v) if !v.trim().is_empty() => Ok(()),
            _ => Err(anyhow!(
                "composant `{}` : variable `{e}` absente/vide — exporter avant `opendomain run`",
                c.id
            )),
        });
    }
    for f in &c.requires.files {
        out.push(if Path::new(f).exists() {
            Ok(())
        } else {
            Err(anyhow!("composant `{}` : fichier `{f}` absent", c.id))
        });
    }
    out
}

/// Attachement des skills choisies aux prompts (exécuteurs LLM).
#[derive(Debug, Clone)]
pub struct SkillAttach {
    /// ids de nœud auxquels s'attacher (`*` = tous).
    pub applies_to: Vec<String>,
    /// Texte borné de la skill.
    pub text: String,
    pub id: String,
}

/// Construit les attachements pour un run : skills **locales** retenues par
/// une résolution du besoin réel de la spec.
pub fn attach_for_run(spec_text: &str, has_gate_commands: bool) -> Vec<SkillAttach> {
    if !registry_exists() {
        return Vec::new();
    }
    let reg = registry_or_empty();
    let needs = needs_of_spec(spec_text, has_gate_commands);
    if needs.is_empty() {
        return Vec::new();
    }
    let res = resolve(&reg, &needs);
    let mut out = Vec::new();
    for c in &res.chosen {
        if c.kind != "skill" || c.source_kind != "local" {
            continue;
        }
        // Honnêteté : un registre qui retient un fichier illisible ment —
        // l'avertissement sort, le run continue sans (l'état est dit, pas
        // avalé).
        let Ok(text) = std::fs::read_to_string(&c.source) else {
            eprintln!(
                "! skill `{}` : `{}` illisible — le composant est retenu par le \
registre mais son fichier n'existe pas ; réinstaller (`opendomain skill add`) \
ou retirer du registre",
                c.id, c.source
            );
            continue;
        };
        out.push(SkillAttach {
            applies_to: if c.applies_to.is_empty() { vec!["*".into()] } else { c.applies_to.clone() },
            text: bounded_text(&text, 2500),
            id: c.id.clone(),
        });
    }
    out
}

/// Bornage par caractères (troncature honnête, avec note visible).
fn bounded_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}\n… (skill tronquée — lire le fichier pour l'intégralité)")
    }
}

/// Filtre les skills applicables à un nœud (`applies-to` : id ou `*`).
pub fn applicable<'a>(attachments: &'a [SkillAttach], node_id: &str) -> Vec<&'a SkillAttach> {
    attachments
        .iter()
        .filter(|a| a.applies_to.iter().any(|p| p == "*" || p == node_id))
        .collect()
}

// ------------------------------------------------------------- commandes ---

// ------------------------------------------------------------------- kit ---
/// Dossier d'installation d'un kind de kit : `skill` → `harness/skills/`
/// (emplacement historique), les autres → `harness/kit/<kind>s/`.
pub fn kit_dir(kind: &str) -> String {
    match kind {
        "skill" => SKILLS_DIR.to_string(),
        "command" => "opendomain/harness/kit/commands".to_string(),
        "tool" => "opendomain/harness/kit/tools".to_string(),
        other => other.to_string(), // vocabulaire fermé vérifié avant appel
    }
}

/// `opendomain kit add <kind> <url|chemin> [--id X] [--provides a,b]` —
/// l'installation modulaire (moule « tout est composant ») : depuis le web
/// **avec provenance** (source + date, visible dans le fichier), ou depuis
/// le disque. Le run ne télécharge jamais en silence.
pub fn kit_add(
    kind: &str,
    source: &str,
    id_opt: Option<String>,
    provides_opt: Option<Vec<String>>,
) -> Result<Component> {
    anyhow::ensure!(
        matches!(kind, "skill" | "command" | "tool"),
        "kind de composant inconnu : `{kind}` — vocabulaire fermé : skill | command | tool"
    );
    let is_url = source.starts_with("http://") || source.starts_with("https://");
    let fetched = if is_url {
        println!("téléchargement : {source}");
        let out =
            crate::harness::adapters::run_captured("curl", &["-fsSL".into(), source.into()], 120)?;
        if !out.status_ok {
            return Err(anyhow!(
                "téléchargement échoué (curl) : {}",
                out.stderr.lines().last().unwrap_or("(inconnue)")
            ));
        }
        out.stdout
    } else {
        std::fs::read_to_string(source).map_err(|e| anyhow!("lecture de `{source}` : {e}"))?
    };
    anyhow::ensure!(
        !fetched.trim().is_empty(),
        "contenu vide — `{source}` n'est pas un composant utilisable"
    );
    let provenance = if is_url {
        Some(format!(
            "<!-- provenance : url={source} — installé le {} -->",
            today()
        ))
    } else {
        None
    };
    let content = match &provenance {
        Some(p) => insert_after_frontmatter(fetched.trim(), p),
        None => fetched,
    };
    let name = source.rsplit(['/', '\\']).next().unwrap_or(kind);
    let comp = install_component(
        &content,
        kind,
        name,
        id_opt.as_deref(),
        provides_opt,
        if is_url { Some(source.to_string()) } else { None },
    )?;
    println!(
        "OK {} {} installé ({}) — fournir : {:?}\n  → déclarer-le dans le kit du nœud (graph.json `kit`) ou les besoins de la spec, puis `opendomain build <slug>`",
        comp.kind, comp.id, comp.source_kind, comp.provides
    );
    Ok(comp)
}

/// `opendomain skill add` : alias historique de `kit add skill`.
pub fn skill_add(
    source: &str,
    id_opt: Option<String>,
    provides_opt: Option<Vec<String>>,
) -> Result<()> {
    kit_add("skill", source, id_opt, provides_opt).map(|_| ())
}

/// Date locale en `AAAA-MM-JJ` (provenance lisible).
fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Conversion civile simple (UTC) — la provenance est une note, pas un oracle.
    let days = now / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Jours de l'ère unix → date civile (algorithme de Howard Hinnant, domaine public).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Insère une ligne de provenance **après** le frontmatter (le `---` initial
/// doit rester la première ligne pour rester parsable).
fn insert_after_frontmatter(content: &str, note: &str) -> String {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");
    if first.trim() == "---" {
        let mut out = vec![first.to_string()];
        for l in lines.by_ref() {
            out.push(l.to_string());
            if l.trim() == "---" {
                break;
            }
        }
        out.push(String::new());
        out.push(note.to_string());
        out.extend(lines.map(String::from));
        out.join("\n")
    } else {
        format!("{note}\n\n{content}")
    }
}

/// Installe un composant de kit (contenu déjà rapatrié) : frontmatter
/// régénéré pour rester canonique, registre mis à jour avec provenance de
/// chemin. Le contenu installé devient `local` — le run ne retélécharge pas.
pub fn install_component(
    content: &str,
    kind: &str,
    original_name: &str,
    force_id: Option<&str>,
    force_provides: Option<Vec<String>>,
    source_url: Option<String>,
) -> Result<Component> {
    anyhow::ensure!(
        matches!(kind, "skill" | "command" | "tool"),
        "kind de composant inconnu : `{kind}` — vocabulaire fermé : skill | command | tool"
    );
    let (fm_id, fm_provides, applies) = parse_skill_frontmatter(content);
    let id = force_id
        .filter(|s| !s.trim().is_empty())
        .map(String::from)
        .or_else(|| (!fm_id.is_empty()).then(|| sanitize_name(&fm_id)))
        .unwrap_or_else(|| sanitize_name(original_name));
    if id.is_empty() {
        return Err(anyhow!("id de composant déduit vide — passer `--id <mon-id>`"));
    }
    let provides = match force_provides {
        Some(p) if !p.is_empty() => p,
        _ if !fm_provides.is_empty() => fm_provides,
        _ => {
            return Err(anyhow!(
                "le composant {id} ({kind}) ne déclare aucune capacité `provides:` — \
ajouter dans son frontmatter (ex. `provides: deploy.vercel`) ou `--provides deploy.vercel`"
            ))
        }
    };
    let rendered = strip_frontmatter(content);
    let dir = kit_dir(kind);
    std::fs::create_dir_all(&dir)?;
    let path = Path::new(&dir).join(format!("{id}.md"));
    std::fs::write(
        &path,
        format!(
            "---\nid: {id}\nkind: {kind}\nprovides: {}\napplies-to: {}\n---\n\n{}",
            provides.join(", "),
            if applies.is_empty() { "*".to_string() } else { applies.join(", ") },
            rendered.trim_start()
        ),
    )?;
    println!("  + {}", path.display());

    let comp = Component {
        id: format!("{kind}/{id}"),
        kind: kind.into(),
        provides,
        requires: Requires::default(),
        source_kind: "local".into(),
        source: path.to_string_lossy().to_string(),
        // provenance : l'origine (url du web ou None si local), la date —
        // une capacité installée du web reste traçable.
        source_url,
        version: None,
        installed_at: Some(today()),
        state: default_state(),
        applies_to: applies,
        preferred: false,
    };
    upsert_registry(comp.clone())?;
    Ok(comp)
}

/// Le vocabulaire fermé des protocoles d'accès externe — la **fermeture**
/// est vérifiée à la construction contre les outils et binaires du nœud ;
/// un protocole non déclaré est une faute de construction, jamais une
/// improvisation au run. Étendre = décision explicite (ici, puis registre).
pub fn protocol_closure(protocol: &str, spec: &AgentSpec) -> Result<()> {
    match protocol.trim().to_lowercase().as_str() {
        "http" => {
            if !spec.tools.iter().any(|t| t == "webfetch") {
                Err(anyhow!(
                    "protocole `http` : le nœud `{}` n'a pas l'outil `webfetch` dans sa \
whitelist — l'ajouter aux `tools` du graphe, sinon le protocole n'est qu'une phrase",
                    spec.id
                ))
            } else {
                Ok(())
            }
        }
        "git" => {
            if !spec.tools.iter().any(|t| matches!(t.as_str(), "bash" | "cmd" | "terminal")) {
                Err(anyhow!(
                    "protocole `git` : le nœud `{}` n'a pas d'outil shell (`bash`) dans sa \
whitelist — l'ajouter aux `tools` du graphe",
                    spec.id
                ))
            } else if crate::harness::doctor::which("git").is_none() {
                Err(anyhow!(
                    "protocole `git` : binaire `git` introuvable sur PATH — installer \
(voir `opendomain doctor`)"
                ))
            } else {
                Ok(())
            }
        }
        "npm" => {
            if !spec.tools.iter().any(|t| matches!(t.as_str(), "bash" | "cmd" | "terminal")) {
                Err(anyhow!(
                    "protocole `npm` : le nœud `{}` n'a pas d'outil shell (`bash`) dans sa \
whitelist — l'ajouter aux `tools` du graphe",
                    spec.id
                ))
            } else if crate::harness::doctor::which("node").is_none() {
                Err(anyhow!(
                    "protocole `npm` : binaire `node` introuvable sur PATH — installer \
(voir `opendomain doctor`)"
                ))
            } else {
                Ok(())
            }
        }
        other => Err(anyhow!(
            "protocole inconnu : `{other}` — vocabulaire fermé du harnais : http, git, npm \
(étendre le vocabulaire : décision explicite dans le code, jamais un run qui improvise)"
        )),
    }
}

/// Brief lisible d'un protocole fermé (projeté dans l'agent de nœud).
fn protocol_brief(protocol: &str) -> String {
    let brief: &str = match protocol.trim().to_lowercase().as_str() {
        "http" => "`http` : accès externe via l'outil `webfetch` — sources citées (url + \
date), jamais improvisées ; le run ne télécharge pas, il consulte",
        "git" => "`git` : opérations de versionnement via shell — le binaire `git` est \
vérifié à la construction",
        "npm" => "`npm` : écosystème Node via shell — `node`/`npm` vérifiés à la construction",
        _ => "",
    };
    brief.to_string()
}

/// Le kit d'un nœud, **résolu** à la construction : ids réels, textes réels.
#[derive(Debug, Clone, Default)]
pub struct NodeKit {
    pub node: String,
    /// skills résolues — attachées au prompt du nœud (`applies_to = [node]`).
    pub skills: Vec<SkillAttach>,
    /// commandes résolues — (id, texte borné), briefées dans l'agent de nœud.
    pub commands: Vec<(String, String)>,
    /// protocoles fermés — briefs projetés dans l'agent de nœud.
    pub protocols: Vec<String>,
}

/// Recherche un composant par **capacité** (`provides`) ou par id, dans un
/// kind donné — préférence : `preferred` → local → premier déclaré.
fn find_component(reg: &Registry, kind: &str, need: &str) -> Option<Component> {
    let mut cands: Vec<&Component> = reg
        .components
        .iter()
        .filter(|c| {
            c.kind == kind && (c.id == need || c.provides.iter().any(|p| p == need))
        })
        .collect();
    cands.sort_by_key(|c| {
        (
            if c.preferred { 0 } else { 1 },
            if c.source_kind == "local" { 0 } else { 1 },
        )
    });
    cands.first().map(|c| (*c).clone())
}

/// Texte d'un composant : local → lire ; distante → rapatriement **explicite**
/// (`--fetch`) avec provenance et installation locale, sinon erreur typée.
fn component_text(c: &Component, fetch: bool) -> Result<String> {
    match c.source_kind.as_str() {
        "local" => std::fs::read_to_string(&c.source).map_err(|e| {
            anyhow!("composant `{}` : lecture de `{}` : {e}", c.id, c.source)
        }),
        _ => {
            if !fetch {
                return Err(anyhow!(
                    "composant `{}` : source distante `{}` — l'installer explicitement \
(`opendomain kit add {} <url>`) ou autoriser le rapatriement à la construction : \
`opendomain build --fetch`",
                    c.id,
                    c.source,
                    c.kind
                ));
            }
            println!("rapatriement : {} ← {}", c.id, c.source);
            let out =
                crate::harness::adapters::run_captured("curl", &["-fsSL".into(), c.source.clone()], 120)?;
            if !out.status_ok {
                return Err(anyhow!(
                    "rapatriement échoué (curl) : {}",
                    out.stderr.lines().last().unwrap_or("(inconnue)")
                ));
            }
            let provenance = format!(
                "<!-- provenance : url={} — rapatrié le {} -->",
                c.source,
                today()
            );
            let content = insert_after_frontmatter(out.stdout.trim(), &provenance);
            let id = c.id.rsplit('/').next().unwrap_or("composant");
            let comp = install_component(
                &content,
                &c.kind,
                id,
                Some(id),
                Some(c.provides.clone()),
                Some(c.source.clone()),
            )?;
            std::fs::read_to_string(&comp.source)
                .map_err(|e| anyhow!("lecture de `{}` : {e}", comp.source))
        }
    }
}

/// Résout le kit de **chaque nœud** du graphe contre le registre — la
/// construction n'admet un nœud équipé que si chaque capacité a un
/// fournisseur réel et chaque protocole est fermé. `fetch = false` (le run)
/// : les sources distantes restent des fautes de construction.
pub fn resolve_kit(
    dag: &opendomain_core::harness::Dag,
    reg: &Registry,
    fetch: bool,
) -> Result<Vec<NodeKit>> {
    let mut out = Vec::new();
    for id in dag.topological_order()? {
        let spec = &dag.nodes[&id];
        if spec.kit.is_empty() {
            continue;
        }
        let mut nk = NodeKit { node: id.clone(), ..Default::default() };
        for sid in &spec.kit.skills {
            let c = find_component(reg, "skill", sid).ok_or_else(|| anyhow!(
                "kit du nœud `{id}` : skill `{sid}` sans fournisseur dans le registre — \
`opendomain kit add skill <url|chemin> --provides {sid}`, ou `opendomain skill new`, \
puis `opendomain build` à nouveau"
            ))?;
            let text = component_text(&c, fetch)?;
            nk.skills.push(SkillAttach {
                applies_to: vec![id.clone()],
                text: bounded_text(&text, 2500),
                id: c.id.clone(),
            });
        }
        for cid in &spec.kit.commands {
            let c = find_component(reg, "command", cid).ok_or_else(|| anyhow!(
                "kit du nœud `{id}` : commande `{cid}` sans fournisseur dans le registre — \
`opendomain kit add command <url|chemin> --provides {cid}`, puis `opendomain build` à nouveau"
            ))?;
            let text = component_text(&c, fetch)?;
            nk.commands.push((c.id.clone(), bounded_text(&text, 1200)));
        }
        for p in &spec.kit.protocols {
            protocol_closure(p, spec)?;
            nk.protocols.push(protocol_brief(p));
        }
        out.push(nk);
    }
    Ok(out)
}

/// Sections de kit projetées dans l'agent de nœud — reviewable AVANT le run,
/// dans le même artefact que les permissions et le contrat de sortie.
pub fn kit_sections(nk: Option<&NodeKit>) -> String {
    let Some(nk) = nk else { return String::new() };
    let mut s = String::new();
    if !nk.skills.is_empty() {
        s.push_str("\n## SKILLS (résolues à la construction)\n");
        for sk in &nk.skills {
            s.push_str(&format!("\n### `{}`\n{}\n", sk.id, sk.text));
        }
    }
    if !nk.commands.is_empty() {
        s.push_str("\n## COMMANDES (vérifiées à la construction)\n");
        for (id, text) in &nk.commands {
            s.push_str(&format!("\n### `{id}`\n{text}\n"));
        }
    }
    if !nk.protocols.is_empty() {
        s.push_str("\n## PROTOCOLES (fermeture vérifiée)\n");
        for p in &nk.protocols {
            s.push_str(&format!("- {p}\n"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILL: &str = "---\nid: revue-design\nprovides: design.revue, figma.tokens\napplies-to: analyste, verificateur\n---\n\n# Revue de design\n\nÉtapes…\n";

    #[test]
    fn frontmatter_parse() {
        let (id, provides, applies) = parse_skill_frontmatter(SKILL);
        assert_eq!(id, "revue-design");
        assert_eq!(provides, vec!["design.revue", "figma.tokens"]);
        assert_eq!(applies, vec!["analyste", "verificateur"]);
    }

    #[test]
    fn sans_frontmatter_id_vide() {
        let (id, p, _) = parse_skill_frontmatter("# juste du texte\n");
        assert!(id.is_empty() && p.is_empty());
    }

    #[test]
    fn registre_roundtrip() {
        let mut r = Registry::default();
        r.upsert(Component {
            id: "skill/revue-design".into(),
            kind: "skill".into(),
            provides: vec!["design.revue".into()],
            requires: Requires::default(),
            source_kind: "local".into(),
            source: "opendomain/harness/skills/revue-design.md".into(),
            applies_to: vec!["*".into()],
            preferred: false,
            ..Default::default()
        });
        r.upsert(Component {
            id: "mcp/figma".into(),
            kind: "mcp".into(),
            provides: vec!["figma.frames".into()],
            requires: Requires {
                bins: vec!["__opendomain_bin_absent__".into()],
                env: vec!["__OPENDOMAIN_ENV_ABSENTE_XYZ__".into()],
                files: vec![],
            },
            source_kind: "remote".into(),
            source: "https://registry/figma-mcp".into(),
            applies_to: vec![],
            preferred: true,
            ..Default::default()
        });
        let json = r.to_json().unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["components"].as_array().unwrap().len(), 2);
        assert_eq!(v["components"][0]["kind"], "skill");
    }

    #[test]
    fn resolution_sans_redondance_et_precedence() {
        // deux fournisseurs du même besoin : le local gagne sur le distant,
        // le preferred passe devant tout ; un composant partagé entre deux
        // besoins n'apparaît qu'UNE fois dans `chosen`.
        let mut r = Registry::default();
        r.upsert(Component {
            id: "mcp/figma".into(),
            kind: "mcp".into(),
            provides: vec!["design.revue".into(), "figma.frames".into()],
            source_kind: "remote".into(),
            preferred: true,
            ..Default::default()
        });
        r.upsert(Component {
            id: "skill/revue".into(),
            kind: "skill".into(),
            provides: vec!["design.revue".into(), "figma.tokens".into()],
            source_kind: "local".into(),
            ..Default::default()
        });
        let res = resolve(
            &r,
            &[
                "design.revue".into(),
                "figma.tokens".into(),
                "figma.frames".into(),
            ],
        );
        assert_eq!(res.misses, Vec::<String>::new());
        assert_eq!(res.chosen.len(), 2, "3 besoins, 2 composants — pas de redondance");
        assert_eq!(res.hits[0].1, "mcp/figma", "preferred d'abord");
        assert_eq!(res.hits[1].1, "skill/revue");
        assert_eq!(res.hits[2].1, "mcp/figma", "composant réutilisé sans doublon");
    }

    #[test]
    fn besoin_sans_fournisseur() {
        let res = resolve(&Registry::default(), &["magie.noire".into()]);
        assert_eq!(res.misses, vec!["magie.noire"]);
        assert!(res.chosen.is_empty());
    }

    #[test]
    fn exigences_machine() {
        let c = Component {
            id: "x".into(),
            requires: Requires {
                bins: vec!["__opendomain_bin_absent__".into()],
                env: vec!["__OPENDOMAIN_ENV_ABSENTE_XYZ__".into()],
                files: vec![],
            },
            ..Default::default()
        };
        let verdicts = check_requirements(&c);
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts.iter().all(|r| r.is_err()));
        // un binaire assurément présent : `cmd` sous windows, `sh` ailleurs
        let common = if cfg!(windows) { "cmd" } else { "sh" };
        let ok = Component {
            id: "y".into(),
            requires: Requires { bins: vec![common.into()], ..Default::default() },
            ..Default::default()
        };
        assert!(check_requirements(&ok).iter().all(|r| r.is_ok()));
    }

    #[test]
    fn besoins_de_la_spec() {
        let text = "# T\n\n## Besoins\n- design.revue\n- figma.tokens\n\n## Critères\n- [ ] x\n";
        let n = needs_of_spec(text, false);
        assert_eq!(n, vec!["design.revue", "figma.tokens"]);
        let n2 = needs_of_spec(text, true);
        assert!(n2.contains(&"terminal".to_string()));
    }

    #[test]
    fn attachement_filtré() {
        let attach = vec![
            SkillAttach { applies_to: vec!["verificateur".into()], text: "A".into(), id: "s".into() },
            SkillAttach { applies_to: vec!["*".into()], text: "B".into(), id: "g".into() },
        ];
        assert_eq!(applicable(&attach, "verificateur").len(), 2);
        assert_eq!(applicable(&attach, "analyste").len(), 1);
        assert_eq!(applicable(&attach, "synthese")[0].text, "B");
    }

    #[test]
    fn bornage() {
        let t: String = std::iter::repeat("x").take(3000).collect();
        let b = bounded_text(&t, 100);
        assert!(b.chars().count() <= 100 + 61, "100 chars + label de troncature");
        assert!(b.contains("tronquée"));
        let court = bounded_text("court", 100);
        assert_eq!(court, "court");
    }

    // ---------------------------------------------------------------- kit ---

    /// Un graphe à un nœud équipé d'un kit donné (fixtures de test du kit).
    fn dag_avec_kit(kit: opendomain_core::harness::Kit) -> opendomain_core::harness::Dag {
        use opendomain_core::harness::{AgentSpec, GraphFile};
        let spec = AgentSpec {
            id: "deployeur".into(),
            role: "déploie".into(),
            tools: vec!["read".into()],
            kit,
            ..Default::default()
        };
        let file = GraphFile { meta: serde_json::Value::Null, nodes: vec![spec] };
        opendomain_core::harness::Dag::build(&file).unwrap()
    }

    #[test]
    fn kit_skill_sans_fournisseur_est_une_erreur_typee() {
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            skills: vec!["deploy.vercel".into()],
            ..Default::default()
        });
        let reg = Registry::default();
        let err = resolve_kit(&dag, &reg, false).unwrap_err().to_string();
        assert!(err.contains("sans fournisseur"), "{err}");
        assert!(err.contains("deploy.vercel"), "{err}");
        assert!(err.contains("kit add skill"), "{err}");
    }

    #[test]
    fn kit_skill_locale_resolue_et_attachee_au_noeud() {
        // fichier de skill local + registre de surface de test
        let dir = std::env::temp_dir().join("od-kit-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("skills")).unwrap();
        let skill_path = dir.join("skills/deploiement.md");
        std::fs::write(
            &skill_path,
            "---\nid: deploiement\nprovides: deploy.vercel\n---\n\n1. vercel build\n2. vercel deploy --prebuilt\n",
        )
        .unwrap();
        let mut reg = Registry::default();
        reg.upsert(Component {
            id: "skill/deploiement".into(),
            kind: "skill".into(),
            provides: vec!["deploy.vercel".into()],
            requires: Requires::default(),
            source_kind: "local".into(),
            source: skill_path.to_string_lossy().to_string(),
            ..Default::default()
        });
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            skills: vec!["deploy.vercel".into()],
            commands: vec!["vercel.deploy".into()],
            ..Default::default()
        });
        // la commande n'est pas dans le registre → erreur typée qui la nomme
        let err = resolve_kit(&dag, &reg, false).unwrap_err().to_string();
        assert!(err.contains("commande `vercel.deploy` sans fournisseur"), "{err}");
        // sans la commande : la skill se résout, attachée au nœud
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            skills: vec!["deploy.vercel".into()],
            ..Default::default()
        });
        let resolved = resolve_kit(&dag, &reg, false).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].node, "deployeur");
        assert_eq!(resolved[0].skills.len(), 1);
        assert_eq!(resolved[0].skills[0].applies_to, vec!["deployeur".to_string()]);
        assert!(resolved[0].skills[0].text.contains("vercel deploy"), "{:?}", resolved[0].skills[0].text);
        // les sections projetées contiennent le texte réel
        let sections = kit_sections(Some(&resolved[0]));
        assert!(sections.contains("## SKILLS"));
        assert!(sections.contains("deploy.vercel"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn kit_source_distante_sans_fetch_est_typee_avec_le_pas_explicite() {
        let mut reg = Registry::default();
        reg.upsert(Component {
            id: "skill/distante".into(),
            kind: "skill".into(),
            provides: vec!["recherche.web".into()],
            requires: Requires::default(),
            source_kind: "remote".into(),
            source: "https://exemple.invalid/skill.md".into(),
            ..Default::default()
        });
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            skills: vec!["recherche.web".into()],
            ..Default::default()
        });
        // sans --fetch : faute de construction typée, le pas nommé
        let err = resolve_kit(&dag, &reg, false).unwrap_err().to_string();
        assert!(err.contains("source distante"), "{err}");
        assert!(err.contains("--fetch"), "{err}");
        // (avec --fetch : tentative réseau réelle — non testée ici, sans réseau)
    }

    #[test]
    fn protocole_inconnu_et_fermeture_http_sont_typees() {
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            protocols: vec!["smtp".into()],
            ..Default::default()
        });
        let err = resolve_kit(&dag, &Registry::default(), false).unwrap_err().to_string();
        assert!(err.contains("protocole inconnu : `smtp`"), "{err}");
        assert!(err.contains("http, git, npm"), "{err}");

        // `http` exige l'outil webfetch — absent → faute de construction
        let dag = dag_avec_kit(opendomain_core::harness::Kit {
            protocols: vec!["http".into()],
            ..Default::default()
        });
        let err = resolve_kit(&dag, &Registry::default(), false).unwrap_err().to_string();
        assert!(err.contains("`webfetch`"), "{err}");

        // avec webfetch dans la whitelist : la fermeture passe
        let file = opendomain_core::harness::GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![opendomain_core::harness::AgentSpec {
                id: "deployeur".into(),
                role: "déploie".into(),
                tools: vec!["read".into(), "webfetch".into()],
                kit: opendomain_core::harness::Kit {
                    protocols: vec!["http".into()],
                    ..Default::default()
                },
                ..Default::default()
            }],
        };
        let dag = opendomain_core::harness::Dag::build(&file).unwrap();
        let resolved = resolve_kit(&dag, &Registry::default(), false).unwrap();
        assert!(resolved[0].protocols[0].contains("webfetch"));
    }

    #[test]
    fn kit_vide_ne_se_resout_pas_et_projetation_vide() {
        let dag = dag_avec_kit(opendomain_core::harness::Kit::default());
        assert!(resolve_kit(&dag, &Registry::default(), false).unwrap().is_empty());
        assert_eq!(kit_sections(None), "");
    }

    #[test]
    fn provenance_apres_frontmatter_et_date_civile() {
        let note = "<!-- provenance : url=https://x — installé le 2026-10-23 -->";
        let out = insert_after_frontmatter("---\nid: a\n---\n\nCorps\n", note);
        assert!(out.starts_with("---\nid: a\n---\n"), "{out}");
        assert!(out.contains(note));
        // sans frontmatter : la note passe en tête
        let out2 = insert_after_frontmatter("# Sans frontmatter\n", note);
        assert!(out2.starts_with("<!-- provenance"), "{out2}");
        // date civile depuis l'ère unix (borne : 2026 existe, 1970 non)
        let (y, m, d) = civil_from_days(20_000);
        assert!(y >= 2024 && (1..=12).contains(&m) && (1..=31).contains(&d), "{y}-{m}-{d}");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn kit_add_refuse_un_kind_hors_vocabulaire() {
        let err = kit_add("mcp", "https://x", None, None).unwrap_err().to_string();
        assert!(err.contains("vocabulaire fermé"), "{err}");
    }
}
