//! `opendomain repatch <run>` — M7 : la re-planification **partielle**.
//!
//! Après attribution STRUCTURELLE, le retour canonique n'est pas un re-plan
//! entier : une sortie vérifiée est un actif — un `y_i` qui passe son gate
//! n'est pas racheté. Le `repatch` matérialise cette localité : réviser σ_i
//! du nœud fautif, le reste du graphe restant figé. La partialité est
//! DÉFINIE, pas négociée :
//!
//! - figé   : `id`, `depends_on`, `contract.input_schema` (arêtes amont
//!   gelées — les nœuds amont sont ancrés) ;
//! - révisé : `role`, `tools`, `contract.output_schema`, `criteria` ;
//! - jamais : `criteria.schema_check` (un gate ne s'éteint pas sur décision
//!   de re-planification) ni le `strict` des gates.
//!
//! Deux éditions de révision :
//! 1. **hors-ligne** (défaut, déterministe) — pas une invention : la seule
//!    détente admise est portée par l'ÉVIDENCE — tous les refus du gate pour
//!    ce nœud (traces, tentatives incluses) sont du type « champ requis `X`
//!    manquant », et `X` ∈ `required`. Tout le reste sort typé et renvoie
//!    vers `--llm` ;
//! 2. **`--llm`** — le méta-agent propose le nœud révisé (historique des
//!    gates + leçons du registre mémoire M4 + contrats amont en entrée) ;
//!    le harnais le VÉRIFIE (partialité, complétude) jusqu'à trois
//!    tentatives, avec feedback typé — la même boucle que le plan via
//!    opencode.
//!
//! `violation de sandbox` refuse la révision de nœud : le harnais a arrêté
//! la COMMANDE avant l'agent — l'élargissement de l'allowlist est une
//! décision explicite (`--allow <binaire>`), jamais automatique.
//!
//! Écriture : le graphe courant `graph/<slug>.json` (+ actif) est révisé —
//! de l'état de travail, comme sous `plan`. Le dossier de run reste
//! append-only : une ligne est ajoutée à `repatch.log`, et la reprise
//! désigne le graphe explicitement : `run <slug> --resume <run> --graph …`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use opendomain_core::harness::{AgentSpec, Failure, FailureKind, GraphFile};
use opendomain_core::harness::TraceEvent;
use serde_json::Value;

/// `opendomain repatch <run> [--llm] [--allow <binaire>]`.
pub fn run(
    run_input: &str,
    use_llm: bool,
    allow_bin: Option<&str>,
    timeout_secs: u64,
) -> Result<()> {
    let (slug, run_dir) = crate::harness::audit::resolve_run(run_input)?;
    let terminal = gather_terminal(&run_dir)?;

    match terminal.failure.kind {
        // Sandbox : faute de RÉGLAGE, pas de décomposition — l'agent n'a
        // jamais tourné ; réviser σ_i ne corrige rien.
        FailureKind::SandboxViolation => sandbox_route(&run_dir, &terminal.failure, allow_bin),

        // Grounding : faute amont (connaissance) — rejouer l'étape de
        // récupération, pas réviser la décomposition.
        FailureKind::GroundingFailure => bail!(
            "grounding_failure — la re-planification ne s'applique pas : la faute est \
amont (connaissance). Rejouer `opendomain ground {slug}` puis \
`opendomain run {slug} --resume {run}`",
            run = run_dir.display()
        ),

        // Panne amont : rien à réviser ni à relancer — reprise différée.
        FailureKind::UpstreamFailure => bail!(
            "panne amont — la re-planification ne s'applique pas : reprendre plus tard : \
`opendomain run {slug} --resume {run}`",
            run = run_dir.display()
        ),

        // Local : les retries avec feedback sont la bonne boucle — la
        // re-planification est réservée au structurel.
        FailureKind::SpecMismatch | FailureKind::CriteriaFailure => bail!(
            "échec local ({}) — pas de re-plan : la boucle locale (retry avec \
feedback) est la bonne. Reprise : `opendomain run {slug} --resume {run}`",
            terminal.failure.kind.label(),
            run = run_dir.display()
        ),

        FailureKind::ContractViolation => {
            if terminal.healed {
                bail!(
                    "l'échec terminal de `{}` est déjà absorbé — le nœud repasse son \
gate à l'exécution réelle dans un segment de reprise ultérieur",
                    terminal.failure.node
                );
            }
            repatch_contract(
                &slug,
                &run_dir,
                &terminal.failure,
                &terminal.evidence,
                use_llm,
                timeout_secs,
            )
        }
    }
}

