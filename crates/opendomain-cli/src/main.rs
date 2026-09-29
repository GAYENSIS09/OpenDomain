//! The `od` command-line interface.
//!
//! Every subcommand is a thin shell over `opendomain_core`: discover a
//! workspace, load the index, hand it to a pure function, render the result.
//! Nothing here mutates a registry except `od init`, `od link`, and the
//! lifecycle commands, and those only append.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use opendomain_core::gates::{self, Gate};
use opendomain_core::graph::{Direction, Graph};
use opendomain_core::ids::{Id, IdKind};
use opendomain_core::report::{self, Style};
use opendomain_core::scaffold;
use opendomain_core::vocab::{ArtifactStatus, AuthorityModel, Profile, RepositoryKind};
use opendomain_core::workspace::{Index, Workspace};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "od",
    version,
    about = "OpenDomain: domain-first, evidence-driven engineering",
    long_about = "OpenDomain keeps a project's domain knowledge as linked Markdown artifacts \
                  and enforces the links. `od` reads that knowledge; it never rewrites your \
                  registries unless a command explicitly says it does."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Project directory to search upwards from. Defaults to the current directory.
    #[arg(short = 'C', long, global = true, value_name = "DIR")]
    directory: Option<PathBuf>,

    /// Emit machine-readable JSON instead of a rendered report.
    #[arg(long, global = true)]
    json: bool,

    /// Disable ANSI colour even on a colour-capable terminal.
    #[arg(long, global = true)]
    no_color: bool,

    /// Print more detail.
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
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
}

#[derive(Args)]
struct InitArgs {
    /// Project name. Defaults to the directory name.
    #[arg(long)]
    name: Option<String>,
    /// One-line description of the project.
    #[arg(long, default_value = "")]
    description: String,
    /// Which parts of the framework to scaffold.
    #[arg(long, value_enum, default_value_t = ProfileArg::Full)]
    profile: ProfileArg,
    /// How specifications relate to code.
    #[arg(long, value_enum, default_value_t = AuthorityArg::SpecAnchored)]
    authority: AuthorityArg,
    /// Whether the repository starts empty, from existing code, or both.
    #[arg(long, value_enum, default_value_t = RepositoryArg::Greenfield)]
    repository: RepositoryArg,
    /// Overwrite files that already exist.
    #[arg(long)]
    force: bool,
    /// Report what would happen without writing anything.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct TraceArgs {
    /// Artifact identifier, for example REQ-001.
    id: String,
    /// Follow the chain forward to implementations and tests instead.
    #[arg(long, short)]
    forward: bool,
    /// Stop at the nearest source rather than walking the whole closure.
    #[arg(long)]
    provenance: bool,
}

#[derive(Args)]
struct ImpactArgs {
    /// Artifact identifier whose change is being considered.
    id: String,
    /// Cap the number of rows printed.
    #[arg(long, short, default_value_t = 50)]
    limit: usize,
}

#[derive(Args)]
struct GraphArgs {
    /// Start the rendering from this artifact instead of the whole graph.
    #[arg(long)]
    root: Option<String>,
    /// Restrict the graph to a few hops from `--root`.
    #[arg(long)]
    depth: Option<usize>,
    /// Cap the number of nodes rendered.
    #[arg(long, short, default_value_t = 200)]
    limit: usize,
    /// Output Graphviz DOT instead of Mermaid.
    #[arg(long)]
    dot: bool,
}

#[derive(Args)]
struct GateArgs {
    /// Gate name (context, research, domain, requirements, opportunity,
    /// specification, plan, implementation, completion) or number 0-8.
    gate: Option<String>,
    /// Run every gate up to and including this one.
    #[arg(long, short)]
    through: Option<u8>,
    /// Accepted for explicitness; blocking findings always fail the command and
    /// advisory ones never do, so this changes nothing today.
    #[arg(long)]
    strict: bool,
}

#[derive(Args)]
struct LinkArgs {
    /// Verify that every referenced identifier exists.
    #[arg(long)]
    check: bool,
    /// Artifact that declares the link.
    id: Option<String>,
    /// Relation name, for example derived-from.
    relation: Option<String>,
    /// Target identifier.
    target: Option<String>,
}

#[derive(Args)]
struct ShowArgs {
    /// Artifact identifier to print.
    id: String,
}

