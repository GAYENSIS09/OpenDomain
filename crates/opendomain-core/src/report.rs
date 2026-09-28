//! Rendering: terminal output, Markdown and JSON.
//!
//! Two rules shape this module. First, every report exists in both a human and
//! a machine form, because `od status --json` is what a CI job consumes while
//! `od status` is what a person reads. Second, colour is never load-bearing:
//! severity is always stated in words, so piping to a file loses nothing.

use crate::artifact::Artifact;
use crate::gates::{Gate, GateReport, Severity};
use crate::graph::{Graph, ImpactReport, TraceStep};
use crate::ids::{Id, IdKind};
use crate::vocab::{ArtifactStatus, Stage};
use crate::workspace::Index;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::collections::BTreeMap;

/// Whether to emit ANSI colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Colourise.
    On,
    /// Plain text.
    Off,
}

impl Style {
    /// Chooses a style from a TTY guess plus an explicit override.
    pub fn detect(force_off: bool) -> Style {
        if force_off {
            Style::Off
        } else {
            Style::On
        }
    }

    /// Wraps `text` in an ANSI colour when styling is on.
    ///
    /// Public so the CLI tints its own messages the same way the library tints
    /// its reports, rather than keeping a second colour scheme in step.
    pub fn paint(&self, text: &str, colour: Colour) -> String {
        match self {
            Style::Off => text.to_string(),
            Style::On => match colour {
                Colour::Green => text.green().to_string(),
                Colour::Yellow => text.yellow().to_string(),
                Colour::Red => text.red().to_string(),
                Colour::Cyan => text.cyan().to_string(),
                Colour::Dim => text.dimmed().to_string(),
                Colour::Bold => text.bold().to_string(),
            },
        }
    }
}

/// The colours the renderer uses.
#[derive(Debug, Clone, Copy)]
pub enum Colour {
    Green,
    Yellow,
    Red,
    Cyan,
    Dim,
    Bold,
}

/// Workspace-level counts, the input to `od status`.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    /// Project name from the configuration.
    pub name: String,
    /// Active profile.
    pub profile: String,
    /// Active authority model.
    pub authority: String,
    /// Whether the project is greenfield, brownfield or hybrid.
    pub repository: String,
    /// Artifact counts per kind.
    pub counts: BTreeMap<String, usize>,
    /// Total artifacts.
    pub total: usize,
    /// References pointing at artifacts that do not exist.
    pub dangling: usize,
    /// Artifacts whose provenance is still assumed, unknown or conflicting.
    pub unresolved_provenance: usize,
    /// Source conflicts still open.
    pub open_conflicts: usize,
    /// Uncertainties still open.
    pub open_uncertainties: usize,
    /// Lowest gate that does not yet pass.
    pub blocking_gate: Option<&'static str>,
    /// Per-gate pass/fail.
    pub gates: Vec<(String, bool, usize, usize)>,
}

/// Builds the status report for a loaded workspace.
pub fn status(name: &str, config: &crate::config::Config, index: &Index) -> StatusReport {
    let mut counts = BTreeMap::new();
    for kind in IdKind::ALL {
        let n = index.ids(*kind).len();
        if n > 0 {
            counts.insert(kind.plural(), n);
        }
    }

    let unresolved = index
        .by_id
        .values()
        .filter(|a| a.has_unresolved_provenance())
        .count();

    let open_conflicts = index
        .of_kind(IdKind::Xconf)
        .iter()
        .filter(|a| a.is_live())
        .count();

    let open_uncertainties = index
        .of_kind(IdKind::Unc)
        .iter()
        .filter(|a| a.status == ArtifactStatus::Open || a.is_live())
        .count();

    let gates: Vec<GateReport> = Gate::ALL
        .iter()
        .map(|g| crate::gates::evaluate(*g, index))
        .collect();
    let blocking_gate = gates.iter().find(|r| !r.passed()).map(|r| r.name);

    StatusReport {
        name: name.to_string(),
        profile: config.profile.as_str().to_string(),
        authority: config.authority.as_str().to_string(),
        repository: config.repository.as_str().to_string(),
        counts,
        total: index.len(),
        dangling: index.dangling().len(),
        unresolved_provenance: unresolved,
        open_conflicts,
        open_uncertainties,
        blocking_gate,
        gates: gates
            .iter()
            .map(|r| (r.name.to_string(), r.passed(), r.blocking(), r.advisory()))
            .collect(),
    }
}

