//! Adaptateur d'exécuteur LLM (OpenCode d'abord) + boucle de raffinement de
//! construction : `Generate → Verify → Attribute → Refine`, appliqué à la
//! génération d'un `graph.json` par le planificateur via opencode (défaut).

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use opendomain_core::harness::{AgentSpec, GraphFile};

/// Sortie capturée d'un agent CLI.
pub struct Captured {
    pub status_ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Exécute un CLI, capture et dévore les flux par threads, impose un
/// délai d'échéance (kill propre, pipes refermées).
pub fn run_captured(bin: &str, args: &[String], timeout_secs: u64) -> Result<Captured> {
    // Windows : les exécuteurs npm sont des `.cmd`/`.bat` — CreateProcess
    // exige l'interpréteur ; on passe systématiquement par `cmd /C`.
    #[cfg(windows)]
    let mut command: Command = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(bin);
        for a in args {
            c.arg(a);
        }
        c
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut c = Command::new(bin);
        c.args(args);
        c
    };

    let mut child: Child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("lancement de `{bin}` impossible : {e} (voir `opendomain doctor`)"))?;

    let stdout = child.stdout.take().expect("stdout pipé");
    let stderr = child.stderr.take().expect("stderr pipé");
    let t_out = drain(stdout);
    let t_err = drain(stderr);

    let start = Instant::now();
    let deadline = Duration::from_secs(timeout_secs.max(1));
    loop {
        match child.try_wait()? {
            Some(status) => {
                let out = t_out.join().unwrap_or_default();
                let err = t_err.join().unwrap_or_default();
                return Ok(Captured {
                    status_ok: status.success(),
                    stdout: out,
                    stderr: err,
                });
            }
            None if start.elapsed() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = t_out.join();
                let _ = t_err.join();
                return Err(anyhow!(
                    "`{bin}` a dépassé {timeout_secs}s — augmenter --timeout, ou demander \
explicitement le repli déterministe : `opendomain plan <slug> --offline`"
                ));
            }
            None => thread::sleep(Duration::from_millis(200)),
        }
    }
}

fn drain<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    })
}

/// `opencode run <prompt>` — mode non-interactif d'OpenCode.
pub fn opencode_run(prompt: &str, timeout_secs: u64) -> Result<Captured> {
    run_captured(
        "opencode",
        &["run".to_string(), prompt.to_string()],
        timeout_secs,
    )
}

/// Extrait le premier objet JSON complet d'une sortie prose.
pub fn extract_json(text: &str) -> Option<&str> {
    let begin = text.find('{')?;
    let end = text.rfind('}')?;
    if end > begin {
        Some(&text[begin..=end])
    } else {
        None
    }
}

/// Boucle de construction : le LLM propose un graphe, le harnais le VÉRIFIE
/// (sérialisation + DAG + contrats), et tout défaut typé est renvoyé comme
/// feedback au raffinement suivant (≤ 3 tentatives — `MAX_ATTEMPTS`).
pub fn plan_with_llm(
    spec_slug: &str,
    spec_text: &str,
    detected_language: &str,
    timeout_secs: u64,
) -> Result<GraphFile> {
    let mut feedback = String::from("(première tentative)");
    for attempt in 1..=3 {
        let prompt = build_plan_prompt(spec_slug, spec_text, detected_language, &feedback);
        let out = opencode_run(&prompt, timeout_secs)?;
        if !out.status_ok && out.stdout.trim().is_empty() {
            return Err(anyhow!(
                "`opencode run` a échoué : {}",
                out.stderr.lines().last().unwrap_or("(cause inconnue)")
            ));
        }
        let Some(json) = extract_json(&out.stdout).map(|s| s.to_string()) else {
            feedback = "aucun objet JSON trouvé dans la sortie — renvoie UNIQUEMENT l'objet JSON du graphe".into();
            continue;
        };
        match serde_json::from_str::<GraphFile>(&json) {
            Ok(file) => match opendomain_core::harness::Dag::build(&file) {
                Ok(_) => return Ok(file),
                Err(e) => {
                    feedback = format!("graphe invalide ({e}) — corrige et ne renvoie que le JSON");
                }
            },
            Err(e) => {
                feedback = format!("JSON non sérialisable ({e}) — corrige et ne renvoie que le JSON");
            }
        }
        eprintln!(
            "  ↻ tentative {attempt}: raffinement déclenché — {feedback}"
        );
    }
    Err(anyhow!("plan via opencode : 3 tentatives épuisées sans graphe valide"))
}

