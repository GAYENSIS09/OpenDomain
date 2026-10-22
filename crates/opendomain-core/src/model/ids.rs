//! Artifact identifiers.
//!
//! OpenDomain identifiers look like `REQ-001` (flat) or `REQ-INVENTORY-012`
//! (scoped). Scoping is what keeps a large project from collapsing into one
//! meaningless sequence, so it is part of the type rather than a convention.
//!
//! ```text
//! REQ-001                  flat identifier
//! REQ-INVENTORY-012        scoped to the inventory domain
//! AC-INVENTORY-012-01      acceptance criterion: 12, number 1
//! ```

use crate::error::{OdError, Result};
use crate::model::vocab::Stage;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The kind of artifact an identifier points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IdKind {
    // ── Stage 1: research ────────────────────────────────────────────────
    /// A retrieved external source.
    Src,
    /// A finding extracted from a source.
    Evd,
    /// An open research question.
    Res,
    // ── Stage 1: domain model ────────────────────────────────────────────
    /// A canonical vocabulary term.
    Term,
    /// A domain or bounded context.
    Dom,
    /// A human with an interest in the system's behaviour.
    Stk,
    /// A role or system that interacts with the system.
    Act,
    /// A business process.
    Proc,
    /// A domain entity.
    Ent,
    /// A domain event.
    Event,
    /// A business rule.
    Rule,
    /// An integration with an external system.
    Int,
    /// A recorded unknown.
    Unc,
    /// A disagreement between two sources.
    Xconf,
    /// A business or domain decision.
    Dec,
    // ── Stage 2: opportunity ─────────────────────────────────────────────
    /// An opportunity space.
    Opp,
    /// A hidden assumption.
    Asm,
    /// A contradiction in the current system.
    Con,
    /// A candidate concept.
    Cpt,
    /// A falsifiable hypothesis.
    Hyp,
    /// A designed experiment.
    Exp,
    /// A concept decision record.
    Cdr,
    // ── Stage 3: specification ───────────────────────────────────────────
    /// A requirement.
    Req,
    /// A non-functional requirement.
    Nfr,
    /// An explicit constraint.
    Cnst,
    /// A frozen baseline.
    Bas,
    /// A use case.
    Uc,
    /// A behavioural specification.
    Spec,
    /// An acceptance criterion.
    Ac,
    /// A technical plan.
    Plan,
    /// An implementation task.
    Task,
    /// A machine-checkable contract.
    Ctr,
    /// An architecture decision record.
    Adr,
    /// A validation scenario.
    Scn,
    /// A verification artifact.
    Test,
    /// A brownfield gap.
    Gap,
    /// A drift finding.
    Drift,
}

