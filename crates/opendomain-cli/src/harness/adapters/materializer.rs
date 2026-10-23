//! Matérialisation — l'étape de l'architecte : écrire les agents de nœud
//! dans l'éditeur (`agent de nœud` par σ_i, permissions V2 projetées,
//! modèle épinglé, budget de pas, contrat + kit résolu). Visible,
//! reviewable, AVANT le run — l'exécution invoque ces ids, elle ne les
//! réécrit pas.
//!
//! Un outil de la whitelist que l'éditeur ne peut pas exprimer est une
//! erreur explicite ([`super::capabilities::projected_actions`]) — jamais
//! une permission fictive glissée dans le fichier. Et le fichier écrit est
//! **relu et vérifié** (`verify_materialized`) : la construction se prouve,
//! elle ne se déclare pas.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use opendomain_core::harness::{AgentSpec, Dag};

use super::capabilities::projected_actions;
use crate::harness::common::NodeBindings;

/// Arborescence `agents/<slug>` si elle a déjà été construite.
pub fn agents_dir(slug: &str) -> PathBuf {
    Path::new("opendomain/harness/agents").join(slug)
}

/// Id d'agent de nœud **namespacé par le run** : `opendomain-<slug>-<id>`
/// (ascii). L'exécution invoque cet id ; la construction l'a défini.
pub fn node_agent_id(spec: &AgentSpec, slug: &str) -> Result<String> {
    let file_id = sanitize_agent_id(spec.id.trim());
    anyhow::ensure!(
        !file_id.is_empty(),
        "id de nœud non projetable : `{}`",
        spec.id
    );
    let slug_id = sanitize_agent_id(slug);
    anyhow::ensure!(!slug_id.is_empty(), "slug de run non projetable : `{slug}`");
    Ok(format!("opendomain-{slug_id}-{file_id}"))
}

/// **Définit** l'agent de nœud — phase 1 (construction) : le fichier, la
/// racine éditeur explicite (production : `.opencode/agent` via
/// `define_node_agents` ; tests : tempdir). Le fichier porte les
/// permissions projetées, le modèle épinglé, le budget de pas, le contrat
/// de sortie ET les sections du kit résolu.
pub fn define_opencode_agent_in(
    spec: &AgentSpec,
    slug: &str,
    kit_sections: &str,
    bindings: &NodeBindings,
    agent_root: &Path,
) -> Result<String> {
    let agent_id = node_agent_id(spec, slug)?;
    let actions = projected_actions(&spec.tools)?;
    let path = agent_root.join(format!("{agent_id}.md"));
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("chemin d'agent inattendu : {}", path.display()))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(
        &path,
        agent_yaml_body(spec, &actions, kit_sections, bindings),
    )?;
    // Read-back de matérialisation : ce qui est écrit est relu et jugé —
    // la conformité du fichier n'est pas une promesse, c'est un contrôle.
    verify_materialized(&actions, bindings, &path)?;
    Ok(agent_id)
}

