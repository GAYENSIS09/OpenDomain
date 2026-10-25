//! First-class run provenance (F14) — the verified half of the
//! run→registries bridge that L9 recorded as "prose and convention".
//!
//! A record may cite the archived run that produced (or verified) it with a
//! first-class optional field:
//!
//! ```toml
//! run = "runs/run-001-reservations"   # or bare `run-001-reservations`, or a full path
//! run-node = "verificateur"           # optional: the node whose verdict is claimed
//! ```
//!
//! [`verify`] checks that citation against the archive on disk: the run
//! directory exists under `opendomain/harness/runs/` (the documented archive
//! location), its `graph.json` and `gates.log` are present, the archived
//! outputs are counted, the run did not terminate in a final failure, and —
//! when a node is cited — that node's last verdict line is read from
//! `gates.log`.
//!
//! The isolation the framework builds on purpose is respected: nothing here
//! imports any execution module. A run archive is a documented directory
//! layout *of the workspace*, so verifying a citation is a pure function of
//! the workspace on disk — the same stance the gates already take.

use crate::model::artifact::Artifact;
use crate::registries::workspace::Index;
use std::path::Path;

/// Where archived runs live, relative to `opendomain/` — the documented
/// archive location, shared with records that cite `run = "…"`.
pub const RUNS_ROOT: &str = "harness/runs";

/// A record's `run = …` citation, normalized to the run name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCitation {
    /// The run name — `run-<NNN>-<slug>` — however it was written.
    pub run: String,
    /// The optional cited node (`run-node = "…"`).
    pub node: Option<String>,
}

/// Reads a record's run citation. Absent field → `None`; a present field is
/// normalized by taking the `run-*` component of whatever path shape was
/// written, so `runs/run-001-x`, `opendomain/harness/runs/run-001-x` and
/// `run-001-x` all resolve the same way and cannot drift by spelling.
pub fn citation(a: &Artifact) -> Option<RunCitation> {
    let raw = a.field("run")?;
    Some(RunCitation {
        run: normalize_run_name(raw),
        node: a
            .field("run-node")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    })
}

/// The `run-<NNN>-<slug>` component of a written path, last one winning.
fn normalize_run_name(raw: &str) -> String {
    let raw = raw.trim().replace('\\', "/");
    raw.split('/')
        .filter(|part| part.trim().starts_with("run-"))
        .next_back()
        .map(|part| part.trim().to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

/// What the archive on disk says about one cited run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunVerification {
    pub run_name: String,
    /// The run directory exists under `opendomain/harness/runs/`.
    pub exists: bool,
    /// The frozen `graph.json` is present — the executed plan, never rewritten.
    pub frozen_graph: bool,
    /// `gates.log` is present — the verdict log, append-only.
    pub gates_log: bool,
    /// How many archived outputs (`context/*.json`) the run carries.
    pub outputs: usize,
    /// The `[!!] final …` line, when the run terminated in a failure.
    pub final_failure: Option<String>,
    /// The cited node's last verdict, when `run-node` is declared.
    pub node_verdict: Option<NodeVerdict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeVerdict {
    /// The node's last `gates.log` line is green — quoted verbatim.
    Verified(String),
    /// The node's last `gates.log` line is red — quoted verbatim.
    Failed(String),
}

impl NodeVerdict {
    pub fn is_ok(&self) -> bool {
        matches!(self, NodeVerdict::Verified(_))
    }
}

/// Verifies one citation against the runs directory. Read-only; a missing
/// archive is a finding, never an error.
pub fn verify(runs_dir: &Path, cited: &RunCitation) -> RunVerification {
    let mut out = RunVerification {
        run_name: cited.run.clone(),
        ..Default::default()
    };
    let dir = runs_dir.join(&cited.run);
    if !dir.is_dir() {
        return out;
    }
    out.exists = true;
    out.frozen_graph = dir.join("graph.json").is_file();
    let log_path = dir.join("gates.log");
    out.gates_log = log_path.is_file();
    if let Ok(entries) = std::fs::read_dir(dir.join("context")) {
        out.outputs = entries
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .count();
    }
    if out.gates_log {
        if let Ok(text) = std::fs::read_to_string(&log_path) {
            parse_log(&text, cited, &mut out);
        }
    }
    out
}

/// Reads `gates.log` (the format `runflow::gates_log` writes): verdict lines
/// `[ok|ko] {start|run-cmd|gate} {node} …` and the terminal `[!!] final …`.
fn parse_log(text: &str, cited: &RunCitation, out: &mut RunVerification) {
    let mut node_line: Option<(bool, String)> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with("[!!] final") {
            out.final_failure = Some(line.to_string());
            continue;
        }
        let (verdict_ok, rest) = if let Some(rest) = line.strip_prefix("[ok] ") {
            (true, rest)
        } else if let Some(rest) = line.strip_prefix("[ko] ") {
            (false, rest)
        } else {
            continue;
        };
        // `run-cmd {node} : …` / `gate   {node} (…)` — the node follows the
        // family word; node ids carry no spaces by construction.
        let node = rest
            .splitn(2, ' ')
            .nth(1)
            .unwrap_or("")
            .split([' ', '('])
            .find(|part| !part.is_empty())
            .unwrap_or("");
        if cited
            .node
            .as_deref()
            .is_some_and(|n| !n.is_empty() && n == node)
        {
            node_line = Some((verdict_ok, line.to_string()));
        }
    }
    if cited.node.is_some() {
        out.node_verdict = node_line.map(|(ok, line)| {
            if ok {
                NodeVerdict::Verified(line)
            } else {
                NodeVerdict::Failed(line)
            }
        });
    }
}

/// One audited citation: who cites, what it cites, and what the disk says.
#[derive(Debug, Clone)]
pub struct CitationRow {
    pub artifact: String,
    pub artifact_prefix: String,
    pub citation: RunCitation,
    pub verification: RunVerification,
}

/// The full provenance audit: every citation checked, plus the runs nobody
/// cites — archived evidence no record claims, which is the bridge's other
/// direction of decay.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceReport {
    pub rows: Vec<CitationRow>,
    /// Runs present on disk but cited by no record.
    pub orphan_runs: Vec<String>,
    pub total_runs: usize,
    /// `false` when `opendomain/harness/runs/` does not exist at all.
    pub runs_dir_exists: bool,
}

