//! The verification subsystem's data layer — the three pieces of Section 9/10
//! that live in *artifacts* rather than in code:
//!
//! 1. **The rigour table** (§9.7): twelve requirement characters, each with a
//!    primary and a secondary verification mechanism. Declared per requirement
//!    in a record's TOML block as `rigour = "…"` and resolved against this
//!    table; an engineering decision, never inferred.
//! 2. **Binding provenance** (§10.10): a `verifies` edge may be GENERATED,
//!    ANNOTATED, ATTRIBUTED or INFERRED. "An unconfirmed binding cannot close a
//!    traceability edge" — INFERRED never counts as explicit evidence.
//! 3. **Verification depth** (§10.9): executed, discovered, declared — recorded
//!    per TEST artifact as `verification-depth = "…"`, or inferred
//!    conservatively (a record with no depth declaration is at best
//!    discovered).
//!
//! Everything here is additive: every field is optional at parse time, so
//! existing workspaces keep loading untouched.

use crate::error::Result;
use crate::model::artifact::Artifact;
use crate::model::ids::Id;
use std::fmt;
use std::str::FromStr;

/// One row of the §9.7 rigour-selection table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigourRow {
    /// The requirement character the row answers.
    pub character: &'static str,
    /// Lower-case phrases accepted for this row — the character's words and
    /// the primary mechanism's words. Deliberate aliasing: engineers think in
    /// mechanisms, and the table lives in one place.
    pub keys: &'static [&'static str],
    /// Primary verification mechanism.
    pub primary: &'static str,
    /// Secondary mechanism — the first reach when the primary cannot observe
    /// the property.
    pub secondary: &'static str,
}

/// The §9.7 table, verbatim. The last row is deliberate: a contested rule is
/// verified by a *scenario*, and a test would silently assert the rule
/// (§9.9 — "a test may not be the justification of a requirement").
pub const RIGOUR_TABLE: &[RigourRow] = &[
    RigourRow {
        character: "Business behaviour, acceptance",
        keys: &["business behaviour", "acceptance", "bdd", "gherkin"],
        primary: "BDD / Gherkin",
        secondary: "Contract test",
    },
    RigourRow {
        character: "Unit-level contract",
        keys: &["unit"],
        primary: "TDD",
        secondary: "Property-based test",
    },
    RigourRow {
        character: "REST interface",
        keys: &["rest"],
        primary: "OpenAPI + schema test",
        secondary: "Pact",
    },
    RigourRow {
        character: "GraphQL interface",
        keys: &["graphql"],
        primary: "GraphQL SDL",
        secondary: "Pact",
    },
    RigourRow {
        character: "gRPC interface",
        keys: &["grpc"],
        primary: "Protocol Buffers",
        secondary: "Pact",
    },
    RigourRow {
        character: "Event interface",
        keys: &["event"],
        primary: "AsyncAPI",
        secondary: "Consumer-driven test",
    },
    RigourRow {
        character: "Service compatibility",
        keys: &["compatibility"],
        primary: "Pact / Specmatic",
        secondary: "Integration",
    },
    RigourRow {
        character: "State-heavy lifecycle",
        keys: &["state", "lifecycle"],
        primary: "State machine / model",
        secondary: "Model-based",
    },
    RigourRow {
        character: "Mathematical invariant",
        keys: &["invariant", "mathematical"],
        primary: "Property-based",
        secondary: "Formal (if tractable)",
    },
    RigourRow {
        character: "Performance envelope",
        keys: &["performance"],
        primary: "Benchmark + load test",
        secondary: "Synthetic data",
    },
    RigourRow {
        character: "Security property",
        keys: &["security"],
        primary: "Adversarial test",
        secondary: "Static analysis",
    },
    RigourRow {
        character: "Uncertain / contested rule",
        keys: &["uncertain", "contested"],
        primary: "Scenario, not test",
        secondary: "Recorded as UNKNOWN",
    },
];

/// Lower-case and collapse every non-alphanumeric run to a single space, so
/// `Scenario, not test`, `scenario not test` and `SCENARIO NOT TEST` agree,
/// and hyphens fold away (`unit-level` ≡ `unit level`).
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = true; // swallows a leading separator
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim().to_string()
}

