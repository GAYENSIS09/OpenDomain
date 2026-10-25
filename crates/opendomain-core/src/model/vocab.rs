//! Controlled vocabularies.
//!
//! Every closed set of values used across OpenDomain artifacts lives here so
//! that the CLI, the JSON Schema and the documentation site cannot drift apart.
//!
//! The epistemic vocabulary is the load-bearing one: OpenDomain's central rule
//! is that a claim's *provenance* is never silently upgraded. `ASSUMED` must
//! never be rendered or reported as if it were `CONFIRMED`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Declares a closed string enum with canonical uppercase spellings, a
/// case-insensitive parser and an `ALL` constant.
macro_rules! enum_set {
    (
        $(#[$meta:meta])*
        $v:ident { $($variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $v {
            $(
                #[serde(rename = $text)]
                $variant,
            )+
        }

        impl $v {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$v] = &[$($v::$variant),+];

            /// Canonical uppercase spelling.
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            /// Canonical spellings joined with `", "`, for error messages.
            pub fn choices() -> String {
                Self::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
            }
        }

        impl fmt::Display for $v {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $v {
            type Err = String;
            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                let norm = s.trim().to_ascii_uppercase().replace([' ', '-'], "_");
                $(
                    if norm == $text {
                        return Ok(Self::$variant);
                    }
                )+
                Err(format!(
                    "unknown {} `{}` (expected one of: {})",
                    stringify!($v), s, Self::choices()
                ))
            }
        }
    };
}

enum_set! {
    /// How a piece of domain knowledge came to be known.
    ///
    /// `ASSUMED` is a first-class value: an unvalidated assumption that was
    /// written down so work could continue is *better* than an unrecorded one.
    KnowledgeType {
        Explicit   => "EXPLICIT",
        Tacit       => "TACIT",
        Hybrid      => "HYBRID",
        Inferred    => "INFERRED",
        Assumed     => "ASSUMED",
        Unknown     => "UNKNOWN",
        Conflicting => "CONFLICTING",
    }
}

enum_set! {
    /// Strength of the support behind an artifact.
    Confidence {
        High   => "HIGH",
        Medium => "MEDIUM",
        Low    => "LOW",
    }
}

enum_set! {
    /// What kind of statement an evidence record makes.
    EvidenceType {
        UserNeed              => "USER_NEED",
        DomainFact            => "DOMAIN_FACT",
        BusinessRule          => "BUSINESS_RULE",
        IndustryPractice      => "INDUSTRY_PRACTICE",
        RegulatoryRequirement => "REGULATORY_REQUIREMENT",
        TechnicalConstraint   => "TECHNICAL_CONSTRAINT",
        CompetitorCapability  => "COMPETITOR_CAPABILITY",
        MarketSignal          => "MARKET_SIGNAL",
        Assumption            => "ASSUMPTION",
        Hypothesis            => "HYPOTHESIS",
        Inference             => "INFERENCE",
    }
}

enum_set! {
    /// What kind of statement a business rule makes.
    RuleType {
        Legal          => "LEGAL",
        Regulatory     => "REGULATORY",
        Organizational => "ORGANIZATIONAL",
        Operational    => "OPERATIONAL",
        Calculation    => "CALCULATION",
        Security       => "SECURITY",
        DataIntegrity  => "DATA_INTEGRITY",
        Workflow       => "WORKFLOW",
    }
}

enum_set! {
    /// What kind of obligation a requirement expresses.
    RequirementType {
        Business            => "BUSINESS",
        Functional          => "FUNCTIONAL",
        NonFunctional       => "NON_FUNCTIONAL",
        Data                => "DATA",
        Security            => "SECURITY",
        Integration         => "INTEGRATION",
        Compliance          => "COMPLIANCE",
        Operational         => "OPERATIONAL",
        TechnicalConstraint => "TECHNICAL_CONSTRAINT",
    }
}

enum_set! {
    /// How important a capability is for the current release.
    ///
    /// Classification is driven by project objectives and evidence, never by
    /// how interesting a capability looks.
    Scope {
        Core        => "CORE",
        Important   => "IMPORTANT",
        Optional    => "OPTIONAL",
        Future      => "FUTURE",
        OutOfScope  => "OUT_OF_SCOPE",
        Experimental=> "EXPERIMENTAL",
    }
}

enum_set! {
    /// Where a brownfield behavior sits relative to intent.
    BehaviorClass {
        CurrentBehavior      => "CURRENT_BEHAVIOR",
        DesiredBehavior      => "DESIRED_BEHAVIOR",
        UndocumentedBehavior => "UNDOCUMENTED_BEHAVIOR",
        AccidentalBehavior   => "ACCIDENTAL_BEHAVIOR",
        DeprecatedBehavior   => "DEPRECATED_BEHAVIOR",
    }
}

enum_set! {
    /// The twelve recurring shapes of pain found in an existing process.
    Friction {
        Pain                 => "PAIN",
        Friction             => "FRICTION",
        Waste                => "WASTE",
        Delay                => "DELAY",
        Repetition           => "REPETITION",
        Duplication          => "DUPLICATION",
        ContextSwitching     => "CONTEXT_SWITCHING",
        InformationAsymmetry => "INFORMATION_ASYMMETRY",
        DecisionBottleneck   => "DECISION_BOTTLENECK",
        ErrorProne           => "ERROR_PRONE",
        InvisibleWork        => "INVISIBLE_WORK",
        UnmetNeed            => "UNMET_NEED",
    }
}

enum_set! {
    /// Transformative moves applied to a problem to generate directions.
    OpportunityOperator {
        Remove     => "REMOVE",
        Automate   => "AUTOMATE",
        Delegate   => "DELEGATE",
        Merge      => "MERGE",
        Split      => "SPLIT",
        Invert     => "INVERT",
        Predict    => "PREDICT",
        Proact     => "PROACT",
        Personalize=> "PERSONALIZE",
        Autonomize => "AUTONOMIZE",
        Combine    => "COMBINE",
        Abstract   => "ABSTRACT",
        Reframe    => "REFRAME",
        Reverse    => "REVERSE",
    }
}

enum_set! {
    /// What kind of novelty a concept actually represents.
    ///
    /// Used to prevent ordinary improvements being relabelled as breakthroughs.
    InnovationClass {
        Replication           => "REPLICATION",
        Improvement           => "IMPROVEMENT",
        Combination           => "COMBINATION",
        Reframing             => "REFRAMING",
        WorkflowInnovation    => "WORKFLOW_INNOVATION",
        MechanismInnovation   => "MECHANISM_INNOVATION",
        BusinessModelInnovation => "BUSINESS_MODEL_INNOVATION",
        SystemInnovation      => "SYSTEM_INNOVATION",
    }
}

enum_set! {
    /// The risk a hypothesis carries, which determines how it is tested.
    HypothesisCategory {
        User        => "USER",
        Technical   => "TECHNICAL",
        Business    => "BUSINESS",
        Behavioral  => "BEHAVIORAL",
        Data        => "DATA",
        Safety      => "SAFETY",
        Performance => "PERFORMANCE",
        Adoption    => "ADOPTION",
        Economic    => "ECONOMIC",
        Operational => "OPERATIONAL",
    }
}

enum_set! {
    /// What an experiment concluded.
    ///
    /// `Inconclusive` is a first-class outcome and must never be reported as
    /// validation.
    ExperimentOutcome {
        Validated           => "VALIDATED",
        PartiallyValidated  => "PARTIALLY_VALIDATED",
        Rejected            => "REJECTED",
        Inconclusive        => "INCONCLUSIVE",
        NeedsMoreEvidence   => "NEEDS_MORE_EVIDENCE",
        Pivot               => "PIVOT",
    }
}

enum_set! {
    /// Relative strength of a claim in the opportunity stage.
    OpportunityConfidence {
        Known      => "KNOWN",
        Supported  => "SUPPORTED",
        Likely     => "LIKELY",
        Inferred   => "INFERRED",
        Hypothesis => "HYPOTHESIS",
        Speculative=> "SPECULATIVE",
        Unknown    => "UNKNOWN",
        Conflicting=> "CONFLICTING",
    }
}

enum_set! {
    /// Blast-radius classification for a change. Returned by `opendomain impact`.
    ImpactClass {
        Direct   => "DIRECT_IMPACT",
        Indirect => "INDIRECT_IMPACT",
        None     => "NO_IMPACT",
        Unknown  => "UNKNOWN",
    }
}

enum_set! {
    /// What a change does to the existing specification set.
    DeltaKind {
        Added      => "ADDED",
        Modified   => "MODIFIED",
        Removed    => "REMOVED",
        Renamed    => "RENAMED",
    }
}

/// Lifecycle position of any artifact. `DRAFT` is never equivalent to `VALIDATED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactStatus {
    Draft,
    Researched,
    Supported,
    Validated,
    Approved,
    Implementing,
    Implemented,
    Verified,
    Blocked,
    Open,
    Conflicting,
    Deprecated,
    Superseded,
}

impl ArtifactStatus {
    /// Every variant, in lifecycle order.
    pub const ALL: &'static [ArtifactStatus] = &[
        ArtifactStatus::Draft,
        ArtifactStatus::Researched,
        ArtifactStatus::Supported,
        ArtifactStatus::Validated,
        ArtifactStatus::Approved,
        ArtifactStatus::Implementing,
        ArtifactStatus::Implemented,
        ArtifactStatus::Verified,
        ArtifactStatus::Blocked,
        ArtifactStatus::Open,
        ArtifactStatus::Conflicting,
        ArtifactStatus::Deprecated,
        ArtifactStatus::Superseded,
    ];

    /// Canonical uppercase spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactStatus::Draft => "DRAFT",
            ArtifactStatus::Researched => "RESEARCHED",
            ArtifactStatus::Supported => "SUPPORTED",
            ArtifactStatus::Validated => "VALIDATED",
            ArtifactStatus::Approved => "APPROVED",
            ArtifactStatus::Implementing => "IMPLEMENTING",
            ArtifactStatus::Implemented => "IMPLEMENTED",
            ArtifactStatus::Verified => "VERIFIED",
            ArtifactStatus::Blocked => "BLOCKED",
            ArtifactStatus::Open => "OPEN",
            ArtifactStatus::Conflicting => "CONFLICTING",
            ArtifactStatus::Deprecated => "DEPRECATED",
            ArtifactStatus::Superseded => "SUPERSEDED",
        }
    }

    /// Position in the linear lifecycle, or `None` for terminal states.
    pub fn rank(self) -> Option<u8> {
        match self {
            ArtifactStatus::Draft => Some(0),
            ArtifactStatus::Researched => Some(1),
            ArtifactStatus::Supported => Some(2),
            ArtifactStatus::Validated => Some(3),
            ArtifactStatus::Approved => Some(4),
            ArtifactStatus::Implementing => Some(5),
            ArtifactStatus::Implemented => Some(6),
            ArtifactStatus::Verified => Some(7),
            _ => None,
        }
    }

    /// True when this status is at least as advanced as `other`.
    ///
    /// Terminal states never satisfy a progress claim, which is what stops a
    /// `SUPERSEDED` artifact from being counted as coverage.
    pub fn at_least(self, other: ArtifactStatus) -> bool {
        match (self.rank(), other.rank()) {
            (Some(a), Some(b)) => a >= b,
            _ => false,
        }
    }

    /// Canonical spellings joined with `", "`, for error messages.
    pub fn choices() -> String {
        Self::ALL
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for ArtifactStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ArtifactStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let norm = s.trim().to_ascii_uppercase().replace([' ', '-'], "_");
        Self::ALL
            .iter()
            .copied()
            .find(|v| v.as_str() == norm)
            .ok_or_else(|| {
                format!(
                    "unknown status `{s}` (expected one of: {})",
                    ArtifactStatus::choices()
                )
            })
    }
}

