//! Sonde de conformité runtime (REQ-005, première pierre) — le miroir
//! automatisé de l'expérience UNC-001 (EVD-005, 2026-10-23).
//!
//! Ce que la sonde prouve : un agent matérialisé par le dépôt est **chargé
//! par l'éditeur réel** avec le ruleset écrit (deny universel d'abord, allow
//! projetés ensuite) — pas seulement écrit sur le disque. Ce que la sonde
//! ne prouve pas : le refus d'une action hors whitelist DURANT un run réel
//! (il exige un appel LLM facturé — il reste une étape manuelle/CI dédiée,
//! documentée dans UNC-001/EVD-005).
//!
//! Version épinglée par **famille** (`v2.*`) : la montée de patch est
//! autorisée, une rupture de famille ou l'absence de l'agent témoin est une
//! faute de conformité. La version exacte observée reste enregistrée dans
//! les registres (EVD-005) et est ré-imprimée à chaque sonde.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use opendomain_core::harness::AgentSpec;

use super::capabilities::projected_actions;
use super::materializer::define_opencode_agent_in;
use crate::harness::common::NodeBindings;

/// La variante cwd-explicite de `transport::run_captured` (S4) : la même
/// discipline de capture et d'échéance, un répertoire projet imposé — la
/// sonde ne doit jamais lire le `.opencode` du dépôt hôte. À fusionner
/// dans `transport` si un second besoin apparaît.
fn run_captured_in(dir: &Path, bin: &str, args: &[&str], timeout_secs: u64) -> Result<String> {
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
    command
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = command
        .spawn()
        .map_err(|e| anyhow!("lancement de `{bin}` impossible : {e}"))?;
    let out = child.stdout.take().expect("stdout pipé");
    let err = child.stderr.take().expect("stderr pipé");
    let drain = |mut r: Box<dyn std::io::Read + Send>| {
        thread::spawn(move || {
            let mut buf = String::new();
            let _ = std::io::Read::read_to_string(&mut r, &mut buf);
            buf
        })
    };
    let t_out = drain(Box::new(out));
    let t_err = drain(Box::new(err));
    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                let stdout = t_out.join().unwrap_or_default();
                let stderr = t_err.join().unwrap_or_default();
                if !status.success() {
                    bail!(
                        "`opencode {args:?}` a échoué : {}",
                        stderr.lines().last().unwrap_or("(cause inconnue)")
                    );
                }
                return Ok(stdout);
            }
            None if start.elapsed() >= Duration::from_secs(timeout_secs.max(1)) => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("`opencode {args:?}` a dépassé {timeout_secs}s — la sonde n'est pas un appel LLM, ce délai signale une installation cassée");
            }
            None => thread::sleep(Duration::from_millis(200)),
        }
    }
}

/// Répertoire de sonde — toujours hors du dépôt hôte, unique par appel.
fn probe_dir(tag: &str) -> Result<PathBuf> {
    let base = std::env::temp_dir();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let pid = std::process::id();
    let d = base.join(format!("opendomain-conformite-{tag}-{pid}-{ts}"));
    std::fs::create_dir_all(d.join(".opencode").join("agent"))?;
    Ok(d)
}

/// L'agent témoin dans la liste rendue par `debug agents`.
fn find_temoin<'a>(listed: &'a serde_json::Value, agent_id: &str) -> Option<&'a serde_json::Value> {
    listed
        .as_array()?
        .iter()
        .find(|x| x.get("id").and_then(|i| i.as_str()) == Some(agent_id))
}

