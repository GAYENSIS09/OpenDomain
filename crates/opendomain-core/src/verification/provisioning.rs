//! The provisioner's decision layer — Section 10 as data plus ordered rules,
//! with every mutation left to the caller.
//!
//! Everything here is a pure function of what the caller detected, so the
//! selection logic is testable without a repository on disk. The one thing this
//! module never produces is a passing assertion (PP4: "Derive structure, never
//! author behaviour"), and it never claims coverage it cannot observe:
//! recording verification depth is the caller's business (§10.9).

use crate::model::artifact::Artifact;
use crate::model::ids::Id;

// ------------------------------------------------------------------ matrix --

/// One row of the §10.4 framework selection matrix. The matrix is deliberate
/// *data, external to the gate logic*: a wrong entry is a correction to this
/// table, not a change of framework. Version pins live in the decision the
/// caller records — never asserted here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixRow {
    /// Human-readable stack name, as in §10.4.
    pub stack: &'static str,
    /// Lower-case token the detector matches manifests against.
    pub token: &'static str,
    /// Manifests that signal this stack (exact file names or `*.ext` globs).
    pub manifests: &'static [&'static str],
    /// Unit (TDD) framework.
    pub unit: &'static str,
    /// Cucumber/Gherkin port — the BDD mechanism (§10.5: every port shares
    /// one process with the unit suite).
    pub bdd: &'static str,
    /// Contract-testing mechanism.
    pub contract: &'static str,
    /// Property-based mechanism; `"--"` when the table names none.
    pub property: &'static str,
    /// Manifest fragments meaning "a BDD framework is already present" —
    /// rule R1 adopts it rather than overriding the team's choice.
    pub existing_bdd_keys: &'static [&'static str],
}

/// The §10.4 matrix, verbatim. `composer.json` is an extension of the paper's
/// detect list so the PHP row is reachable; it is recorded here, in the data,
/// where a wrong entry can be corrected without touching the rules.
pub const MATRIX: &[MatrixRow] = &[
    MatrixRow {
        stack: "Rust + cargo",
        token: "rust",
        manifests: &["Cargo.toml"],
        unit: "built-in #[test]",
        bdd: "cucumber-rs",
        contract: "pact-reference",
        property: "proptest",
        existing_bdd_keys: &["cucumber-rs", "cucumber"],
    },
    MatrixRow {
        stack: "Python + pytest",
        token: "python",
        manifests: &["pyproject.toml", "requirements.txt"],
        unit: "pytest",
        bdd: "pytest-bdd / behave",
        contract: "pact-python",
        property: "hypothesis",
        existing_bdd_keys: &["pytest-bdd", "behave", "cucumber"],
    },
    MatrixRow {
        stack: "JS/TS + npm",
        token: "js-ts",
        manifests: &["package.json"],
        unit: "vitest (or existing jest)",
        bdd: "@cucumber/cucumber-js (playwright-bdd for UI)",
        contract: "@pact-foundation",
        property: "fast-check",
        existing_bdd_keys: &["@cucumber/cucumber-js", "cucumber-js", "playwright-bdd", "cucumber"],
    },
    MatrixRow {
        stack: "Java + Maven/Gradle",
        token: "java",
        manifests: &["pom.xml", "build.gradle", "build.gradle.kts"],
        unit: "JUnit 5",
        bdd: "cucumber-jvm",
        contract: "pact-jvm",
        property: "jqwik",
        existing_bdd_keys: &["cucumber-jvm", "cucumber-java", "cucumber"],
    },
    MatrixRow {
        stack: "C# + .NET",
        token: "csharp",
        manifests: &["*.csproj"],
        unit: "xUnit",
        bdd: "SpecFlow",
        contract: "pact-net",
        property: "FsCheck",
        existing_bdd_keys: &["specflow", "cucumber"],
    },
    MatrixRow {
        stack: "Go",
        token: "go",
        manifests: &["go.mod"],
        unit: "std testing",
        bdd: "godog",
        contract: "pact-go",
        property: "rapid",
        existing_bdd_keys: &["godog"],
    },
    MatrixRow {
        stack: "Ruby",
        token: "ruby",
        manifests: &["Gemfile", "*.gemspec"],
        unit: "RSpec / Minitest",
        bdd: "cucumber",
        contract: "pact-ruby",
        property: "rspec",
        existing_bdd_keys: &["cucumber", "spinach"],
    },
    MatrixRow {
        stack: "PHP",
        token: "php",
        manifests: &["composer.json"],
        unit: "PHPUnit",
        bdd: "behat",
        contract: "pact-php",
        property: "--",
        existing_bdd_keys: &["behat", "cucumber"],
    },
];