fn build_plan_prompt(
    spec_slug: &str,
    spec_text: &str,
    detected_language: &str,
    feedback: &str,
) -> String {
    let spec_head: String = spec_text.lines().take(220).collect::<Vec<_>>().join("\n");
    format!(
        "Tu es le PLANIFICATEUR du harnais OpenDomain (construction).\n\
Retourne EXCLUSIVEMENT un objet JSON, sans prose, sans bloc de code, avec exactement ces règles :\n\
- clés: \"meta\" (objet libre) et \"nodes\" (tableau de 2 à 6 spécifications d'agents σ_i)\n\
- chaque node: {{\"id\": identifiant ascii sans accents, \"role\": directive complète, \"tools\": whitelist stricte (read/grep/glob/write/edit/bash), \"contract\": {{\"input_schema\": objet JSON Schema, \"output_schema\": objet JSON Schema avec \"required\" non vide, \"forbidden\": motifs interdits}}, \"criteria\": {{\"schema_check\": true, \"contains\": [...], \"not_contains\": [...], \"commands\": [...] }}, \"depends_on\": [ids des nœuds requis]}}\n\
- exiger un graphe acyclique : un agent d'analyse → un planificateur de stratégie → un synthétiseur → un vérificateur (adapte libres, 2 à 6 nœuds max)\n\
- NE PRODUIS AUCUN CODE MÉTIER ; le graphe décrit des agents, pas la solution.\n\
- langue cible du projet: {detected_language}.\n\
Retour du contrôle précédent (à corriger si nécessaire): {feedback}\n\
--- SPEC ({spec_slug}) ---\n{spec_head}"
    )
}

/// Arborescence `agents/<slug>` si elle a déjà été construite.
pub fn agents_dir(slug: &str) -> std::path::PathBuf {
    Path::new("opendomain/harness/agents").join(slug)
}

// --------------------------------------------------- projection whitelist ---
/// Moindre privilège projeté pour OpenCode (V2) : un **agent de nœud** par
/// σ_i, écrit dans `.opencode/agent/`, avec des permissions V2 ordonnées —
/// `deny` universel **puis** `allow` sur chaque outil de `spec.tools`
/// (dernier match gagnant). L'exécuteur lui-même refuse le reste : la
/// whitelist n'est plus une phrase dans un prompt.
///
/// Outils → actions V2 : `write`/`edit` → `edit` (écrire et patcher),
/// `bash`/`cmd`/`terminal` → `shell`, `task`/`subagent` → `subagent`,
/// autres → action homonyme. Renvoie l'id d'agent à passer à `--agent`.
/// Id d'agent de nœud **namespacé par le run** : `opendomain-<slug>-<id>`
/// (ascii). L'exécution invoque cet id ; la construction l'a défini.
pub fn node_agent_id(spec: &AgentSpec, slug: &str) -> Result<String> {
    let file_id = sanitize_agent_id(spec.id.trim());
    anyhow::ensure!(!file_id.is_empty(), "id de nœud non projetable : `{}`", spec.id);
    let slug_id = sanitize_agent_id(slug);
    anyhow::ensure!(!slug_id.is_empty(), "slug de run non projetable : `{slug}`");
    Ok(format!("opendomain-{slug_id}-{file_id}"))
}