/// Resolves a declared `rigour = "…"` value against the §9.7 table.
///
/// Matches exact identity first (the full character or the full primary
/// mechanism), then whole-alias equality (`TDD`, `BDD`), then the loosest
/// phrase-inclusion pass, so `"Business behaviour, acceptance"` with any
/// trailing wording still resolves. Returns `None` for a value the table does
/// not recognise — a declaration error the caller must surface, never a
/// silent default.
pub fn resolve(declared: &str) -> Option<&'static RigourRow> {
    let declared = normalize(declared);
    RIGOUR_TABLE
        .iter()
        .find(|row| {
            let character = normalize(row.character);
            let primary = normalize(row.primary);
            declared == character || declared == primary
        })
        .or_else(|| {
            RIGOUR_TABLE
                .iter()
                .find(|row| row.keys.iter().any(|k| normalize(k) == declared))
        })
        .or_else(|| {
            RIGOUR_TABLE
                .iter()
                .find(|row| row.keys.iter().any(|k| declared.contains(&normalize(k))))
        })
}

/// The row a record declares, when the value resolves against the table.
pub fn declared_rigour(a: &Artifact) -> Option<&'static RigourRow> {
    a.field("rigour").and_then(resolve)
}

/// The declared `rigour` value when it does **not** resolve — a misspelled
/// character or an invented mechanism. Surfacing it is the difference between
/// a table that disciplines and a table that documents.
pub fn unknown_rigour(a: &Artifact) -> Option<&str> {
    let raw = a.field("rigour")?;
    match resolve(raw) {
        Some(_) => None,
        None => Some(raw),
    }
}

// ------------------------------------------------------- binding provenance--

/// How a `verifies` edge between a TEST and a criterion was established.
///
/// §10.10: the framework never presents the origins as equivalent. A link
/// matched from a naming convention is a hypothesis about intent, and
/// presenting it as generated traceability would be epistemic laundering
/// performed by the framework itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingProvenance {
    /// Emitted by the provisioner; the tag is baked into a generated file
    /// and regenerates without drifting (§10.8).
    Generated,
    /// A `// odc:<ID>` comment in the test, written or reviewed by a human.
    Annotated,
    /// A language-native marker attribute.
    Attributed,
    /// A naming convention matched against the glossary. Never explicit.
    Inferred,
}

impl BindingProvenance {
    /// The literal as written in a record's TOML block.
    pub fn as_str(&self) -> &'static str {
        match self {
            BindingProvenance::Generated => "GENERATED",
            BindingProvenance::Annotated => "ANNOTATED",
            BindingProvenance::Attributed => "ATTRIBUTED",
            BindingProvenance::Inferred => "INFERRED",
        }
    }

    /// §10.10: GENERATED, ANNOTATED and ATTRIBUTED are explicit and HIGH;
    /// INFERRED is LOW and can only close an edge *with human confirmation*.
    pub fn confidence(&self) -> &'static str {
        match self {
            BindingProvenance::Inferred => "LOW",
            _ => "HIGH",
        }
    }

    /// `true` for the one provenance that cannot close a traceability edge
    /// on its own.
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, BindingProvenance::Inferred)
    }
}

impl fmt::Display for BindingProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for BindingProvenance {
    type Err = String;
    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        const VALID: &[(&str, BindingProvenance)] = &[
            ("GENERATED", BindingProvenance::Generated),
            ("ANNOTATED", BindingProvenance::Annotated),
            ("ATTRIBUTED", BindingProvenance::Attributed),
            ("INFERRED", BindingProvenance::Inferred),
        ];
        let upper = normalize(text).replace(' ', "");
        VALID
            .iter()
            .find(|(name, _)| normalize(name) == upper)
            .map(|(_, b)| *b)
            .ok_or_else(|| format!(
                "binding provisionance must be GENERATED, ANNOTATED, ATTRIBUTED or INFERRED, found: {text}"
            ))
    }
}

/// A record's `binding = { … }` inline table (§10.10), read from the record's
/// extra fields so the parser stays untouched:
///
/// ```toml
/// binding = { to = "AC-004-03", provisionance = "ANNOTATED", confirmed = false }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBinding {
    /// The criterion (or rule) the test binds to.
    pub to: Id,
    /// How the binding was established.
    pub provisionance: BindingProvenance,
    /// A human has confirmed an inferred binding. Only meaningful when
    /// provisionance is INFERRED; always `false` otherwise.
    pub confirmed: bool,
    /// Free-form note, as written.
    pub note: Option<String>,
}

// -------------------------------------------------- verification depth ------

/// §10.9 verification depth. Every audit output states which depth produced
/// it: "A project reporting discovered coverage has not demonstrated that
/// anything works, and the framework must not present it as though it had."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDepth {
    /// The suite was run against the current implementation and its result was
    /// observed — recorded, or derivable from a completed verification context.
    Executed,
    /// The artifact exists and is bound; it was not run here.
    Discovered,
    /// A link is asserted in an artifact; nothing was observed.
    Declared,
}