#[derive(Args)]
struct AdvanceArgs {
    /// Artifact identifier to move.
    id: String,
    /// The status to move it to.
    status: String,
}

#[derive(Args)]
struct ProposeArgs {
    /// Short kebab-case slug for the change.
    slug: String,
    /// One-line summary of what the change does.
    #[arg(long, default_value = "")]
    summary: String,
}

#[derive(Args)]
struct ApplyArgs {
    /// Change slug, without the `changes/` prefix.
    slug: String,
}

#[derive(Args)]
struct ArchiveArgs {
    /// Change slug to move under `changes/archive/`.
    slug: String,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ProfileArg {
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
enum AuthorityArg {
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
enum RepositoryArg {
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

fn main() {
    let cli = Cli::parse();
    let style = Style::detect(cli.no_color || cli.json);
    if let Err(err) = run(&cli, style) {
        eprintln!("od: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli, style: Style) -> Result<()> {
    match &cli.command {
        Command::Init(args) => cmd_init(args, cli, style),
        Command::Reference => cmd_reference(cli, style),
        Command::Status => with_index(cli, style, |ws, index, _| {
            let report = report::status("opendomain", &ws.config, index);
            if cli.json {
                println!("{}", to_json(&report)?);
            } else {
                println!("{}", report::render_status(&report, style));
            }
            Ok(())
        }),
        Command::Trace(args) => with_index(cli, style, |_ws, index, _| {
            cmd_trace(args, index, style, cli)
        }),
        Command::Impact(args) => with_index(cli, style, |_ws, index, _| {
            cmd_impact(args, index, style, cli)
        }),
        Command::Graph(args) => with_index(cli, style, |_ws, index, _| cmd_graph(args, index)),
        Command::Gate(args) => with_index(cli, style, |_ws, index, _| {
            cmd_gate(args, index, style, cli)
        }),
        Command::Audit => with_index(cli, style, |_ws, index, _| cmd_audit(index, style, cli)),
        Command::Link(args) => {
            with_index(cli, style, |ws, index, _| cmd_link(args, ws, index, style))
        }
        Command::Show(args) => with_index(cli, style, |_ws, index, _| {
            cmd_show(args, index, style, cli)
        }),
        Command::Advance(args) => with_index(cli, style, |ws, index, _| {
            cmd_advance(args, ws, index, style)
        }),
        Command::Propose(args) => {
            with_index(cli, style, |ws, _index, _| cmd_propose(args, ws, style))
        }
        Command::Apply(args) => with_index(cli, style, |ws, _index, _| cmd_apply(args, ws, style)),
        Command::Archive(args) => {
            with_index(cli, style, |ws, _index, _| cmd_archive(args, ws, style))
        }
    }
}

/// Resolves the project directory a command should operate on.
fn start_dir(cli: &Cli) -> PathBuf {
    cli.directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Discovers the workspace, loads the index, and hands both to `body`.
///
/// Every read-only command goes through here so that workspace discovery and
/// error reporting stay identical across the CLI.
fn with_index<F>(cli: &Cli, style: Style, body: F) -> Result<()>
where
    F: FnOnce(&Workspace, &Index, Style) -> Result<()>,
{
    let start = start_dir(cli);
    let ws = Workspace::discover(&start)
        .with_context(|| format!("no OpenDomain workspace found from {}", start.display()))?;
    let index = ws.load().context("failed to load the workspace index")?;
    body(&ws, &index, style)
}

fn parse_id(raw: &str) -> Result<Id> {
    // `BadId`'s Display already names the expected shape, so no extra wrapping.
    Id::parse(raw).map_err(|e| anyhow!("{}", e))
}

fn parse_gate(raw: &str) -> Result<Gate> {
    if let Ok(n) = raw.parse::<u8>() {
        if let Some(g) = Gate::by_number(n) {
            return Ok(g);
        }
        bail!("no gate numbered {}; gates run 0-8", n);
    }
    Gate::by_name(&raw.to_ascii_lowercase())
        .ok_or_else(|| anyhow!("unknown gate '{}'; run `od reference` for the list", raw))
}

fn cmd_init(args: &InitArgs, cli: &Cli, style: Style) -> Result<()> {
    let start = start_dir(cli);
    let name = args
        .name
        .clone()
        .or_else(|| {
            start
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "opendomain-project".into());

    let mut config = scaffold::default_config(&name, args.profile.into(), args.authority.into());
    config.repository = args.repository.into();
    if !args.description.is_empty() {
        config
            .project
            .insert("description".into(), args.description.clone());
    }

    let od_dir = start.join("opendomain");
    if args.dry_run {
        let templates = scaffold::templates::for_profile(config.profile);
        // `opendomain.toml` is written by `scaffold` itself rather than by a
        // template, so it has to be counted and listed here explicitly. Without
        // this the preview under-reported by one file, and the count did not
        // match what a real run created.
        let config_exists = od_dir.join("opendomain.toml").exists();
        let total = templates.len() + usize::from(!config_exists);
        println!("would create {total} file(s) under {}", od_dir.display());
        if !config_exists {
            println!("  opendomain.toml");
        }
        for t in templates {
            println!("  {}", t.path);
        }
        // The agent files go to the project root and follow different rules from
        // the registries, so the preview has to follow them too. Listing all
        // eleven as "would create" would be wrong twice over: a fresh run does
        // not create them all, and an existing `AGENTS.md` is never replaced
        // even under `--force`.
        let root = start.clone();
        let mut would_write = 0usize;
        let mut would_keep = 0usize;
        for t in scaffold::agent_contract::all() {
            let exists = root.join(&t.path).exists();
            let keep = exists && t.path == "AGENTS.md";
            if keep {
                would_keep += 1;
            } else if !exists {
                would_write += 1;
            }
            let note = match (keep, exists) {
                (true, _) => "  (left alone; --force does not replace this)",
                (_, true) => "  (unchanged; --force would refresh it)",
                _ => "",
            };
            println!("  {}{note}", t.path);
        }
        println!("\nagent instructions: {would_write} to write, {would_keep} left alone");
        return Ok(());
    }

    let report = scaffold::scaffold(&od_dir, &config, args.force)?;
    if cli.json {
        println!("{}", to_json(&report)?);
        return Ok(());
    }
    if report.is_noop() {
        println!(
            "{}",
            style_ok(
                style,
                "workspace already initialised; nothing to do (use --force to overwrite)"
            )
        );
    } else {
        println!(
            "{}",
            style_ok(
                style,
                &format!(
                    "initialised {} — {} created, {} already present, {} agent file(s)",
                    od_dir.display(),
                    report.created.len(),
                    report.skipped.len(),
                    report.agent_files.len()
                )
            )
        );
    }
    for p in &report.created {
        println!("  + {p}");
    }
    for p in &report.skipped {
        println!("  = {p} (unchanged)");
    }
    if !report.agent_files.is_empty() {
        println!("\nagent instructions:");
        for p in &report.agent_files {
            println!("  + {p}");
        }
    }
    println!("\nnext: od status");
    Ok(())
}

fn cmd_reference(cli: &Cli, style: Style) -> Result<()> {
    if cli.json {
        let gates: Vec<serde_json::Value> = Gate::ALL
            .iter()
            .map(|g| {
                serde_json::json!({
                    "number": g.number,
                    "name": g.name,
                    "stage": g.stage.as_str(),
                    "question": g.question,
                })
            })
            .collect();
        let kinds: Vec<serde_json::Value> = IdKind::ALL
            .iter()
            .map(|k| {
                serde_json::json!({
                    "prefix": k.prefix(),
                    "label": k.label(),
                    "path": k.default_path(),
                })
            })
            .collect();
        let rules: Vec<serde_json::Value> = gates::RULES
            .iter()
            .map(|r| {
                serde_json::json!({
                    "slug": r.full_slug(),
                    "gate": r.gate,
                    "gate_name": r.gate().name,
                    "severity": if r.severity == gates::Severity::Blocking {
                        "blocking"
                    } else {
                        "advisory"
                    },
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "gates": gates,
                "artifact_kinds": kinds,
                "rules": rules,
            })
        );
        return Ok(());
    }
    println!("gates\n");
    for g in Gate::ALL {
        println!("  {}  {:<14} {}", g.number, g.name, g.question);
    }
    println!("\nartifact kinds\n");
    for k in IdKind::ALL {
        println!("  {:<6} {:<24} {}", k.prefix(), k.label(), k.default_path());
    }

    // The rule catalogue. An agent has to be able to find out what will be
    // checked before it writes anything, not only after it trips a rule: a rule
    // that is invisible until it fires is a rule you find out about by failing.
    let blocking = gates::RULES
        .iter()
        .filter(|r| r.severity == gates::Severity::Blocking)
        .count();
    println!(
        "\nrules ({} total: {} blocking, {} advisory)\n",
        gates::RULES.len(),
        blocking,
        gates::RULES.len() - blocking
    );
    for g in Gate::ALL {
        let rules: Vec<&gates::Rule> = gates::RULES.iter().filter(|r| r.gate == g.number).collect();
        if rules.is_empty() {
            continue;
        }
        println!("  gate {} — {}", g.number, g.name);
        for r in rules {
            let mark = if r.severity == gates::Severity::Blocking {
                "BLOCK"
            } else {
                "adv  "
            };
            println!("    {mark}  {}", r.full_slug());
        }
    }
    let _ = style;
    Ok(())
}

fn cmd_trace(args: &TraceArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let id = parse_id(&args.id)?;
    if index.get(&id).is_none() {
        bail!("{} is not defined in this workspace", id);
    }
    let graph = Graph::build(index);
    let steps = if args.provenance {
        graph.provenance_chain(&id, index)
    } else {
        graph.reachable(
            &id,
            index,
            if args.forward {
                Direction::Forward
            } else {
                Direction::Backward
            },
        )
    };
    if steps.is_empty() {
        bail!("{} has no links in that direction", id);
    }
    if cli.json {
        println!("{}", to_json(&steps)?);
    } else {
        println!("{}", report::render_trace(&id, &steps, style));
    }
    Ok(())
}

fn cmd_impact(args: &ImpactArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let id = parse_id(&args.id)?;
    if index.get(&id).is_none() {
        bail!("{} is not defined in this workspace", id);
    }
    let graph = Graph::build(index);
    let report = graph.impact(&id, index);
    if cli.json {
        println!("{}", to_json(&report)?);
        return Ok(());
    }
    println!("{}", report::render_impact(&report, style));
    if args.limit > 0 {
        println!(
            "(showing up to {} entries; use --limit 0 for all)",
            args.limit
        );
    }
    Ok(())
}

fn cmd_graph(args: &GraphArgs, index: &Index) -> Result<()> {
    let graph = Graph::build(index);
    match &args.root {
        None => {
            let text = if args.dot {
                graph.to_dot(index)
            } else {
                graph.to_mermaid(index, args.limit)
            };
            println!("{text}");
        }
        Some(root) => {
            let start = parse_id(root)?;
            if index.get(&start).is_none() {
                bail!("{} is not defined in this workspace", start);
            }
            let seen = subgraph(&graph, &start, args.depth);
            println!("{}", mermaid_of(&graph, index, &seen, args.limit));
        }
    }
    Ok(())
}

/// Collects the ids within `depth` hops of `root`, in either direction.
///
/// Deliberately walks one hop at a time via the adjacency accessors: a
/// `Graph::reachable` call already returns the whole closure, which would make
/// `--depth` meaningless.
fn subgraph(graph: &Graph, root: &Id, depth: Option<usize>) -> std::collections::BTreeSet<Id> {
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(root.clone());
    let limit = depth.unwrap_or(usize::MAX);
    let mut frontier = vec![root.clone()];
    for _ in 0..limit {
        let mut next = Vec::new();
        for id in &frontier {
            for edge in graph.edges_from(id) {
                if seen.insert(edge.to.clone()) {
                    next.push(edge.to.clone());
                }
            }
            for edge in graph.edges_to(id) {
                if seen.insert(edge.from.clone()) {
                    next.push(edge.from.clone());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    seen
}

/// Renders the edges whose endpoints are both inside `keep`.
fn mermaid_of(
    graph: &Graph,
    index: &Index,
    keep: &std::collections::BTreeSet<Id>,
    limit: usize,
) -> String {
    let mut out = String::from("flowchart TD\n");
    for id in keep.iter().take(limit) {
        let title = index
            .get(id)
            .map(|a| a.title.clone())
            .unwrap_or_else(|| "<unresolved>".into());
        out.push_str(&format!("    {}[{}]\n", node_label(id), quote(&title)));
    }
    for e in graph.edges() {
        if keep.contains(&e.from) && keep.contains(&e.to) {
            out.push_str(&format!(
                "    {} -->|{}| {}\n",
                node_label(&e.from),
                e.relation.as_str(),
                node_label(&e.to)
            ));
        }
    }
    out
}

fn node_label(id: &Id) -> String {
    id.to_string().replace('-', "_")
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "'"))
}

fn cmd_gate(args: &GateArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let reports = match (&args.gate, args.through) {
        (Some(name), _) => vec![gates::evaluate(parse_gate(name)?, index)],
        (None, Some(through)) => gates::evaluate_up_to(index, through),
        (None, None) => gates::evaluate_up_to(index, 8),
    };

    let failed: Vec<&opendomain_core::gates::GateReport> =
        reports.iter().filter(|r| !r.passed()).collect();

    if cli.json {
        println!("{}", to_json(&reports)?);
    } else {
        for r in &reports {
            println!("{}", report::render_gate(r, style));
        }
        if failed.is_empty() {
            println!("{}", style_ok(style, "all requested gates passed"));
        } else {
            println!(
                "{}",
                format!("{} of {} gate(s) failed", failed.len(), reports.len())
            );
        }
    }

    if !failed.is_empty() {
        // `--strict` is redundant today: a blocking finding always fails, and an
        // advisory never does. It is accepted so scripts can be explicit.
        let _ = args.strict;
        bail!("quality gates reported blocking findings");
    }
    Ok(())
}

fn cmd_audit(index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let report = report::audit(index);
    if cli.json {
        println!("{}", to_json(&report)?);
    } else {
        println!("{}", report::render_audit(&report, style));
    }
    Ok(())
}

fn cmd_link(args: &LinkArgs, ws: &Workspace, index: &Index, style: Style) -> Result<()> {
    if args.check {
        let dangling = index.dangling();
        if dangling.is_empty() {
            println!("{}", style_ok(style, "every link resolves"));
            return Ok(());
        }
        let total: usize = dangling.values().map(|v| v.len()).sum();
        for (id, missing) in &dangling {
            for m in missing {
                println!(
                    "{}",
                    report::render_finding_line(
                        "link/dangling",
                        &format!("{} references {}, which does not exist", id, m),
                        style,
                    )
                );
            }
        }
        bail!("{} dangling reference(s)", total);
    }

    // Anything other than `--check` needs a complete id/relation/target triple.
    let (raw_id, relation, raw_target) = match (&args.id, &args.relation, &args.target) {
        (Some(i), Some(r), Some(t)) => (i.as_str(), r.as_str(), t.as_str()),
        _ => bail!(
            "`od link` needs either --check, or an id plus a relation plus a target.\n\
             examples:\n  \
               od link --check\n  \
               od link REQ-001 derived-from EVD-014"
        ),
    };

    let id = parse_id(raw_id)?;
    let target = parse_id(raw_target)?;
    let rel = opendomain_core::Relation::parse(relation)
        .ok_or_else(|| anyhow!("unknown relation '{}'", relation))?;

    if index.get(&id).is_none() {
        bail!("{} is not defined in this workspace", id);
    }
    if index.get(&target).is_none() {
        bail!(
            "{} is not defined in this workspace, so the link would dangle",
            target
        );
    }

    let file = index
        .get(&id)
        .map(|a| a.file.clone())
        .ok_or_else(|| anyhow!("{} has no file on disk", id))?;
    let path = ws.root.join(&file);
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;

    let key = rel.as_str();
    let line = format!("{} = [\"{}\"]", key, target);
    if let Some(added) = append_link(&mut text, &id, &line) {
        std::fs::write(&path, &text).with_context(|| format!("cannot write {}", path.display()))?;
        println!(
            "{}",
            style_ok(
                style,
                &format!("{}: added {} = [{}]", file.display(), key, target)
            )
        );
        if added {
            println!("  (created a `toml` block for this artifact)");
        }
    } else {
        println!(
            "{}",
            format!(
                "{}: {} already links to {} - nothing to do",
                file.display(),
                key,
                target
            )
        );
    }
    Ok(())
}

/// Adds `line` to the artifact's `toml` block, creating that block if needed.
///
/// Returns `true` when a new fenced block had to be created, which the caller
/// reports because inserting one into prose deserves a visible mention.
fn append_link(text: &mut String, id: &Id, line: &str) -> Option<bool> {
    let id_text = id.to_string();
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();

    // Find this artifact's heading. The parser accepts `## ID — Title` and a
    // bare `## ID`; accept either so a hand-written heading is not a hard error.
    let start = lines.iter().position(|l| {
        let t = l.trim();
        t == format!("## {}", id_text) || t.starts_with(&format!("## {} ", id_text))
    })?;

    // Find this artifact's fenced ```toml block, if it has one. Stop at the
    // next `##` heading so a block belonging to a later artifact is never
    // mistaken for this one's.
    let mut toml_open: Option<usize> = None;
    let mut in_fence = false;
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        if l.starts_with("## ") {
            break;
        }
        if l.trim_start().starts_with("```") {
            if in_fence {
                in_fence = false;
            } else if l.to_ascii_lowercase().contains("toml") && toml_open.is_none() {
                toml_open = Some(i);
            } else {
                in_fence = !l.to_ascii_lowercase().contains("toml");
            }
        }
    }

    let mut updated = lines.clone();
    match toml_open {
        Some(open) => {
            // Insert just before the closing fence.
            let mut close = open + 1;
            while close < updated.len() && !updated[close].trim_start().starts_with("```") {
                if updated[close].trim() == line {
                    return None; // already linked
                }
                close += 1;
            }
            if close >= updated.len() {
                return None; // unterminated fence: refuse to guess
            }
            updated.insert(close, line.to_string());
            *text = updated.join("\n");
            Some(false)
        }
        None => {
            // No metadata block yet. Create one directly under the heading.
            let at = start + 1;
            updated.insert(at, String::new());
            updated.insert(at + 1, "```toml".into());
            updated.insert(at + 2, line.to_string());
            updated.insert(at + 3, "```".into());
            *text = updated.join("\n");
            Some(true)
        }
    }
}

fn cmd_show(args: &ShowArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let id = parse_id(&args.id)?;
    let artifact = index
        .get(&id)
        .ok_or_else(|| anyhow!("{} is not defined in this workspace", id))?;
    if cli.json {
        println!("{}", to_json(artifact)?);
    } else {
        println!("{}", report::render_artifact(artifact, style));
    }
    Ok(())
}

fn cmd_advance(args: &AdvanceArgs, ws: &Workspace, index: &Index, style: Style) -> Result<()> {
    let id = parse_id(&args.id)?;
    let target_status: ArtifactStatus = args
        .status
        .parse()
        .map_err(|e: String| anyhow!("unknown status '{}': {}", args.status, e))?;

    let artifact = index
        .get(&id)
        .ok_or_else(|| anyhow!("{} is not defined in this workspace", id))?;

    if let Some(current) = artifact.status.rank() {
        if let Some(next) = target_status.rank() {
            if next < current {
                bail!(
                    "{} is {}; moving it back to {} loses recorded progress. Edit the file by hand \
                     if that is really what you want.",
                    id,
                    artifact.status.as_str(),
                    target_status.as_str()
                );
            }
        }
    }

    let file = artifact.file.clone();
    let path = ws.root.join(&file);
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;

    let old = format!("status = \"{}\"", artifact.status.as_str());
    let new = format!("status = \"{}\"", target_status.as_str());
    if !text.contains(&old) {
        bail!(
            "{}: expected a `{}` line for {}. Add it by hand, then re-run.",
            file.display(),
            old,
            id
        );
    }
    let occurrences = text.matches(&old).count();
    if occurrences > 1 {
        bail!(
            "{}: found {} occurrences of `{}`; refusing to guess which one is {}",
            file.display(),
            occurrences,
            old,
            id
        );
    }
    text = text.replacen(&old, &new, 1);
    std::fs::write(&path, &text).with_context(|| format!("cannot write {}", path.display()))?;
    println!(
        "{}",
        style_ok(
            style,
            &format!(
                "{}: {} -> {}",
                id,
                artifact.status.as_str(),
                target_status.as_str()
            )
        )
    );
    Ok(())
}

fn cmd_propose(args: &ProposeArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    if slug.is_empty() {
        bail!("a change needs a name");
    }
    let dir = ws.root.join("changes").join(&slug);
    if dir.exists() {
        bail!("change '{}' already exists at {}", slug, dir.display());
    }
    std::fs::create_dir_all(dir.join("specs"))
        .with_context(|| format!("cannot create {}", dir.display()))?;

    let summary = if args.summary.is_empty() {
        "TODO: describe the change in one sentence.".to_string()
    } else {
        args.summary.clone()
    };

    let proposal = format!(
        "# {slug}\n\n\
         > {summary}\n\n\
         ## Why\n\n\
         What is the problem, and what evidence shows it is real?\n\n\
         ## What changes\n\n\
         _Describe the delta against the current state._\n\n\
         ## Impact\n\n\
         _Which artifacts gain, lose, or change status._\n",
        slug = slug,
        summary = summary
    );
    write_new(&dir.join("proposal.md"), &proposal)?;

    let tasks = "# Tasks\n\n\
                  | ID | Task | Artifact | Status |\n\
                  |----|------|----------|--------|\n\n\
                  _Add one row per unit of work. Reference the artifact each task serves._\n";
    write_new(&dir.join("tasks.md"), tasks)?;

    let design = format!(
        "# {slug} design\n\n\
         ## Context\n\n\
         _The forces at play: constraints, prior decisions, related changes._\n\n\
         ## Decision\n\n\
         _What was decided._\n\n\
         ## Consequences\n\n\
         _What becomes easier, what becomes harder, what is now locked in._\n",
        slug = slug
    );
    write_new(&dir.join("design.md"), &design)?;

    let spec_readme = "# Specifications\n\n\
                      _One file per affected capability, holding the deltas rather than \
                      restating current behaviour._\n";
    write_new(&dir.join("specs").join("README.md"), spec_readme)?;

    let change = format!(
        "<!-- Opendomain change record. The CLI reads `status` to place this change. -->\n\
         status = \"PROPOSED\"\n\
         created = \"{}\"\n",
        today()
    );
    write_new(&dir.join("change.toml"), &change)?;

    println!(
        "{}",
        style_ok(style, &format!("proposed change '{}'", slug))
    );
    for f in [
        "proposal.md",
        "tasks.md",
        "design.md",
        "specs/README.md",
        "change.toml",
    ] {
        println!("  + changes/{slug}/{f}", slug = slug);
    }
    println!("\nnext: fill in proposal.md, then `od apply {}`", slug);
    Ok(())
}

fn cmd_apply(args: &ApplyArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    let dir = ws.root.join("changes").join(&slug);
    let change = dir.join("change.toml");
    if !change.is_file() {
        bail!(
            "no change '{}' at {}. Create it with `od propose {}`.",
            slug,
            dir.display(),
            slug
        );
    }
    let text = std::fs::read_to_string(&change)
        .with_context(|| format!("cannot read {}", change.display()))?;
    if !text.contains("status = \"APPLIED\"") {
        let updated = text.replacen("status = \"PROPOSED\"", "status = \"APPLIED\"", 1);
        std::fs::write(&change, updated)
            .with_context(|| format!("cannot write {}", change.display()))?;
    }
    println!(
        "{}",
        style_ok(style, &format!("change '{}' marked applied", slug))
    );
    println!("  now update the registries, then run `od gate specification`");
    Ok(())
}

fn cmd_archive(args: &ArchiveArgs, ws: &Workspace, style: Style) -> Result<()> {
    let slug = slugify(&args.slug);
    let from = ws.root.join("changes").join(&slug);
    if !from.is_dir() {
        bail!("no change directory at {}", from.display());
    }
    let to = ws.root.join("changes").join("archive").join(&slug);
    if to.exists() {
        bail!("{} already exists", to.display());
    }
    std::fs::create_dir_all(to.parent().expect("archive has a parent"))
        .with_context(|| format!("cannot create {}", to.parent().unwrap().display()))?;
    std::fs::rename(&from, &to)
        .with_context(|| format!("cannot move {} to {}", from.display(), to.display()))?;
    println!(
        "{}",
        style_ok(
            style,
            &format!("archived '{}' to changes/archive/{}", slug, slug)
        )
    );
    Ok(())
}

fn write_new(path: &Path, body: &str) -> Result<()> {
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    std::fs::write(path, body).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Tints a success message, respecting the global `--no-color` / `--json` flag.
fn style_ok(style: Style, msg: &str) -> String {
    style.paint(msg, opendomain_core::report::Colour::Green)
}

/// Serialises a value as pretty JSON, or fails with context.
fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value).context("cannot serialise output as JSON")
}