enum_set! {
    /// The three ordered stages of the lifecycle.
    ///
    /// Stage order is load-bearing: it is what stops specification from
    /// happening before the concept deserves to be specified.
    Stage {
        Domain        => "DOMAIN",
        Opportunity   => "OPPORTUNITY",
        Specification => "SPECIFICATION",
    }
}

impl Stage {
    /// Ordinal position, used for ordering reports.
    pub fn ordinal(self) -> u8 {
        match self {
            Stage::Domain => 0,
            Stage::Opportunity => 1,
            Stage::Specification => 2,
        }
    }
}

enum_set! {
    /// How a specification is allowed to relate to code. See the SDD authority models.
    AuthorityModel {
        SpecFirst    => "SPEC_FIRST",
        SpecAnchored => "SPEC_ANCHORED",
        SpecAsSource => "SPEC_AS_SOURCE",
    }
}

impl Default for AuthorityModel {
    fn default() -> Self {
        AuthorityModel::SpecAnchored
    }
}

impl AuthorityModel {
    /// One-line justification for when this model applies.
    pub fn guidance(self) -> &'static str {
        match self {
            AuthorityModel::SpecFirst => {
                "Throwaway prototypes and short-lived work. The spec must still exist first."
            }
            AuthorityModel::SpecAnchored => {
                "Default for production systems. Spec and code reconcile through tests; \
                 behaviour changes update spec, tests, then code."
            }
            AuthorityModel::SpecAsSource => {
                "Only with trustworthy generators (OpenAPI, Protobuf, AsyncAPI). \
                 LLM-written application code is not spec-as-source."
            }
        }
    }
}

enum_set! {
    /// Whether a project starts from nothing, from code, or from both.
    RepositoryKind {
        Greenfield => "GREENFIELD",
        Brownfield => "BROWNFIELD",
        Hybrid     => "HYBRID",
    }
}

impl Default for RepositoryKind {
    fn default() -> Self {
        RepositoryKind::Greenfield
    }
}

enum_set! {
    /// How much of the framework a project opts into.
    ///
    /// `LEAN` starts at Stage 3 and assumes the domain model already exists
    /// elsewhere; it is not a licence to skip Stage 1.
    Profile {
        Lean   => "LEAN",
        Core   => "CORE",
        Full   => "FULL",
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::Core
    }
}

impl Profile {
    /// `true` when the profile includes Stage 2 (Opportunity).
    pub fn has_opportunity(self) -> bool {
        matches!(self, Profile::Full)
    }

    /// `true` when the profile expects the domain registries to exist.
    pub fn has_domain(self) -> bool {
        !matches!(self, Profile::Lean)
    }
}