impl IdKind {
    /// Every kind, in canonical order.
    pub const ALL: &'static [IdKind] = &[
        IdKind::Src,
        IdKind::Evd,
        IdKind::Res,
        IdKind::Term,
        IdKind::Dom,
        IdKind::Stk,
        IdKind::Act,
        IdKind::Proc,
        IdKind::Ent,
        IdKind::Event,
        IdKind::Rule,
        IdKind::Int,
        IdKind::Unc,
        IdKind::Xconf,
        IdKind::Dec,
        IdKind::Opp,
        IdKind::Asm,
        IdKind::Con,
        IdKind::Cpt,
        IdKind::Hyp,
        IdKind::Exp,
        IdKind::Cdr,
        IdKind::Req,
        IdKind::Nfr,
        IdKind::Cnst,
        IdKind::Bas,
        IdKind::Uc,
        IdKind::Spec,
        IdKind::Ac,
        IdKind::Plan,
        IdKind::Task,
        IdKind::Ctr,
        IdKind::Adr,
        IdKind::Scn,
        IdKind::Test,
        IdKind::Gap,
        IdKind::Drift,
    ];

    /// The canonical prefix, e.g. `REQ`.
    pub fn prefix(self) -> &'static str {
        match self {
            IdKind::Src => "SRC",
            IdKind::Evd => "EVD",
            IdKind::Res => "RES",
            IdKind::Term => "TERM",
            IdKind::Dom => "DOM",
            IdKind::Stk => "STK",
            IdKind::Act => "ACT",
            IdKind::Proc => "PROC",
            IdKind::Ent => "ENT",
            IdKind::Event => "EVENT",
            IdKind::Rule => "RULE",
            IdKind::Int => "INT",
            IdKind::Unc => "UNC",
            IdKind::Xconf => "XCONF",
            IdKind::Dec => "DEC",
            IdKind::Opp => "OPP",
            IdKind::Asm => "ASM",
            IdKind::Con => "CON",
            IdKind::Cpt => "CPT",
            IdKind::Hyp => "HYP",
            IdKind::Exp => "EXP",
            IdKind::Cdr => "CDR",
            IdKind::Req => "REQ",
            IdKind::Nfr => "NFR",
            IdKind::Cnst => "CNST",
            IdKind::Bas => "BAS",
            IdKind::Uc => "UC",
            IdKind::Spec => "SPEC",
            IdKind::Ac => "AC",
            IdKind::Plan => "PLAN",
            IdKind::Task => "TASK",
            IdKind::Ctr => "CTR",
            IdKind::Adr => "ADR",
            IdKind::Scn => "SCN",
            IdKind::Test => "TEST",
            IdKind::Gap => "GAP",
            IdKind::Drift => "DRIFT",
        }
    }

    /// Human-readable singular name, used in reports and errors.
    pub fn label(self) -> &'static str {
        match self {
            IdKind::Src => "source",
            IdKind::Evd => "evidence",
            IdKind::Res => "research question",
            IdKind::Term => "term",
            IdKind::Dom => "domain",
            IdKind::Stk => "stakeholder",
            IdKind::Act => "actor",
            IdKind::Proc => "process",
            IdKind::Ent => "entity",
            IdKind::Event => "event",
            IdKind::Rule => "business rule",
            IdKind::Int => "integration",
            IdKind::Unc => "uncertainty",
            IdKind::Xconf => "source conflict",
            IdKind::Dec => "domain decision",
            IdKind::Opp => "opportunity",
            IdKind::Asm => "assumption",
            IdKind::Con => "contradiction",
            IdKind::Cpt => "concept",
            IdKind::Hyp => "hypothesis",
            IdKind::Exp => "experiment",
            IdKind::Cdr => "concept decision",
            IdKind::Req => "requirement",
            IdKind::Nfr => "non-functional requirement",
            IdKind::Cnst => "constraint",
            IdKind::Bas => "baseline",
            IdKind::Uc => "use case",
            IdKind::Spec => "specification",
            IdKind::Ac => "acceptance criterion",
            IdKind::Plan => "plan",
            IdKind::Task => "task",
            IdKind::Ctr => "contract",
            IdKind::Adr => "decision record",
            IdKind::Scn => "scenario",
            IdKind::Test => "test",
            IdKind::Gap => "gap",
            IdKind::Drift => "drift finding",
        }
    }

    /// Plural label, for report headings.
    ///
    /// English does not pluralise by appending `s`: `process` becomes
    /// `processes` and `entity` becomes `entities`, so the irregulars are
    /// listed rather than derived.
    pub fn plural(self) -> String {
        match self {
            IdKind::Xconf => "source conflicts".into(),
            IdKind::Int => "integrations".into(),
            IdKind::Dec => "domain decisions".into(),
            IdKind::Proc => "processes".into(),
            IdKind::Act => "actors".into(),
            IdKind::Ent => "entities".into(),
            IdKind::Cnst => "constraints".into(),
            IdKind::Cdr => "concept decisions".into(),
            IdKind::Ac => "acceptance criteria".into(),
            IdKind::Nfr => "non-functional requirements".into(),
            // `-y` becomes `-ies`, not `-ys`.
            IdKind::Unc => "uncertainties".into(),
            IdKind::Opp => "opportunities".into(),
            IdKind::Hyp => "hypotheses".into(),
            // `evidence` is mass noun in this framework: the plural reads as a
            // count of things, so the singular label doubles as the heading.
            IdKind::Evd => "evidence".into(),
            other => format!("{}s", other.label()),
        }
    }

    /// `true` when an artifact of this kind asserts something about the world
    /// or about what the system should do, and therefore owes the reader a
    /// declared epistemic state (G2-B3).
    ///
    /// The distinction is between artifacts that *claim* and artifacts that
    /// *record process*. A requirement, a business rule and a piece of evidence
    /// assert; a task, a plan, a test and a drift finding describe work done or
    /// work to do. Requiring an epistemic state of a task would be noise: a task
    /// is not a statement that can be wrong about the domain, and G2-B3 applied
    /// to it would flag every artifact in the workspace on day one, which is how
    /// a gate gets switched off.
    ///
    /// The excluded kinds are listed explicitly rather than derived, so that
    /// adding a kind later forces a decision about it.
    pub fn makes_knowledge_claim(self) -> bool {
        use IdKind::*;
        match self {
            // Knowledge substrate: what was read, and what it was taken to mean.
            Src | Evd | Res | Term | Xconf => true,
            // Domain model: statements about how the domain behaves.
            Dom | Stk | Act | Proc | Ent | Event | Rule | Int => true,
            // An uncertainty and a domain decision both assert something, and an
            // UNC that does not declare its epistemic state cannot express *why*
            // it is unknown, which is the field's entire purpose.
            Unc | Dec => true,
            // Opportunity: assumptions, concepts and hypotheses are claims.
            Opp | Asm | Con | Cpt | Hyp | Exp | Cdr => true,
            // Requirements and specifications assert what must be true.
            Req | Nfr | Cnst | Bas | Uc | Spec | Ac | Ctr | Scn => true,
            // Delivery and verification process. These record that something was
            // done, not that something about the domain is known to hold.
            Plan | Task | Adr | Test | Gap | Drift => false,
        }
    }

    /// The stage this artifact belongs to.
    pub fn stage(self) -> Stage {
        use IdKind::*;
        match self {
            Src | Evd | Res | Term | Dom | Stk | Act | Proc | Ent | Event | Rule | Int | Unc
            | Xconf | Dec => Stage::Domain,
            Opp | Asm | Con | Cpt | Hyp | Exp | Cdr => Stage::Opportunity,
            Req | Nfr | Cnst | Bas | Uc | Spec | Ac | Plan | Task | Ctr | Adr | Scn | Test
            | Gap | Drift => Stage::Specification,
        }
    }

    /// The registry file this kind lives in, relative to `opendomain/`.
    pub fn default_path(self) -> &'static str {
        use IdKind::*;
        match self {
            Src => "domain/research/sources.md",
            Evd => "domain/research/evidence.md",
            Res => "domain/research/questions.md",
            Term => "domain/glossary.md",
            Dom => "domain/domain-map.md",
            Stk => "domain/stakeholders.md",
            Act => "domain/actors.md",
            Proc => "domain/processes.md",
            Ent => "domain/entities.md",
            Event => "domain/events.md",
            Rule => "domain/business-rules.md",
            Int => "domain/integrations.md",
            Unc => "domain/uncertainties.md",
            Xconf => "domain/conflicts.md",
            Dec => "domain/decisions.md",
            Opp => "opportunity/opportunities.md",
            Asm => "opportunity/assumptions.md",
            Con => "opportunity/contradictions.md",
            Cpt => "opportunity/concepts.md",
            Hyp => "opportunity/hypotheses.md",
            Exp => "opportunity/experiments.md",
            Cdr => "opportunity/decisions.md",
            Req => "requirements/requirements.md",
            Nfr => "requirements/non-functional.md",
            Cnst => "requirements/constraints.md",
            Bas => "requirements/baselines.md",
            Uc => "use-cases/use-case-model.md",
            Spec => "specs/specs.md",
            Ac => "specs/acceptance-criteria.md",
            Plan => "plans/plans.md",
            Task => "tasks/tasks.md",
            Ctr => "contracts/contracts.md",
            Adr => "architecture/decisions/records.md",
            Scn => "validation/scenarios.md",
            Test => "validation/acceptance-tests.md",
            Gap => "validation/gaps.md",
            Drift => "validation/drift.md",
        }
    }

    /// Resolves a prefix, accepting the aliases used in the source notes.
    pub fn from_prefix(prefix: &str) -> Result<IdKind> {
        let norm = prefix.trim().to_ascii_uppercase();
        if let Some(kind) = Self::ALL.iter().copied().find(|k| k.prefix() == norm) {
            return Ok(kind);
        }
        // Aliases: longer or more explicit spellings that appear in the wild.
        let alias = match norm.as_str() {
            "CONFLICT" => Some(IdKind::Xconf),
            "CONTRACT" => Some(IdKind::Ctr),
            "CONCEPT" => Some(IdKind::Cpt),
            "BASELINE" => Some(IdKind::Bas),
            "SCENARIO" => Some(IdKind::Scn),
            "ADR" => Some(IdKind::Adr),
            "NFR" | "NONFUNCTIONAL" => Some(IdKind::Nfr),
            "CONSTRAINT" | "CONSTR" => Some(IdKind::Cnst),
            "REQS" => Some(IdKind::Req),
            "UCASE" => Some(IdKind::Uc),
            _ => None,
        };
        alias.ok_or_else(|| OdError::UnknownPrefix {
            prefix: prefix.to_string(),
            known: Self::ALL
                .iter()
                .map(|k| k.prefix())
                .collect::<Vec<_>>()
                .join(", "),
        })
    }

    /// Canonical spellings joined with `", "`, for error messages.
    pub fn prefixes() -> String {
        Self::ALL
            .iter()
            .map(|k| k.prefix())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix())
    }
}