/// Audits the whole workspace: every artifact carrying `run = …` is verified
/// against the archive it names, and every archived run that no record cites
/// is listed. Pure read; nothing is written.
pub fn audit(index: &Index, opendomain_dir: &Path) -> ProvenanceReport {
    let runs_dir = opendomain_dir.join(RUNS_ROOT);
    let mut report = ProvenanceReport {
        runs_dir_exists: runs_dir.is_dir(),
        ..Default::default()
    };

    let mut artifacts: Vec<&Artifact> = index.by_id.values().collect();
    artifacts.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));
    let mut cited = Vec::new();
    for a in artifacts {
        if let Some(cit) = citation(a) {
            let v = verify(&runs_dir, &cit);
            cited.push(v.run_name.clone());
            report.rows.push(CitationRow {
                artifact: a.id.to_string(),
                artifact_prefix: a.id.kind().prefix().to_string(),
                citation: cit,
                verification: v,
            });
        }
    }

    if report.runs_dir_exists {
        if let Ok(entries) = std::fs::read_dir(&runs_dir) {
            let mut runs: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with("run-"))
                .collect();
            runs.sort();
            report.total_runs = runs.len();
            report.orphan_runs = runs
                .into_iter()
                .filter(|r| !cited.iter().any(|c| c == r))
                .collect();
        }
    }
    report
}

impl ProvenanceReport {
    /// The number of citations whose archive exists, is complete (frozen
    /// graph + gates.log), did not end in a final failure, and — when a node
    /// was cited — shows that node green.
    pub fn verified_count(&self) -> usize {
        self.rows.iter().filter(|r| row_ok(&r.verification)).count()
    }

    /// The rest: citations that name a missing or broken archive.
    pub fn broken(&self) -> Vec<&CitationRow> {
        self.rows
            .iter()
            .filter(|r| !row_ok(&r.verification))
            .collect()
    }
}

fn row_ok(v: &RunVerification) -> bool {
    v.exists
        && v.frozen_graph
        && v.gates_log
        && v.final_failure.is_none()
        && v.node_verdict.as_ref().map(|n| n.is_ok()).unwrap_or(true)
}