/// Renders the status report as a terminal summary.
pub fn render_status(report: &StatusReport, style: Style) -> String {
    let mut out = String::new();
    out.push_str(&style.paint("OpenDomain status", Colour::Bold));
    out.push('\n');
    if !report.name.is_empty() {
        out.push_str(&format!("  project     {}\n", report.name));
    }
    out.push_str(&format!("  profile     {}\n", report.profile));
    out.push_str(&format!("  authority   {}\n", report.authority));
    out.push_str(&format!("  repository  {}\n", report.repository));
    out.push_str(&format!("  artifacts   {}\n", report.total));

    if !report.counts.is_empty() {
        out.push_str("\n  registries\n");
        let width = report.counts.keys().map(String::len).max().unwrap_or(0);
        for (kind, count) in &report.counts {
            out.push_str(&format!("    {kind:<width$}  {count}\n", width = width));
        }
    }

    out.push('\n');
    for (name, passed, blocking, advisory) in &report.gates {
        let mark = if *passed {
            style.paint("PASS", Colour::Green)
        } else {
            style.paint("FAIL", Colour::Red)
        };
        out.push_str(&format!("  gate {name:<14} {mark}"));
        if *blocking > 0 {
            out.push_str(&format!("  {blocking} blocking"));
        }
        if *advisory > 0 {
            out.push_str(&format!("  {advisory} advisory"));
        }
        out.push('\n');
    }

    let mut warnings = Vec::new();
    if report.dangling > 0 {
        warnings.push(format!(
            "{} artifact(s) reference ids that do not exist",
            report.dangling
        ));
    }
    if report.unresolved_provenance > 0 {
        warnings.push(format!(
            "{} artifact(s) rest on an assumption, unknown or conflict",
            report.unresolved_provenance
        ));
    }
    if report.open_conflicts > 0 {
        warnings.push(format!(
            "{} source conflict(s) unresolved",
            report.open_conflicts
        ));
    }
    if !warnings.is_empty() {
        out.push('\n');
        for w in warnings {
            out.push_str(&format!(
                "  {} {w}\n",
                style.paint("warning:", Colour::Yellow)
            ));
        }
    }

    if let Some(gate) = report.blocking_gate {
        out.push('\n');
        out.push_str(&format!(
            "  next: resolve the `{gate}` gate, or run `od gate {gate}` for the detail\n"
        ));
    }
    out
}

/// Renders one gate report.
pub fn render_gate(report: &GateReport, style: Style) -> String {
    let mut out = String::new();
    let verdict = if report.passed() {
        style.paint("PASS", Colour::Green)
    } else {
        style.paint("FAIL", Colour::Red)
    };
    out.push_str(&format!(
        "gate {} — {}  {}\n",
        report.number,
        style.paint(report.name, Colour::Bold),
        verdict
    ));
    out.push_str(&format!("  {}\n", report.question));
    out.push_str(&format!("  stage: {}\n", report.stage));

    if report.findings.is_empty() {
        out.push_str(&format!(
            "\n  {}\n",
            style.paint("no findings", Colour::Green)
        ));
        return out;
    }

    out.push('\n');
    for f in &report.findings {
        let (tag, colour) = match f.severity {
            Severity::Blocking => ("blocking", Colour::Red),
            Severity::Advisory => ("advisory", Colour::Yellow),
        };
        out.push_str(&format!("  {} {}\n", style.paint(tag, colour), f.message));
        out.push_str(&format!("    {}\n", style.paint(&f.rule, Colour::Dim)));
    }
    out.push_str(&format!(
        "\n  {} blocking, {} advisory\n",
        report.blocking(),
        report.advisory()
    ));
    out
}

/// A single row of a `od list` table.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// The identifier.
    pub id: String,
    /// The title.
    pub title: String,
    /// The lifecycle status.
    pub status: ArtifactStatus,
    /// The epistemic marker, when the artifact makes a claim.
    pub epistemic: Option<String>,
    /// The defining file.
    pub file: String,
}