/// A parsed artifact identifier.
///
/// ```text
/// kind  prefix that selects the artifact family
/// scope optional discriminator, usually the bounded context
/// number monotonic sequence within (kind, scope)
/// sub    optional disambiguator, e.g. acceptance criterion 1 of 12
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id {
    kind: IdKind,
    scope: Option<String>,
    number: u32,
    sub: Option<u32>,
}

impl Id {
    /// Builds a flat identifier.
    pub fn new(kind: IdKind, number: u32) -> Self {
        Id {
            kind,
            scope: None,
            number,
            sub: None,
        }
    }

    /// Builds a scoped identifier.
    pub fn scoped(kind: IdKind, scope: impl Into<String>, number: u32) -> Self {
        Id {
            kind,
            scope: Some(normalize_scope(scope.into())),
            number,
            sub: None,
        }
    }

    /// Adds a sub-number, producing e.g. `AC-INVENTORY-012-01`.
    pub fn with_sub(mut self, sub: u32) -> Self {
        self.sub = Some(sub);
        self
    }

    /// The artifact family.
    pub fn kind(&self) -> IdKind {
        self.kind
    }

    /// The scope, if any.
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// The sequence number.
    pub fn number(&self) -> u32 {
        self.number
    }

    /// The sub-number, if any.
    pub fn sub(&self) -> Option<u32> {
        self.sub
    }