impl VerificationDepth {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationDepth::Executed => "executed",
            VerificationDepth::Discovered => "discovered",
            VerificationDepth::Declared => "declared",
        }
    }
}

impl fmt::Display for VerificationDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VerificationDepth {
    type Err = String;
    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        match normalize(text).as_str() {
            "executed" => Ok(VerificationDepth::Executed),
            "discovered" => Ok(VerificationDepth::Discovered),
            "declared" => Ok(VerificationDepth::Declared),
            _ => Err(format!(
                "verification-depth must be executed, discovered or declared, found: {text}"
            )),
        }
    }
}

/// Reads a record's declared `verification-depth`.
pub fn declared_depth(a: &Artifact) -> Result<Option<VerificationDepth>> {
    match a.field("verification-depth") {
        None => Ok(None),
        Some(raw) => VerificationDepth::from_str(raw).map(Some).map_err(|e| {
            crate::error::OdError::artifact(a.file.as_path(), format!("line {}: {e}", a.line))
        }),
    }
}

/// The depth the record *actually presents without a declaration* — the
/// conservative inference of §10.9. A test with a binding or a command is at
/// least `discovered`; everything else is merely `declared`.
pub fn inferred_depth(a: &Artifact) -> VerificationDepth {
    if a.field("verification-depth").is_some() {
        // The declared value wins; the caller has already parsed or failed it.
        match declared_depth(a) {
            Ok(Some(d)) => return d,
            _ => return VerificationDepth::Declared,
        }
    }
    if a.field("automation").is_some() || a.field("binding").is_some() {
        VerificationDepth::Discovered
    } else {
        VerificationDepth::Declared
    }
}

// ----------------------------------------------------- binding as TOML ------

/// Reads a record's `binding = { … }` inline table out of the artifact's
/// extra fields, with the same fail-loud contract as `promotions`: a
/// malformed entry is an error, not a skipped row, because a binding record
/// that silently fails to load is indistinguishable from no binding.
pub fn read_binding(a: &Artifact) -> Result<Option<TestBinding>> {
    let value = match a.extra.get("binding") {
        None => return Ok(None),
        Some(v) => v,
    };
    let table = value.as_table().ok_or_else(|| {
        crate::error::OdError::artifact(
            a.file.as_path(),
            format!(
                "line {}: `binding` must be an inline table `{{
 to = …, provisionance = … }}`",
                a.line
            ),
        )
    })?;

    let at = a.line; // pinpoint inside the TOML block, kept simple
    let err = |msg: String| {
        crate::error::OdError::artifact(a.file.as_path(), format!("line {at}: `binding`: {msg}"))
    };

    let to_raw = table
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("`binding.to` must be a string id".into()))?;
    let to = Id::parse(to_raw).map_err(|e| err(e.to_string()))?;

    let prov_raw = table
        .get("provisionance")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("`binding.provisionance` must be a string".into()))?;
    let provisionance = prov_raw.parse::<BindingProvenance>().map_err(|e| err(e))?;

    let confirmed = match table.get("confirmed") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| err("`binding.confirmed` must be a boolean".into()))?,
    };

    let note = match table.get("note") {
        None => None,
        Some(v) => Some(
            v.as_str()
                .ok_or_else(|| err("`binding.note` must be a string".into()))?
                .to_string(),
        ),
    };

    Ok(Some(TestBinding {
        to,
        provisionance,
        confirmed: if provisionance.needs_confirmation() {
            confirmed
        } else {
            // Only an inferred binding can demand confirmation; recording it
            // for explicit ones would be noise.
            false
        },
        note,
    }))
}

// ------------------------------------------------------------ audit ---------

/// One TEST artifact's binding audit row — the substance behind
/// `opendomain bind` (design checks G8-B5/B6/B7 plus the §10.10 record).
#[derive(Debug, Clone)]
pub struct BindingAudit {
    pub id: Id,
    pub status: crate::model::vocab::ArtifactStatus,
    /// The recorded binding, when there is one.
    pub binding: Option<TestBinding>,
    /// The naming-convention hit against the glossary — a *hypothesis*,
    /// whatever it matches.
    pub name_match: Option<&'static str>,
    pub depth: VerificationDepth,
    /// Shape issues the audit reports rather than asserting away.
    pub claims_executed_without_command: bool,
}