/// What a repository-level scan handed over. The CLI fills it from the file
/// system only — "Detection reads only the repository. No network, no
/// prompting" (§10.3). The detection result's epistemic status is INFERRED,
/// MEDIUM confidence, and every record produced from it says so.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detection {
    /// `(file name relative to the repo root, text)` pairs. Enough for R1 to
    /// read dependencies without knowing a package manager.
    pub manifests: Vec<(String, String)>,
    /// Repository-relative entry-point signatures found — the shape of an
    /// "application boundary" (§10.3). The caller computes them from a fixed
    /// probe list, and the result stays visible in the decision record
    /// instead of hidden inside a heuristic.
    pub entry_signatures: Vec<String>,
}

impl Detection {
    /// The matrix row matching the detected stack. Multiple stacks are not
    /// guessed: manifest order decides, and the decision record states the
    /// pick so a human can reject it.
    pub fn stack(&self) -> Option<&'static MatrixRow> {
        MATRIX.iter().find(|row| {
            self.manifests
                .iter()
                .any(|(name, _)| row.manifests.iter().any(|m| matches_manifest(name, m)))
        })
    }

    /// Rule R1: a BDD framework already present in the manifest.
    pub fn existing_bdd(&self) -> Option<&'static str> {
        let row = self.stack()?;
        let haystack = self.manifests.iter().fold(String::new(), |acc, (_, text)| {
            format!("{acc}\n{}", text.to_lowercase())
        });
        row.existing_bdd_keys
            .iter()
            .find(|key| haystack.contains(*key))
            .copied()
    }

    /// Every matrix row whose manifests were seen — used to explain a pick
    /// (and to warn when two stacks were detected and the first won).
    pub fn matched_rows(&self) -> Vec<&'static MatrixRow> {
        MATRIX
            .iter()
            .filter(|row| {
                self.manifests
                    .iter()
                    .any(|(name, _)| row.manifests.iter().any(|m| matches_manifest(name, m)))
            })
            .collect()
    }

    /// A summary for the decision record: what was seen, first-selected stack.
    pub fn summary(&self) -> String {
        let names: Vec<String> = self
            .manifests
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let stack = self.stack().map(|r| r.stack).unwrap_or("none");
        format!(
            "manifests: [{}]; entry signatures: [{}]; selected stack: {}",
            names.join(", "),
            self.entry_signatures.join(", "),
            stack
        )
    }
}

/// Manifest matching against the matrix: exact file name (compared on the
/// *file-name part*, so nested paths like `crates/x/Cargo.toml` match
/// `Cargo.toml`) or a case-insensitive `*.ext` glob.
fn matches_manifest(path: &str, spec: &str) -> bool {
    let name = std::path::Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if let Some(ext) = spec.strip_prefix("*.") {
        name.ends_with(&ext.to_lowercase())
    } else {
        name == spec.to_lowercase()
    }
}

// -------------------------------------------------------------- selection ---

/// Ordered selection rules, first match wins (§10.4). The agent is not asked
/// to choose and is not permitted to invent a framework; a choice outside the
/// table is an error, not a preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// R1 — a BDD framework already in the manifest: adopt it, install nothing.
    /// (Brownfield preservation; overriding an existing team's deliberate
    /// choice is the most common unforced harm in this subsystem.)
    AdoptExisting,
    /// R2 — the language has an established Cucumber/Gherkin port.
    EstablishedPort,
    /// R4 — no established port: shim mode, a thin Gherkin parser and driver
    /// over the project's native runner.
    Shim,
}