/// The one-line verdict the CLI prints for a row.
pub fn verdict_label(v: &RunVerification) -> &'static str {
    if !v.exists {
        return "MISSING";
    }
    if !v.frozen_graph || !v.gates_log {
        return "INCOMPLETE";
    }
    if v.final_failure.is_some() {
        return "ENDED-IN-FAILURE";
    }
    match &v.node_verdict {
        Some(nv) if !nv.is_ok() => "NODE-RED",
        Some(_) => "VERIFIED",
        None => "ARCHIVED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registries::parse;

    fn artifact_with_run(id: &str, toml_extra: &str) -> Artifact {
        let text = format!(
            "## {id} - Something executed\n\n```toml\nstatus = \"VERIFIED\"\n{toml_extra}\n```\n"
        );
        let doc = parse::parse(
            std::path::Path::new("validation/acceptance-tests.md"),
            std::path::PathBuf::from("validation/acceptance-tests.md"),
            &text,
        )
        .unwrap();
        doc.artifacts.into_iter().next().unwrap()
    }

    fn write_run(dir: &Path, name: &str, log: &str) {
        let run = dir.join(RUNS_ROOT).join(name);
        std::fs::create_dir_all(run.join("context")).unwrap();
        std::fs::write(run.join("graph.json"), "{}").unwrap();
        std::fs::write(run.join("gates.log"), log).unwrap();
        std::fs::write(run.join("context/verificateur.json"), "{}").unwrap();
    }

    const GREEN_LOG: &str = concat!(
        "# gates y_i ∈ C_i (par ordre d'exécution)\n",
        "[ ] start  analyste\n",
        "[ok] run-cmd verificateur : cargo test\n",
        "[ok] gate   verificateur (attempts: 1)\n",
        "[ok] gate   synthese (attempts: 2)\n",
    );

    #[test]
    fn citation_normalizes_every_path_shape_the_same_way() {
        let a = artifact_with_run("TEST-001", "run = \"runs/run-007-reservations\"");
        assert_eq!(
            citation(&a),
            Some(RunCitation {
                run: "run-007-reservations".into(),
                node: None
            })
        );
        let a = artifact_with_run(
            "TEST-001",
            "run = \"opendomain/harness/runs/run-007-reservations\"",
        );
        assert_eq!(citation(&a).unwrap().run, "run-007-reservations");
        let a = artifact_with_run("TEST-001", "run = \"run-007-reservations\"");
        assert_eq!(citation(&a).unwrap().run, "run-007-reservations");
        let a = artifact_with_run(
            "TEST-001",
            "run = \"run-007-reservations\"\nrun-node = \"verificateur\"",
        );
        assert_eq!(citation(&a).unwrap().node.as_deref(), Some("verificateur"));
        // No field, no citation — nothing inferred.
        let a = artifact_with_run("TEST-001", "level = \"INTEGRATION\"");
        assert!(citation(&a).is_none());
    }

    #[test]
    fn a_green_cited_node_verifies_against_its_archive() {
        let dir = temp_dir("green");
        write_run(&dir, "run-001-demo", GREEN_LOG);
        let cited = RunCitation {
            run: "run-001-demo".into(),
            node: Some("verificateur".into()),
        };
        let v = verify(&dir.join(RUNS_ROOT), &cited);
        assert!(v.exists);
        assert!(v.frozen_graph);
        assert!(v.gates_log);
        assert_eq!(v.outputs, 1);
        assert!(v.final_failure.is_none());
        assert_eq!(
            v.node_verdict,
            Some(NodeVerdict::Verified(
                "[ok] gate   verificateur (attempts: 1)".into()
            ))
        );
        assert!(row_ok(&v));
        assert_eq!(verdict_label(&v), "VERIFIED");
        cleanup(&dir);
    }

    #[test]
    fn a_red_node_verdict_and_a_final_failure_are_findings_not_errors() {
        let dir = temp_dir("red");
        write_run(
            &dir,
            "run-002-demo",
            concat!(
                "# gates\n",
                "[ok] gate   verificateur (attempts: 1)\n",
                "[ko] gate   verificateur (attempt 2) — criteria_failure : mismatch\n",
                "[!!] final  verificateur — criteria_failure → directive\n",
            ),
        );
        let v = verify(
            &dir.join(RUNS_ROOT),
            &RunCitation {
                run: "run-002-demo".into(),
                node: Some("verificateur".into()),
            },
        );
        // The LAST verdict line wins: the node went red on attempt 2.
        assert_eq!(
            v.node_verdict,
            Some(NodeVerdict::Failed(
                "[ko] gate   verificateur (attempt 2) — criteria_failure : mismatch".into()
            ))
        );
        assert!(v.final_failure.is_some());
        assert_eq!(verdict_label(&v), "ENDED-IN-FAILURE");
        assert!(!row_ok(&v));
        cleanup(&dir);
    }

    #[test]
    fn a_citation_naming_a_missing_run_is_the_point_of_the_check() {
        let dir = temp_dir("missing");
        let v = verify(
            &dir.join(RUNS_ROOT),
            &RunCitation {
                run: "run-099-ghost".into(),
                node: None,
            },
        );
        assert!(!v.exists);
        assert!(!v.frozen_graph);
        assert!(!v.gates_log);
        assert_eq!(v.outputs, 0);
        assert_eq!(verdict_label(&v), "MISSING");
        assert!(!row_ok(&v));
        cleanup(&dir);
    }

    #[test]
    fn the_audit_lists_citations_and_the_runs_nobody_claims() {
        use crate::registries::workspace::Index;
        let dir = temp_dir("audit");
        write_run(&dir, "run-003-cited", GREEN_LOG);
        write_run(&dir, "run-004-orphan", GREEN_LOG);

        let mut index = Index::default();
        for (id, extra) in [
            (
                "TEST-001",
                "run = \"run-003-cited\"\nrun-node = \"verificateur\"",
            ),
            ("TEST-002", "run = \"run-099-ghost\""),
        ] {
            let a = artifact_with_run(id, extra);
            index.by_id.insert(a.id.clone(), a);
        }
        let report = audit(&index, &dir);
        assert!(report.runs_dir_exists);
        assert_eq!(report.total_runs, 2);
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.verified_count(), 1);
        assert_eq!(report.broken().len(), 1);
        assert_eq!(report.orphan_runs, vec!["run-004-orphan"]);
        cleanup(&dir);
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("opd-prov-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
