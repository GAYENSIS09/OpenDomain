//! Quality gates.
//!
//! A gate is a machine-checkable predicate over the registries. Its job is to
//! make the framework's central promise — *you may not skip a phase without
//! saying so* — enforceable rather than aspirational.
//!
//! Findings come in two severities:
//!
//! * **Blocking** — the phase produced nothing usable. `opendomain gate` fails.
//! * **Advisory** — a rule of discipline has been broken that does not make the
//!   output wrong. Reported, and counted in a non-zero exit code only under
//!   `--strict`.
//!
//! Gates never mutate the workspace. Reading them as pure functions keeps
//! `opendomain audit` trustworthy: running it twice gives the same answer.

use crate::model::artifact::Artifact;
use crate::model::ids::{Id, IdKind};
use crate::model::vocab::{ArtifactStatus, Confidence, KnowledgeType, Stage};
use crate::registries::workspace::Index;
use serde::Serialize;
use std::collections::BTreeMap;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A discipline breach worth reporting.
    Advisory,
    /// The phase cannot be considered complete.
    Blocking,
}

/// One thing a gate noticed.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// `blocking` or `advisory`.
    pub severity: Severity,
    /// Stable identifier for the rule, e.g. `gate-1/no-sources`.
    pub rule: String,
    /// One-line human explanation.
    pub message: String,
    /// The artifact the finding is about, when it is about one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// The file the finding is about, when it is about one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// The result of evaluating one gate.
#[derive(Debug, Clone, Serialize)]
pub struct GateReport {
    /// Gate number, 0 through 8.
    pub number: u8,
    /// Stable gate name, e.g. `research`.
    pub name: &'static str,
    /// The stage this gate guards.
    pub stage: Stage,
    /// What the gate demands, in one sentence.
    pub question: &'static str,
    /// All findings, blocking first.
    pub findings: Vec<Finding>,
}

impl GateReport {
    /// Number of blocking findings.
    pub fn blocking(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Blocking)
            .count()
    }

    /// Number of advisory findings.
    pub fn advisory(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Advisory)
            .count()
    }

    /// `true` when the gate passes, i.e. has no blocking findings.
    pub fn passed(&self) -> bool {
        self.blocking() == 0
    }
}

/// Every gate, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    /// Position in the sequence, 0 through 8.
    pub number: u8,
    /// Stable name, also used as the CLI argument.
    pub name: &'static str,
    /// The stage this gate guards.
    pub stage: Stage,
    /// The question the gate answers.
    pub question: &'static str,
}

impl Gate {
    /// All gates, in lifecycle order.
    ///
    /// The single source of truth for what a gate is. `RULES` names these by
    /// number rather than by value so that adding a gate here cannot leave the
    /// rule catalogue referring to a gate that does not exist.
    pub const ALL: &'static [Gate] = &[
        Gate {
            number: 0,
            name: "context",
            stage: Stage::Domain,
            question: "Do we know what we are building, and what is already true?",
        },
        Gate {
            number: 1,
            name: "research",
            stage: Stage::Domain,
            question: "Is the domain understood well enough to derive requirements?",
        },
        Gate {
            number: 2,
            name: "domain",
            stage: Stage::Domain,
            question: "Is the domain model coherent, owned and cross-linked?",
        },
        Gate {
            number: 3,
            name: "requirements",
            stage: Stage::Domain,
            question: "Is every requirement justified, testable and traced?",
        },
        Gate {
            number: 4,
            name: "opportunity",
            stage: Stage::Opportunity,
            question: "Has anything been challenged, and is the chosen concept earned?",
        },
        Gate {
            number: 5,
            name: "specification",
            stage: Stage::Specification,
            question: "Does the specification describe observable behaviour and nothing else?",
        },
        Gate {
            number: 6,
            name: "plan",
            stage: Stage::Specification,
            question: "Is there a bounded, ordered plan that can realise the specification?",
        },
        Gate {
            number: 7,
            name: "implementation",
            stage: Stage::Specification,
            question: "Did implementation satisfy the acceptance criteria without drift?",
        },
        Gate {
            number: 8,
            name: "completion",
            stage: Stage::Specification,
            question: "Is the whole chain coherent, tested and free of critical unknowns?",
        },
    ];

    /// Looks a gate up by number.
    pub fn by_number(n: u8) -> Option<Gate> {
        Gate::ALL.iter().copied().find(|g| g.number == n)
    }

    /// Looks a gate up by name.
    pub fn by_name(name: &str) -> Option<Gate> {
        let norm = name.trim().to_ascii_lowercase();
        Gate::ALL
            .iter()
            .copied()
            .find(|g| g.name == norm || g.number.to_string() == norm)
    }
}

/// One rule the gates can raise.
///
/// The rule slug is what an agent cites when it reports a finding, so the set of
/// slugs is part of the tool's contract rather than an implementation detail. It
/// used not to be enumerable: each gate function hardcoded its own slugs, which
/// meant nothing outside `gates.rs` could list what would be checked. An agent
/// could only learn the rules by tripping them.
///
/// This table is the declared catalogue. It duplicates the slugs in the gate
/// functions, and a test fails if the two disagree in either direction, so the
/// duplication cannot rot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// The slug suffix, without the `gate-N/` prefix. `opendomain gate` prints the full
    /// slug; this is the part that says what the rule is about.
    pub slug: &'static str,
    /// The number of the gate that raises it.
    pub gate: u8,
    /// Whether finding it fails `opendomain gate`.
    pub severity: Severity,
}

impl Rule {
    /// The slug as `opendomain` prints it, e.g. `gate-1/no-sources`.
    pub fn full_slug(&self) -> String {
        format!("gate-{}/{}", self.gate, self.slug)
    }

    /// The gate this rule belongs to. Always resolves: a test asserts that every
    /// catalogue entry names a gate that exists, so a number out of range fails
    /// there rather than at some later point in a report.
    pub fn gate(&self) -> Gate {
        Gate::by_number(self.gate)
            .unwrap_or_else(|| unreachable!("RULES names gate {}, which does not exist", self.gate))
    }
}

/// Every rule the gates can raise, ordered by gate then slug.
pub const RULES: &[Rule] = &[
    Rule {
        slug: "assumption-reached-production",
        gate: 8,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "concept-without-mechanism",
        gate: 4,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "concepts-without-hypotheses",
        gate: 4,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "contract-without-spec",
        gate: 7,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "contracts-without-tests",
        gate: 7,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "dangling-reference",
        gate: 8,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "decision-without-evidence",
        gate: 4,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "dependency-cycle",
        gate: 6,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "domain-not-cross-analysed",
        gate: 2,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "domain-without-purpose",
        gate: 0,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "draft-requirement",
        gate: 3,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "duplicate-requirement",
        gate: 3,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "duplicate-term",
        gate: 2,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "empty-criterion",
        gate: 5,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "empty-evidence",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "empty-registry",
        gate: 2,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "empty-specification",
        gate: 5,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "entity-without-owner",
        gate: 2,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "epistemic-laundering",
        gate: 7,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "experiment-without-outcome",
        gate: 4,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "feature-cloning",
        gate: 4,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "how-not-what",
        gate: 5,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "implemented-without-verification",
        gate: 7,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "low-confidence",
        gate: 1,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "no-concept-decision",
        gate: 4,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-domain-map",
        gate: 0,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-evidence",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-opportunities",
        gate: 4,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "no-requirements",
        gate: 3,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-sources",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-specifications",
        gate: 5,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-stakeholders",
        gate: 0,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-statement",
        gate: 3,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "no-transition-history",
        gate: 7,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "nothing-completed",
        gate: 8,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "nothing-implemented",
        gate: 7,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "nothing-to-plan",
        gate: 6,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "open-question-untracked",
        gate: 1,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "open-uncertainty",
        gate: 8,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "orphan-evidence",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "orphan-test",
        gate: 7,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "ownership-without-concept",
        gate: 2,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "placeholder-context",
        gate: 0,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "plan-without-approach",
        gate: 6,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "requirement-without-passing-test",
        gate: 8,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "resolved-uncertainty-still-open",
        gate: 2,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "single-source",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "spec-without-concept",
        gate: 4,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "spec-without-verification",
        gate: 5,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "task-without-test",
        gate: 6,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "tasks-without-plan",
        gate: 6,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "undeclared-epistemic",
        gate: 1,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unfalsifiable-hypothesis",
        gate: 4,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unjustified-requirement",
        gate: 3,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unknown-not-tracked",
        gate: 1,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "unresolved-conflict",
        gate: 8,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unresolved-drift",
        gate: 7,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unresolved-provenance",
        gate: 3,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "untested-requirement",
        gate: 3,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "untraced-specification",
        gate: 5,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "untraced-task",
        gate: 6,
        severity: Severity::Blocking,
    },
    Rule {
        slug: "unverified-contract",
        gate: 8,
        severity: Severity::Advisory,
    },
    Rule {
        slug: "vague-quantifier",
        gate: 3,
        severity: Severity::Advisory,
    },
];