impl Rule {
    /// The one-line justification written into the decision record.
    pub fn as_str(&self, stack: Option<&MatrixRow>, existing: Option<&str>) -> String {
        match self {
            Rule::AdoptExisting => format!(
                "R1 — a BDD framework is already present in the manifest ({}): adopted, nothing installed",
                existing.unwrap_or("<unknown>")
            ),
            Rule::EstablishedPort => match stack {
                Some(row) => format!(
                    "R2 — {} has an established Cucumber/Gherkin port: {}",
                    row.stack, row.bdd
                ),
                None => "R2 — an established Cucumber/Gherkin port exists".to_string(),
            },
            Rule::Shim => {
                "R4 — no established port: shim mode, a thin Gherkin driver over the native runner"
                    .to_string()
            }
        }
    }
}

/// §10.13 inputs: the numbers and flags the refusal reads. All come from
/// workspace state, never from a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddCriteria {
    /// Behavioural acceptance criteria in the workspace (AC records).
    pub behavioural_acs: usize,
    /// Configured minimum (`[verification] minimal-acceptance-criteria`,
    /// default 3).
    pub minimum: usize,
    /// `true` when some use case names a primary actor.
    pub named_actor_use_case: bool,
    /// The declared BDD-off profile: a first-class mode, not a violation.
    pub bdd_off: bool,
    /// The change-set is marked experimental and non-production.
    pub experimental: bool,
    /// `true` when an application boundary was detected (entry point or
    /// inbound interface); a library without one gets no hollow scenarios.
    pub boundary_detected: bool,
}

/// §10.13: "Refusal must emit a decision record naming the alternative
/// mechanism — a naming-convention binding, or scenario artifacts without
/// execution."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Every reason that applied, wording close to the §10.13 bullets.
    pub reasons: Vec<String>,
    /// The alternative mechanism, as named in the refusal record.
    pub alternative: &'static str,
}

/// What provisioning decided. A refusal is a real outcome, not an error —
/// it is recorded with its reasons and produces nothing else (§10.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Generate — with the rule and the framework the table chose. `stack`
    /// is `None` in shim mode (R4): nothing was picked, and the record must
    /// not pretend otherwise.
    Generate {
        rule: Rule,
        stack: Option<&'static MatrixRow>,
        existing: Option<&'static str>,
    },
    /// Refuse — with all reasons and the alternative mechanism on record.
    Refused(Refusal),
}

impl Selection {
    /// The chosen rule, absent on refusal.
    pub fn rule(&self) -> Option<Rule> {
        match self {
            Selection::Generate { rule, .. } => Some(*rule),
            Selection::Refused(_) => None,
        }
    }
}

/// Applies the §10.13 gate first (it refuses regardless of the table), then
/// the ordered rules R1 → R2 → R4. R3 is encoded by the refusal conditions
/// themselves: "no business-actor-facing behaviour" is the missing use case
/// with a named actor.
pub fn select(detection: &Detection, criteria: &BddCriteria) -> Selection {
    let mut reasons = Vec::new();
    if criteria.behavioural_acs < criteria.minimum {
        reasons.push(format!(
            "fewer than the configured minimum behavioural acceptance criteria exist ({} < {}); BDD for one scenario is indirection with no payoff",
            criteria.behavioural_acs, criteria.minimum
        ));
    }
    if !criteria.named_actor_use_case {
        reasons.push("no use case with a named primary actor exists".to_string());
    }
    if criteria.experimental {
        reasons.push("the change is marked EXPERIMENTAL and non-production".to_string());
    }
    if !criteria.boundary_detected {
        reasons.push("the project is a library with no application boundary detected".to_string());
    }
    if criteria.bdd_off {
        reasons.push("the project has declared the BDD-off profile".to_string());
        return Selection::Refused(Refusal {
            reasons,
            alternative:
                "naming-convention binding (BDD-off profile): acceptance-criterion identifiers carried in test names",
        });
    }
    if !reasons.is_empty() {
        return Selection::Refused(Refusal {
            reasons,
            alternative: "naming-convention binding, or scenario artifacts without execution",
        });
    }

    if let Some(found) = detection.existing_bdd() {
        return Selection::Generate {
            rule: Rule::AdoptExisting,
            stack: detection.stack(),
            existing: Some(found),
        };
    }
    if let Some(row) = detection.stack() {
        return Selection::Generate {
            rule: Rule::EstablishedPort,
            stack: Some(row),
            existing: None,
        };
    }
    Selection::Generate {
        rule: Rule::Shim,
        stack: None,
        existing: None,
    }
}

