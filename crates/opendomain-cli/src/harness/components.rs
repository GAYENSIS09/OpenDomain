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
    /// Nœuds auxquels la skill s'attache (ids du graphe, ou `*`).
    pub applies_to: Vec<String>,
    /// Retenu en priorité par le résolveur.
    pub preferred: bool,
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
            });
        }
        Ok(out)
    }

    /// Insère (écrase l'`id` existant : un index reste petit et réécrit).
    pub fn upsert(&mut self, c: Component) {
        self.components.retain(|x| x.id != c.id);
        self.components.push(c);
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
/// puis met à jour le registre. Le frontmatter est régénéré pour rester
/// canonique (id fiable, provides/applies-to normalisés).
pub fn install_skill(
    content: &str,
    original_name: &str,
    force_id: Option<&str>,
    force_provides: Option<Vec<String>>,
) -> Result<Component> {
    let (fm_id, fm_provides, applies) = parse_skill_frontmatter(content);
    let id = force_id
        .filter(|s| !s.trim().is_empty())
        .map(String::from)
        .or_else(|| {
            let n = fm_id.is_empty();
            (!n).then(|| sanitize_name(&fm_id))
        })
        .unwrap_or_else(|| sanitize_name(original_name));
    if id.is_empty() {
        return Err(anyhow!("id de skill déduit vide — passer `--id <mon-id>`"));
    }
    let provides = match force_provides {
        Some(p) if !p.is_empty() => p,
        _ if !fm_provides.is_empty() => fm_provides,
        _ => {
            return Err(anyhow!(
                "la skill {id} ne déclare aucune capacité `provides:` — \
ajouter dans son frontmatter (ex. `provides: design.revue`) ou `--provides design.revue`"
            ))
        }
    };
    let rendered = strip_frontmatter(content);
    std::fs::create_dir_all(SKILLS_DIR)?;
    let path = Path::new(SKILLS_DIR).join(format!("{id}.md"));
    std::fs::write(
        &path,
        format!(
            "---\nid: {id}\nprovides: {}\napplies-to: {}\n---\n\n{}",
            provides.join(", "),
            if applies.is_empty() { "*".to_string() } else { applies.join(", ") },
            rendered.trim_start()
        ),
    )?;
    println!("  + {}", path.display());

    let comp = Component {
        id: format!("skill/{id}"),
        kind: "skill".into(),
        provides,
        requires: Requires::default(),
        source_kind: "local".into(),
        source: path.to_string_lossy().to_string(),
        applies_to: applies,
        preferred: false,
    };
    upsert_registry(comp.clone())?;
    Ok(comp)
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
    reg.upsert(c);
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
        let Ok(text) = std::fs::read_to_string(&c.source) else { continue };
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

/// `opendomain skill add <url-or-chemin> [--id X] [--provides a,b]`.
pub fn skill_add(source: &str, id_opt: Option<String>, provides_opt: Option<Vec<String>>) -> Result<()> {
    let content = if source.starts_with("http://") || source.starts_with("https://") {
        println!("téléchargement : {source}");
        let out = crate::harness::adapters::run_captured("curl", &["-fsSL".into(), source.into()], 120)?;
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
        !content.trim().is_empty(),
        "contenu vide — `{source}` n'est pas une skill utilisable"
    );
    let name = source.rsplit(['/', '\\']).next().unwrap_or("skill");
    let comp = install_skill(&content, name, id_opt.as_deref(), provides_opt)?;
    println!(
        "OK {} installée ({}) — fournir : {:?}\n  → déclarer les besoins dans `## Besoins` de la spec, puis `opendomain resolve <slug>`",
        comp.id, comp.source_kind, comp.provides
    );
    Ok(())
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
}
