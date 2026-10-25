//! La définition clap de `opendomain` : commandes, arguments et conversions.
//!
//! Un seul fichier porte `enum Command` : le test côté core
//! (`scaffold::tests::cli_commands`) le scrape pour vérifier que le contrat
//! méta-agent (`AGENTS.md`) ne nomme que des commandes qui existent.

use clap::{Args, Parser, Subcommand, ValueEnum};
use opendomain_core::vocab::{AuthorityModel, Profile, RepositoryKind};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "opendomain",
    version,
    about = "OpenDomain: evidence-driven engineering",
    long_about = "OpenDomain keeps a project's domain knowledge as linked Markdown artifacts \
                  and enforces the links with nine deterministic gates. `opendomain` reads \
                  that knowledge; it never rewrites your registries unless a command \
                  explicitly says it does."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Project directory to search upwards from. Defaults to the current directory.
    #[arg(short = 'C', long, global = true, value_name = "DIR")]
    pub directory: Option<PathBuf>,

    /// Emit machine-readable JSON instead of a rendered report.
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable ANSI colour even on a colour-capable terminal.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Print more detail.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create `opendomain/` with every registry, config, and agent instructions.
    Init(InitArgs),
    /// Summarise the workspace: counts per registry, coverage, current gate.
    Status,
    /// Explain why an artifact exists, or what implements and verifies it.
    Trace(TraceArgs),
    /// List what a change to an artifact would affect.
    Impact(ImpactArgs),
    /// Render the traceability graph.
    Graph(GraphArgs),
    /// Run a quality gate.
    Gate(GateArgs),
    /// Report traceability health across the whole workspace.
    Audit,
    /// Check or create links between artifacts.
    Link(LinkArgs),
    /// Show one artifact's full record.
    Show(ShowArgs),
    /// Print the next free identifier of a kind. Read-only: reserves nothing,
    /// so confirm the id is still free if the registry changed since.
    New(NewArgs),
    /// Move an artifact along its lifecycle.
    Advance(AdvanceArgs),
    /// Start a change proposal, following the OpenSpec layout.
    Propose(ProposeArgs),
    /// Mark a change as applied.
    Apply(ApplyArgs),
    /// Archive a completed change.
    Archive(ArchiveArgs),
    /// List the available gates and artifact kinds.
    Reference,
    /// Print the §9.7 rigour table and audit the workspace's rigour
    /// declarations (Section 9 verification subsystem).
    Rigour,
    /// Detect the stack, apply the ordered selection rules, and generate
    /// executable-specification scaffolds — or refuse, on record (Section 10).
    Provision {
        /// Print what would be written without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Explicit name of the default regeneration (the paper's interface);
        /// scaffolds regenerate idempotently on every run.
        #[arg(long)]
        sync: bool,
    },
    /// Audit test→criterion bindings: provenance, verification depth, and the
    /// design checks of Section 10.9/10.10 (reported findings, not gate rules).
    Bind,
    /// Verify each record's archived-run citation (`run = "…"`) against the
    /// run it names, and list the runs no record claims (F14/L9 bridge).
    Provenance,
}

#[derive(Args)]
pub struct InitArgs {
    /// Project name. Defaults to the directory name.
    #[arg(long)]
    pub name: Option<String>,
    /// Which parts of the framework to scaffold.
    #[arg(long, value_enum, default_value_t = ProfileArg::Full)]
    pub profile: ProfileArg,
    /// How specifications relate to code.
    #[arg(long, value_enum, default_value_t = AuthorityArg::SpecAnchored)]
    pub authority: AuthorityArg,
    /// Whether the repository starts empty, from existing code, or both.
    #[arg(long, value_enum, default_value_t = RepositoryArg::Greenfield)]
    pub repository: RepositoryArg,
    /// Overwrite files that already exist.
    #[arg(long)]
    pub force: bool,
    /// Report what would happen without writing anything.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct TraceArgs {
    /// Artifact identifier, for example REQ-001.
    pub id: String,
    /// Follow the chain forward to implementations and tests instead.
    #[arg(long, short)]
    pub forward: bool,
    /// Stop at the nearest source rather than walking the whole closure.
    #[arg(long)]
    pub provenance: bool,
}