// ----------------------------------------------------------- generation -----

/// A criterion attached to a generated scenario: its id and the display text
/// to place after it. `None` when the criterion is referenced but not yet
/// recorded — the generated file says so instead of inventing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriterionRef {
    pub id: Id,
    pub text: Option<String>,
}

/// The `# derives-from:` provenance comment (§10.8): the upstream ids of the
/// requirement — rules, evidence — plus the requirement itself, joined on one
/// line so the tag set regenerates without drift (PP5).
pub fn provenance_comment(requirement: &Artifact, upstream: &[Id]) -> String {
    let mut ids: Vec<String> = upstream.iter().map(|i| i.to_string()).collect();
    let self_id = requirement.id.to_string();
    if !ids.iter().any(|i| *i == self_id) {
        ids.push(self_id);
    }
    format!("# derives-from: {}", ids.join(", "))
}

/// Renders the `.feature` file for one requirement (§10.8): a tag per
/// criterion, the provenance comment, and empty scenario skeletons. The step
/// lines are guidance, not assertions — binding them is a human act at an
/// application port (§9.4), and a generated placeholder can never be green.
pub fn feature_for(requirement: &Artifact, criteria: &[CriterionRef]) -> String {
    let mut out = String::new();
    out.push_str("# generated by `opendomain provision` — regenerate with --sync;\n# an edit to a generated file is a visible diff (PP5)\n");
    out.push_str(&format!("Feature: {} — {}\n", requirement.id, requirement.title));
    // §10.8: every generated file carries the provenance comment, so the tag
    // set regenerates without drift (PP5). The criteria the file tags are the
    // requirement's *targets*, not its provenance — they never appear in the
    // comment; rules and evidence do.
    let upstream: Vec<Id> = requirement
        .upstream()
        .into_iter()
        .filter(|id| id.kind() != crate::model::ids::IdKind::Ac)
        .collect();
    let provenance = provenance_comment(requirement, &upstream);
    out.push_str(&provenance);
    out.push('\n');

    if criteria.is_empty() {
        out.push_str("  # no acceptance criterion recorded for this requirement yet\n");
        out.push_str("  # (the refusal gate needs the AC set before BDD pays for itself — §10.13)\n");
        return out;
    }

    for c in criteria {
        match &c.text {
            Some(text) => {
                // One line per Gherkin; a criterion that wraps collapses onto
                // one line rather than breaking the feature file.
                let text = text.trim().replace('\n', " ");
                out.push_str(&format!("\n  @{}\n", c.id));
                out.push_str(&format!("  Scenario: {} — {}\n", c.id, text));
            }
            None => {
                out.push_str(&format!(
                    "\n  @{}\n  Scenario: {} — [criterion not yet recorded; write it in the AC registry]\n",
                    c.id, c.id
                ));
            }
        }
        out.push_str("    # derive the steps from the criterion — do not paste them blind\n");
        out.push_str("    Given <the context the criterion names>\n");
        out.push_str("    When <the behaviour the criterion observes>\n");
        out.push_str("    Then <the observable outcome the criterion states>\n");
    }
    out
}