    /// The stage this identifier belongs to.
    pub fn stage(&self) -> Stage {
        self.kind.stage()
    }

    /// True when the identifier carries a scope.
    pub fn is_scoped(&self) -> bool {
        self.scope.is_some()
    }

    /// Parses an identifier, accepting scoped and sub-numbered forms.
    ///
    /// ```
    /// # use opendomain_core::ids::Id;
    /// # use opendomain_core::error::Result;
    /// let flat: Id = "REQ-007".parse().unwrap();
    /// assert_eq!(flat.scope(), None);
    ///
    /// let scoped: Id = "REQ-INVENTORY-012".parse().unwrap();
    /// assert_eq!(scoped.scope(), Some("INVENTORY"));
    ///
    /// let ac = "AC-INVENTORY-012-01".parse::<Id>().unwrap();
    /// assert_eq!(ac.sub(), Some(1));
    /// ```
    pub fn parse(raw: &str) -> Result<Id> {
        let text = raw.trim().to_ascii_uppercase();
        if text.is_empty() {
            return Err(OdError::BadId(raw.to_string()));
        }

        let parts: Vec<&str> = text.split('-').collect();
        if parts.len() < 2 {
            return Err(OdError::BadId(raw.to_string()));
        }

        // The first segment must be a known prefix.
        let kind = IdKind::from_prefix(parts[0])?;
        let rest = &parts[1..];

        // Trailing all-digit segments become number, then sub.
        let mut digits: Vec<u32> = Vec::new();
        let mut scope_parts: Vec<&str> = Vec::new();
        for part in rest {
            if let Ok(n) = part.parse::<u32>() {
                digits.push(n);
            } else {
                scope_parts.push(part);
            }
        }
        if digits.is_empty() || digits.len() > 2 {
            return Err(OdError::BadId(raw.to_string()));
        }

        let sub = if digits.len() == 2 {
            Some(digits[1])
        } else {
            None
        };
        let number = digits[0];
        let scope = if scope_parts.is_empty() {
            None
        } else {
            Some(normalize_scope(scope_parts.join("-")))
        };

        Ok(Id {
            kind,
            scope,
            number,
            sub,
        })
    }