// ------------------------------------------------------- contrat (révision) ---

fn repatch_contract(
    slug: &str,
    run_dir: &Path,
    failure: &Failure,
    evidence: &[String],
    use_llm: bool,
    timeout_secs: u64,
) -> Result<()> {
    let graph_raw = std::fs::read_to_string(run_dir.join("graph.json"))?;
    let mut file: GraphFile = serde_json::from_str(&graph_raw)?;

    let pos = file
        .nodes
        .iter()
        .position(|n| n.id == failure.node)
        .ok_or_else(|| anyhow!("nœud `{}` absent du graphe figé du run", failure.node))?;
    let orig = file.nodes[pos].clone();

    let revised = if use_llm {
        revise_with_llm(&orig, failure, evidence, timeout_secs)?
    } else {
        match relax_from_evidence(&orig, evidence) {
            Some(rev) => rev,
            None => bail!(
                "révision hors-ligne refusée — les refus du gate ne portent pas une \
détente `required` pure (évidence : {} message(s)) ; une révision proposée \
par le méta-agent est re-vérifiée par le harnais : `opendomain repatch {run} --llm`",
                evidence.len(),
                run = run_dir.display()
            ),
        }
    };

    // Admission de la révision : partialité + complétude (Garantie 3 —
    // le même contrôle que `plan` et `build`, jamais court-circuité).
    validate_revision(&orig, &revised)?;

    let downstream: Vec<String> = file
        .nodes
        .iter()
        .filter(|n| n.id != failure.node && n.depends_on.iter().any(|d| *d == failure.node))
        .map(|n| n.id.clone())
        .collect();

    let before = required_of(&orig.contract.output_schema);
    file.nodes[pos] = revised;
    let after = required_of(&file.nodes[pos].contract.output_schema);

    // Le graphe révisé reste un DAG admissible.
    let dag = opendomain_core::harness::Dag::build(&file)?;
    crate::harness::plan::admission_complete(&dag)?;

    // Écriture : état de travail, comme sous `plan`. Les runs figés
    // (`runs/*/graph.json`) ne bougent PAS — append-only.
    let graph_dir = Path::new("opendomain/harness/graph");
    std::fs::create_dir_all(graph_dir)?;
    let archive = graph_dir.join(format!("{slug}.json"));
    let active = graph_dir.join("graph.json");
    let payload = serde_json::to_string_pretty(&file)?;
    std::fs::write(&archive, &payload)?;
    std::fs::write(&active, &payload)?;

    // Trace de la révision dans le run (le seul fichier appendé).
    append_repatch_log(
        run_dir,
        &format!(
            "[{}] {} — {} — mode={} — required {:?} -> {:?} — graphe : {}",
            crate::harness::audit::chrono_stamp(),
            failure.node,
            failure.kind.label(),
            if use_llm { "llm" } else { "hors-ligne" },
            before,
            after,
            archive.display()
        ),
    )?;

    println!(
        "OK repatch : nœud `{}` révisé (évidence : {} refus(s) de gate) — graphe : {}",
        failure.node,
        evidence.len(),
        archive.display()
    );
    println!(
        "  conservé : id, depends_on = {:?}, contrat d'entrée (amonts vérifiés = actifs gelés)",
        orig.depends_on
    );
    if before != after {
        println!("  détente : required {before:?} -> {after:?} (champ conservé en `properties`)");
    }
    if !downstream.is_empty() {
        println!("  aval à ré-exécuter : {}", downstream.join(", "));
    }
    println!("  prochain pas : opendomain build {slug} — kit rafraîchi");
    println!(
        "  puis reprise : opendomain run {slug} --resume {run} --graph {graph}",
        run = run_dir.display(),
        graph = archive.display()
    );
    Ok(())
}

