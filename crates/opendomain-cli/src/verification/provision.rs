//! `opendomain provision` — la détection (repository-only), les règles de
//! sélection ordonnées R1-R4, le refus de §10.13, puis les spécifications
//! exécutables générées : scaffolds `.feature` taggés et la décision
//! (ProvisioningDecision) enregistrée comme artefact DEC. Rien n'est
//! installé.

use anyhow::{bail, Context as _, Result};
use opendomain_core::ids::IdKind;
use opendomain_core::provisioning::{self, BddCriteria, CriterionRef, Detection, Selection};
use opendomain_core::Workspace;
use std::collections::{BTreeMap, BTreeSet};

use super::common::{today, REQUIREMENT_KINDS};

/// Repository-only stack detection (§10.3: "Detection reads only the
/// repository. No network, no prompting."),
fn detection(root: &std::path::Path) -> Detection {
    let single: &[&str] = &[
        "Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        "package.json",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "go.mod",
        "Gemfile",
        "composer.json",
    ];
    let mut manifests: Vec<(String, String)> = single
        .iter()
        .filter_map(|name| {
            std::fs::read_to_string(root.join(name))
                .ok()
                .map(|text| ((*name).to_string(), text))
        })
        .collect();
    // `*.csproj` sits anywhere shallow; scan the root and its direct children.
    let mut collect_csproj = |dir: &std::path::Path| {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "csproj").unwrap_or(false) {
                    if let (Some(name), Ok(text)) = (
                        p.file_name().map(|f| f.to_string_lossy().to_string()),
                        std::fs::read_to_string(&p),
                    ) {
                        manifests.push((name, text));
                    }
                }
            }
        }
    };
    collect_csproj(root);
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                collect_csproj(&e.path());
            }
        }
    }
    manifests.sort_by(|a, b| a.0.cmp(&b.0));

    // Application-boundary probe: a fixed list, deliberately small. An
    // undetected boundary is the §10.13 refusal, not a guess.
    let probes = [
        "src/main.rs",
        "src/main.ts",
        "src/main.js",
        "src/main.py",
        "main.go",
        "Program.cs",
        "app.py",
        "manage.py",
        "index.php",
    ];
    let entry_signatures: Vec<String> = probes
        .iter()
        .filter(|p| root.join(p).exists())
        .map(|p| (*p).to_string())
        .collect();

    Detection {
        manifests,
        entry_signatures,
    }
}

/// The DEC records' next free number (read-only look, mirrors `opendomain new`:
/// reserves nothing, so a race is visible rather than silent).
fn next_dec_id(index: &opendomain_core::workspace::Index) -> opendomain_core::Id {
    let mut max = 0u32;
    for a in index.of_kind(IdKind::Dec) {
        let text = a.id.to_string();
        let digits = text
            .split('-')
            .last()
            .map(|seg| {
                seg.chars()
                    .all(|c| c.is_ascii_digit())
                    .then(|| seg.to_string())
            })
            .flatten()
            .unwrap_or_default();
        if let Ok(n) = digits.parse::<u32>() {
            max = max.max(n);
        }
    }
    opendomain_core::Id::parse(&format!("DEC-{:03}", max + 1))
        .unwrap_or_else(|_| opendomain_core::Id::parse("DEC-999").unwrap())
}

/// Which criteria attach to which requirement (§10.8). Both declared
/// directions compose: an AC record derived *from* the requirement attaches to
/// it, and a requirement's `verifies` list can name criteria not (yet)
/// recorded — those generate placeholders, never invented prose.
fn criteria_per_requirement(
    index: &opendomain_core::workspace::Index,
) -> BTreeMap<String, Vec<CriterionRef>> {
    let mut map: BTreeMap<String, Vec<CriterionRef>> = BTreeMap::default();
    let requirement_kinds: BTreeSet<String> = REQUIREMENT_KINDS
        .iter()
        .map(|k| k.prefix().to_string())
        .collect();

    // AC side: attach to requirement-kind parents.
    for ac in index.of_kind(IdKind::Ac) {
        for parent in ac.upstream() {
            if requirement_kinds.contains(&parent.kind().prefix().to_string()) {
                map.entry(parent.to_string())
                    .or_default()
                    .push(CriterionRef {
                        id: ac.id.clone(),
                        text: Some(ac.title.clone()),
                    });
            }
        }
    }

    // Requirement side: declared `verifies` ids that no AC record covers.
    for req in REQUIREMENT_KINDS.iter().flat_map(|k| index.of_kind(*k)) {
        if let Some(list) = req.links.get(&opendomain_core::Relation::Verifies) {
            let entry = map.entry(req.id.to_string()).or_default();
            for target in list {
                if target.kind() != IdKind::Ac {
                    continue;
                }
                let declared = target.to_string();
                if !entry.iter().any(|c| c.id.to_string() == declared) {
                    entry.push(CriterionRef {
                        id: target.clone(),
                        text: None,
                    });
                }
            }
        }
    }
    map
}

/// Appends a markdown section at the end of a registry file, creating the file
/// with `header` when it does not exist. Append-only: existing content is
/// never rewritten, because rewriting a registry to add a decision would
/// teach every reader to stop trusting the file.
fn append_section(path: &std::path::Path, header: &str, section: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut out = std::fs::read_to_string(path).unwrap_or_else(|_| header.to_string());
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push_str("\n");
    out.push_str(section);
    std::fs::write(path, out).with_context(|| format!("append {}", path.display()))
}

fn append_failure_row(path: &std::path::Path, row: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut out = if std::fs::metadata(path).is_ok() {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        provisioning::FAILURE_LOG_HEADER.to_string()
    };
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push_str(row);
    std::fs::write(path, out).with_context(|| format!("append {}", path.display()))
}

