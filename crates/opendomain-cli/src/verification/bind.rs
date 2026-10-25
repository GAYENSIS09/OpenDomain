//! `opendomain bind` — l'audit de provenance des bindings (§10.10) avec la
//! profondeur de vérification (§10.9). Les checks G8-B5/B6 sont rapportés
//! comme *findings*, pas comme des règles de gate : aucun workspace legacy
//! n'échoue soudain à une gate qu'il n'a jamais demandée.

use anyhow::Result;
use opendomain_core::ids::IdKind;
use opendomain_core::verification::{self, BindingProvenance, VerificationDepth};
use std::collections::BTreeSet;

/// `opendomain bind` — the binding-provenance audit: findings, never gate
/// rules. The F13 measurement (why INFERRED bindings may or may not ever be
/// allowed to close an edge) falls out of the inferred-share number.
pub fn bind(index: &opendomain_core::workspace::Index, json: bool) -> Result<()> {
    let glossary: Vec<String> = index
        .of_kind(IdKind::Term)
        .iter()
        .map(|t| t.title.to_lowercase())
        .collect();

    let mut tests: Vec<&opendomain_core::Artifact> =
        index.of_kind(IdKind::Test).into_iter().collect();
    tests.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

    let mut rows = Vec::new();
    let mut executed = 0usize;
    let mut discovered = 0usize;
    let mut declared = 0usize;
    let mut prov_none = 0usize;
    let mut prov_generated = 0usize;
    let mut prov_annotated = 0usize;
    let mut prov_attributed = 0usize;
    let mut prov_inferred = 0usize;
    let mut inferred_share = 0usize;
    let mut unconfirmed_inferred = Vec::new();
    let mut claimed_executed = Vec::new();
    let mut by_naming = Vec::new();

    let mut covered: BTreeSet<String> = BTreeSet::new();

    for t in &tests {
        let binding = verification::read_binding(t)?;
        let automation = t.field("automation").is_some();
        let declared_executed_without_command = matches!(
            verification::declared_depth(t).ok().flatten(),
            Some(VerificationDepth::Executed)
        ) && !automation;
        let depth = verification::inferred_depth(t);
        match depth {
            VerificationDepth::Executed => executed += 1,
            VerificationDepth::Discovered => discovered += 1,
            VerificationDepth::Declared => declared += 1,
        }
        match &binding {
            None => {
                prov_none += 1;
                if let Some(_term) = name_match(&t.title, &glossary) {
                    inferred_share += 1; // F13 dataset
                }
            }
            Some(b) => match b.provisionance {
                BindingProvenance::Generated => {
                    prov_generated += 1;
                    if b.to.kind() == IdKind::Ac {
                        covered.insert(b.to.to_string());
                    }
                }
                BindingProvenance::Annotated => {
                    prov_annotated += 1;
                    if b.to.kind() == IdKind::Ac {
                        covered.insert(b.to.to_string());
                    }
                }
                BindingProvenance::Attributed => {
                    prov_attributed += 1;
                    if b.to.kind() == IdKind::Ac {
                        covered.insert(b.to.to_string());
                    }
                }
                BindingProvenance::Inferred => {
                    prov_inferred += 1;
                    inferred_share += 1;
                    if b.to.kind() == IdKind::Ac {
                        covered.insert(b.to.to_string());
                    }
                    if !b.confirmed {
                        unconfirmed_inferred.push(format!(
                            "{} → {}: INFERRED binding, unconfirmed — cannot close the traceability edge (G8-B6, design condition)",
                            t.id, b.to
                        ));
                    }
                }
            },
        }
        if binding.is_none() {
            if let Some(_term) = name_match(&t.title, &glossary) {
                by_naming.push(t.id.to_string());
            }
        }
        if declared_executed_without_command
            || (t.status == opendomain_core::ArtifactStatus::Verified && !automation)
        {
            claimed_executed.push(t.id.to_string());
        }
        // The plain `verifies = [...]` relation still counts as coverage —
        // but its provenance is *none* until a binding table says otherwise.
        if let Some(list) = t.links.get(&opendomain_core::Relation::Verifies) {
            for id in list {
                if id.kind() == IdKind::Ac {
                    covered.insert(id.to_string());
                }
            }
        }

        rows.push(serde_json::json!({
            "id": t.id.to_string(),
            "status": t.status.to_string(),
            "depth": depth.as_str(),
            "binding": binding.as_ref().map(|b| serde_json::json!({
                "to": b.to.to_string(),
                "provisionance": b.provisionance.as_str(),
                "confidence": b.provisionance.confidence(),
                "confirmed": b.confirmed,
            })),
            "name_match": name_match(&t.title, &glossary),
        }));
    }

    let uncovered: Vec<String> = index
        .of_kind(IdKind::Ac)
        .iter()
        .filter(|ac| !covered.contains(&ac.id.to_string()))
        .map(|ac| ac.id.to_string())
        .collect();

    let total = tests.len();
    if json {
        let report = serde_json::json!({
            "tests": total,
            "depth": { "executed": executed, "discovered": discovered, "declared": declared },
            "provenance": {
                "none": prov_none,
                "generated": prov_generated,
                "annotated": prov_annotated,
                "attributed": prov_attributed,
                "inferred": prov_inferred,
            },
            "inferred_share": if total > 0 { inferred_share as f64 / total as f64 } else { 0.0 },
            "unconfirmed_inferred": unconfirmed_inferred,
            "claimed_executed_without_command": claimed_executed,
            "inferred_by_naming_only": by_naming,
            "criteria_without_test": uncovered,
            "rows": rows,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!(
        "Binding provenance & verification depth (Sections 10.9/10.10) — audit, not a gate:\n"
    );
    println!(
        "tests: {total} — depth: executed {executed}/ discovered {discovered}/ declared {declared}"
    );
    println!(
        "  provenance: none {prov_none}, generated {prov_generated}, annotated {prov_annotated}, attributed {prov_attributed}, inferred {prov_inferred}"
    );
    if total > 0 {
        println!("  inferred share (F13 dataset): {inferred_share}/{total}");
    }
    let mut flagged = false;
    if !unconfirmed_inferred.is_empty() {
        flagged = true;
        println!("\nunconfirmed INFERRED bindings (design check G8-B6 — an unconfirmed binding cannot close an edge):");
        for l in &unconfirmed_inferred {
            println!("  - {l}");
        }
    }
    if !claimed_executed.is_empty() {
        flagged = true;
        println!("\nclaimed executed without a recorded command (design check G8-B5 — claimed-versus-actual):");
        for id in &claimed_executed {
            println!("  - {id} — declare `automation` or demote the depth to discovered");
        }
    }
    if !by_naming.is_empty() {
        flagged = true;
        println!("\nnaming-convention candidates only (record `binding = {{ to = …, provisionance = \"INFERRED\" }}` and confirm after review):");
        for id in by_naming.iter().take(8) {
            println!("  - {id}");
        }
        if by_naming.len() > 8 {
            println!("  … +{} more", by_naming.len() - 8);
        }
    }
    if !uncovered.is_empty() {
        flagged = true;
        let shown: Vec<String> = uncovered.iter().take(5).cloned().collect();
        println!(
            "\ncriteria without any test: {}{} — a tagged-and-green chain over an uncovered criterion is not coverage",
            shown.join(", "),
            if uncovered.len() > 5 { format!(" … +{}", uncovered.len() - 5) } else { String::new() }
        );
    }
    if !flagged {
        println!(
            "\nnothing flagged: every binding is explicit or confirmed, every depth is earned."
        );
    }
    println!(
        "\nnext: `opendomain provision --sync` regenerates tags; confirm INFERRED bindings with\nbinding.confirm = true only after human review — never the reverse."
    );
    Ok(())
}

/// Word-level overlap between a test's title and a glossary term — the
/// naming-convention inference. Always a *hypothesis*, never a recorded edge.
fn name_match(title: &str, glossary: &[String]) -> Option<String> {
    let words: BTreeSet<String> = title
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().count() >= 4)
        .collect();
    if words.is_empty() {
        return None;
    }
    for term in glossary {
        if term
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 4)
            .any(|w| words.contains(w))
        {
            return Some(term.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naming_inference_requires_a_four_letter_overlap() {
        let glossary = vec!["Stock reservation at order time".to_lowercase()];
        // Overlap on "reservation" (≥4 chars) → a hypothesis, nothing more.
        assert!(name_match("Reservation check runs green", &glossary).is_some());
        // No overlap → nothing inferred, and nothing invented.
        assert!(name_match("Pricing roundtrip", &glossary).is_none());
    }
}