/// The file name of a requirement's `.feature`: lower-cased id, scoped id
/// preserved (`REQ-INVENTORY-014` → `req-inventory-014.feature`).
pub fn feature_path(requirement: &Artifact) -> String {
    format!("verification/features/{}.feature", requirement.id.to_string().to_lowercase())
}

/// The generated README body: how the scaffolds bind, per §10.7, and what the
/// provisioner deliberately did *not* do (no install, no behaviour).
pub fn readme_for(selection: &Selection, detection_summary: &str, date: &str) -> String {
    let mut out = String::new();
    out.push_str("# Verification scaffolds — generated, never authored\n\n");
    out.push_str(&format!("Detection ({date}): {detection_summary}\n\n"));
    match selection {
        Selection::Generate { rule, stack, existing } => {
            out.push_str(&format!("Selection: {}\n\n", rule.as_str(*stack, *existing)));
            if let Some(row) = stack {
                out.push_str(&format!(
                    "| Mechanism | Chosen |\n|---|---|\n| Unit (TDD) | {} |\n| BDD | {} |\n| Contract | {} |\n| Property-based | {} |\n\n",
                    row.unit, row.bdd, row.contract, row.property
                ));
            } else {
                out.push_str("Shim mode: the stack is outside the matrix. The generated features are portable Gherkin; wire them to the project's own runner.\n\n");
            }
            out.push_str("## What is here\n\n");
            out.push_str("- `features/*.feature` — one per requirement, tags `@<REQ>` and `@<AC-*-*-*>`, provenance in `# derives-from:`. Regenerate with `opendomain provision --sync`; edits to generated files are visible diffs (PP5).\n");
            out.push_str("- Step definitions are **not** generated behaviour. A step definition binds a step to an application port and starts failing (`todo!` / `pending`) until implemented — PP4: the provisioner derives structure and never authors behaviour, and it never generates a passing assertion.\n");
            out.push_str("## What was deliberately not done\n\n");
            out.push_str("- Nothing was installed. The exact install command is recorded in the decision record for review; provisioned-but-uninstalled tooling leaves no stack trace to find later.\n");
            out.push_str("- No verification depth is claimed here. A generated scenario without a run is `discovered` at best (§10.9); only a recorded execution is `executed`.\n");
        }
        Selection::Refused(refusal) => {
            out.push_str("Provisioning was **refused**:\n\n");
            for r in &refusal.reasons {
                out.push_str(&format!("  - {r}\n"));
            }
            out.push_str(&format!(
                "\nAlternative mechanism on record: {}\n\n",
                refusal.alternative
            ));
        }
    }
    out
}

// ------------------------------------------------------- decision record ----

/// The ProvisioningDecision `DEC` record (§10.2) rendered as a registry-style
/// section: stack detected, frameworks chosen, the rule applied, versions
/// *not* pinned because nothing was installed, and the epistemic status of
/// the detection (INFERRED, MEDIUM) — the required DEC fields match the
/// decisions template (`status`, `decision`, `context`, `reason`,
/// `consequences`).
pub fn decision_record(
    id: &Id,
    selection: &Selection,
    detection_summary: &str,
    date: &str,
) -> String {
    let (heading, toml_block) = match selection {
        Selection::Generate { rule, stack, existing } => {
            let stack_name = stack.map(|r| r.stack).unwrap_or("unlisted stack (shim mode)");
            let frameworks = match (stack, rule) {
                (Some(row), _) => format!(
                    "unit={}; bdd={}; contract={}; property={}",
                    row.unit, row.bdd, row.contract, row.property
                ),
                (None, _) => "unit=native runner; bdd=shim; contract/--; property/--".to_string(),
            };
            let _ = existing;
            (
                format!("## {id} — Provisioning decision: {}", stack_name),
                format!(
                    "status = \"APPROVED\"\ndecision = \"provision the verification subsystem for {stack_name}: {frameworks}\"\ncontext = \"detection read only repository manifests and entry probes ({detection_summary})\"\nreason = \"{}\"\nconsequences = \"scaffolds under opendomain/verification/ contain no passing assertion; step definitions are bound by hand at an application port. Install commands are recorded for review — nothing was installed, no version was pinned because no dependency was added\"\nepistemic = \"INFERRED\"\nconfidence = \"MEDIUM\"\ndate = \"{date}\"\n",
                    rule.as_str(*stack, *existing)
                ),
            )
        }
        Selection::Refused(_) => {
            // Refusals are recorded through `refusal_record` callers; this
            // branch keeps the signature total without inventing prose.
            unreachable!("callers render refusals with refusal_record")
        }
    };
    format!("{heading}\n\n```toml\n{toml_block}```\n")
}