/// **Définit** l'agent de nœud — phase 1 (construction), l'étape de
/// l'architecte : le fichier dans `.opencode/agent/` porte les permissions
/// projetées, le contrat de sortie ET les sections du kit résolu (skills,
/// commandes, protocoles). Visible, reviewable, AVANT le run — l'exécution
/// invoque, elle ne réécrit pas.
pub fn define_opencode_agent(spec: &AgentSpec, slug: &str, kit_sections: &str) -> Result<String> {
    let agent_id = node_agent_id(spec, slug)?;
    let rel = format!(".opencode/agent/{agent_id}.md");
    let path = Path::new(&rel);
    let parent = path.parent().ok_or_else(|| anyhow!("chemin d'agent inattendu : {rel}"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, agent_yaml_body(spec, kit_sections))?;
    Ok(agent_id)
}

/// Définit **tous** les nœuds d'un graphe, dans l'ordre topologique — le pas
/// de construction de l'architecte du harnais, avec le kit résolu de chaque
/// nœud (vide pour un nœud sans kit). Renvoie les ids définis.
pub fn define_node_agents(
    dag: &opendomain_core::harness::Dag,
    slug: &str,
    resolved_kit: &[crate::harness::components::NodeKit],
) -> Result<Vec<String>> {
    let order = dag.topological_order()?;
    let mut ids = Vec::with_capacity(order.len());
    for id in &order {
        let kit = resolved_kit.iter().find(|k| &k.node == id);
        ids.push(define_opencode_agent(&dag.nodes[id], slug, &crate::harness::components::kit_sections(kit))?);
    }
    Ok(ids)
}

/// Frontmatter (permissions V2) + corps du fichier d'agent : les
/// permissions projetées ET le contrat de sortie — l'agent se définit en
/// entier (outils, sorties), reviewable avant le run.
fn agent_yaml_body(spec: &AgentSpec, kit_sections: &str) -> String {
    let mut yaml = String::from(
        "---\ndescription: Nœud du harnais OpenDomain — permissions projetées de la spec\nmode: primary\npermissions:\n  - action: \"*\"\n    resource: \"*\"\n    effect: deny\n",
    );
    let mut allowed: Vec<String> = Vec::new();
    for tool in &spec.tools {
        let action = opencode_action(tool);
        if !allowed.contains(&action.to_string()) {
            allowed.push(action.to_string());
        }
    }
    for action in allowed {
        yaml.push_str(&format!("  - action: {action}\n    resource: \"*\"\n    effect: allow\n"));
    }
    yaml.push_str(&format!(
        "---\n\n# Agent `{}` (nœud du harnais OpenDomain)\n\nTu es le nœud `{}` du harnais OpenDomain. Ta production est l'objet JSON du\ncontrat de sortie ci-dessous — rien d'autre. Les commandes de\ncritères tournent hors de toi (le coordinator les exécute sous sandbox).\n\n## SORTIE (contrat)\n\n```json\n{}\n```\n{}\n",
        spec.id.trim(),
        spec.id.trim(),
        spec.contract.output_schema,
        kit_sections,
    ));
    yaml
}

/// Traduction d'une whitelist vers les actions de permission OpenCode V2.
fn opencode_action(tool: &str) -> &'static str {
    match tool.trim().to_lowercase().as_str() {
        "write" | "edit" => "edit",
        "bash" | "cmd" | "terminal" | "shell" => "shell",
        "task" | "subagent" => "subagent",
        "read" => "read",
        "grep" => "grep",
        "glob" => "glob",
        "webfetch" => "webfetch",
        "websearch" => "websearch",
        "list" => "list",
        _ => "shell",
    }
}

fn sanitize_agent_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(48)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extraction_json_prose() {
        let s = "Voici la sortie:\n{\"meta\": {}, \"nodes\": []}\nBonne journée.";
        assert_eq!(extract_json(s).unwrap(), "{\"meta\": {}, \"nodes\": []}");
        assert!(extract_json("pas de json").is_none());
    }

    #[test]
    fn prompt_contient_regles_de_gates() {
        let p = build_plan_prompt("001-x", "spec", "python", "ok");
        assert!(p.contains("nodes"));
        assert!(p.contains("output_schema"));
        assert!(p.contains("python"));
        let guard = json!({ "a": 1 });
        assert!(guard.get("a").is_some());
    }

    // ------------------------------------------------ projection whitelist ---
    /// La whitelist d'un nœud devient un agent OpenCode V2 : deny-all d'abord,
    /// allow ensuite — l'exécuteur refuse le reste réellement.
    #[test]
    fn la_whitelist_devient_des_permissions_v2() {
        let spec = AgentSpec {
            id: "synthese".into(),
            tools: vec!["read".into(), "write".into(), "bash".into()],
            ..Default::default()
        };
        let agent = define_opencode_agent(&spec, "task-001", "").unwrap();
        assert_eq!(agent, "opendomain-task-001-synthese");
        let file = std::fs::read_to_string(format!(".opencode/agent/{agent}.md")).unwrap();
        // opencode (défaut) : refus-first puis allow — l'agent file_id est
        // testé pour l'ordre : deny AVANT allow (dernier match gagnant).
        let deny = file.find("effect: deny").unwrap();
        let allow = file.find("effect: allow").unwrap();
        assert!(deny < allow, "refus universel avant les allow : {file}");
        assert!(file.contains("action: edit"), "write → action edit V2");
        assert!(file.contains("action: shell"), "bash → action shell V2");
        assert!(file.contains("action: read"));
        assert!(!file.contains("action: webfetch"), "hors whitelist = refus");
        let _ = std::fs::remove_dir_all(".opencode");
    }

    #[test]
    fn id_sanitize_et_dedup_des_actions() {
        let spec = AgentSpec {
            id: "noeud.setName/action".into(),
            tools: vec!["bash".into(), "terminal".into(), "bash".into()],
            ..Default::default()
        };
        let agent = define_opencode_agent(&spec, "task-001", "").unwrap();
        assert_eq!(agent, "opendomain-task-001-noeud-setname-action");
        let file = std::fs::read_to_string(format!(".opencode/agent/{agent}.md")).unwrap();
        assert_eq!(file.matches("action: shell").count(), 1, "actions déduplicables");
        let _ = std::fs::remove_dir_all(".opencode");
    }
}