/// Re-lecture de conformité d'un agent de nœud matérialisé.
///
/// Vérifie, sur le **fichier réel** (pas sur la mémoire) :
/// 1. un `deny` universel (`action: "*"`) **avant** tout `allow` — le deny
///    universel ne se déplace pas ;
/// 2. exactement les actions projetées portent une permission `allow`
///    (manquante ou excédentaire = erreur, jamais l'oubli silencieux) ;
/// 3. `model:` / `steps:` présents quand l'épinglage les demande, absents
///    quand il ne les demande pas.
pub fn verify_materialized(actions: &[String], bindings: &NodeBindings, path: &Path) -> Result<()> {
    let read = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("agent matérialisé illisible : {} : {e}", path.display()))?;
    let mut pending: Option<String> = None;
    let mut deny_universal = false;
    let mut allowed_seen = false;
    let mut allowed: Vec<String> = Vec::new();
    for line in read.lines() {
        let t = line.trim();
        if let Some(a) = t.strip_prefix("- action: ") {
            pending = Some(a.trim_matches('"').to_string());
        } else if t == "effect: deny" {
            if pending.as_deref() != Some("*") {
                bail!(
                    "le deny doit porter `action: \"*\"` (trouvé {:?}) — moindre privilège mal matérialisé",
                    pending
                );
            }
            if allowed_seen {
                bail!(
                    "un `deny` arrive après un `allow` dans {} — l'ordre V2 impose le deny universel d'abord",
                    path.display()
                );
            }
            deny_universal = true;
            pending = None;
        } else if t == "effect: allow" {
            match pending.take() {
                Some(a) => allowed.push(a),
                None => bail!("`allow` sans action en tête de bloc — frontmatter V2 malformé"),
            }
            allowed_seen = true;
        }
    }
    if !deny_universal {
        bail!(
            "aucun `deny` universel dans {} — moindre privilège non matérialisé",
            path.display()
        );
    }
    if allowed.as_slice() != actions {
        let missing: Vec<&String> = actions.iter().filter(|a| !allowed.contains(a)).collect();
        let excess: Vec<&String> = allowed.iter().filter(|a| !actions.contains(a)).collect();
        bail!(
            "allow écrit ≠ allow projeté dans {} — manquants : {:?} ; excédentaires : {:?} — \
l'écart entre le fichier et la whitelist σ_i est un défaut, pas un détail",
            path.display(),
            missing,
            excess
        );
    }
    // Épinglage : présent au mot près quand demandé, absent sinon — un
    // fichier qui diverge du réglage est un défaut, pas une préférence.
    if let Some(m) = &bindings.model {
        let needle = format!("model: {m}");
        if !read.lines().any(|l| l.trim() == needle) {
            bail!("agent matérialisé sans l'épinglage attendu (`{needle}`) — le run ne lirait pas le réglage");
        }
    } else if read.lines().any(|l| l.trim().starts_with("model:")) {
        bail!("agent matérialisé porte `model:` sans épinglage — la borne doit venir du réglage, jamais du hasard");
    }
    if let Some(s) = bindings.steps {
        let needle = format!("steps: {s}");
        if !read.lines().any(|l| l.trim() == needle) {
            bail!("agent matérialisé sans la borne attendue (`{needle}`) — un run sans borne est un coût non contrôlé");
        }
    } else if read.lines().any(|l| l.trim().starts_with("steps:")) {
        bail!("agent matérialisé porte `steps:` sans épinglage — la borne doit venir du réglage, jamais du hasard");
    }
    Ok(())
}

/// Définit **tous** les nœuds d'un graphe, dans l'ordre topologique, avec
/// le kit résolu de chaque nœud (vide pour un nœud sans kit) et les
/// liaisons `[harness]` de la config. Renvoie les ids définis.
pub fn define_node_agents(
    dag: &Dag,
    slug: &str,
    resolved_kit: &[crate::harness::components::NodeKit],
) -> Result<Vec<String>> {
    define_node_agents_in(dag, slug, resolved_kit, Path::new(".opencode/agent"))
}

/// La même définition du stock de nœuds, racine éditeur explicite (tests).
pub fn define_node_agents_in(
    dag: &Dag,
    slug: &str,
    resolved_kit: &[crate::harness::components::NodeKit],
    agent_root: &Path,
) -> Result<Vec<String>> {
    let bindings = crate::harness::common::load_harness_bindings()?;
    let order = dag.topological_order()?;
    let mut ids = Vec::with_capacity(order.len());
    for id in &order {
        let kit = resolved_kit.iter().find(|k| &k.node == id);
        ids.push(define_opencode_agent_in(
            &dag.nodes[id],
            slug,
            &crate::harness::components::kit_sections(kit),
            &bindings,
            agent_root,
        )?);
    }
    Ok(ids)
}