#[derive(Args)]
pub struct ImpactArgs {
    /// Artifact identifier whose change is being considered.
    pub id: String,
    /// Cap the number of rows printed.
    #[arg(long, short, default_value_t = 50)]
    pub limit: usize,
}

#[derive(Args)]
pub struct GraphArgs {
    /// Start the rendering from this artifact instead of the whole graph.
    #[arg(long)]
    pub root: Option<String>,
    /// Restrict the graph to a few hops from `--root`.
    #[arg(long)]
    pub depth: Option<usize>,
    /// Cap the number of nodes rendered.
    #[arg(long, short, default_value_t = 200)]
    pub limit: usize,
    /// Output Graphviz DOT instead of Mermaid.
    #[arg(long)]
    pub dot: bool,
}

#[derive(Args)]
pub struct GateArgs {
    /// Gate name (context, research, domain, requirements, opportunity,
    /// specification, plan, implementation, completion) or number 0-8.
    pub gate: Option<String>,
    /// Run every gate up to and including this one.
    #[arg(long, short)]
    pub through: Option<u8>,
    /// Accepted for explicitness; blocking findings always fail the command and
    /// advisory ones never do, so this changes nothing today.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args)]
pub struct LinkArgs {
    /// Verify that every referenced identifier exists.
    #[arg(long)]
    pub check: bool,
    /// Artifact that declares the link.
    pub id: Option<String>,
    /// Relation name, for example derived-from.
    pub relation: Option<String>,
    /// Target identifier.
    pub target: Option<String>,
}

#[derive(Args)]
pub struct ShowArgs {
    /// Artifact identifier to print.
    pub id: String,
}

#[derive(Args)]
pub struct NewArgs {
    /// Artifact kind prefix, for example REQ, EVD or RULE.
    pub kind: String,
    /// Scope for scoped identifiers, for example INVENTORY in REQ-INVENTORY-014.
    #[arg(long)]
    pub scope: Option<String>,
}

#[derive(Args)]
pub struct AdvanceArgs {
    /// Artifact identifier to move.
    pub id: String,
    /// The status to move it to.
    pub status: String,
}

#[derive(Args)]
pub struct ProposeArgs {
    /// Short kebab-case slug for the change.
    pub slug: String,
    /// One-line summary of what the change does.
    #[arg(long, default_value = "")]
    pub summary: String,
}

#[derive(Args)]
pub struct ApplyArgs {
    /// Change slug, without the `changes/` prefix.
    pub slug: String,
}

#[derive(Args)]
pub struct ArchiveArgs {
    /// Change slug to move under `changes/archive/`.
    pub slug: String,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum ProfileArg {
    /// Start at Stage 3; the domain model already exists elsewhere.
    Lean,
    /// Stages 0, 1, 3-8: domain, requirements through completion.
    Core,
    /// Every stage, including opportunity discovery.
    Full,
}

impl From<ProfileArg> for Profile {
    fn from(a: ProfileArg) -> Profile {
        match a {
            ProfileArg::Lean => Profile::Lean,
            ProfileArg::Core => Profile::Core,
            ProfileArg::Full => Profile::Full,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum AuthorityArg {
    SpecFirst,
    SpecAnchored,
    SpecAsSource,
}

impl From<AuthorityArg> for AuthorityModel {
    fn from(a: AuthorityArg) -> AuthorityModel {
        match a {
            AuthorityArg::SpecFirst => AuthorityModel::SpecFirst,
            AuthorityArg::SpecAnchored => AuthorityModel::SpecAnchored,
            AuthorityArg::SpecAsSource => AuthorityModel::SpecAsSource,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum RepositoryArg {
    Greenfield,
    Brownfield,
    Hybrid,
}

impl From<RepositoryArg> for RepositoryKind {
    fn from(a: RepositoryArg) -> RepositoryKind {
        match a {
            RepositoryArg::Greenfield => RepositoryKind::Greenfield,
            RepositoryArg::Brownfield => RepositoryKind::Brownfield,
            RepositoryArg::Hybrid => RepositoryKind::Hybrid,
        }
    }
}