/// Evaluates one gate against a loaded workspace.
pub fn evaluate(gate: Gate, index: &Index) -> GateReport {
    let mut findings = Vec::new();
    match gate.name {
        "context" => gate_context(index, &mut findings),
        "research" => gate_research(index, &mut findings),
        "domain" => gate_domain(index, &mut findings),
        "requirements" => gate_requirements(index, &mut findings),
        "opportunity" => gate_opportunity(index, &mut findings),
        "specification" => gate_specification(index, &mut findings),
        "plan" => gate_plan(index, &mut findings),
        "implementation" => gate_implementation(index, &mut findings),
        "completion" => gate_completion(index, &mut findings),
        _ => findings.push(blocking("gate/unknown", "unrecognised gate", None)),
    }
    findings.sort_by_key(|f| (f.severity, f.rule.clone()));
    GateReport {
        number: gate.number,
        name: gate.name,
        stage: gate.stage,
        question: gate.question,
        findings,
    }
}

/// Evaluates every gate up to and including `through`.
pub fn evaluate_up_to(index: &Index, through: u8) -> Vec<GateReport> {
    Gate::ALL
        .iter()
        .copied()
        .filter(|g| g.number <= through)
        .map(|g| evaluate(g, index))
        .collect()
}

// ── Gate 0: context ───────────────────────────────────────────────────────

/// Gate 0 — the project has been inspected before anything was invented.
fn gate_context(index: &Index, out: &mut Vec<Finding>) {
    if index.of_kind(IdKind::Dom).is_empty() {
        out.push(blocking(
            "gate-0/no-domain-map",
            "no domain is declared — the project is not understood yet",
            None,
        ));
    }
    if index.of_kind(IdKind::Stk).is_empty() {
        out.push(blocking(
            "gate-0/no-stakeholders",
            "no stakeholders recorded: who is affected is unknown",
            None,
        ));
    }
    for a in index.of_kind(IdKind::Dom) {
        if a.field("purpose").is_none() && a.field("goal").is_none() {
            out.push(advisory(
                "gate-0/domain-without-purpose",
                format!("{} declares no purpose or goal", a.id),
                Some(&a.id),
            ));
        }
    }

    // Quantity is not quality: placeholders satisfy the existence checks
    // above while confirming nothing. A context made only of unconfirmed
    // declarations still passes — but it must say so on the report.
    let doms = index.of_kind(IdKind::Dom);
    let stks = index.of_kind(IdKind::Stk);
    if !doms.is_empty()
        && !stks.is_empty()
        && doms
            .iter()
            .chain(stks.iter())
            .all(|a| a.has_unresolved_provenance())
    {
        out.push(advisory(
            "gate-0/placeholder-context",
            "every domain and stakeholder is ASSUMED, UNKNOWN or CONFLICTING: the context \
             exists but nothing in it is confirmed — verify or correct it before modelling",
            None,
        ));
    }
}

// ── Gate 1: research ─────────────────────────────────────────────────────

/// Gate 1 — research is saturated enough to support the next stage.
fn gate_research(index: &Index, out: &mut Vec<Finding>) {
    let sources = index.of_kind(IdKind::Src);
    let evidence = index.of_kind(IdKind::Evd);

    if sources.is_empty() {
        out.push(blocking(
            "gate-1/no-sources",
            "no source registry entries: every domain claim is currently unsourced",
            None,
        ));
    }
    if evidence.is_empty() {
        out.push(blocking(
            "gate-1/no-evidence",
            "no evidence extracted: sources exist but nothing was read out of them",
            None,
        ));
    }

    for e in &evidence {
        if e.upstream().is_empty() {
            out.push(blocking(
                "gate-1/orphan-evidence",
                format!("{} is not derived from any source", e.id),
                Some(&e.id),
            ));
        }
        if e.body.trim().is_empty() {
            out.push(blocking(
                "gate-1/empty-evidence",
                format!("{} has no quoted statement or context", e.id),
                Some(&e.id),
            ));
        }
    }

    // G2-B3. An artifact that makes a claim but does not say how it is known is
    // the condition the whole framework exists to prevent, and it is invisible
    // downstream: every later gate reads `epistemic`, finds it absent, and has
    // nothing to object to. Scoped to kinds that make knowledge claims — see
    // `IdKind::makes_knowledge_claim` for why tasks and plans are excluded.
    for a in index.by_id.values() {
        if a.id.kind().makes_knowledge_claim() && a.epistemic_undeclared() {
            out.push(blocking(
                "gate-1/undeclared-epistemic",
                format!(
                    "{} is a {} that declares no epistemic state: it must say whether it is \
                     EXPLICIT, TACIT, HYBRID, INFERRED, ASSUMED, UNKNOWN or CONFLICTING",
                    a.id,
                    a.id.kind().label()
                ),
                Some(&a.id),
            ));
        }
    }

    // A single source supporting every claim is a correlation, not corroboration.
    let mut cited: BTreeMap<Id, usize> = BTreeMap::new();
    for e in &evidence {
        for src in e.upstream() {
            *cited.entry(src).or_default() += 1;
        }
    }
    if evidence.len() >= 3 && cited.len() == 1 {
        out.push(blocking(
            "gate-1/single-source",
            format!(
                "all {} evidence records cite one source: a claim repeated by the same \
                 origin is not corroborated",
                evidence.len()
            ),
            None,
        ));
    }

    for e in &evidence {
        if e.epistemic == Some(KnowledgeType::Unknown) && !index.of_kind(IdKind::Unc).is_empty() {
            out.push(advisory(
                "gate-1/unknown-not-tracked",
                format!(
                    "{} is UNKNOWN; make sure it is in the uncertainty registry",
                    e.id
                ),
                Some(&e.id),
            ));
        }
        if e.confidence == Some(Confidence::Low) {
            out.push(advisory(
                "gate-1/low-confidence",
                format!("{} rests on LOW confidence evidence", e.id),
                Some(&e.id),
            ));
        }
    }

    // Open questions nobody followed up: unknowns parked in the research
    // questions registry instead of tracked as UNC, where gate 8 can see
    // them. An `answers` edge (or any dependent) counts as follow-up.
    for q in index.of_kind(IdKind::Res) {
        if q.status == ArtifactStatus::Open && dependents_of(index, &q.id).is_empty() {
            out.push(advisory(
                "gate-1/open-question-untracked",
                format!(
                    "{} is OPEN with nothing answering or tracking it: record it in \
                     domain/uncertainties.md or answer it before it goes stale",
                    q.id
                ),
                Some(&q.id),
            ));
        }
    }
}

// ── Gate 2: domain model ──────────────────────────────────────────────────

