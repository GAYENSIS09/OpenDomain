//! Quality gates.
//!
//! A gate is a machine-checkable predicate over the registries. Its job is to
//! make the framework's central promise — *you may not skip a phase without
//! saying so* — enforceable rather than aspirational.
//!
//! Findings come in two severities:
//!
//! * **Blocking** — the phase produced nothing usable. `od gate` fails.
//! * **Advisory** — a rule of discipline has been broken that does not make the
//!   output wrong. Reported, and counted in a non-zero exit code only under
//!   `--strict`.
//!
//! Gates never mutate the workspace. Reading them as pure functions keeps
//! `od audit` trustworthy: running it twice gives the same answer.

use crate::artifact::Artifact;
use crate::ids::{Id, IdKind};
use crate::vocab::{ArtifactStatus, Confidence, KnowledgeType, Stage};
use crate::workspace::Index;
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
            .filter(|(r, _)| *r == crate::artifact::Relation::OwnedBy)
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
        if a.links.get(&crate::artifact::Relation::OwnedBy).is_none() {
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
                .get(&crate::artifact::Relation::DependsOn)
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
        if !s.links.contains_key(&crate::artifact::Relation::Verifies) {
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
            && !t.links.contains_key(&crate::artifact::Relation::Verifies)
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
            .filter(|(r, _)| *r == crate::artifact::Relation::DependsOn)
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
fn gate_implementation(index: &Index, out: &mut Vec<Finding>) {
    let tests = index.of_kind(IdKind::Test);
    let contracts = index.of_kind(IdKind::Ctr);

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
}

/// Breadth-first search downstream for a `VERIFIED` test.
fn reachable_verified(index: &Index, start: &Id) -> bool {
    let graph = crate::graph::Graph::build(index);
    graph
        .reachable(start, index, crate::graph::Direction::Forward)
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
    crate::graph::Graph::build(index).dependents(id)
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
    use crate::parse;
    use crate::workspace::Index;
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
                "purpose = \"warehouse\"",
                "Warehouse management.",
            ),
            ("STK-001", "", "Warehouse manager."),
            (
                "SRC-001",
                "type = \"STANDARD\"",
                "Title: Warehouse management. URL: https://example.com",
            ),
            (
                "SRC-002",
                "type = \"BOOK\"",
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
            ("TERM-001", "owned-by = \"DOM-001\"", "Canonical term."),
            ("ACT-001", "owned-by = \"DOM-001\"", "Warehouse operator."),
            ("PROC-001", "derived-from = [\"EVD-001\"]", "Receive goods."),
            ("ENT-001", "owned-by = \"DOM-001\"", "An item."),
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
                "derived-from = [\"REQ-001\"]\nverifies = [\"AC-001-01\"]\nstatus = \"APPROVED\"",
                "Scenario: receive within remaining quantity. Then the stock increases.",
            ),
            (
                "AC-001-01",
                "derived-from = [\"SPEC-001\"]",
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
            ("SRC-001", "", "A."),
            ("SRC-002", "", "B."),
            ("EVD-001", "derived-from = [\"SRC-001\"]", "A."),
            ("EVD-002", "derived-from = [\"SRC-002\"]", "B."),
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
}