    /// The next free flat identifier of this kind, given the ids already used.
    ///
    /// This is how `opendomain new` keeps sequences unique without a database.
    pub fn next_flat(kind: IdKind, existing: &[Id]) -> Id {
        let max = existing
            .iter()
            .filter(|i| i.kind == kind && i.scope.is_none())
            .map(|i| i.number)
            .max()
            .unwrap_or(0);
        Id::new(kind, max + 1)
    }

    /// The next free identifier for a given scope.
    pub fn next_scoped(kind: IdKind, scope: Option<&str>, existing: &[Id]) -> Id {
        match scope {
            Some(s) => {
                let max = existing
                    .iter()
                    .filter(|i| i.kind == kind && i.scope() == Some(s))
                    .map(|i| i.number)
                    .max()
                    .unwrap_or(0);
                Id::scoped(kind, s, max + 1)
            }
            None => Id::next_flat(kind, existing),
        }
    }
}

fn normalize_scope(scope: String) -> String {
    scope
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.prefix())?;
        if let Some(scope) = &self.scope {
            write!(f, "-{scope}")?;
        }
        write!(f, "-{:03}", self.number)?;
        if let Some(sub) = self.sub {
            write!(f, "-{sub:02}")?;
        }
        Ok(())
    }
}

impl FromStr for Id {
    type Err = OdError;
    fn from_str(s: &str) -> Result<Self> {
        Id::parse(s)
    }
}