/// Gate 2 — the model is coherent and every important concept has an owner.
fn gate_domain(index: &Index, out: &mut Vec<Finding>) {
    for kind in [
        IdKind::Term,
        IdKind::Act,
        IdKind::Proc,
        IdKind::Ent,
        IdKind::Rule,
        IdKind::Dom,
    ] {
        if index.of_kind(kind).is_empty() {
            out.push(blocking(
                "gate-2/empty-registry",
                format!("no {} has been modelled", kind.plural()),
                None,
            ));
        }
    }

    // Vocabulary discipline: the same word silently meaning two things is the
    // defect that produces a hundred subtle bugs downstream.
    let mut by_title: BTreeMap<String, Vec<&Artifact>> = BTreeMap::new();
    for t in index.of_kind(IdKind::Term) {
        let key = normalize(&t.title);
        if !key.is_empty() {
            by_title.entry(key).or_default().push(t);
        }
    }
    for (key, terms) in &by_title {
        if terms.len() > 1 {
            out.push(blocking(
                "gate-2/duplicate-term",
                format!(
                    "{} terms are titled `{}`: merge them or record the distinction",
                    terms.len(),
                    key
                ),
                Some(&terms[0].id),
            ));
        }
    }

    // Ownership: a concept consumed by more than one domain must be owned once.
    let mut owners: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for a in index.by_id.values() {
        for owner in a
            .edges()
            .filter(|(r, _)| *r == crate::model::artifact::Relation::OwnedBy)
        {
            owners.entry(owner.1).or_default().push(a.id.clone());
        }
    }
    for (concept, consumers) in &owners {
        if consumers.len() > 1 {
            // Multiple consumers are normal; multiple *owners* are not. This
            // gate fires on the concept being owned by conflicting domains,
            // which surfaces as the same concept listed under two owners.
            let declared = index.get(concept);
            if declared.is_none() {
                out.push(advisory(
                    "gate-2/ownership-without-concept",
                    format!(
                        "{} concept(s) point at owner {concept}, which is not modelled",
                        consumers.len()
                    ),
                    None,
                ));
            }
        }
    }

    for a in index.of_kind(IdKind::Ent) {
        if a.links.get(&crate::model::artifact::Relation::OwnedBy).is_none() {
            out.push(advisory(
                "gate-2/entity-without-owner",
                format!("{} has no owning domain", a.id),
                Some(&a.id),
            ));
        }
    }

    // Cross-domain analysis is mandatory for multi-domain projects.
    let domains = index.of_kind(IdKind::Dom);
    if domains.len() > 1 {
        for d in &domains {
            let has_links = d
                .links
                .get(&crate::model::artifact::Relation::DependsOn)
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            if !has_links && d.field("depends-on").is_none() {
                out.push(advisory(
                    "gate-2/domain-not-cross-analysed",
                    format!(
                        "{} declares no dependency on any other domain; run cross-domain analysis",
                        d.id
                    ),
                    Some(&d.id),
                ));
            }
        }
    }

    // Recorded unknowns must not be silently dropped.
    for u in index.of_kind(IdKind::Unc) {
        if u.status == ArtifactStatus::Validated {
            out.push(advisory(
                "gate-2/resolved-uncertainty-still-open",
                format!("{} is VALIDATED but still listed as an uncertainty", u.id),
                Some(&u.id),
            ));
        }
    }
}

// ── Gate 3: requirements ─────────────────────────────────────────────────

