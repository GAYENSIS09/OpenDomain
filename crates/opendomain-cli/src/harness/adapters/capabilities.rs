//! Ce que l'éditeur cible peut **exprimer** — la frontière entre le graphe
//! abstrait (whitelist d'outils `σ_i`) et les permissions réellement
//! possibles dans le frontmatter V2 de l'éditeur.
//!
//! Contrat de la mission : *une incompatibilité entre le graphe abstrait
//! et les capacités de l'éditeur doit produire une erreur explicite, jamais
//! une garantie fictive*. L'ancien `opencode_action` répondait `shell` à
//! tout outil inconnu — un outil conceptuel recevait un droit shell quoi
//! qu'il fasse. Il n'y a plus de branche `_ =>` : la table ci-dessous est
//! le vocabulaire complet, hors table = refus typé.

use anyhow::{bail, Result};
use opendomain_core::harness::AgentSpec;

/// L'éditeur cible du run (un exécuteur par run, documenté — capacités
/// déclarées ici seule source).
pub const EDITOR: &str = "opencode";

/// Vocabulaire tools → actions de permission V2, une seule table.
/// `None` = l'outil n'est pas exprimable.
pub fn action_of(tool: &str) -> Option<&'static str> {
    match tool.trim().to_lowercase().as_str() {
        "write" | "edit" => Some("edit"),
        "bash" | "cmd" | "terminal" | "shell" => Some("shell"),
        "task" | "subagent" => Some("subagent"),
        "read" => Some("read"),
        "grep" => Some("grep"),
        "glob" => Some("glob"),
        "webfetch" => Some("webfetch"),
        "websearch" => Some("websearch"),
        "list" => Some("list"),
        _ => None,
    }
}

/// Les actions projetées d'une whitelist, dédupliquées dans l'ordre
/// déclaré — un outil non exprimable est une erreur explicite, pas un
/// droit shell en silence.
pub fn projected_actions(tools: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut manquants: Vec<&str> = Vec::new();
    for t in tools {
        match action_of(t) {
            Some(a) => {
                if !out.iter().any(|x| x == a) {
                    out.push(a.to_string());
                }
            }
            None => manquants.push(t.as_str()),
        }
    }
    if !manquants.is_empty() {
        bail!(
            "EditorIncompat : outils non exprimables par l'éditeur {EDITOR} : {} — \
aucune permission fictive n'est attribuée ; retirer l'outil de la whitelist \
de la σ_i, ou le couvrir par un protocole déclaré du kit",
            manquants.join(", ")
        );
    }
    Ok(out)
}

/// Ancre d'admission : les gaps d'expressivité d'un `σ_i` pour l'éditeur
/// cible. Injecté dans `harness::admission` — le même moteur décide, tout
/// refus est typé avant toute écriture.
pub fn gaps(spec: &AgentSpec) -> Vec<(String, String)> {
    spec.tools
        .iter()
        .filter(|t| action_of(t).is_none())
        .map(|t| {
            (
                format!("outil `{}`", t.trim()),
                format!(
                    "non exprimable par l'éditeur {EDITOR} — actions connues : edit, shell, \
subagent, read, grep, glob, webfetch, websearch, list — retirer l'outil de la \
whitelist de la σ_i, ou le couvrir par un protocole de kit"
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn le_vocabulaire_connait_les_outils_expedies() {
        for t in [
            "read",
            "grep",
            "glob",
            "write",
            "edit",
            "bash",
            "cmd",
            "terminal",
            "shell",
            "task",
            "subagent",
            "webfetch",
            "websearch",
            "list",
        ] {
            assert!(action_of(t).is_some(), "{t} doit rester exprimable");
        }
        assert_eq!(action_of("write"), Some("edit"));
        assert_eq!(action_of("bash"), Some("shell"));
        assert_eq!(action_of("task"), Some("subagent"));
        assert!(
            action_of("oracle").is_none(),
            "hors vocabulaire = non exprimable"
        );
    }

    #[test]
    fn une_whitelist_inconnue_est_une_erreur_explicite_sans_droit_fictif() {
        let err = projected_actions(&["read".into(), "oracle".into()]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("EditorIncompat"), "typé : {msg}");
        assert!(msg.contains("oracle"), "outil nommé : {msg}");
        assert!(
            msg.contains("aucune permission fictive"),
            "garantie : {msg}"
        );
    }

    #[test]
    fn les_actions_sont_dedupliquees_dans_l_ordre_declare() {
        let ok = projected_actions(&["bash".into(), "terminal".into(), "bash".into()]).unwrap();
        assert_eq!(ok, vec!["shell".to_string()]);
    }

    #[test]
    fn les_besoins_de_capacite_parviennent_au_moteur_d_admission() {
        // Graphe avec un outil hors vocabulaire : l'admission refuse avec
        // un message typé "capacités" — le même moteur, l'encre commune.
        let spec = opendomain_core::harness::AgentSpec {
            id: "faiseur".into(),
            role: "rôle".into(),
            tools: vec!["oracle".into()],
            ..Default::default()
        };
        let file = opendomain_core::harness::GraphFile {
            meta: json!({}),
            nodes: vec![spec],
        };
        // La complétude refuse aussi (tools réductés à l'outil inconnu sous
        // la whitelist `oracle`... ici tools = ["oracle"] pas vide) : les
        // deux refus sont indépendants, on vérifie que la route capacité
        // précède le message de complétude quand l'outil est le seul aigu.
        let err = format!(
            "{:#}",
            opendomain_core::harness::admit(&file, &gaps).expect_err("interdit fictif refuse")
        );
        assert!(err.contains("capacités non satisfaites"), "message : {err}");
        assert!(err.contains("oracle"), "outil nommé : {err}");
    }
}