impl Serialize for Id {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Id::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl std::str::FromStr for IdKind {
    type Err = OdError;
    fn from_str(s: &str) -> Result<Self> {
        IdKind::from_prefix(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_identifier() {
        let id = Id::parse("REQ-007").unwrap();
        assert_eq!(id.kind(), IdKind::Req);
        assert_eq!(id.number(), 7);
        assert_eq!(id.scope(), None);
        assert_eq!(id.sub(), None);
    }

    #[test]
    fn parses_scoped_identifier() {
        let id = Id::parse("REQ-Inventory-012").unwrap();
        assert_eq!(id.kind(), IdKind::Req);
        assert_eq!(id.scope(), Some("INVENTORY"));
        assert_eq!(id.number(), 12);
    }

    #[test]
    fn parses_multi_token_scope() {
        let id = Id::parse("CTR-STOCK-API-001").unwrap();
        assert_eq!(id.kind(), IdKind::Ctr);
        assert_eq!(id.scope(), Some("STOCK-API"));
        assert_eq!(id.number(), 1);
    }

    #[test]
    fn parses_sub_numbered_identifier() {
        let id = Id::parse("AC-STOCK-003-01").unwrap();
        assert_eq!(id.kind(), IdKind::Ac);
        assert_eq!(id.scope(), Some("STOCK"));
        assert_eq!(id.number(), 3);
        assert_eq!(id.sub(), Some(1));
        assert_eq!(id.to_string(), "AC-STOCK-003-01");
    }

    #[test]
    fn round_trips_through_display() {
        for raw in [
            "SRC-001",
            "EVD-031",
            "TERM-014",
            "RULE-INVENTORY-003",
            "SPEC-021",
            "TASK-001",
            "AC-INVENTORY-012-01",
        ] {
            let id = Id::parse(raw).unwrap();
            assert_eq!(id.to_string(), raw, "round trip failed for {raw}");
        }
    }

    #[test]
    fn accepts_long_form_aliases() {
        assert_eq!(Id::parse("CONTRACT-001").unwrap().kind(), IdKind::Ctr);
        assert_eq!(Id::parse("CONFLICT-001").unwrap().kind(), IdKind::Xconf);
        assert_eq!(Id::parse("CONCEPT-001").unwrap().kind(), IdKind::Cpt);
    }

    #[test]
    fn rejects_malformed_identifiers() {
        for raw in ["", "REQ", "REQ-", "REQ-abc", "ZZZ-001", "REQ-1-2-3"] {
            assert!(Id::parse(raw).is_err(), "{raw} should not parse");
        }
    }

    #[test]
    fn allocates_next_free_number() {
        let existing = vec![Id::new(IdKind::Req, 1), Id::new(IdKind::Req, 4)];
        assert_eq!(Id::next_flat(IdKind::Req, &existing).to_string(), "REQ-005");
    }

    #[test]
    fn allocates_next_free_scoped_number() {
        let existing = vec![
            Id::scoped(IdKind::Req, "INVENTORY", 2),
            Id::scoped(IdKind::Req, "SALES", 7),
        ];
        let next = Id::next_scoped(IdKind::Req, Some("INVENTORY"), &existing);
        assert_eq!(next.to_string(), "REQ-INVENTORY-003");
    }

    #[test]
    fn every_kind_has_a_unique_prefix_and_label() {
        let mut seen = std::collections::BTreeSet::new();
        for kind in IdKind::ALL {
            assert!(seen.insert(kind.prefix()), "duplicate prefix for {kind:?}");
            assert!(!kind.label().is_empty());
            assert!(kind.default_path().ends_with(".md"));
        }
    }

    #[test]
    fn plurals_are_not_formed_by_appending_s() {
        // The default `label() + "s"` is wrong for these, and gate messages
        // read as "no processs has been modelled" if it leaks through.
        assert_eq!(IdKind::Proc.plural(), "processes");
        assert_eq!(IdKind::Ent.plural(), "entities");
        assert_eq!(IdKind::Act.plural(), "actors");
        assert_eq!(IdKind::Cnst.plural(), "constraints");
        assert_eq!(IdKind::Ac.plural(), "acceptance criteria");
        assert_eq!(IdKind::Nfr.plural(), "non-functional requirements");
        assert_eq!(IdKind::Evd.plural(), "evidence");
        // `-y` plurals: `opendomain status` prints these as registry headings, so a
        // leaked default reads as "hypothesiss" in the first thing a user runs.
        assert_eq!(IdKind::Unc.plural(), "uncertainties");
        assert_eq!(IdKind::Opp.plural(), "opportunities");
        assert_eq!(IdKind::Hyp.plural(), "hypotheses");
        // ...and the regular cases must keep working.
        assert_eq!(IdKind::Req.plural(), "requirements");
        assert_eq!(IdKind::Term.plural(), "terms");
    }

    #[test]
    fn every_kind_round_trips_through_from_prefix() {
        for kind in IdKind::ALL {
            assert_eq!(IdKind::from_prefix(kind.prefix()).unwrap(), *kind);
        }
    }

    #[test]
    fn stages_cover_every_kind() {
        for stage in [Stage::Domain, Stage::Opportunity, Stage::Specification] {
            assert!(
                IdKind::ALL.iter().any(|k| k.stage() == stage),
                "stage {stage} has no artifacts"
            );
        }
    }
}