/// Gate 3 — requirements are justified, atomic and testable.
fn gate_requirements(index: &Index, out: &mut Vec<Finding>) {
    let reqs: Vec<&Artifact> = index
        .of_kind(IdKind::Req)
        .into_iter()
        .chain(index.of_kind(IdKind::Nfr))
        .collect();

    if reqs.is_empty() {
        out.push(blocking(
            "gate-3/no-requirements",
            "no requirements have been derived from the domain model",
            None,
        ));
        return;
    }

    for r in &reqs {
        if r.upstream().is_empty() {
            out.push(blocking(
                "gate-3/unjustified-requirement",
                format!("{} has no evidence and no rule behind it", r.id),
                Some(&r.id),
            ));
        }
        if dependents_of(index, &r.id).is_empty() {
            out.push(advisory(
                "gate-3/untested-requirement",
                format!("{} has no specification or test downstream", r.id),
                Some(&r.id),
            ));
        }
        if r.status == ArtifactStatus::Draft {
            out.push(advisory(
                "gate-3/draft-requirement",
                format!("{} is still DRAFT", r.id),
                Some(&r.id),
            ));
        }
        if r.has_unresolved_provenance() {
            out.push(blocking(
                "gate-3/unresolved-provenance",
                format!(
                    "{} is {}: resolve it, or record it as an uncertainty, before implementing",
                    r.id,
                    r.epistemic.map(|k| k.as_str()).unwrap_or("?")
                ),
                Some(&r.id),
            ));
        }
        // Vague quantification is the single most common quality failure.
        for phrase in VAGUE_PHRASES {
            if r.body.to_lowercase().contains(&phrase.to_lowercase()) {
                out.push(advisory(
                    "gate-3/vague-quantifier",
                    format!(
                        "{} uses the unmeasurable phrase `{phrase}`; replace it with a criterion",
                        r.id
                    ),
                    Some(&r.id),
                ));
                break;
            }
        }
        if r.body.trim().is_empty() {
            out.push(blocking(
                "gate-3/no-statement",
                format!("{} has no statement text", r.id),
                Some(&r.id),
            ));
        }
    }

    // A requirement is not accepted because it sounds reasonable; look for
    // duplicates that would be implemented twice.
    let mut seen: BTreeMap<String, Vec<&Id>> = BTreeMap::new();
    for r in &reqs {
        let key = normalize(&r.body);
        if key.len() > 40 {
            seen.entry(key).or_default().push(&r.id);
        }
    }
    for (_body, ids) in seen {
        if ids.len() > 1 {
            out.push(blocking(
                "gate-3/duplicate-requirement",
                format!(
                    "{} share identical statement text: {}",
                    ids.len(),
                    ids.iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Some(ids[0]),
            ));
        }
    }
}

/// Phrases that cannot be verified and therefore cannot be an acceptance criterion.
const VAGUE_PHRASES: &[&str] = &[
    "should be fast",
    "should be intuitive",
    "should be powerful",
    "should be intelligent",
    "should be scalable",
    "should be user-friendly",
    "as fast as possible",
    "best possible",
    "reasonable performance",
];

// ── Gate 4: opportunity ──────────────────────────────────────────────────

/// Gate 4 — something was actually challenged before anything was specified.
fn gate_opportunity(index: &Index, out: &mut Vec<Finding>) {
    let opportunities = index.of_kind(IdKind::Opp);
    let _assumptions = index.of_kind(IdKind::Asm);
    let concepts = index.of_kind(IdKind::Cpt);
    let hypotheses = index.of_kind(IdKind::Hyp);
    let experiments = index.of_kind(IdKind::Exp);
    let decisions = index.of_kind(IdKind::Cdr);

    if opportunities.is_empty() {
        out.push(advisory(
            "gate-4/no-opportunities",
            "no opportunity recorded: the current design was never challenged",
            None,
        ));
    }
    if concepts.len() >= 2 && hypotheses.is_empty() {
        out.push(blocking(
            "gate-4/concepts-without-hypotheses",
            "several concepts exist but none is expressed as a falsifiable hypothesis",
            None,
        ));
    }
    if !concepts.is_empty() && decisions.is_empty() {
        out.push(blocking(
            "gate-4/no-concept-decision",
            "concepts exist but no concept decision record explains which was chosen",
            None,
        ));
    }

    for d in &decisions {
        if d.upstream().is_empty() {
            out.push(blocking(
                "gate-4/decision-without-evidence",
                format!("{} cites no evidence, assumption or experiment", d.id),
                Some(&d.id),
            ));
        }
    }

    // Anti-innovation: a concept that is just competitor features is cloning.
    for c in &concepts {
        let class = c.field("class").unwrap_or("");
        if class.eq_ignore_ascii_case("REPLICATION")
            && c.field("rationale")
                .unwrap_or("")
                .to_lowercase()
                .contains("competitor")
        {
            out.push(blocking(
                "gate-4/feature-cloning",
                format!(
                    "{} is justified by competitor parity: competitor functionality is \
                     evidence, not a requirement",
                    c.id
                ),
                Some(&c.id),
            ));
        }
        if c.field("mechanism").is_none() && c.field("key-difference").is_none() {
            out.push(advisory(
                "gate-4/concept-without-mechanism",
                format!("{} states no novel mechanism or key difference", c.id),
                Some(&c.id),
            ));
        }
    }

    for h in &hypotheses {
        if h.field("metric").is_none() || h.field("fails-when").is_none() {
            out.push(blocking(
                "gate-4/unfalsifiable-hypothesis",
                format!(
                    "{} has no metric or no failure condition: it cannot be tested",
                    h.id
                ),
                Some(&h.id),
            ));
        }
    }

    for e in &experiments {
        if e.field("outcome").is_none() && e.status == ArtifactStatus::Implemented {
            out.push(advisory(
                "gate-4/experiment-without-outcome",
                format!("{} ran but records no outcome", e.id),
                Some(&e.id),
            ));
        }
    }

    // A concept must not be specified while it is still speculative.
    for s in index.of_kind(IdKind::Spec) {
        if s.field("concept").is_none() && !concepts.is_empty() {
            out.push(advisory(
                "gate-4/spec-without-concept",
                format!(
                    "{} exists although concepts have not been converged: was it specified \
                     too early?",
                    s.id
                ),
                Some(&s.id),
            ));
            break;
        }
    }
}

// ── Gate 5: specification ────────────────────────────────────────────────

/// Gate 5 — specifications describe behaviour, not implementation.
fn gate_specification(index: &Index, out: &mut Vec<Finding>) {
    let specs = index.of_kind(IdKind::Spec);
    if specs.is_empty() {
        out.push(blocking(
            "gate-5/no-specifications",
            "no behavioural specifications exist yet",
            None,
        ));
        return;
    }

    for s in &specs {
        if s.upstream().is_empty() {
            out.push(blocking(
                "gate-5/untraced-specification",
                format!("{} is not derived from a requirement or use case", s.id),
                Some(&s.id),
            ));
        }
        if s.body.trim().is_empty() {
            out.push(blocking(
                "gate-5/empty-specification",
                format!("{} has no scenarios", s.id),
                Some(&s.id),
            ));
        }
        // A specification that names a technology is prescribing HOW.
        let body = s.body.to_lowercase();
        for tech in IMPLEMENTATION_DETAIL {
            if body.contains(tech) {
                out.push(advisory(
                    "gate-5/how-not-what",
                    format!(
                        "{} mentions `{tech}`: a specification states observable behaviour; \
                         the technology belongs in the plan",
                        s.id
                    ),
                    Some(&s.id),
                ));
                break;
            }
        }
        if !s.links.contains_key(&crate::model::artifact::Relation::Verifies) {
            out.push(advisory(
                "gate-5/spec-without-verification",
                format!("{} has no linked acceptance criteria or test", s.id),
                Some(&s.id),
            ));
        }
    }

    for a in index.of_kind(IdKind::Ac) {
        if a.body.trim().is_empty() {
            out.push(blocking(
                "gate-5/empty-criterion",
                format!("{} has no observable outcome", a.id),
                Some(&a.id),
            ));
        }
    }
}

/// Technology names that indicate a specification has drifted into a plan.
const IMPLEMENTATION_DETAIL: &[&str] = &[
    "postgresql",
    "mysql",
    "mongodb",
    "redis",
    "kafka",
    "docker",
    "kubernetes",
    "sqlalchemy",
    "prisma",
    "hibernate",
    "microservice",
    "graphql",
    "repository class",
];

// ── Gate 6: plan ─────────────────────────────────────────────────────────

/// Gate 6 — the work is decomposed, ordered and bounded.
fn gate_plan(index: &Index, out: &mut Vec<Finding>) {
    let tasks = index.of_kind(IdKind::Task);
    let plans = index.of_kind(IdKind::Plan);

    // A gate that passes because there is nothing to assess is honest only
    // if it says so. Absence of work is not evidence of plannable work.
    if tasks.is_empty() && plans.is_empty() {
        out.push(advisory(
            "gate-6/nothing-to-plan",
            "no tasks and no plans exist yet: this gate passes vacuously — there is nothing to assess",
            None,
        ));
    }

    if !tasks.is_empty() && plans.is_empty() {
        out.push(blocking(
            "gate-6/tasks-without-plan",
            "implementation tasks exist but no technical plan justifies them",
            None,
        ));
    }

    for t in &tasks {
        if t.upstream().is_empty() {
            out.push(blocking(
                "gate-6/untraced-task",
                format!(
                    "{} implements nothing: no spec or requirement upstream",
                    t.id
                ),
                Some(&t.id),
            ));
        }
        if t.downstream().is_empty()
            && !t.links.contains_key(&crate::model::artifact::Relation::Verifies)
            && t.status != ArtifactStatus::Draft
        {
            out.push(advisory(
                "gate-6/task-without-test",
                format!("{} is in flight with no test attached", t.id),
                Some(&t.id),
            ));
        }
    }

    for p in &plans {
        if p.field("authority").is_none() && p.field("approach").is_none() {
            out.push(advisory(
                "gate-6/plan-without-approach",
                format!("{} records no approach or authority model", p.id),
                Some(&p.id),
            ));
        }
    }

    // Cycles in the dependency order mean the work can never start.
    for cycle in find_cycles(&tasks) {
        out.push(blocking(
            "gate-6/dependency-cycle",
            format!("dependency cycle: {}", cycle.join(" -> ")),
            None,
        ));
    }
}

/// Detects cycles among task dependencies. Kept local so the gate has no
/// dependency on the graph module.
fn find_cycles(tasks: &[&Artifact]) -> Vec<Vec<String>> {
    let mut deps: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for t in tasks {
        let d: Vec<Id> = t
            .edges()
            .filter(|(r, _)| *r == crate::model::artifact::Relation::DependsOn)
            .map(|(_, id)| id)
            .filter(|id| tasks.iter().any(|t| t.id == *id))
            .collect();
        deps.insert(t.id.clone(), d);
    }

    let mut cycles = Vec::new();
    let mut state: BTreeMap<Id, u8> = BTreeMap::new(); // 0 unvisited, 1 open, 2 done
    let mut stack: Vec<Id> = Vec::new();

    fn visit(
        node: &Id,
        deps: &BTreeMap<Id, Vec<Id>>,
        state: &mut BTreeMap<Id, u8>,
        stack: &mut Vec<Id>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        state.insert(node.clone(), 1);
        stack.push(node.clone());
        for next in deps.get(node).map(|v| v.as_slice()).unwrap_or(&[]) {
            match state.get(next).copied().unwrap_or(0) {
                0 => visit(next, deps, state, stack, cycles),
                1 => {
                    if let Some(pos) = stack.iter().position(|n| n == next) {
                        let mut loop_path: Vec<String> =
                            stack[pos..].iter().map(|n| n.to_string()).collect();
                        loop_path.push(next.to_string());
                        if !cycles.contains(&loop_path) {
                            cycles.push(loop_path);
                        }
                    }
                }
                _ => {}
            }
        }
        stack.pop();
        state.insert(node.clone(), 2);
    }

    for id in deps.keys() {
        if state.get(id).copied().unwrap_or(0) == 0 {
            visit(id, &deps, &mut state, &mut stack, &mut cycles);
        }
    }
    cycles
}

// ── Gate 7: implementation ───────────────────────────────────────────────

/// Gate 7 — what was built matches what was specified.
// ── Gate 7: implementation ────────────────────────────────────────────────

/// G8-B2, epistemic laundering.
///
/// A status change from a state that does not support assertion to one that
/// does, with no intervening `EXPLICIT` evidence, is the framework's central
/// failure mode: the artifact reads as validated because its status says so,
/// while nothing about how it is known has changed.
///
/// Two predicates, because they answer different questions and have different
/// false-positive profiles.
///
/// *Current state* — the artifact is `ASSUMED` or `INFERRED` and is
/// `VALIDATED` or `VERIFIED` right now. This is decidable with no history at
/// all, and it is unambiguous: whatever else is true, the artifact asserts more
/// than its provenance supports.
///
/// *Recorded transition* — a promotion is recorded from `ASSUMED` or `INFERRED`
/// into `VALIDATED` or `VERIFIED`, and the evidence cited at that moment is
/// absent, dangling, or itself not `EXPLICIT`. This catches the subtler case
/// the current-state test cannot: the author edited `epistemic` to `EXPLICIT`
/// in the same commit as the status, so the artifact now reads clean while the
/// promotion was a bare status edit.
///
/// The recorded-transition predicate needs a workspace that records promotions.
/// A workspace that records none is reported as advisory, because the absence of
/// history is not evidence of laundering — it is evidence that laundering is
/// currently undetectable, which is a different and lesser problem.
fn gate_epistemic_laundering(index: &Index, out: &mut Vec<Finding>) {
    for a in index.by_id.values() {
        if !a.id.kind().makes_knowledge_claim() {
            continue;
        }

        if a.asserts_beyond_its_provenance() {
            out.push(blocking(
                "gate-7/epistemic-laundering",
                format!(
                    "{} is {} but its epistemic state is still {}: a status that asserts \
                     validity cannot rest on a claim that was never established",
                    a.id,
                    a.status,
                    a.epistemic.map(|k| k.as_str()).unwrap_or("undeclared")
                ),
                Some(&a.id),
            ));
            continue;
        }

        for p in a.suspect_promotions() {
            let unexplained: Vec<String> = p
                .evidence
                .iter()
                .filter(|id| match index.get(id) {
                    Some(ev) => ev.epistemic != Some(KnowledgeType::Explicit),
                    None => true,
                })
                .map(|id| match index.get(id) {
                    Some(ev) => format!(
                        "{id} is {}",
                        ev.epistemic.map(|k| k.as_str()).unwrap_or("undeclared")
                    ),
                    None => format!("{id} does not exist"),
                })
                .collect();

            if p.evidence.is_empty() {
                out.push(blocking(
                    "gate-7/epistemic-laundering",
                    format!(
                        "{} was promoted from {} to {} citing no evidence: the epistemic state \
                         did not change, only the status did",
                        a.id,
                        p.from.map(|k| k.as_str()).unwrap_or("undeclared"),
                        p.to
                    ),
                    Some(&a.id),
                ));
            } else if !unexplained.is_empty() {
                out.push(blocking(
                    "gate-7/epistemic-laundering",
                    format!(
                        "{} was promoted from {} to {} citing evidence that is not EXPLICIT: {}",
                        a.id,
                        p.from.map(|k| k.as_str()).unwrap_or("undeclared"),
                        p.to,
                        unexplained.join(", ")
                    ),
                    Some(&a.id),
                ));
            }
        }
    }

    let claims = index
        .by_id
        .values()
        .filter(|a| a.id.kind().makes_knowledge_claim())
        .count();
    if claims > 0
        && !index
            .by_id
            .values()
            .any(|a| a.id.kind().makes_knowledge_claim() && !a.promotions.is_empty())
    {
        out.push(advisory(
            "gate-7/no-transition-history",
            "no artifact records a `promotions` history, so a status change cannot be \
             distinguished from a status edit: this workspace cannot currently detect \
             laundering that was accompanied by an edit to the epistemic state",
            None,
        ));
    }
}

/// Gate 7 — the work is implemented against a specification, and verified.
fn gate_implementation(index: &Index, out: &mut Vec<Finding>) {
    let tests = index.of_kind(IdKind::Test);
    let contracts = index.of_kind(IdKind::Ctr);

    gate_epistemic_laundering(index, out);

    for s in index.of_kind(IdKind::Spec) {
        if s.status == ArtifactStatus::Implemented || s.status == ArtifactStatus::Verified {
            if dependents_of(index, &s.id).is_empty() {
                out.push(blocking(
                    "gate-7/implemented-without-verification",
                    format!("{} is marked {} but nothing verifies it", s.id, s.status),
                    Some(&s.id),
                ));
            }
        }
    }

    for t in &tests {
        if t.upstream().is_empty() {
            out.push(blocking(
                "gate-7/orphan-test",
                format!("{} verifies nothing", t.id),
                Some(&t.id),
            ));
        }
    }

    for c in &contracts {
        if c.upstream().is_empty() {
            out.push(advisory(
                "gate-7/contract-without-spec",
                format!("{} is not derived from a specification", c.id),
                Some(&c.id),
            ));
        }
    }

    // Same vacuous-pass honesty as gate 6: nothing marked IMPLEMENTING or
    // beyond means the implementation stage has not started, not that it
    // succeeded.
    let delivered = index
        .of_kind(IdKind::Spec)
        .into_iter()
        .chain(index.of_kind(IdKind::Task))
        .chain(index.of_kind(IdKind::Test))
        .any(|a| a.status.at_least(ArtifactStatus::Implementing));
    if !delivered {
        out.push(advisory(
            "gate-7/nothing-implemented",
            "nothing is marked IMPLEMENTING or beyond: this gate passes vacuously — there is nothing to assess",
            None,
        ));
    }

    for d in index.of_kind(IdKind::Drift) {
        if !d.is_resolved() {
            out.push(blocking(
                "gate-7/unresolved-drift",
                format!("{} is still open: spec and implementation disagree", d.id),
                Some(&d.id),
            ));
        }
    }

    if !contracts.is_empty() && tests.is_empty() {
        out.push(advisory(
            "gate-7/contracts-without-tests",
            "contracts are declared but no test verifies them",
            None,
        ));
    }
}

// ── Gate 8: completion ───────────────────────────────────────────────────

/// Gate 8 — the whole chain is coherent.
fn gate_completion(index: &Index, out: &mut Vec<Finding>) {
    let dangling = index.dangling();
    for (id, missing) in &dangling {
        out.push(blocking(
            "gate-8/dangling-reference",
            format!(
                "{} references undefined artifact(s): {}",
                id,
                missing
                    .iter()
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Some(id),
        ));
    }

    for k in index.of_kind(IdKind::Xconf) {
        if k.status == ArtifactStatus::Open || k.status == ArtifactStatus::Conflicting {
            out.push(blocking(
                "gate-8/unresolved-conflict",
                format!(
                    "{} is a source conflict that was never resolved: one source was \
                     chosen arbitrarily or none was",
                    k.id
                ),
                Some(&k.id),
            ));
        }
    }

    for u in index.of_kind(IdKind::Unc) {
        if u.status == ArtifactStatus::Open {
            out.push(advisory(
                "gate-8/open-uncertainty",
                format!(
                    "{} is still open; record it as a risk or resolve it before release",
                    u.id
                ),
                Some(&u.id),
            ));
        }
    }

    for a in index.by_id.values() {
        if a.has_unresolved_provenance()
            && a.status != ArtifactStatus::Open
            && a.status != ArtifactStatus::Conflicting
            && a.id.stage() == Stage::Specification
        {
            out.push(blocking(
                "gate-8/assumption-reached-production",
                format!(
                    "{} is {} but has reached the specification stage: an unresolved \
                     assumption must not be built on",
                    a.id,
                    a.epistemic.map(|k| k.as_str()).unwrap_or("?")
                ),
                Some(&a.id),
            ));
        }
    }

    for c in index.of_kind(IdKind::Ctr) {
        if c.status == ArtifactStatus::Implemented && dependents_of(index, &c.id).is_empty() {
            out.push(advisory(
                "gate-8/unverified-contract",
                format!("{} is implemented but not verified by a test", c.id),
                Some(&c.id),
            ));
        }
    }

    // Every requirement needs a verified test somewhere downstream.
    for r in index.of_kind(IdKind::Req) {
        if r.status.at_least(ArtifactStatus::Implemented) {
            let verified = reachable_verified(index, &r.id);
            if !verified {
                out.push(blocking(
                    "gate-8/requirement-without-passing-test",
                    format!("{} is implemented but no downstream test is verified", r.id),
                    Some(&r.id),
                ));
            }
        }
    }

    // Completion over an empty chain is the vacuous pass this framework is
    // most often accused of: coherent, tested and free of unknowns — of nothing.
    if index.of_kind(IdKind::Req).is_empty() && index.of_kind(IdKind::Test).is_empty() {
        out.push(advisory(
            "gate-8/nothing-completed",
            "no requirements and no tests exist yet: this gate passes vacuously — there is nothing to assess",
            None,
        ));
    }
}

/// Breadth-first search downstream for a `VERIFIED` test.
fn reachable_verified(index: &Index, start: &Id) -> bool {
    let graph = crate::analysis::graph::Graph::build(index);
    graph
        .reachable(start, index, crate::analysis::graph::Direction::Forward)
        .into_iter()
        .any(|step| {
            step.id.kind() == IdKind::Test
                && index
                    .get(&step.id)
                    .map(|t| t.status == ArtifactStatus::Verified)
                    .unwrap_or(false)
        })
}

/// The artifacts that declare a link to `id`, and are therefore downstream of it.
///
/// The gates need this rather than `Artifact::downstream`, because dependents
/// live on the *incoming* edges of an artifact and a single `Artifact` cannot
/// see those.
fn dependents_of(index: &Index, id: &Id) -> Vec<Id> {
    crate::analysis::graph::Graph::build(index).dependents(id)
}

/// A blocking finding: the phase cannot be considered complete.
fn blocking(rule: &str, message: impl Into<String>, id: Option<&Id>) -> Finding {
    Finding {
        severity: Severity::Blocking,
        rule: rule.to_string(),
        message: message.into(),
        id: id.cloned(),
        file: None,
    }
}

/// An advisory finding: a discipline breach that does not make the output wrong.
fn advisory(rule: &str, message: impl Into<String>, id: Option<&Id>) -> Finding {
    Finding {
        severity: Severity::Advisory,
        rule: rule.to_string(),
        message: message.into(),
        id: id.cloned(),
        file: None,
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registries::parse;
    use crate::registries::workspace::Index;
    use std::path::PathBuf;

    /// Builds an index where each artifact's title equals its identifier.
    fn index_from(specs: &[(&str, &str, &str)]) -> Index {
        let titled: Vec<(&str, &str, &str, &str)> = specs
            .iter()
            .map(|(id, block, body)| (*id, *id, *block, *body))
            .collect();
        index_titled(&titled)
    }

    /// Builds an index with explicit `(id, title, toml, body)` entries, for the
    /// gates that key off the title rather than the metadata.
    fn index_titled(specs: &[(&str, &str, &str, &str)]) -> Index {
        let mut text = String::new();
        for (id, title, block, body) in specs {
            text.push_str(&format!(
                "## {id} — {title}\n\n```toml\n{block}\n```\n\n{body}\n\n"
            ));
        }
        let doc = parse::parse(std::path::Path::new("t.md"), PathBuf::from("t.md"), &text).unwrap();
        let mut index = Index::default();
        for a in doc.artifacts {
            index
                .by_kind
                .entry(a.id.kind())
                .or_default()
                .push(a.id.clone());
            index.by_id.insert(a.id.clone(), a);
        }
        index
    }

    fn rules(report: &GateReport) -> Vec<&str> {
        report.findings.iter().map(|f| f.rule.as_str()).collect()
    }

    /// A workspace that passes gates 0-3 and 5-6, for negative tests.
    fn healthy() -> Index {
        index_from(&[
            (
                "DOM-001",
                "purpose = \"warehouse\"\nepistemic = \"INFERRED\"",
                "Warehouse management.",
            ),
            ("STK-001", "epistemic = \"EXPLICIT\"", "Warehouse manager."),
            (
                "SRC-001",
                "type = \"STANDARD\"\nepistemic = \"EXPLICIT\"",
                "Title: Warehouse management. URL: https://example.com",
            ),
            (
                "SRC-002",
                "type = \"BOOK\"\nepistemic = \"EXPLICIT\"",
                "Title: Logistics. URL: https://example.org",
            ),
            (
                "EVD-001",
                "derived-from = [\"SRC-001\"]\nepistemic = \"EXPLICIT\"\nconfidence = \"HIGH\"",
                "Stock movements are recorded.",
            ),
            (
                "EVD-002",
                "derived-from = [\"SRC-002\"]\nepistemic = \"INFERRED\"\nconfidence = \"MEDIUM\"",
                "Operators reconcile stock monthly.",
            ),
            (
                "TERM-001",
                "owned-by = \"DOM-001\"\nepistemic = \"EXPLICIT\"",
                "Canonical term.",
            ),
            (
                "ACT-001",
                "owned-by = \"DOM-001\"\nepistemic = \"EXPLICIT\"",
                "Warehouse operator.",
            ),
            (
                "PROC-001",
                "derived-from = [\"EVD-001\"]\nepistemic = \"EXPLICIT\"",
                "Receive goods.",
            ),
            (
                "ENT-001",
                "owned-by = \"DOM-001\"\nepistemic = \"EXPLICIT\"",
                "An item.",
            ),
            (
                "RULE-001",
                "derived-from = [\"EVD-001\"]\nepistemic = \"EXPLICIT\"",
                "Stock SHALL never go negative.",
            ),
            (
                "REQ-001",
                "derived-from = [\"RULE-001\"]\nepistemic = \"EXPLICIT\"\nstatus = \"VALIDATED\"",
                "The system SHALL reject a receipt exceeding remaining quantity.",
            ),
            (
                "SPEC-001",
                "derived-from = [\"REQ-001\"]\nverifies = [\"AC-001-01\"]\nstatus = \"APPROVED\"\nepistemic = \"EXPLICIT\"",
                "Scenario: receive within remaining quantity. Then the stock increases.",
            ),
            (
                "AC-001-01",
                "derived-from = [\"SPEC-001\"]\nepistemic = \"EXPLICIT\"",
                "When a valid receipt is confirmed, the stock increases by that quantity.",
            ),
            (
                "PLAN-001",
                "derived-from = [\"SPEC-001\"]\napproach = \"modular monolith\"",
                "One deployable unit.",
            ),
            (
                "TASK-001",
                "derived-from = [\"SPEC-001\"]\nverifies = [\"TEST-001\"]",
                "Implement receipt validation.",
            ),
            (
                "TEST-001",
                "verifies = [\"SPEC-001\"]\nstatus = \"VERIFIED\"",
                "Unit test for over-receipt rejection.",
            ),
        ])
    }

    #[test]
    fn a_claim_without_an_epistemic_state_is_flagged() {
        let index = index_from(&[(
            "RULE-001",
            "derived-from = [\"EVD-001\"]",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-1/undeclared-epistemic"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_task_without_an_epistemic_state_is_not_flagged() {
        // A task records work to be done, not a claim about the domain, so
        // requiring an epistemic state of it would be a false positive on every
        // workspace. This test pins that scoping decision.
        let index = index_from(&[(
            "TASK-001",
            "derived-from = [\"SPEC-001\"]",
            "Implement receipt validation.",
        )]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        assert!(
            !rules(&report).contains(&"gate-1/undeclared-epistemic"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_validated_assumption_is_laundering() {
        let index = index_from(&[(
            "RULE-001",
            "epistemic = \"ASSUMED\"\nstatus = \"VALIDATED\"",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_draft_assumption_is_not_laundering() {
        // The same epistemic state is fine when the status does not yet claim
        // the artifact holds. A gate that flagged this would push authors to
        // declare EXPLICIT to silence it, which is the behaviour it exists to
        // prevent.
        let index = index_from(&[(
            "RULE-001",
            "epistemic = \"ASSUMED\"\nstatus = \"DRAFT\"",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            !rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_promotion_citing_no_evidence_is_laundering() {
        // The case the current-state test cannot see: `epistemic` was edited to
        // EXPLICIT in the same commit as the status, so the artifact reads clean
        // while the promotion itself was a bare status edit.
        let index = index_from(&[(
            "RULE-001",
            "epistemic = \"EXPLICIT\"\nstatus = \"VALIDATED\"\npromotions = [\n  { from = \"ASSUMED\", to = \"VALIDATED\" }\n]",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_promotion_citing_non_explicit_evidence_is_laundering() {
        let index = index_from(&[
            (
                "EVD-001",
                "epistemic = \"INFERRED\"",
                "Stock movements are recorded.",
            ),
            (
                "RULE-001",
                "epistemic = \"EXPLICIT\"\nstatus = \"VALIDATED\"\npromotions = [\n  { from = \"ASSUMED\", to = \"VALIDATED\", evidence = [\"EVD-001\"] }\n]",
                "Stock SHALL never go negative.",
            ),
        ]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_promotion_citing_missing_evidence_is_laundering() {
        let index = index_from(&[(
            "RULE-001",
            "epistemic = \"EXPLICIT\"\nstatus = \"VALIDATED\"\npromotions = [\n  { from = \"ASSUMED\", to = \"VALIDATED\", evidence = [\"EVD-404\"] }\n]",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_promotion_citing_explicit_evidence_is_accepted() {
        let index = index_from(&[
            (
                "EVD-001",
                "epistemic = \"EXPLICIT\"",
                "Stock movements are recorded.",
            ),
            (
                "RULE-001",
                "epistemic = \"EXPLICIT\"\nstatus = \"VALIDATED\"\npromotions = [\n  { from = \"ASSUMED\", to = \"VALIDATED\", evidence = [\"EVD-001\"], date = \"2026-09-29\" }\n]",
                "Stock SHALL never go negative.",
            ),
        ]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            !rules(&report).contains(&"gate-7/epistemic-laundering"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn a_workspace_with_no_history_says_so_without_accusing_anyone() {
        // Absence of history is not evidence of laundering. It is evidence that
        // laundering is currently undetectable, which is a lesser and different
        // problem, so it is advisory and names no artifact.
        let index = index_from(&[(
            "RULE-001",
            "epistemic = \"EXPLICIT\"",
            "Stock SHALL never go negative.",
        )]);
        let report = evaluate(Gate::by_name("implementation").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-7/no-transition-history"),
            "{:?}",
            rules(&report)
        );
        let finding = report
            .findings
            .iter()
            .find(|f| f.rule.as_str() == "gate-7/no-transition-history")
            .unwrap();
        assert_eq!(finding.severity, Severity::Advisory);
        assert!(finding.id.is_none());
    }

    #[test]
    fn a_malformed_promotion_is_an_error_rather_than_a_skipped_row() {
        // A promotion record that silently fails to load is indistinguishable
        // from no promotion having happened, which is the ambiguity the record
        // exists to remove.
        let text = "## RULE-001 - A rule\n\n```toml\nstatus = \"VALIDATED\"\npromotions = [\n  \"ASSUMED\"\n]\n```\n\nBody.\n";
        let err =
            parse::parse(std::path::Path::new("t.md"), PathBuf::from("t.md"), text).unwrap_err();
        assert!(err.to_string().contains("promotions[0]"), "{err}");
    }

    #[test]
    fn a_promotion_with_an_unknown_status_is_rejected() {
        let text = "## RULE-001 - A rule\n\n```toml\npromotions = [\n  { from = \"ASSUMED\", to = \"LAUNDERED\" }\n]\n```\n\nBody.\n";
        let err =
            parse::parse(std::path::Path::new("t.md"), PathBuf::from("t.md"), text).unwrap_err();
        assert!(err.to_string().contains("promotions[0].to"), "{err}");
    }

    #[test]
    fn gate_names_and_numbers_agree() {
        for (i, g) in Gate::ALL.iter().enumerate() {
            assert_eq!(g.number as usize, i, "gate {g:?} is out of order");
            assert_eq!(Gate::by_number(g.number), Some(*g));
            assert_eq!(Gate::by_name(g.name), Some(*g));
        }
    }

    #[test]
    fn gate_1_blocks_on_empty_research() {
        let report = evaluate(Gate::by_name("research").unwrap(), &index_from(&[]));
        assert!(!report.passed());
        assert!(rules(&report).contains(&"gate-1/no-sources"));
        assert!(rules(&report).contains(&"gate-1/no-evidence"));
    }

    #[test]
    fn gate_1_blocks_on_orphan_evidence() {
        let index = index_from(&[("EVD-001", "confidence = \"HIGH\"", "Something.")]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-1/orphan-evidence"));
    }

    #[test]
    fn gate_1_blocks_when_everything_cites_one_source() {
        let index = index_from(&[
            ("SRC-001", "", "A."),
            ("EVD-001", "derived-from = [\"SRC-001\"]", "A."),
            ("EVD-002", "derived-from = [\"SRC-001\"]", "B."),
            ("EVD-003", "derived-from = [\"SRC-001\"]", "C."),
        ]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-1/single-source"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn gate_1_passes_with_two_independent_sources() {
        let index = index_from(&[
            ("SRC-001", "epistemic = \"EXPLICIT\"", "A."),
            ("SRC-002", "epistemic = \"EXPLICIT\"", "B."),
            (
                "EVD-001",
                "derived-from = [\"SRC-001\"]\nepistemic = \"EXPLICIT\"",
                "A.",
            ),
            (
                "EVD-002",
                "derived-from = [\"SRC-002\"]\nepistemic = \"EXPLICIT\"",
                "B.",
            ),
        ]);
        assert!(evaluate(Gate::by_name("research").unwrap(), &index).passed());
    }

    #[test]
    fn gate_2_flags_duplicate_terms() {
        let index = index_titled(&[
            ("TERM-001", "Purchase order", "", "Canonical term."),
            ("TERM-002", "purchase order", "", "A second spelling of it."),
        ]);
        let report = evaluate(Gate::by_name("domain").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-2/duplicate-term"));
    }

    #[test]
    fn gate_3_blocks_unjustified_requirement() {
        let index = index_from(&[(
            "REQ-001",
            "epistemic = \"EXPLICIT\"",
            "The system SHALL do X.",
        )]);
        let report = evaluate(Gate::by_name("requirements").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-3/unjustified-requirement"));
    }

    #[test]
    fn gate_3_blocks_assumed_requirement_reaching_build() {
        let index = index_from(&[(
            "REQ-001",
            "derived-from = [\"ASM-001\"]\nepistemic = \"ASSUMED\"",
            "The system SHALL do X.",
        )]);
        let report = evaluate(Gate::by_name("requirements").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-3/unresolved-provenance"));
    }

    #[test]
    fn gate_3_flags_vague_quantifiers() {
        let index = index_from(&[(
            "REQ-001",
            "derived-from = [\"RULE-001\"]\nepistemic = \"EXPLICIT\"",
            "The system should be fast.",
        )]);
        let report = evaluate(Gate::by_name("requirements").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-3/vague-quantifier"));
    }

    #[test]
    fn gate_4_requires_a_concept_decision() {
        let index = index_from(&[(
            "CPT-001",
            "mechanism = \"predictive\"",
            "Predictive replenishment.",
        )]);
        let report = evaluate(Gate::by_name("opportunity").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-4/no-concept-decision"));
    }

    #[test]
    fn gate_4_blocks_falsification_less_hypothesis() {
        let index = index_from(&[("HYP-001", "", "Users will love it.")]);
        let report = evaluate(Gate::by_name("opportunity").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-4/unfalsifiable-hypothesis"));
    }

    #[test]
    fn gate_4_blocks_feature_cloning() {
        let index = index_from(&[(
            "CPT-001",
            "class = \"REPLICATION\"\nrationale = \"our competitor has it\"",
            "Same dashboard as competitor.",
        )]);
        let report = evaluate(Gate::by_name("opportunity").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-4/feature-cloning"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn gate_5_flags_implementation_detail_in_a_spec() {
        let index = index_from(&[(
            "SPEC-001",
            "derived-from = [\"REQ-001\"]\nverifies = [\"AC-001-01\"]",
            "Scenario: use PostgreSQL with SQLAlchemy and store the row.",
        )]);
        let report = evaluate(Gate::by_name("specification").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-5/how-not-what"));
    }

    #[test]
    fn gate_5_blocks_untraced_specification() {
        let index = index_from(&[("SPEC-001", "", "Scenario: something happens.")]);
        let report = evaluate(Gate::by_name("specification").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-5/untraced-specification"));
    }

    #[test]
    fn gate_6_detects_dependency_cycles() {
        let index = index_from(&[
            (
                "TASK-001",
                "depends-on = [\"TASK-002\"]\nverifies = [\"TEST-001\"]",
                "A.",
            ),
            (
                "TASK-002",
                "depends-on = [\"TASK-001\"]\nverifies = [\"TEST-001\"]",
                "B.",
            ),
        ]);
        let report = evaluate(Gate::by_name("plan").unwrap(), &index);
        assert!(
            rules(&report).contains(&"gate-6/dependency-cycle"),
            "{:?}",
            rules(&report)
        );
    }

    #[test]
    fn gate_0_flags_all_assumed_context_as_placeholder() {
        // Placeholders satisfy the existence checks but confirm nothing: the
        // gate still passes, and the report says the context is unconfirmed.
        let index = index_from(&[
            (
                "DOM-001",
                "purpose = \"x\"\nepistemic = \"ASSUMED\"\nconfidence = \"LOW\"",
                "Declared at init.",
            ),
            ("STK-001", "epistemic = \"ASSUMED\"\nconfidence = \"LOW\"", "Declared at init."),
        ]);
        let report = evaluate(Gate::by_name("context").unwrap(), &index);
        assert!(report.passed());
        assert!(rules(&report).contains(&"gate-0/placeholder-context"));
    }

    #[test]
    fn gate_0_stays_quiet_when_context_is_confirmed() {
        let index = index_from(&[
            (
                "DOM-001",
                "purpose = \"x\"\nepistemic = \"INFERRED\"\nconfidence = \"MEDIUM\"",
                "Modelled.",
            ),
            ("STK-001", "epistemic = \"EXPLICIT\"", "Interviewed."),
        ]);
        let report = evaluate(Gate::by_name("context").unwrap(), &index);
        assert!(!rules(&report).contains(&"gate-0/placeholder-context"));
    }

    #[test]
    fn gate_1_flags_open_question_with_no_follow_up() {
        let index = index_from(&[("RES-001", "status = \"OPEN\"\nepistemic = \"UNKNOWN\"", "What?")]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        // Other rules block (no sources at all); the new rule itself is advisory.
        assert!(rules(&report).contains(&"gate-1/open-question-untracked"));
        let finding = report
            .findings
            .iter()
            .find(|f| f.rule == "gate-1/open-question-untracked")
            .unwrap();
        assert_eq!(finding.severity, Severity::Advisory);
    }

    #[test]
    fn gate_6_7_8_say_they_pass_vacuously() {
        // Absence of work must read as absence, not as success.
        let empty = index_from(&[]);
        for (gate, slug) in [
            ("plan", "gate-6/nothing-to-plan"),
            ("implementation", "gate-7/nothing-implemented"),
            ("completion", "gate-8/nothing-completed"),
        ] {
            let report = evaluate(Gate::by_name(gate).unwrap(), &empty);
            assert!(report.passed(), "{gate} should stay advisory-only");
            assert!(rules(&report).contains(&slug), "{gate} should report {slug}");
        }
    }

    #[test]
    fn gate_8_blocks_dangling_references() {
        let index = index_from(&[(
            "REQ-001",
            "derived-from = [\"EVD-404\"]\nepistemic = \"EXPLICIT\"",
            "The system SHALL do X.",
        )]);
        let report = evaluate(Gate::by_name("completion").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-8/dangling-reference"));
    }

    #[test]
    fn gate_8_blocks_assumption_reaching_specification() {
        let index = index_from(&[(
            "REQ-001",
            "derived-from = [\"ASM-001\"]\nepistemic = \"ASSUMED\"\nstatus = \"VALIDATED\"",
            "The system SHALL do X.",
        )]);
        let report = evaluate(Gate::by_name("completion").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-8/assumption-reached-production"));
    }

    #[test]
    fn gate_8_blocks_unresolved_source_conflict() {
        let index = index_from(&[("XCONF-001", "status = \"OPEN\"", "Sources disagree.")]);
        let report = evaluate(Gate::by_name("completion").unwrap(), &index);
        assert!(rules(&report).contains(&"gate-8/unresolved-conflict"));
    }

    #[test]
    fn healthy_workspace_passes_every_gate_it_populates() {
        let index = healthy();
        for gate in Gate::ALL {
            let report = evaluate(*gate, &index);
            assert!(
                report.passed(),
                "gate {} unexpectedly failed: {:?}",
                gate.name,
                report.findings
            );
        }
    }

    #[test]
    fn blocking_findings_sort_before_advisory() {
        let index = index_from(&[("EVD-001", "", "orphan and low confidence")]);
        let report = evaluate(Gate::by_name("research").unwrap(), &index);
        let first_advisory = report
            .findings
            .iter()
            .position(|f| f.severity == Severity::Advisory)
            .unwrap_or(usize::MAX);
        let last_blocking = report
            .findings
            .iter()
            .rposition(|f| f.severity == Severity::Blocking)
            .unwrap_or(0);
        assert!(last_blocking < first_advisory);
    }

    /// Slugs the gate functions can actually emit, read from this file's source.
    ///
    /// The catalogue `RULES` and the `blocking`/`advisory` call sites are two
    /// statements of the same fact, and either can drift from the other without
    /// any compiler noticing. Scraping the source is blunt but it is the only
    /// thing that sees a rule nobody put in the catalogue.
    fn slugs_emitted_in_source() -> Vec<(String, String, String)> {
        let lines: Vec<&str> = include_str!("gates.rs").lines().collect();
        let mut found = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            // Only emission sites: a push whose constructor names a severity.
            // Test assertions and the catalogue itself name slugs too, and
            // picking those up would make every catalogue entry look
            // self-confirming.
            let sev = if t.starts_with("out.push(blocking(") {
                "blocking"
            } else if t.starts_with("out.push(advisory(") {
                "advisory"
            } else {
                continue;
            };
            // The slug is the first string literal after the constructor: its
            // first argument, which may be on the next line. Working in lines
            // rather than byte offsets avoids cutting a multi-byte character in
            // half, which is a panic rather than a wrong answer.
            let after: String = lines[i + 1..]
                .iter()
                .take(4)
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            let Some(start) = after.find('"') else {
                continue;
            };
            let rest = &after[start + 1..];
            let Some(end) = rest.find('"') else { continue };
            if let Some(rest) = rest[..end].strip_prefix("gate-") {
                if let Some((num, name)) = rest.split_once('/') {
                    found.push((num.to_string(), name.to_string(), sev.to_string()));
                }
            }
        }
        found
    }

    #[test]
    fn the_rule_catalogue_lists_every_rule_the_gates_emit() {
        let emitted = slugs_emitted_in_source();
        assert!(
            emitted.len() > 50,
            "source scan found only {} emission sites; the scan is broken, not the catalogue",
            emitted.len()
        );
        let mut catalogued: Vec<(String, String)> = RULES
            .iter()
            .map(|r| (r.gate.to_string(), r.slug.to_string()))
            .collect();
        catalogued.sort();

        for (num, name, _) in &emitted {
            let key = (num.clone(), name.clone());
            assert!(
                catalogued.contains(&key),
                "the gates emit `gate-{num}/{name}` but it is missing from RULES; \
                 `opendomain reference` would not be able to list it"
            );
        }
    }

    #[test]
    fn the_rule_catalogue_has_no_rule_the_gates_do_not_emit() {
        let emitted = slugs_emitted_in_source();
        for r in RULES {
            let found = emitted
                .iter()
                .any(|(n, s, _)| *n == r.gate.to_string() && *s == r.slug);
            assert!(
                found,
                "RULES lists `{}` but no gate function emits it; the catalogue is \
                 advertising a rule that does not exist",
                r.full_slug()
            );
        }
    }

    #[test]
    fn the_rule_catalogue_has_no_duplicates() {
        let mut slugs: Vec<String> = RULES.iter().map(|r| r.full_slug()).collect();
        let before = slugs.len();
        slugs.sort();
        slugs.dedup();
        assert_eq!(before, slugs.len(), "RULES contains a duplicate slug");
    }

    #[test]
    fn every_catalogued_rule_names_a_gate_that_exists() {
        for r in RULES {
            assert!(
                r.gate <= 8,
                "{} names gate {}, and there are only nine gates (0-8)",
                r.full_slug(),
                r.gate
            );
            // Panics if the number is unknown, which is the point: a rule that
            // cannot be traced to a gate cannot be run.
            let _ = r.gate();
        }
    }

    #[test]
    fn the_catalogue_severity_matches_what_the_gate_constructs() {
        // The catalogue says which rules block. If that is wrong, an agent
        // deciding whether to fix a finding is being misled, and `opendomain gate`'s
        // exit code will not match the table `opendomain reference` printed.
        let mut emitted: Vec<(String, String, String)> = slugs_emitted_in_source();
        emitted.sort();
        emitted.dedup();
        for r in RULES {
            let key = (r.gate.to_string(), r.slug.to_string(), String::new());
            let found = emitted
                .iter()
                .find(|(n, s, _)| *n == key.0 && *s == key.1)
                .unwrap_or_else(|| panic!("{} is not emitted by any gate", r.full_slug()));
            let want = if r.severity == Severity::Blocking {
                "blocking"
            } else {
                "advisory"
            };
            assert_eq!(
                found.2,
                want,
                "{} is listed as {} in RULES but the gate constructs it as {}",
                r.full_slug(),
                want,
                found.2
            );
        }
    }
}