/// La seule détente hors-ligne admise : portée par l'évidence. `Some(nœud
/// révisé)` quand TOUS les refus du gate sont purement « champ requis `X`
/// manquant », l'union des `X` intersectant `required` ; `None` sinon.
fn relax_from_evidence(orig: &AgentSpec, evidence: &[String]) -> Option<AgentSpec> {
    if evidence.is_empty() {
        return None;
    }
    let names = missing_required_names(evidence)?;
    let req: BTreeSet<String> = orig
        .contract
        .output_schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
        .unwrap_or_default();
    let admissibles: Vec<String> = names.into_iter().filter(|n| req.contains(n)).collect();
    if admissibles.is_empty() {
        return None;
    }
    let mut rev = orig.clone();
    relax_required(&mut rev.contract.output_schema, &admissibles);
    Some(rev)
}

/// Tant que TOUT message d'évidence est purement « champ requis `X`
/// manquant », renvoie l'union des `X` — sinon `None` : les messages mixtes
/// (champs + longueurs + types) ne prouvent rien au-delà du `required`.
fn missing_required_names(evidence: &[String]) -> Option<Vec<String>> {
    let mut agg: BTreeSet<String> = BTreeSet::new();
    for msg in evidence {
        let found = missing_required(msg);
        if found.is_empty() {
            return None;
        }
        agg.extend(found);
    }
    Some(agg.into_iter().collect())
}