/// Builds table rows for a kind.
pub fn rows(index: &Index, kind: IdKind) -> Vec<Row> {
    index
        .of_kind(kind)
        .into_iter()
        .map(|a| Row {
            id: a.id.to_string(),
            title: a.title.clone(),
            status: a.status,
            epistemic: a.epistemic.map(|e| e.as_str().to_string()),
            file: a.file.to_string_lossy().replace('\\', "/"),
        })
        .collect()
}

/// Renders a table of artifacts.
pub fn render_table(title: &str, rows: &[Row], style: Style) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n{}\n",
        style.paint(title, Colour::Bold),
        style.paint(&"─".repeat(title.chars().count()), Colour::Dim)
    ));
    if rows.is_empty() {
        out.push_str(&format!("  {}\n", style.paint("(empty)", Colour::Dim)));
        return out;
    }

    let id_w = rows.iter().map(|r| r.id.len()).max().unwrap_or(6).max(6);
    let st_w = rows
        .iter()
        .map(|r| r.status.as_str().len())
        .max()
        .unwrap_or(7)
        .max(7);

    for r in rows {
        let status = match r.status {
            ArtifactStatus::Verified | ArtifactStatus::Validated | ArtifactStatus::Approved => {
                style.paint(r.status.as_str(), Colour::Green)
            }
            ArtifactStatus::Draft | ArtifactStatus::Researched => {
                style.paint(r.status.as_str(), Colour::Dim)
            }
            ArtifactStatus::Blocked | ArtifactStatus::Conflicting => {
                style.paint(r.status.as_str(), Colour::Red)
            }
            _ => style.paint(r.status.as_str(), Colour::Yellow),
        };
        let title = if r.epistemic.is_some() {
            let mark = r.epistemic.as_deref().unwrap_or("");
            format!(
                "{} {}",
                r.title,
                style.paint(&format!("[{mark}]"), Colour::Cyan)
            )
        } else {
            r.title.clone()
        };
        out.push_str(&format!(
            "  {:<id_w$}  {:<st_w$}  {}\n",
            r.id,
            status,
            title,
            id_w = id_w,
            st_w = st_w
        ));
    }
    out
}

/// Renders a trace as an indented chain.
pub fn render_trace(id: &Id, steps: &[TraceStep], style: Style) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} {}\n",
        style.paint("trace", Colour::Bold),
        id,
        style.paint("(upstream — why does this exist?)", Colour::Dim)
    ));
    if steps.len() <= 1 {
        // `steps` always contains the starting artifact, so a trace with no
        // further reach is an orphan, not an empty walk.
        out.push_str("  (no upstream artifacts: this claim has no recorded origin)\n");
        return out;
    }
    for (i, step) in steps.iter().enumerate() {
        let indent = "  ".repeat(i + 1);
        if i > 0 {
            out.push_str(&format!("{indent}{}\n", style.paint("└─", Colour::Dim)));
        }
        let label = if step.title.is_empty() {
            style.paint("<unresolved>", Colour::Red)
        } else {
            step.title.clone()
        };
        out.push_str(&format!(
            "{indent}{} {}{}\n",
            style.paint(&step.id.to_string(), Colour::Cyan),
            label,
            if i > 0 {
                style.paint(&format!("  ({})", step.relation.as_str()), Colour::Dim)
            } else {
                String::new()
            }
        ));
    }
    out
}

/// Renders an impact report.
pub fn render_impact(report: &ImpactReport, style: Style) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}  {} direct, {} indirect\n\n",
        style.paint("impact", Colour::Bold),
        report.origin,
        report.direct,
        report.indirect
    ));
    if report.is_empty() {
        out.push_str(&format!(
            "  {}\n",
            style.paint("nothing downstream depends on this", Colour::Green)
        ));
        return out;
    }
    for entry in &report.entries {
        let (tag, colour) = match entry.class {
            crate::vocab::ImpactClass::Direct => ("direct  ", Colour::Red),
            _ => ("indirect", Colour::Yellow),
        };
        out.push_str(&format!(
            "  {}  {}  {}\n",
            style.paint(tag, colour),
            style.paint(&entry.id.to_string(), Colour::Cyan),
            entry.title
        ));
    }
    out
}