/// `opendomain provision [--dry-run]`
pub fn provision(
    ws: &Workspace,
    index: &opendomain_core::workspace::Index,
    dry_run: bool,
) -> Result<()> {
    let date = today();
    // §10.3: detection reads the *repository*, not the knowledge workspace —
    // manifests and entry probes live at the project root.
    let detection = detection(&ws.project_root);
    let cfg = &ws.config.verification;
    let named_actor = index.of_kind(IdKind::Uc).iter().any(|uc| {
        uc.field("primary-actor")
            .map_or(false, |s| !s.trim().is_empty())
    });
    let input = BddCriteria {
        behavioural_acs: index.of_kind(IdKind::Ac).len(),
        minimum: cfg.minimal_acceptance_criteria as usize,
        named_actor_use_case: named_actor,
        bdd_off: cfg.bdd_off,
        experimental: cfg.experimental,
        boundary_detected: !detection.entry_signatures.is_empty(),
    };
    let summary = detection.summary();
    let selection = provisioning::select(&detection, &input);

    println!("detection (repository-only; recorded as INFERRED, MEDIUM confidence): {summary}");
    println!(
        "gate inputs: {} behavioural ACs (minimum {}), named-actor use case: {}, BDD-off: {}, experimental: {}, boundary detected: {}",
        input.behavioural_acs,
        input.minimum,
        named_actor,
        cfg.bdd_off,
        cfg.experimental,
        input.boundary_detected
    );

    match &selection {
        Selection::Refused(refusal) => {
            if dry_run {
                println!("\n[dry-run] would refuse and record the decision; nothing written.");
                return Ok(());
            }
            let dec_id = next_dec_id(index);
            let decisions = ws.path(IdKind::Dec.default_path());
            append_section(
                &decisions,
                "# Domain decisions\n",
                &provisioning::refusal_record(&dec_id, refusal, &summary, &date),
            )?;
            let log = ws.path("verification/provisioning-failures.md");
            append_failure_row(&log, &provisioning::failure_row(&date, refusal))?;
            println!();
            for r in &refusal.reasons {
                println!("refused: {r}");
            }
            println!("alternative on record: {}", refusal.alternative);
            println!(
                "decision recorded: {dec_id} ; failure log: opendomain/verification/provisioning-failures.md"
            );
            bail!(
                "provisioning refused — the decision record names the alternative mechanism (Section 10.13)"
            );
        }
        Selection::Generate {
            rule,
            stack,
            existing,
        } => {
            let per_req = criteria_per_requirement(index);
            let mut requirements: Vec<&opendomain_core::Artifact> = REQUIREMENT_KINDS
                .iter()
                .flat_map(|k| index.of_kind(*k))
                .filter(|a| {
                    a.is_live()
                        && per_req
                            .get(&a.id.to_string())
                            .map_or(false, |c| !c.is_empty())
                })
                .collect();
            requirements.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

            if dry_run {
                if requirements.is_empty() {
                    println!("\n[dry-run] no requirement carries an attached criterion — nothing to generate.");
                }
                for r in &requirements {
                    let rel = provisioning::feature_path(r);
                    println!(
                        "[dry-run] would write opendomain/{rel} ({} criteria)",
                        per_req.get(&r.id.to_string()).map_or(0, |c| c.len())
                    );
                }
                println!("[dry-run] would write opendomain/verification/README.md");
                println!(
                    "[dry-run] would record the decision as a DEC artifact — nothing installed."
                );
                return Ok(());
            }

            let mut written = 0usize;
            for r in &requirements {
                let criteria = per_req.get(&r.id.to_string()).cloned().unwrap_or_default();
                let feature = provisioning::feature_for(r, &criteria);
                let rel = provisioning::feature_path(r);
                let path = ws.path(&rel);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create {}", parent.display()))?;
                }
                std::fs::write(&path, feature)
                    .with_context(|| format!("write {}", path.display()))?;
                written += 1;
            }
            if written == 0 {
                println!(
                    "\nno live requirement carries an attached criterion yet — the scaffolds would be hollow\n\
                     (PP4). Record specifications first; `opendomain rigour` audits the declarations."
                );
            }
            std::fs::write(
                ws.path("verification/README.md"),
                provisioning::readme_for(&selection, &summary, &date),
            )
            .with_context(|| "write opendomain/verification/README.md")?;

            // §10.2: the decision is an artifact (kind DEC), not a log line.
            // A repeat run with an identical selection appends no duplicate.
            let already = index
                .of_kind(IdKind::Dec)
                .iter()
                .any(|d| d.title.contains("Provisioning decision"));
            if already {
                println!("decision: already on record — no duplicate appended");
            } else {
                let dec_id = next_dec_id(index);
                let decisions = ws.path(IdKind::Dec.default_path());
                append_section(
                    &decisions,
                    "# Domain decisions\n",
                    &provisioning::decision_record(&dec_id, &selection, &summary, &date),
                )?;
                println!(
                    "decision: {dec_id} recorded in opendomain/{} (INFERRED/MEDIUM — detection only)",
                    IdKind::Dec.default_path()
                );
            }

            println!();
            println!("selection: {}", rule.as_str(*stack, *existing));
            if let Some(row) = stack {
                println!(
                    "matrix row (Section 10.4): unit={}; bdd={}; contract={}; property={}",
                    row.unit, row.bdd, row.contract, row.property
                );
            }
            println!("written: opendomain/verification/README.md + {written} feature file(s)");
            println!(
                "nothing installed: the recorded frameworks are a decision, not an install; step\n\
                 definitions are bound by hand at an application port (Section 9.4) — PP4 holds."
            );
            Ok(())
        }
    }
}