/// The refusal decision record (§10.13): reasons, the alternative mechanism,
/// and the same epistemic honesty as the positive decision.
pub fn refusal_record(id: &Id, refusal: &Refusal, detection_summary: &str, date: &str) -> String {
    let reasons: String = refusal
        .reasons
        .iter()
        .map(|r| format!("  - {r}\n"))
        .collect();
    format!(
        "## {id} — Provisioning refusal (BDD not provisioned)\n\n```toml\nstatus = \"APPROVED\"\ndecision = \"do not provision BDD; the provisioning gate refused\"\ncontext = \"detection read only repository manifests and entry probes ({detection_summary})\"\nreason = \"the rigorous refusal conditions applied (Section 10.13)\"\nconsequences = \"nothing was generated; the alternative mechanism is on record. A refusal that produces silence would be indistinguishable from an oversight — this record is the antidote\"\nepistemic = \"INFERRED\"\nconfidence = \"MEDIUM\"\ndate = \"{date}\"\n```\n\n**Reasons**\n\n{reasons}\n**Alternative mechanism**\n\n{}\n",
        refusal.alternative
    )
}

/// One appended row of the provisioning-failure log (§10.14, F12): the raw
/// material a *corpus* needs, local to the workspace until someone harvests it.
pub fn failure_row(date: &str, refusal: &Refusal) -> String {
    let reasons = refusal.reasons.join("; ");
    format!(
        "- {date} — refused: {reasons} — alternative: {}\n",
        refusal.alternative
    )
}

/// The failure-log file header, written when the log does not exist yet.
pub const FAILURE_LOG_HEADER: &str = "# Provisioning refusals & failures (F12 corpus raw material)\n\nAppended by `opendomain provision`. Detection and selection failures live here so the selection matrix (Section 10.4) can be revised on evidence rather than opinion, and the false-refusal rate of the provisioning gate measured rather than assumed.\n";

#[cfg(test)]
mod tests {
    use super::*;

    fn detection(pairs: &[(&str, &str)]) -> Detection {
        Detection {
            manifests: pairs
                .iter()
                .map(|(n, c)| (n.to_string(), c.to_string()))
                .collect(),
            entry_signatures: vec!["src/main.rs".to_string()],
        }
    }

    fn criteria() -> BddCriteria {
        BddCriteria {
            behavioural_acs: 5,
            minimum: 3,
            named_actor_use_case: true,
            bdd_off: false,
            experimental: false,
            boundary_detected: true,
        }
    }

    #[test]
    fn manifest_matching_is_exact_or_extension_glob() {
        assert!(matches_manifest("Cargo.toml", "Cargo.toml"));
        assert!(matches_manifest("crates/x/Cargo.toml", "Cargo.toml"));
        assert!(matches_manifest("app.csproj", "*.csproj"));
        assert!(matches_manifest("App.CSPROJ", "*.csproj"));
        assert!(!matches_manifest("package.json", "Cargo.toml"));
        assert!(!matches_manifest("cargo.toml.bak", "Cargo.toml"));
    }