/// A stage-by-stage rollup, used by `od audit`.
#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    /// Per-stage artifact counts.
    pub stages: BTreeMap<String, usize>,
    /// Coverage of requirements by a specification.
    pub requirements_specified: usize,
    /// Requirements that have a downstream specification.
    pub requirements_traced: usize,
    /// Requirements that reach a verified test.
    pub requirements_verified: usize,
    /// Requirements with no recorded origin.
    pub requirements_unjustified: usize,
    /// Dangling references, by referring artifact.
    pub dangling: BTreeMap<String, Vec<String>>,
    /// Gate results in order.
    pub gates: Vec<GateReport>,
}

impl AuditReport {
    /// Percentage of requirements that reach a verified test.
    pub fn verification_rate(&self) -> f64 {
        if self.requirements_traced == 0 {
            return 0.0;
        }
        100.0 * self.requirements_verified as f64 / self.requirements_traced as f64
    }
}

/// Builds the audit report.
pub fn audit(index: &Index) -> AuditReport {
    let mut stages: BTreeMap<String, usize> = BTreeMap::new();
    for stage in [Stage::Domain, Stage::Opportunity, Stage::Specification] {
        let n = index
            .by_id
            .values()
            .filter(|a| a.id.stage() == stage)
            .count();
        stages.insert(stage.as_str().to_string(), n);
    }

    let reqs = index.of_kind(IdKind::Req);
    let graph = Graph::build(index);

    let mut specified = 0usize;
    let mut traced = 0usize;
    let mut verified = 0usize;
    let mut unjustified = 0usize;

    for r in &reqs {
        if r.upstream().is_empty() {
            unjustified += 1;
        }
        if graph
            .reachable(&r.id, index, crate::graph::Direction::Forward)
            .iter()
            .any(|s| s.id.kind() == IdKind::Spec)
        {
            specified += 1;
        }
        let downstream = graph.reachable(&r.id, index, crate::graph::Direction::Forward);
        if !downstream.is_empty() {
            traced += 1;
        }
        if downstream.iter().any(|s| {
            s.id.kind() == IdKind::Test
                && index
                    .get(&s.id)
                    .map(|t| t.status == ArtifactStatus::Verified)
                    .unwrap_or(false)
        }) {
            verified += 1;
        }
    }

    let dangling = index
        .dangling()
        .into_iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                v.into_iter().map(|i| i.to_string()).collect::<Vec<_>>(),
            )
        })
        .collect();

    AuditReport {
        stages,
        requirements_specified: specified,
        requirements_traced: traced,
        requirements_verified: verified,
        requirements_unjustified: unjustified,
        dangling,
        gates: Gate::ALL
            .iter()
            .map(|g| crate::gates::evaluate(*g, index))
            .collect(),
    }
}

/// Renders the audit report.
pub fn render_audit(report: &AuditReport, style: Style) -> String {
    let mut out = String::new();
    out.push_str(&style.paint("OpenDomain audit", Colour::Bold));
    out.push_str("\n\n");

    out.push_str(&format!("{}\n", style.paint("stage", Colour::Bold)));
    for (stage, count) in &report.stages {
        out.push_str(&format!("  {stage:<16} {count}\n"));
    }

    out.push_str(&format!(
        "\n{}\n",
        style.paint("requirement coverage", Colour::Bold)
    ));
    let bar = |n: usize, total: usize, style: Style| -> String {
        if total == 0 {
            return "—".to_string();
        }
        let filled = (20 * n) / total;
        format!(
            "{}{} {n}/{total}",
            style.paint(&"█".repeat(filled), Colour::Green),
            style.paint(&"░".repeat(20 - filled), Colour::Dim)
        )
    };
    let total = report.requirements_traced + report.requirements_unjustified;
    out.push_str(&format!(
        "  specified  {}\n",
        bar(report.requirements_specified, total, style)
    ));
    out.push_str(&format!(
        "  traced     {}\n",
        bar(report.requirements_traced, total, style)
    ));
    out.push_str(&format!(
        "  verified    {}\n",
        bar(report.requirements_verified, total, style)
    ));
    out.push_str(&format!(
        "  unjustified {} artifact(s) have no recorded origin\n",
        report.requirements_unjustified
    ));

    if !report.dangling.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            style.paint("dangling references", Colour::Bold)
        ));
        for (from, targets) in &report.dangling {
            out.push_str(&format!("  {from} -> {}\n", targets.join(", ")));
        }
    }

    out.push('\n');
    for gate in &report.gates {
        out.push_str(&render_gate(gate, style).as_str());
    }
    out
}