/// La sonde : matérialise un agent témoin (format du dépôt) dans un
/// répertoire propre, interroge l'éditeur réel (`opencode debug agents`)
/// et confronte le ruleset chargé au ruleset écrit.
///
/// Renvoie la version de l'éditeur observée (pour la trace). Échec typé si
/// l'agent témoin est absent, si la famille de version n'est plus `v2.*`,
/// ou si le ruleset chargé diverge du ruleset écrit.
pub fn sonde_conformite_editeur(timeout_secs: u64) -> Result<String> {
    if crate::harness::doctor::which("opencode").is_none() {
        bail!(
            "ConformiteIgnoree : `opencode` absent du PATH — la sonde de conformité n'est pas \
skippée en silence ; installez l'éditeur (opendomain doctor) pour l'exécuter"
        );
    }
    let dir = probe_dir("sonde")?;

    // Échauffement : la première invocation dans un répertoire neuf a
    // observé une liste vide (leçon EVD-005 : le comportement est « à deux
    // temps ») — forcer l'initialisation AVANT le témoin, puis lister.
    let _ = run_captured_in(&dir, "opencode", &["debug", "agents"], timeout_secs)?;

    // ---- témoin au format du dépôt ---------------------------------------
    let spec = AgentSpec {
        id: "temoin".into(),
        role: "agent témoin de la sonde de conformité".into(),
        tools: vec!["read".into(), "grep".into()],
        ..Default::default()
    };
    let bindings = NodeBindings { model: None, steps: None };
    let agent_root = dir.join(".opencode").join("agent");
    let agent_id = define_opencode_agent_in(&spec, "conformite", "", &bindings, &agent_root)?;
    let ecrit = projected_actions(&spec.tools)?;

    // ---- ce que l'éditeur RÉEL a chargé -----------------------------------
    let version = run_captured_in(&dir, "opencode", &["--version"], timeout_secs)?
        .split_whitespace()
        .find(|t| t.starts_with('v') && t[1..].chars().next().map_or(false, |c| c.is_ascii_digit()))
        .unwrap_or_default()
        .to_string();
    if version.is_empty() {
        bail!(
            "ConformiteVersion : numéro de version non trouvé dans la sortie de `opencode \
--version` — installation cassée ou format de sortie changé (EVD-005)"
        );
    }
    if !version.starts_with("v2.") {
        bail!(
            "ConformiteVersion : famille d'éditeur inattendue (`{version}`) — la sonde \
atteste la famille v2 ; une autre famille demande une re-qualification (EVD-005)"
        );
    }
    let listed = run_captured_in(&dir, "opencode", &["debug", "agents"], timeout_secs)?;
    let agents: serde_json::Value = serde_json::from_str(&listed)
        .map_err(|e| anyhow!("sortie de `debug agents` non parsable : {e}"))?;
    let entry = find_temoin(&agents, &agent_id)
        .map(serde_json::Value::clone)
        .unwrap_or(serde_json::Value::Null);
    let entry = if entry.is_null() {
        // Re-lecture bornée : le chargement a observé un délai ; on ne
        // déclare pas la fiction sans avoir épuisé les re-lectures.
        let mut entry = entry;
        for tentative in 1..=3 {
            thread::sleep(Duration::from_millis(300 * tentative));
            let listed = run_captured_in(&dir, "opencode", &["debug", "agents"], timeout_secs)?;
            let parsed: serde_json::Value = serde_json::from_str(&listed)
                .map_err(|e| anyhow!("sortie de `debug agents` non parsable : {e}"))?;
            if let Some(found) = find_temoin(&parsed, &agent_id) {
                entry = found.clone();
                break;
            }
            eprintln!("  ↻ sonde : tentative {tentative} — témoin pas encore listé");
        }
        entry
    } else {
        entry
    };
    if entry.is_null() {
        bail!(
            "ConformiteAbsente : l'agent témoin `{agent_id}` matérialisé au format du dépôt \
n'a PAS été chargé par l'éditeur {version} (re-lectures épuisées) — la \
matérialisation est une fiction (voir UNC-001/EVD-005)"
        );
    }

    // ---- le ruleset net == le ruleset voulu --------------------------------
    // Le runtime PRÉFIXE ses défauts (`*:allow`, garde-fous `.env`) avant
    // les règles du fichier, et ajoute `browser: deny` à la fin (EVD-005).
    // Avec « dernier match gagnant », l'équivalent net du ruleset écrit est
    // donc préservé SI notre deny universel arrive après les défauts —
    // c'est la sémantique nette qu'on juge, jamais la position littérale.
    let rules = entry
        .get("permissions")
        .and_then(|p| p.as_array())
        .ok_or_else(|| anyhow!("agent témoin sans ruleset lisible"))?;
    let as_rule = |r: &serde_json::Value| -> (String, String, String) {
        (
            r.get("action").and_then(|a| a.as_str()).unwrap_or("").to_string(),
            r.get("resource").and_then(|a| a.as_str()).unwrap_or("").to_string(),
            r.get("effect").and_then(|a| a.as_str()).unwrap_or("").to_string(),
        )
    };
    let rules_vec: Vec<(String, String, String)> =
        rules.iter().map(|r| as_rule(r)).collect();
    // C1 — notre deny universel est bien chargé.
    let deny_pos = rules_vec
        .iter()
        .position(|(a, _, eff)| a == "*" && eff == "deny")
        .ok_or_else(|| {
            anyhow!(
                "ConformiteEcart : aucun deny universel dans le ruleset chargé par \
l'éditeur {version} — le moindre privilège projeté est une fiction"
            )
        })?;
    // C2 — aucun deny non-universel écrit : le runtime ne doit pas voir
    // passer autre chose que le deny `*` de la matérialisation.
    if let Some((a, _, _)) = rules_vec
        .iter()
        .take(deny_pos)
        .rev()
        .find(|(a, _, eff)| eff == "deny" && a != "browser" && a != "*")
    {
        bail!("ConformiteEcart : deny non-universel inattendu (`{a}`) avant le nôtre");
    }
    // C3 — chaque action projetée est autorisée, et rien d'écrit au-delà :
    // pas de règle allow générique APRÈS notre deny (elle ré-ouvrirait tout).
    if rules_vec
        .iter()
        .skip(deny_pos + 1)
        .any(|(a, _, eff)| eff == "allow" && a == "*")
    {
        bail!(
            "ConformiteEcart : une règle générique allow arrive APRÈS le deny universel \
chargé — elle ré-ouvrirait tout ; la matérialisation ne plus corriger au-delà"
        );
    }
    for action in &ecrit {
        let ok = rules.iter().any(|r| {
            let (a, res, eff) = as_rule(r);
            a == *action && res == "*" && eff == "allow"
        });
        if !ok {
            bail!(
                "ConformiteEcart : l'action `{action}` projetée par la whitelist n'est pas \
autorisée dans le ruleset chargé par l'éditeur {version}"
            );
        }
    }
    // C4 — sonde inverse : une action NON projetée doit tomber sur le deny
    // net (dernière règle matchant `webfetch` = notre deny universel, car
    // aucun allow webfetch n'est écrit après lui).
    let fuir = "webfetch";
    let net_fuir = rules_vec
        .iter()
        .position(|(a, _, _)| a == fuir)
        .map(|p| rules_vec[p..].first())
        .unwrap_or(None);
    match net_fuir {
        Some((a, _, eff)) if a == fuir && eff == "allow" => {
            bail!("ConformiteEcart : `{fuir}` (hors whitelist) est nettement autorisé — moindre privilège non appliqué");
        }
        _ => {}
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La sonde tourne pour de vrai quand l'éditeur est présent — c'est le
    /// test de conformité épinglé par famille (REQ-005, première pierre).
    /// Sans éditeur : refus typé, jamais un skip silencieux.
    #[test]
    fn la_sonde_prouve_le_chargement_reel() {
        match sonde_conformite_editeur(60) {
            Ok(version) => {
                assert!(version.starts_with("v2."), "famille v2 attendue : {version}");
                println!("conformité attestée sur {version}");
            }
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("ConformiteIgnoree"),
                    "échec autre qu'absence d'éditeur : {msg}"
                );
                eprintln!("sonde ignorée (éditeur absent) : {msg}");
            }
        }
    }
}
