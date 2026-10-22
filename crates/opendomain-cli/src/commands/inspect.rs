//! Les lectures de traçabilité : `trace` (la chaîne d'un artefact),
//! `impact` (ce que sa modification affecte), `graph` (rendu Mermaid/DOT avec
//! `--root`/`--depth`), `show` (la fiche complète d'un artefact).

use anyhow::{anyhow, bail, Result};
use opendomain_core::graph::{Direction, Graph};
use opendomain_core::ids::Id;
use opendomain_core::report::{self, Style};
use opendomain_core::workspace::Index;

use super::args::{Cli, GraphArgs, ImpactArgs, ShowArgs, TraceArgs};
use super::context::{parse_id, to_json};

pub fn cmd_trace(args: &TraceArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
    let id = parse_id(&args.id)?;
    if index.get(&id).is_none() {
        bail!("{} is not defined in this workspace", id);
    }
    let graph = Graph::build(index);
    // Provenance is a backward walk (nearest source first), like the default.
    let direction = if args.forward {
        Direction::Forward
    } else {
        Direction::Backward
    };
    let steps = if args.provenance {
        graph.provenance_chain(&id, index)
    } else {
        graph.reachable(&id, index, direction)
    };
    if steps.is_empty() {
        bail!("{} has no links in that direction", id);
    }
    if cli.json {
        println!("{}", to_json(&steps)?);
    } else {
        println!("{}", report::render_trace(&id, &steps, direction, style));
    }
    Ok(())
}

pub fn cmd_impact(args: &ImpactArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
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

pub fn cmd_graph(args: &GraphArgs, index: &Index) -> Result<()> {
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

pub fn cmd_show(args: &ShowArgs, index: &Index, style: Style, cli: &Cli) -> Result<()> {
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