/// Frontmatter (permissions V2) + corps du fichier d'agent : le modèle
/// épinglé, le budget de pas, les permissions projetées ET le contrat de
/// sortie — l'agent se définit en entier (modèle, pas, outils, sorties),
/// reviewable avant le run. Ordre V2 : description/mode → model → steps →
/// permissions ; aucune clé V1 (`permission`, `temperature`, `maxSteps`)
/// — un fichier qui porte `permissions` est lu comme V2 et ses champs V1
/// restants seraient ignorés avec un avertissement.
fn agent_yaml_body(
    spec: &AgentSpec,
    actions: &[String],
    kit_sections: &str,
    bindings: &NodeBindings,
) -> String {
    let mut yaml = String::from(
        "---\ndescription: Nœud du harnais OpenDomain — permissions projetées de la spec\nmode: primary\n",
    );
    // Modèle épinglé + budget de pas : le run ne dépend plus du modèle par
    // défaut du CLI (une panne de quota ne tue plus un run entier) et ne
    // tourne plus sans borne (au dernier pas, résumé sans outils).
    if let Some(model) = &bindings.model {
        yaml.push_str(&format!("model: {model}\n"));
    }
    if let Some(steps) = bindings.steps {
        yaml.push_str(&format!("steps: {steps}\n"));
    }
    yaml.push_str("permissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n");
    for action in actions {
        yaml.push_str(&format!(
            "  - action: {action}\n    resource: \"*\"\n    effect: allow\n"
        ));
    }
    yaml.push_str(&format!(
        "---\n\n# Agent `{}` (nœud du harnais OpenDomain)\n\nTu es le nœud `{}` du harnais OpenDomain. Ta production est l'objet JSON du\ncontrat de sortie ci-dessous — rien d'autre. Les commandes de\ncritères tournent hors de toi (le coordinateur les exécute sous sandbox).\n\n## SORTIE (contrat)\n\n```json\n{}\n```\n{}\n",
        spec.id.trim(),
        spec.id.trim(),
        spec.contract.output_schema,
        kit_sections,
    ));
    yaml
}