    #[test]
    fn detection_selects_the_stack_by_manifest_order() {
        let d = detection(&[("package.json", "{}"), ("Cargo.toml", "[workspace]")]);
        let row = d.stack().expect("rust and js both detected");
        // MATRIX order decides: Rust comes before JS/TS.
        assert_eq!(row.token, "rust");
        assert!(matches_manifest("thing.csproj", "*.csproj"));
    }

    #[test]
    fn unknown_stack_matches_no_row() {
        assert!(detection(&[("mix.exs", "~")]).stack().is_none());
        assert!(detection(&[("mix.exs", "~")])
            .existing_bdd()
            .is_none());
    }

    #[test]
    fn r1_adopts_an_existing_bdd_framework_without_installing() {
        let d = detection(&[(
            "Cargo.toml",
            "[dependencies]\ncucumber = \"0.21\"\n",
        )]);
        let selection = select(&d, &criteria());
        match selection {
            Selection::Generate { rule, existing, .. } => {
                assert_eq!(rule, Rule::AdoptExisting);
                // R1 honours what the manifest *declares* — the matched
                // fragment, not the matrix port's name.
                assert_eq!(existing, Some("cucumber"));
            }
            other => panic!("expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn r2_picks_the_established_port_when_nothing_is_installed() {
        let d = detection(&[("pyproject.toml", "[project]\nname = \"w\"\n")]);
        match select(&d, &criteria()) {
            Selection::Generate { rule, stack, existing } => {
                assert_eq!(rule, Rule::EstablishedPort);
                assert_eq!(existing, None);
                let row = stack.expect("known stack");
                assert_eq!(row.bdd, "pytest-bdd / behave");
            }
            other => panic!("expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn r4_is_shim_mode_with_no_fabricated_stack() {
        let d = detection(&[("mix.exs", "[deps]\nphoenix = \"1.7\"\n")]);
        match select(&d, &criteria()) {
            Selection::Generate { rule, stack, .. } => {
                assert_eq!(rule, Rule::Shim);
                assert!(stack.is_none(), "shim must not fake a stack pick");
            }
            other => panic!("expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn refusal_collects_every_reason_and_names_the_alternative() {
        let d = detection(&[("Cargo.toml", "[dependencies]\n")]);
        let too_few = BddCriteria {
            behavioural_acs: 1,
            minimum: 3,
            named_actor_use_case: false,
            bdd_off: false,
            experimental: true,
            boundary_detected: false,
        };
        match select(&d, &too_few) {
            Selection::Refused(refusal) => {
                assert_eq!(refusal.reasons.len(), 4, "{:?}", refusal.reasons);
                assert!(refusal.reasons[0].contains("fewer than the configured minimum"));
                assert_eq!(
                    refusal.alternative,
                    "naming-convention binding, or scenario artifacts without execution"
                );
            }
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    #[test]
    fn bdd_off_profile_refuses_with_the_profile_alternative() {
        let d = detection(&[("Cargo.toml", "")]);
        let off = BddCriteria {
            bdd_off: true,
            ..criteria()
        };
        match select(&d, &off) {
            Selection::Refused(refusal) => {
                assert!(refusal.reasons[0].contains("BDD-off profile"));
                assert!(refusal.alternative.contains("BDD-off profile"));
            }
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_generated_feature_carries_tags_provenance_and_placeholders_only() {
        use crate::registries::parse as p;
        let path = std::path::Path::new("requirements/requirements.md");
        let file = std::path::PathBuf::from("requirements/requirements.md");
        let text = "## REQ-004 — Reject a receipt exceeding the outstanding quantity\n\n```toml\ntype = \"FUNCTIONAL\"\nstatus = \"VALIDATED\"\n```\n\nThe system SHALL reject.\n";
        let doc = p::parse(path, file, text).unwrap();
        let req = &doc.artifacts[0];

        let upstream = vec![
            Id::parse("RULE-001").unwrap(),
            Id::parse("EVD-014").unwrap(),
        ];
        let feature = feature_for(
            req,
            &[
                CriterionRef {
                    id: Id::parse("AC-004-01").unwrap(),
                    text: Some("When the accepted quantity equals the remaining\nquantity, the receipt is confirmed.".to_string()),
                },
                CriterionRef {
                    id: Id::parse("AC-004-02").unwrap(),
                    text: None,
                },
            ],
        );

        assert!(feature.contains("Feature: REQ-004 — Reject a receipt"), "{feature}");
        assert!(feature.contains("@AC-004-01"), "{feature}");
        assert!(feature.contains("Scenario: AC-004-01 — When the accepted quantity equals the remaining quantity,"), "{feature}");
        assert!(feature.contains("[criterion not yet recorded; write it in the AC registry]"), "want debug:\n{feature:?}");
        assert!(feature.contains("Given <the context the criterion names>"), "{feature}");
        assert!(feature.contains("# generated by `opendomain provision`"), "{feature}");
        // §10.8: the provenance comment lands in the generated file, listing
        // the requirement's upstream (here: none besides itself).
        assert!(feature.contains("# derives-from"), "{feature}");
        // Generated steps are placeholders, never green assertions.
        assert!(feature.contains("<the observable outcome the criterion states>"));
        assert_eq!(
            provenance_comment(req, &upstream),
            "# derives-from: RULE-001, EVD-014, REQ-004"
        );
    }

    #[test]
    fn a_refusal_produces_a_decision_record_with_the_alternative() {
        use crate::registries::parse as p;
        let path = std::path::Path::new("requirements/requirements.md");
        let file = std::path::PathBuf::from("requirements/requirements.md");
        let text = "## REQ-004 — Nothing\n\n```toml\ntype = \"FUNCTIONAL\"\nstatus = \"DRAFT\"\n```\n";
        let doc = p::parse(path, file, text).unwrap();
        let req = &doc.artifacts[0];
        let empty_feature = feature_for(req, &[]);
        assert!(empty_feature.contains("no acceptance criterion recorded"));

        let refusal = Refusal {
            reasons: vec!["fewer than the configured minimum behavioural acceptance criteria exist (0 < 3); BDD for one scenario is indirection with no payoff".into()],
            alternative: "naming-convention binding, or scenario artifacts without execution",
        };
        let record = refusal_record(&Id::parse("DEC-001").unwrap(), &refusal, "manifests: []", "2026-10-22");
        assert!(record.contains("Provisioning refusal"));
        assert!(record.contains("nothing was generated"));
        assert!(record.contains("naming-convention binding"));
        assert!(record.contains("2026-10-22"));
        assert_eq!(
            failure_row("2026-10-22", &refusal),
            "- 2026-10-22 — refused: fewer than the configured minimum behavioural acceptance criteria exist (0 < 3); BDD for one scenario is indirection with no payoff — alternative: naming-convention binding, or scenario artifacts without execution\n"
        );
    }

    #[test]
    fn a_generate_decision_record_records_stack_flags_and_no_install() {
        let selection = Selection::Generate {
            rule: Rule::EstablishedPort,
            stack: Some(MATRIX.iter().find(|r| r.token == "rust").unwrap()),
            existing: None,
        };
        let record = decision_record(
            &Id::parse("DEC-004").unwrap(),
            &selection,
            "manifests: [Cargo.toml]; selected stack: Rust + cargo",
            "2026-10-22",
        );
        assert!(record.contains("## DEC-004 — Provisioning decision: Rust + cargo"));
        assert!(record.contains("epistemic = \"INFERRED\""), "{record}");
        assert!(record.contains("confidence = \"MEDIUM\""), "{record}");
        assert!(record.contains("nothing was installed"), "{record}");
        assert!(record.contains("cucumber-rs"), "{record}");
    }

    #[test]
    fn detection_summary_is_explicit_about_what_was_seen() {
        let d = detection(&[("Cargo.toml", "[workspace]")]);
        assert_eq!(
            d.summary(),
            "manifests: [Cargo.toml]; entry signatures: [src/main.rs]; selected stack: Rust + cargo"
        );
    }
}