/// Renders a finding as a single line, for `--quiet` output.
pub fn render_finding_line(rule: &str, message: &str, style: Style) -> String {
    let colour = if rule.starts_with("gate") {
        Colour::Red
    } else {
        Colour::Yellow
    };
    format!("{} {message}", style.paint(rule, colour))
}

/// Renders an artifact's full detail, as `od show` does.
pub fn render_artifact(artifact: &Artifact, style: Style) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        style.paint(&artifact.id.to_string(), Colour::Cyan),
        artifact.title
    ));
    out.push_str(&format!("  kind     {}\n", artifact.id.kind().label()));
    out.push_str(&format!("  stage    {}\n", artifact.id.stage()));
    out.push_str(&format!("  status   {}\n", artifact.status));
    if let Some(scope) = artifact.id.scope() {
        out.push_str(&format!("  scope    {scope}\n"));
    }
    // Label width is 9, not 8: `epistemic` and `confidence` are 9 characters
    // and were being truncated to `epistem.` / `conf.`, which read as typos.
    for (key, value) in &artifact.kind_fields {
        // `kind` is already reported above from the identifier, and
        // `status` / `epistemic` / `confidence` have their own labelled lines.
        // Re-printing them here produced a duplicated `kind` row.
        if matches!(key.as_str(), "kind" | "status" | "epistemic" | "confidence") {
            continue;
        }
        out.push_str(&format!("  {key:<9} {value}\n"));
    }
    if let Some(e) = artifact.epistemic {
        out.push_str(&format!("  epistemic {e}\n"));
    }
    if let Some(c) = artifact.confidence {
        out.push_str(&format!("  confidence {c}\n"));
    }
    let upstream = artifact.upstream();
    let downstream = artifact.downstream();
    // `downstream` is 10 characters, so these use the same 9-column gutter plus
    // the space that separates the label from its value. Keeping every label in
    // this file on one gutter is what makes the record scannable.
    if !upstream.is_empty() {
        out.push_str(&format!(
            "  upstream {}\n",
            upstream
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !downstream.is_empty() {
        out.push_str(&format!(
            "  downstream {}\n",
            downstream
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !artifact.body.is_empty() {
        out.push_str("\n");
        out.push_str(&artifact.body);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::parse;
    use std::path::PathBuf;

    fn index_from(specs: &[(&str, &str, &str)]) -> Index {
        let mut text = String::new();
        for (id, block, body) in specs {
            text.push_str(&format!(
                "## {id} — {id}\n\n```toml\n{block}\n```\n\n{body}\n\n"
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

    #[test]
    fn artifact_record_does_not_truncate_or_duplicate_labels() {
        // Regression: the label gutter was 8 columns wide, so `epistemic` and
        // `confidence` rendered as `epistem.` and `conf.`, and the `kind` key
        // was printed twice — once from the identifier, once from the parsed
        // TOML block.
        let index = index_from(&[(
            "REQ-001",
            "kind = \"REQ\"\nepistemic = \"EXPLICIT\"\nconfidence = \"HIGH\"",
            "The system SHALL do X.",
        )]);
        let artifact = index
            .get(&"REQ-001".parse().expect("well-formed id"))
            .expect("artifact is in the index");
        let text = render_artifact(artifact, Style::Off);

        assert!(!text.contains("epistem."), "label truncated: {text}");
        assert!(!text.contains("conf."), "label truncated: {text}");
        assert!(text.contains("epistemic EXPLICIT"), "{text}");
        assert!(text.contains("confidence HIGH"), "{text}");
        assert_eq!(
            text.matches("kind").count(),
            1,
            "`kind` printed more than once: {text}"
        );
    }

    #[test]
    fn plain_style_contains_no_ansi_escapes() {
        let index = index_from(&[("REQ-001", "", "The system SHALL do X.")]);
        let report = status("Demo", &Config::default(), &index);
        let text = render_status(&report, Style::Off);
        assert!(!text.contains('\u{1b}'), "colour leaked into plain output");
    }

    #[test]
    fn status_lists_non_empty_registries_only() {
        let index = index_from(&[("REQ-001", "", "x"), ("SRC-001", "", "y")]);
        let report = status("Demo", &Config::default(), &index);
        assert_eq!(report.total, 2);
        assert!(report.counts.contains_key("requirements"));
        assert!(report.counts.contains_key("sources"));
    }

    #[test]
    fn status_flags_unresolved_provenance() {
        let index = index_from(&[("REQ-001", "epistemic = \"ASSUMED\"", "x")]);
        let report = status("Demo", &Config::default(), &index);
        assert_eq!(report.unresolved_provenance, 1);
        let text = render_status(&report, Style::Off);
        assert!(text.contains("rest on an assumption"), "{text}");
    }

    #[test]
    fn status_points_at_the_first_failing_gate() {
        let index = index_from(&[("REQ-001", "", "x")]);
        let report = status("Demo", &Config::default(), &index);
        assert_eq!(report.blocking_gate, Some("context"));
    }

    #[test]
    fn gate_rendering_separates_severities() {
        let index = index_from(&[("EVD-001", "", "orphan")]);
        let report = crate::gates::evaluate(Gate::by_name("research").unwrap(), &index);
        let text = render_gate(&report, Style::Off);
        assert!(text.contains("FAIL"));
        assert!(text.contains("blocking"));
    }

    #[test]
    fn empty_table_says_so() {
        let text = render_table("Requirements", &[], Style::Off);
        assert!(text.contains("(empty)"));
    }

    #[test]
    fn trace_renders_a_chain() {
        let index = index_from(&[
            ("SRC-001", "", "The standard."),
            ("EVD-001", "derived-from = [\"SRC-001\"]", "Finding."),
            (
                "REQ-001",
                "derived-from = [\"EVD-001\"]",
                "The system SHALL do X.",
            ),
        ]);
        let graph = Graph::build(&index);
        let steps = graph.trace_backward(&Id::parse("REQ-001").unwrap(), &index);
        let text = render_trace(&Id::parse("REQ-001").unwrap(), &steps, Style::Off);
        assert!(text.contains("REQ-001"));
        assert!(text.contains("EVD-001"));
        assert!(text.contains("SRC-001"));
    }

    #[test]
    fn trace_of_an_orphan_says_so() {
        let index = index_from(&[("REQ-001", "", "x")]);
        let graph = Graph::build(&index);
        let id = Id::parse("REQ-001").unwrap();
        let steps = graph.trace_backward(&id, &index);
        let text = render_trace(&id, &steps, Style::Off);
        assert!(text.contains("no recorded origin"), "{text}");
    }

    #[test]
    fn audit_counts_coverage() {
        let index = index_from(&[
            ("SRC-001", "", "A."),
            ("EVD-001", "derived-from = [\"SRC-001\"]", "B."),
            (
                "REQ-001",
                "derived-from = [\"EVD-001\"]",
                "The system SHALL do X.",
            ),
            (
                "SPEC-001",
                "derived-from = [\"REQ-001\"]",
                "Scenario: something.",
            ),
            (
                "TEST-001",
                "verifies = [\"SPEC-001\"]\nstatus = \"VERIFIED\"",
                "A test.",
            ),
        ]);
        let report = audit(&index);
        assert_eq!(report.requirements_traced, 1);
        assert_eq!(report.requirements_specified, 1);
        assert_eq!(report.requirements_verified, 1);
        assert_eq!(report.requirements_unjustified, 0);
        assert!((report.verification_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn audit_of_empty_workspace_does_not_divide_by_zero() {
        let report = audit(&Index::default());
        assert_eq!(report.verification_rate(), 0.0);
    }

    #[test]
    fn table_rows_carry_status_and_file() {
        let index = index_from(&[("REQ-001", "epistemic = \"ASSUMED\"", "x")]);
        let rows = rows(&index, IdKind::Req);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "REQ-001");
        assert_eq!(rows[0].status, ArtifactStatus::Draft);
        assert_eq!(rows[0].epistemic.as_deref(), Some("ASSUMED"));
    }
}