/// Le message du gate : `<path> : champ requis `X` manquant` ; les écarts
/// sont joints par `« ; »` (un suffixe typé éventuel est ignoré en queue).
fn missing_required(msg: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in msg.split(';') {
        let head = part.split(" — ").next().unwrap_or(part);
        let Some((_, tail)) = head.split_once("champ requis `") else {
            continue;
        };
        if let Some((name, rest)) = tail.split_once('`') {
            if rest.trim_start().starts_with("manquant") {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Retirer `names` de `required` (les `properties` restent déclarées) :
/// la détente est du contrat, pas la suppression d'un champ.
fn relax_required(schema: &mut Value, names: &[String]) {
    let Some(obj) = schema.as_object_mut() else { return };
    if let Some(req) = obj.get_mut("required").and_then(Value::as_array_mut) {
        req.retain(|r| names.iter().all(|n| r.as_str() != Some(n.as_str())));
    }
}

fn required_of(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// La partialité, mécanique : id, arêtes amont et contrat d'entrée ne se
/// négocient pas ; le gate de schéma reste armé ; la complétude est exigée
/// (le même contrôle que `plan`/`build` — Garantie 3).
fn validate_revision(orig: &AgentSpec, rev: &AgentSpec) -> Result<()> {
    if rev.id != orig.id {
        bail!("partialité : `id` modifié — il restera celui du run : {} (refusé)", orig.id);
    }
    if rev.depends_on != orig.depends_on {
        bail!("partialité : `depends_on` modifié — les arêtes amont restent figées");
    }
    if rev.contract.input_schema != orig.contract.input_schema {
        bail!("partialité : `contract.input_schema` modifié — les sorties amont vérifiées restent des actifs gelés");
    }
    if !rev.criteria.schema_check {
        bail!("partialité : `criteria.schema_check` ne peut pas être éteint — un nœud re-planifié reste sous gate strict");
    }
    let manquants: Vec<String> = crate::harness::common::agent_completeness(rev)
        .into_iter()
        .filter(|(_, ok, _)| !ok)
        .map(|(_, _, field)| field)
        .collect();
    if !manquants.is_empty() {
        bail!("nœud révisé incomplet : {}", manquants.join(", "));
    }
    Ok(())
}

// ------------------------------------------------------------------ sandbox ---

fn sandbox_route(run_dir: &Path, failure: &Failure, allow_bin: Option<&str>) -> Result<()> {
    println!("SandboxViolation — `{}` — {}", failure.node, failure.message);
    println!(
        "  la décomposition n'est pas fautive : le harnais a arrêté la COMMANDE avant \
l'agent — réviser le nœud ne corrige rien"
    );

    match allow_bin {
        Some(bin) => {
            let bin = bin.trim().to_string();
            if !bare_bin_ok(&bin) {
                bail!("`{bin}` n'est pas un nom de binaire admissible");
            }
            // Un seul canal de réglages : la config générale du workspace.
            // Lue (validée), modifiée, réécrite par le même rendu que
            // `opendomain init` — le fichier écrit est le fichier qui parse.
            let ws_dir = Path::new("opendomain");
            let mut cfg = opendomain_core::config::Config::load(ws_dir)
                .map_err(|_| anyhow!("config absente ou invalide — lancer `opendomain init` d'abord"))?;
            if cfg.sandbox.allow.iter().any(|a| a == &bin) {
                println!("  `{bin}` est déjà dans l'allowlist — rien à écrire");
            } else {
                cfg.sandbox.allow.push(bin.clone());
                std::fs::write(
                    ws_dir.join("opendomain.toml"),
                    opendomain_core::scaffold::render_config(&cfg),
                )?;
                println!("  OK `{bin}` inscrit dans [sandbox] allow de opendomain/opendomain.toml (décision explicite, jamais automatique)");
            }
            println!(
                "reprise : opendomain run --resume {run} (même graphe — rien à re-planifier)",
                run = run_dir.display()
            );
            append_repatch_log(
                run_dir,
                &format!(
                    "[{}] {} — {} — mode=sandbox --allow {bin}",
                    crate::harness::audit::chrono_stamp(),
                    failure.node,
                    failure.kind.label(),
                ),
            )?;
            Ok(())
        }
        None => {
            match sandbox_bin_from(&failure.message) {
                Some(bin) => println!(
                    "  binaire en cause `{bin}` ; pour l'inscrire une fois légitime : \
`opendomain repatch {run} --allow {bin}`",
                    run = run_dir.display()
                ),
                None => println!(
                    "  binaire en cause non reconnaissable ; \
`opendomain repatch {run} --allow <binaire>`",
                    run = run_dir.display()
                ),
            }
            bail!("allowlist inchangée — l'élargissement est une décision explicite (`--allow <binaire>`), jamais automatique")
        }
    }
}

/// Binaire nommé par le message du gate (`SandboxViolation : \`bin\` n'est pas…`).
fn sandbox_bin_from(msg: &str) -> Option<String> {
    let (_, tail) = msg.split_once("SandboxViolation :")?;
    if !tail.contains("pas dans l'allowlist") {
        return None;
    }
    let (_, rest) = tail.split_once('`')?;
    let (bin, _) = rest.split_once('`')?;
    Some(bin.trim().to_string())
}

/// Nom de binaire : lettres/chiffres, `-`, `_`, `.` — pas de chemin ni joker.
fn bare_bin_ok(bin: &str) -> bool {
    !bin.is_empty()
        && bin
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Append-only : la leçon re-planifiée s'inscrit dans le journal du run.
fn append_repatch_log(run_dir: &Path, line: &str) -> Result<()> {
    use std::io::Write;
    let path = run_dir.join("repatch.log");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(format!("{line}\n").as_bytes())?;
    Ok(())
}

// ------------------------------------------------------------------- traces ---

/// L'échec terminal : fichiers `trace*.jsonl` du run, ordre chronologique →
/// le DERNIER `Failed` toutes traces confondues.
struct TerminalFailure {
    failure: Failure,
    /// Messages des refus du gate pour ce nœud, tentatives incluses.
    evidence: Vec<String>,
    /// Le nœud a repassé son gate à l'exécution RÉELLE après ce terminal
    /// (tentative ≥ 1 — un `Passed(0)` n'est qu'une reprise ré-indexée).
    healed: bool,
}

fn gather_terminal(run_dir: &Path) -> Result<TerminalFailure> {
    let files = trace_files(run_dir)?;
    let mut events: Vec<TraceEvent> = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let ev: TraceEvent = serde_json::from_str(line)
                .map_err(|e| anyhow!("trace illisible ({}) : {e}", file.display()))?;
            events.push(ev);
        }
    }

    let Some(idx) = events
        .iter()
        .rposition(|e| matches!(e, TraceEvent::Failed { .. }))
    else {
        bail!(
            "aucun échec terminal dans les traces de {} — rien à re-planifier \
(si l'échec antérieur est déjà réglé, lancer un run neuf)",
            run_dir.display()
        );
    };
    let failure = match &events[idx] {
        TraceEvent::Failed { failure, .. } => failure.clone(),
        _ => unreachable!(),
    };
    let healed = events[idx + 1..].iter().any(|e| matches!(
        e,
        TraceEvent::Passed { node, attempts } if node == &failure.node && *attempts >= 1
    ));
    let evidence: Vec<String> = {
        let mut ev: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                TraceEvent::Retry { node, message, .. } if node == &failure.node => {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect();
        // L'évidence inclut le terminal — chronologie complète des refus.
        ev.push(failure.message.clone());
        ev
    };

    Ok(TerminalFailure { failure, evidence, healed })
}

/// Fichiers de trace ordonnés du plus ancien au plus récent :
/// `trace.jsonl` (0), `trace-resume-01.jsonl` (1), …
fn trace_files(run_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<(usize, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(run_dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let idx = if name == "trace.jsonl" {
                Some(0)
            } else if let Some(rest) = name.strip_prefix("trace-resume-") {
                rest.trim_end_matches(".jsonl").parse::<usize>().ok()
            } else {
                None
            };
            if let Some(n) = idx {
                files.push((n, run_dir.join(&name)));
            }
        }
    }
    files.sort_by_key(|(n, _)| *n);
    if files.is_empty() {
        bail!("aucune trace dans {}", run_dir.display());
    }
    Ok(files.into_iter().map(|(_, p)| p).collect())
}

// ------------------------------------------------------------------- LLM ---

/// La boucle classique (Generate → Verify → Refine, ≤ 3) mais sur UN nœud :
/// le méta-agent propose σ_i révisé, le harnais re-vérifie partialité et
/// complétude — les axes figés ne se négocient pas, même côté modèle.
fn revise_with_llm(
    orig: &AgentSpec,
    failure: &Failure,
    evidence: &[String],
    timeout_secs: u64,
) -> Result<AgentSpec> {
    // La leçon M4 du registre mémoire entre dans le prompt (l'arc
    // attribute → memoria → re-plan, fermé ici).
    let lessons = lessons_for(&orig.id);
    let mut feedback = String::from("(première tentative)");
    for _attempt in 1..=3 {
        let prompt = revise_prompt(orig, failure, evidence, &lessons, &feedback);
        let out = crate::harness::adapters::opencode_run(&prompt, timeout_secs)?;
        let Some(json) = crate::harness::adapters::extract_json(&out.stdout).map(|s| s.to_string()) else {
            feedback = String::from(
                "aucun objet JSON dans la sortie — renvoyer UNIQUEMENT l'objet JSON du nœud révisé",
            );
            continue;
        };
        match serde_json::from_str::<AgentSpec>(&json) {
            Ok(mut rev) => {
                // Les axes figés ne se négocient pas, même côté modèle.
                rev.id = orig.id.clone();
                rev.depends_on = orig.depends_on.clone();
                rev.contract.input_schema = orig.contract.input_schema.clone();
                rev.prompt = orig.prompt.clone();
                match validate_revision(orig, &rev) {
                    Ok(()) => return Ok(rev),
                    Err(e) => feedback = e.to_string(),
                }
            }
            Err(e) => {
                feedback = format!(
                    "JSON non conforme à σ_i ({e}) — renvoyer le nœud COMPLET : id, \
role, tools, contract, criteria, depends_on"
                )
            }
        }
    }
    bail!(
        "la révision --llm n'a pas été admise en 3 tentative(s) — dernier défaut : \
{feedback} ; corriger le graphe à la main puis `opendomain validate`"
    )
}

/// Leçons M4 pour ce nœud (registre mémoire) — ≤ 5, du plus récent au plus ancien.
fn lessons_for(node: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(Path::new("opendomain/harness/memory/registre.md")) else {
        return Vec::new();
    };
    let needle = format!(" | {node} | ");
    let mut out: Vec<String> = text
        .lines()
        .filter(|l| l.contains(&needle))
        .map(String::from)
        .rev()
        .take(5)
        .collect();
    out.reverse();
    out
}

fn revise_prompt(
    orig: &AgentSpec,
    failure: &Failure,
    evidence: &[String],
    lessons: &[String],
    feedback: &str,
) -> String {
    format!(
        "# Re-planification PARTIELLE (OpenDomain, M7)\n\
\n\
Une décomposition d'agents a échoué sur un nœud précis ; l'attribution est \
STRUCTURELLE (violation de contrat) : les retries locaux sont épuisés, la \
spécification σ du nœud est fautive. Révise UNIQUEMENT ce nœud : mêmes `id` \
et `depends_on` (les nœuds amont sont vérifiés et resteront des actifs \
gelés), même `contract.input_schema`. RENVOIE UNIQUEMENT l'objet JSON du \
nœud complet révisé.\n\
\n\
## Nœud original σ_i\n```json\n{orig}\n```\n\
\n\
## Échec terminal — {kind} : {message}\n\
## Historique complet des refus du gate (même nœud, tentative après tentative)\n{evidence}\n\
## Leçons du passé (mémoire, même nœud)\n{lessons}\n\
\n\
## Feedback du harnais sur la proposition précédente\n{feedback}\n\
\n\
## Consignes\n\
- `id` et `depends_on` : INCHANGÉS (ils seront forcés) ;\n\
- `contract.output_schema` : conforme à ce que cet agent PEUT produire — les nœuds aval liront ces champs ;\n\
- `criteria.schema_check` reste true : aucun détournement du gate est admissible ;\n\
- rôle, whitelist d'outils, critères : complets et cohérents (le harnais les vérifie avant écriture).\n",
        orig = serde_json::to_string_pretty(orig).unwrap_or_default(),
        kind = failure.kind.label(),
        message = failure.message,
        evidence = bullet_lines(evidence),
        lessons = bullet_lines(lessons),
        feedback = feedback,
    )
}

/// Puces `« - … »` ; vide → `(aucune)` — lisibilité du prompt.
fn bullet_lines(items: &[String]) -> String {
    if items.is_empty() {
        "(aucune)".to_string()
    } else {
        items
            .iter()
            .map(|l| format!("- {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ------------------------------------------------------------------- tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use opendomain_core::harness::{Contract, Criteria};
    use serde_json::json;

    fn node(schema: Value) -> AgentSpec {
        AgentSpec {
            id: "verificateur".into(),
            role: "vérifie".into(),
            tools: vec!["read".into()],
            contract: Contract {
                input_schema: json!({"type": "object"}),
                output_schema: schema,
                forbidden: vec![],
            },
            criteria: Criteria::default(),
            ..Default::default()
        }
    }

    #[test]
    fn la_detente_hors_ligne_porte_le_motif_required_pur() {
        let orig = node(json!({
            "type": "object",
            "required": ["verdict", "signature"],
            "properties": { "verdict": {"type": "string"}, "signature": {"type": "string"} }
        }));
        let ev = vec![
            "$ : champ requis `signature` manquant".into(),
            "$ : champ requis `signature` manquant".into(),
        ];
        let rev = relax_from_evidence(&orig, &ev).expect("détente admise");
        assert_eq!(required_of(&rev.contract.output_schema), vec!["verdict"]);
        // le champ reste déclaré en `properties`
        assert!(rev
            .contract
            .output_schema
            .get("properties")
            .and_then(|p| p.get("signature"))
            .is_some());
    }

    #[test]
    fn la_detente_refuse_levidence_mixte() {
        let orig = node(json!({
            "type": "object",
            "required": ["verdict", "raison"],
            "properties": { "verdict": {"type": "string"}, "raison": {"type": "string"} }
        }));
        // un autre motif mêlé aux mêmes tentatives → aucune détente
        let ev = vec![
            "$ : champ requis `raison` manquant".into(),
            "$.raison : longueur < minLength (10)".into(),
        ];
        assert!(relax_from_evidence(&orig, &ev).is_none());
    }

    #[test]
    fn la_detente_refuse_levidence_hors_contrat() {
        let orig = node(json!({
            "type": "object",
            "required": ["verdict"],
            "properties": { "verdict": {"type": "string"} }
        }));
        let ev = vec!["$ : champ requis `signature` manquant".into()];
        // `signature` n'est pas un `required` actuel → l'évidence ne porte rien
        assert!(relax_from_evidence(&orig, &ev).is_none());
    }

    #[test]
    fn la_partialite_refuse_ce_qui_est_fige() {
        let orig = node(json!({
            "type": "object",
            "required": ["a"],
            "properties": { "a": {"type": "string"} }
        }));
        let mut rev = orig.clone();
        assert!(validate_revision(&orig, &rev).is_ok());

        rev.depends_on = vec!["autre".into()];
        assert!(validate_revision(&orig, &rev).is_err(), "depends_on figé");

        let mut rev = orig.clone();
        rev.contract.input_schema = json!({"type": "array"});
        assert!(validate_revision(&orig, &rev).is_err(), "entrée figée");

        let mut rev = orig.clone();
        rev.criteria.schema_check = false;
        assert!(validate_revision(&orig, &rev).is_err(), "gate ne s'éteint pas");
    }

    #[test]
    fn la_partialite_exige_les_champs_et_le_gate() {
        let orig = node(json!({
            "type": "object",
            "required": ["a"],
            "properties": { "a": {"type": "string"} }
        }));
        // critères vides → incomplet (Garantie 3)
        let mut rev = orig.clone();
        rev.criteria = Criteria {
            schema_check: false,
            ..Criteria::default()
        };
        assert!(validate_revision(&orig, &rev).is_err(), "gate désarmé + critères vides");
    }

    #[test]
    fn la_config_sandbox_recoit_le_binaire_une_fois() {
        // Le canal unique : la section [sandbox] de la config générale.
        let mut cfg = opendomain_core::config::Config::parse(
            "[gates]\nstrict = true\n\n[sandbox]\nenabled = true\nallow = [\"cargo\", \"npm\"]\n",
        )
        .unwrap();
        assert!(!cfg.sandbox.allow.iter().any(|a| a == "curl"));
        cfg.sandbox.allow.push("curl".into());
        assert_eq!(cfg.sandbox.allow, vec!["cargo", "npm", "curl"]);
        // idempotent : déjà présent, on ne duplique pas
        let already: Vec<String> = cfg.sandbox.allow.clone();
        if !cfg.sandbox.allow.iter().any(|a| a == "curl") {
            cfg.sandbox.allow.push("curl".into());
        }
        assert_eq!(cfg.sandbox.allow, already);
        // le rendu de l'init est le rendu de la réécriture : le fichier
        // écrit est le fichier qui parse
        let text = opendomain_core::scaffold::render_config(&cfg);
        let back = opendomain_core::config::Config::parse(&text).unwrap();
        assert!(back.sandbox.allow.contains(&"curl".to_string()));
    }

    #[test]
    fn le_binaire_est_nomme_depuis_le_message() {
        let msg = "SandboxViolation : `curl` n'est pas dans l'allowlist [\"cargo\"] — \
ajouter au [sandbox] allow de opendomain/opendomain.toml si légitime";
        assert_eq!(sandbox_bin_from(msg).as_deref(), Some("curl"));
        // message d'un autre type → non reconnu
        assert!(sandbox_bin_from("SandboxViolation : délai atteint").is_none());
        // nom de binaire admissible
        assert!(bare_bin_ok("rg.exe"));
        assert!(!bare_bin_ok("../evil") && !bare_bin_ok("a b"));
    }
}