fn sanitize_agent_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(48)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::common::NodeBindings;

    /// Racine temporaire pour les fichiers d'agent de test — la pollution
    /// CWD historique de `cargo test` n'est plus tolérée.
    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opendomain-adapters-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// La whitelist d'un nœud devient un agent OpenCode V2 : deny-all
    /// d'abord, allow ensuite — l'exécuteur refuse le reste réellement.
    #[test]
    fn la_whitelist_devient_des_permissions_v2() {
        let root = scratch("v2");
        let spec = AgentSpec {
            id: "synthese".into(),
            tools: vec!["read".into(), "write".into(), "bash".into()],
            ..Default::default()
        };
        let agent = define_opencode_agent_in(
            &spec,
            "task-001",
            "",
            &NodeBindings {
                model: None,
                steps: None,
            },
            &root,
        )
        .unwrap();
        assert_eq!(agent, "opendomain-task-001-synthese");
        let file = std::fs::read_to_string(root.join(format!("{agent}.md"))).unwrap();
        // opencode (défaut) : refus-first puis allow — l'ordre est testé :
        // deny AVANT allow (dernier match gagnant).
        let deny = file.find("effect: deny").unwrap();
        let allow = file.find("effect: allow").unwrap();
        assert!(deny < allow, "refus universel avant les allow : {file}");
        assert!(file.contains("action: edit"), "write → action edit V2");
        assert!(file.contains("action: shell"), "bash → action shell V2");
        assert!(file.contains("action: read"));
        assert!(!file.contains("action: webfetch"), "hors whitelist = refus");
        // Aucune liaison → aucune ligne model/steps : le défaut OpenCode
        // reste (un sous-agent hérite du modèle de sa session parente).
        assert!(!file.contains("model:"), "{file}");
        assert!(!file.contains("steps:"), "{file}");
    }

    #[test]
    fn id_sanitize_et_dedup_des_actions() {
        let root = scratch("ids");
        let spec = AgentSpec {
            id: "noeud.setName/action".into(),
            tools: vec!["bash".into(), "terminal".into(), "bash".into()],
            ..Default::default()
        };
        let agent = define_opencode_agent_in(
            &spec,
            "task-001",
            "",
            &NodeBindings {
                model: None,
                steps: None,
            },
            &root,
        )
        .unwrap();
        assert_eq!(agent, "opendomain-task-001-noeud-setname-action");
        let file = std::fs::read_to_string(root.join(format!("{agent}.md"))).unwrap();
        assert_eq!(
            file.matches("action: shell").count(),
            1,
            "actions dédupliquées"
        );
    }

    /// Les liaisons `[harness]` se projettent dans le frontmatter V2, dans
    /// l'ordre canonique (description/mode → model → steps → permissions).
    #[test]
    fn le_modele_et_les_pas_se_projetent_dans_le_frontmatter() {
        let root = scratch("bindings");
        let spec = AgentSpec {
            id: "analyste".into(),
            tools: vec!["read".into()],
            ..Default::default()
        };
        let bindings = NodeBindings {
            model: Some("opencode-go/glm-5.3-flash".into()),
            steps: Some(40),
        };
        let agent = define_opencode_agent_in(&spec, "task-001", "", &bindings, &root).unwrap();
        let file = std::fs::read_to_string(root.join(format!("{agent}.md"))).unwrap();
        assert!(file.contains("model: opencode-go/glm-5.3-flash"), "{file}");
        assert!(file.contains("steps: 40"), "{file}");
        let model = file.find("model:").unwrap();
        let steps = file.find("steps:").unwrap();
        let perms = file.find("permissions:").unwrap();
        assert!(
            model < steps && steps < perms,
            "ordre V2 du frontmatter : {file}"
        );
    }

    /// Un outil que l'éditeur ne peut pas exprimer refuse la matérialisation
    /// avec l'erreur explicite — pas de permission fictive écrite.
    #[test]
    fn outil_non_exprimable_refuse_la_materialisation() {
        let root = scratch("incompat");
        let spec = AgentSpec {
            id: "faiseur".into(),
            tools: vec!["read".into(), "oracle".into()],
            ..Default::default()
        };
        let err = define_opencode_agent_in(
            &spec,
            "task-001",
            "",
            &NodeBindings {
                model: None,
                steps: None,
            },
            &root,
        )
        .expect_err("EditorIncompat attendu");
        let msg = format!("{err}");
        assert!(msg.contains("EditorIncompat"), "typé : {msg}");
        assert!(msg.contains("oracle"), "outil nommé : {msg}");
        assert!(
            !std::fs::exists(root.join("opendomain-task-001-faiseur.md")).unwrap_or(false),
            "aucun fichier écrit avec le droit fictif"
        );
    }

    // ------------------------------------------------ read-back (S5) ---

    /// Un fichier conforme passe la re-lecture — le contraste avec les
    /// scénarios de détection qui suivent est le test.
    #[test]
    fn le_read_back_accueille_un_fichier_conforme() {
        let actions = vec!["read".to_string(), "edit".to_string()];
        let dir = scratch("readback-ok");
        let path = dir.join("agent.md");
        std::fs::write(
            &path,
            "---\ndescription: x\nmode: primary\n---\npermissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n  - action: read\n    resource: \"*\"\n    effect: allow\n  - action: edit\n    resource: \"*\"\n    effect: allow\n---\n\n# corps\n",
        )
        .unwrap();
        verify_materialized(
            &actions,
            &NodeBindings {
                model: None,
                steps: None,
            },
            &path,
        )
        .expect("fichier conforme doit passer");
    }

    #[test]
    fn le_read_back_refuse_un_allow_manquant_ou_excedentaire() {
        let actions = vec!["read".to_string()];
        let dir = scratch("readback-ecart");
        // allow projeté absent
        let path = dir.join("manquant.md");
        std::fs::write(
            &path,
            "permissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n  - action: edit\n    resource: \"*\"\n    effect: allow\n",
        )
        .unwrap();
        let err = verify_materialized(
            &actions,
            &NodeBindings {
                model: None,
                steps: None,
            },
            &path,
        )
        .expect_err("allow manquant doit se voir");
        assert!(
            format!("{err}").contains("excédentaires"),
            "message : {err}"
        );
        // allow excédentaire
        let path2 = dir.join("excedentaire.md");
        std::fs::write(
            &path2,
            "permissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n  - action: read\n    resource: \"*\"\n    effect: allow\n  - action: webfetch\n    resource: \"*\"\n    effect: allow\n",
        )
        .unwrap();
        let err = verify_materialized(
            &actions,
            &NodeBindings {
                model: None,
                steps: None,
            },
            &path2,
        )
        .expect_err("allow excédentaire doit se voir");
        assert!(
            format!("{err}").contains("webfetch"),
            "excédentaire nommé : {err}"
        );
    }

    #[test]
    fn le_read_back_refuse_le_deny_absent_ou_deplace() {
        let dir = scratch("readback-deny");
        let actions: Vec<String> = vec![];
        // deny absent — moindre privilège non matérialisé
        let path = dir.join("sans-deny.md");
        std::fs::write(
            &path,
            "permissions:\n  - action: read\n    resource: \"*\"\n    effect: allow\n",
        )
        .unwrap();
        assert!(verify_materialized(
            &actions,
            &NodeBindings {
                model: None,
                steps: None
            },
            &path
        )
        .is_err());
        // deny déplacé après un allow — l'ordre V2 est violé
        let path2 = dir.join("deny-apres.md");
        std::fs::write(
            &path2,
            "permissions:\n  - action: read\n    resource: \"*\"\n    effect: allow\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n",
        )
        .unwrap();
        let err = verify_materialized(
            &actions,
            &NodeBindings {
                model: None,
                steps: None,
            },
            &path2,
        )
        .expect_err("deny déplacé doit se voir");
        assert!(format!("{err}").contains("d'abord"), "message : {err}");
    }

    #[test]
    fn le_read_back_exige_l_epinglage_promis_et_interdit_le_hasard() {
        let dir = scratch("readback-bindings");
        let actions: Vec<String> = vec![];
        let bound = NodeBindings {
            model: Some("opencode-go/glm-5.3-flash".into()),
            steps: Some(40),
        };
        // Épinglage promis → absent du fichier = refus, modèle nommé.
        let path = dir.join("sans-bindings.md");
        std::fs::write(
            &path,
            "permissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n",
        )
        .unwrap();
        let err = verify_materialized(&actions, &bound, &path)
            .expect_err("épinglage promis doit être relu");
        assert!(
            format!("{err}").contains("glm-5.3-flash"),
            "modèle nommé : {err}"
        );
        // Inverse : aucun épinglage réclamé, mais le fichier en porte un.
        let libre = NodeBindings {
            model: None,
            steps: None,
        };
        std::fs::write(
            &path,
            "model: opencode-go/glm-5.3-flash\npermissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n",
        )
        .unwrap();
        let err = verify_materialized(&actions, &libre, &path)
            .expect_err("model non demandé doit se voir");
        assert!(
            format!("{err}").contains("jamais du hasard"),
            "message : {err}"
        );
    }

    /// Fixture d'intégration : un petit graphe à deux nœuds se matérialise
    /// en entier dans un tempdir, chaque fichier relu conforme au passage.
    #[test]
    fn deux_noeuds_se_materialisent_et_se_relisent() {
        let root = scratch("integration");
        let a = AgentSpec {
            id: "analyste".into(),
            role: "rôle a".into(),
            tools: vec!["read".into()],
            ..Default::default()
        };
        let v = AgentSpec {
            id: "verificateur".into(),
            role: "rôle v".into(),
            tools: vec!["grep".into()],
            ..Default::default()
        };
        let file = opendomain_core::harness::GraphFile {
            meta: serde_json::Value::Null,
            nodes: vec![a, v],
        };
        let dag = Dag::build(&file).unwrap();
        let ids =
            define_node_agents_in(&dag, "t-spec", &[], &root).expect("matérialisation complète");
        assert_eq!(ids.len(), 2, "les deux nœuds définis : {ids:?}");
        for id in &ids {
            let p = root.join(format!("{id}.md"));
            assert!(p.exists(), "fichier présent : {}", p.display());
        }
    }
}
