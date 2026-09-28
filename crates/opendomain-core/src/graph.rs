//! The traceability graph.
//!
//! Everything OpenDomain knows is stored as flat registries; the graph is
//! derived. It answers the only two questions that really matter:
//!
//! * `od trace` — *why does this exist?* walks upstream to sources.
//! * `od impact` — *what breaks if this changes?* walks downstream to tests.
//!
//! Both directions are computed on demand from the artifact links, so a stale
//! derived cache can never disagree with the registries it came from.

use crate::artifact::Relation;
use crate::ids::{Id, IdKind};
use crate::vocab::ImpactClass;
use crate::workspace::Index;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A directed edge in the traceability graph.
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    /// The artifact the edge leaves.
    pub from: Id,
    /// The relation that connects the two.
    pub relation: Relation,
    /// The artifact the edge enters.
    pub to: Id,
    /// `false` when the target does not exist in the workspace.
    pub resolved: bool,
}

/// One step of a traced path.
#[derive(Debug, Clone, Serialize)]
pub struct TraceStep {
    /// The artifact reached.
    pub id: Id,
    /// Title of the artifact, or the empty string when unresolved.
    pub title: String,
    /// How this step was reached from the previous one.
    pub relation: Relation,
    /// Hop count from the starting artifact.
    pub depth: usize,
}

/// The whole workspace as a graph.
#[derive(Debug, Default, Clone)]
pub struct Graph {
    /// Outgoing edges, keyed by source.
    outgoing: BTreeMap<Id, Vec<Edge>>,
    /// Incoming edges, keyed by target.
    incoming: BTreeMap<Id, Vec<Edge>>,
    /// Every identifier known to the workspace, resolved or not.
    nodes: BTreeSet<Id>,
}

impl Graph {
    /// Builds a graph from a loaded workspace index.
    pub fn build(index: &Index) -> Graph {
        let mut outgoing: BTreeMap<Id, Vec<Edge>> = BTreeMap::new();
        let mut incoming: BTreeMap<Id, Vec<Edge>> = BTreeMap::new();
        let mut nodes: BTreeSet<Id> = index.by_id.keys().cloned().collect();

        for artifact in index.by_id.values() {
            for (relation, target) in artifact.edges() {
                nodes.insert(target.clone());
                let edge = Edge {
                    from: artifact.id.clone(),
                    relation,
                    to: target.clone(),
                    resolved: index.by_id.contains_key(&target),
                };
                outgoing
                    .entry(artifact.id.clone())
                    .or_default()
                    .push(edge.clone());
                incoming.entry(target).or_default().push(edge);
            }
        }

        Graph {
            outgoing,
            incoming,
            nodes,
        }
    }

    /// Number of artifacts in the workspace.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the workspace, resolved and unresolved.
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }

    /// `true` when the identifier appears anywhere in the workspace.
    pub fn contains(&self, id: &Id) -> bool {
        self.nodes.contains(id)
    }

    /// Edges leaving `id`.
    pub fn edges_from(&self, id: &Id) -> &[Edge] {
        self.outgoing.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Edges entering `id`.
    pub fn edges_to(&self, id: &Id) -> &[Edge] {
        self.incoming.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Every edge in the graph, in a stable order.
    ///
    /// Useful for rendering a whole workspace without reaching into the
    /// adjacency maps, which stay private so the traversal invariants hold.
    pub fn edges(&self) -> Vec<&Edge> {
        self.outgoing.values().flatten().collect()
    }

    /// Traces upstream: from an artifact to the evidence and sources behind it.
    pub fn trace_backward(&self, start: &Id, index: &Index) -> Vec<TraceStep> {
        self.reachable(start, index, Direction::Backward)
    }

    /// Traces downstream: from an artifact to what implements and verifies it.
    pub fn trace_forward(&self, start: &Id, index: &Index) -> Vec<TraceStep> {
        self.reachable(start, index, Direction::Forward)
    }

    /// Artifacts that depend on `id`, by looking at its **incoming** edges.
    ///
    /// An artifact is downstream when it declares a link *to* `id`, because the
    /// declaration is written by the dependent. `SPEC-001 derived-from REQ-001`
    /// puts `SPEC-001` on the incoming edge list of `REQ-001`, which is exactly
    /// what "if `REQ-001` changes, `SPEC-001` is affected" means.
    pub fn dependents(&self, id: &Id) -> Vec<Id> {
        self.edges_to(id).iter().map(|e| e.from.clone()).collect()
    }

    /// Every artifact reachable from `start` in one direction, breadth-first.
    ///
    /// The two directions are dual, and the duality is the whole point:
    ///
    /// * **Backward** follows *outgoing* edges of an upstream relation, because
    ///   `REQ-004 derived-from RULE-001` is written with the dependent first.
    /// * **Forward** follows *incoming* edges of any relation, because anything
    ///   that names this artifact is, by that naming, downstream of it.
    pub fn reachable(&self, start: &Id, index: &Index, dir: Direction) -> Vec<TraceStep> {
        let mut steps: Vec<TraceStep> = Vec::new();
        let mut seen: BTreeSet<Id> = BTreeSet::new();
        seen.insert(start.clone());
        // Each queue entry carries the relation actually crossed to reach the
        // node, so a step reports the edge it came from rather than a constant.
        let mut queue: VecDeque<(Id, Relation, usize)> = VecDeque::new();
        queue.push_back((start.clone(), Relation::DerivedFrom, 0));

        while let Some((current, via, depth)) = queue.pop_front() {
            steps.push(self.titled_step(&current, via, depth, index));
            for (edge, relation) in self.step_edges(&current, dir) {
                if seen.contains(&edge) {
                    continue;
                }
                seen.insert(edge.clone());
                queue.push_back((edge, relation, depth + 1));
            }
        }
        steps
    }

    /// The adjacent artifacts in one direction, paired with the relation used to
    /// reach them, following the convention documented on [`Graph::reachable`].
    fn step_edges(&self, current: &Id, dir: Direction) -> Vec<(Id, Relation)> {
        match dir {
            Direction::Backward => self
                .edges_from(current)
                .iter()
                .filter(|e| e.relation.is_upstream())
                .map(|e| (e.to.clone(), e.relation))
                .collect(),
            Direction::Forward => self
                .edges_to(current)
                .iter()
                .map(|e| (e.from.clone(), e.relation))
                .collect(),
        }
    }

    fn titled_step(&self, id: &Id, relation: Relation, depth: usize, index: &Index) -> TraceStep {
        TraceStep {
            id: id.clone(),
            title: index
                .get(id)
                .map(|a| a.title.clone())
                .unwrap_or_else(|| "<unresolved>".into()),
            relation,
            depth,
        }
    }

    /// The upstream chain from `start` to the nearest source.
    ///
    /// This is the literal answer to *why does this exist?*: the shortest
    /// upstream walk that terminates at a `SRC` artifact. Falls back to the
    /// full backward closure when no source is reachable, so the caller always
    /// gets something actionable.
    pub fn provenance_chain(&self, start: &Id, index: &Index) -> Vec<TraceStep> {
        if let Some(chain) = self.shortest_upstream_to_source(start, index) {
            return chain;
        }
        self.reachable(start, index, Direction::Backward)
            .into_iter()
            .map(|mut s| {
                s.title = index
                    .get(&s.id)
                    .map(|a| a.title.clone())
                    .unwrap_or_else(|| "<unresolved>".into());
                s
            })
            .collect()
    }

    /// Breadth-first search upstream for the closest `SRC` node.
    fn shortest_upstream_to_source(&self, start: &Id, index: &Index) -> Option<Vec<TraceStep>> {
        if start.kind() == IdKind::Src {
            return Some(vec![self.titled_step(
                start,
                Relation::DerivedFrom,
                0,
                index,
            )]);
        }
        let mut seen: BTreeSet<Id> = BTreeSet::new();
        seen.insert(start.clone());
        // Breadth-first with the chain carried along, so a source reached at
        // depth 3 always wins over one reached at depth 5.
        let mut queue: VecDeque<Vec<Id>> = VecDeque::new();
        queue.push_back(vec![start.clone()]);

        while let Some(path) = queue.pop_front() {
            let current = path.last().expect("path is never empty");
            for edge in self.edges_from(current) {
                if !edge.relation.is_upstream() || seen.contains(&edge.to) {
                    continue;
                }
                seen.insert(edge.to.clone());
                let mut extended = path.clone();
                extended.push(edge.to.clone());
                if edge.to.kind() == IdKind::Src {
                    return Some(
                        extended
                            .iter()
                            .enumerate()
                            .map(|(i, id)| self.titled_step(id, Relation::DerivedFrom, i, index))
                            .collect(),
                    );
                }
                queue.push_back(extended);
            }
        }
        None
    }

    /// The blast radius of changing `start`, split by how direct it is.
    pub fn impact(&self, start: &Id, index: &Index) -> ImpactReport {
        let mut direct = BTreeSet::new();
        let mut indirect = BTreeSet::new();

        let mut queue: VecDeque<(Id, usize)> = VecDeque::new();
        queue.push_back((start.clone(), 0));
        let mut seen: BTreeSet<Id> = BTreeSet::new();
        seen.insert(start.clone());

        while let Some((current, depth)) = queue.pop_front() {
            for (next, _) in self.step_edges(&current, Direction::Forward) {
                if seen.contains(&next) {
                    continue;
                }
                seen.insert(next.clone());
                if depth == 0 {
                    direct.insert(next.clone());
                } else {
                    indirect.insert(next.clone());
                }
                queue.push_back((next, depth + 1));
            }
        }

        let mut by_artifact = Vec::new();
        for id in direct.iter().chain(indirect.iter()) {
            if let Some(a) = index.get(id) {
                by_artifact.push(ImpactEntry {
                    id: id.clone(),
                    title: a.title.clone(),
                    kind: a.id.kind(),
                    class: if direct.contains(id) {
                        ImpactClass::Direct
                    } else {
                        ImpactClass::Indirect
                    },
                    status: a.status,
                    file: a.file.clone(),
                });
            } else {
                by_artifact.push(ImpactEntry {
                    id: id.clone(),
                    title: "<unresolved>".into(),
                    kind: id.kind(),
                    class: if direct.contains(id) {
                        ImpactClass::Direct
                    } else {
                        ImpactClass::Indirect
                    },
                    status: crate::vocab::ArtifactStatus::Open,
                    file: Default::default(),
                });
            }
        }

        ImpactReport {
            origin: start.clone(),
            direct: direct.len(),
            indirect: indirect.len(),
            entries: by_artifact,
        }
    }

    /// Renders the graph in Mermaid flowchart syntax.
    pub fn to_mermaid(&self, index: &Index, limit: usize) -> String {
        let mut out = String::from("flowchart LR\n");
        for (i, id) in self.nodes.iter().enumerate() {
            if i >= limit {
                out.push_str("  %% truncated\n");
                break;
            }
            let label = index
                .get(id)
                .map(|a| a.title.clone())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "unresolved".into());
            out.push_str(&format!("  {}[\"{}: {}\"]\n", node(id), id, escape(&label)));
        }
        for edges in self.outgoing.values() {
            for e in edges {
                if e.relation.is_downstream() || e.relation.is_upstream() {
                    out.push_str(&format!(
                        "  {} -->|{}| {}\n",
                        node(&e.from),
                        e.relation.as_str(),
                        node(&e.to)
                    ));
                }
            }
        }
        out
    }

    /// Renders the graph in Graphviz DOT syntax.
    pub fn to_dot(&self, index: &Index) -> String {
        let mut out =
            String::from("digraph opendomain {\n  rankdir=LR;\n  node [shape=box, fontsize=10];\n");
        for id in &self.nodes {
            let label = index
                .get(id)
                .map(|a| a.title.clone())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "unresolved".into());
            out.push_str(&format!(
                "  \"{}\" [label=\"{}\\n{}\"];\n",
                id,
                id,
                escape(&label)
            ));
        }
        for edges in self.outgoing.values() {
            for e in edges {
                out.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                    e.from,
                    e.to,
                    e.relation.as_str()
                ));
            }
        }
        out.push_str("}\n");
        out
    }
}

fn node(id: &Id) -> String {
    format!("n_{}", id.to_string().replace('-', "_"))
}

fn escape(text: &str) -> String {
    text.replace('"', "'").replace(['\n', '\r'], " ")
}

/// Which way to walk the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Toward the reason an artifact exists.
    Backward,
    /// Toward the evidence that an artifact is correct.
    Forward,
}

/// One artifact in an impact report.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactEntry {
    /// The affected artifact.
    pub id: Id,
    /// Its title.
    pub title: String,
    /// Its kind.
    pub kind: crate::ids::IdKind,
    /// How directly it is affected.
    pub class: ImpactClass,
    /// Its lifecycle status.
    pub status: crate::vocab::ArtifactStatus,
    /// The file that defines it.
    pub file: std::path::PathBuf,
}

/// The result of `od impact`.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    /// The artifact the analysis started from.
    pub origin: Id,
    /// Number of directly affected artifacts.
    pub direct: usize,
    /// Number of transitively affected artifacts.
    pub indirect: usize,
    /// Every affected artifact, direct first.
    pub entries: Vec<ImpactEntry>,
}

impl ImpactReport {
    /// `true` when nothing downstream depends on the artifact.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::Relation;
    use crate::ids::Id;
    use crate::parse;
    use crate::workspace::Index;
    use std::path::PathBuf;

    /// Builds an index from `## ID` headings, wiring `derived-from` edges from a
    /// compact edge list, so graph tests do not depend on the Markdown parser.
    fn build_fixture(specs: &[(&str, &[(&str, &str)])]) -> (Index, Graph) {
        let mut text = String::new();
        for (id, edges) in specs {
            text.push_str(&format!("## {id} — {id}\n\n```toml\n"));
            // Group by relation: TOML rejects a repeated key, so two edges under
            // the same relation must become one array.
            let mut by_relation: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for (rel, target) in *edges {
                by_relation.entry(rel).or_default().push(target);
            }
            for (rel, targets) in by_relation {
                let list = targets
                    .iter()
                    .map(|t| format!("\"{t}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                text.push_str(&format!("{rel} = [{list}]\n"));
            }
            text.push_str("```\n\nbody\n\n");
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
        let graph = Graph::build(&index);
        (index, graph)
    }

    fn chain() -> (Index, Graph) {
        build_fixture(&[
            ("SRC-001", &[]),
            ("EVD-001", &[("derived-from", "SRC-001")]),
            ("RULE-001", &[("derived-from", "EVD-001")]),
            ("REQ-001", &[("derived-from", "RULE-001")]),
            ("SPEC-001", &[("derived-from", "REQ-001")]),
            (
                "TEST-001",
                &[("verifies", "SPEC-001"), ("verifies", "REQ-001")],
            ),
        ])
    }

    #[test]
    fn backward_trace_reaches_the_source() {
        let (index, graph) = chain();
        let steps = graph.trace_backward(&Id::parse("REQ-001").unwrap(), &index);
        let ids: Vec<String> = steps.iter().map(|s| s.id.to_string()).collect();
        assert_eq!(ids[0], "REQ-001");
        assert!(ids.contains(&"SRC-001".to_string()), "{ids:?}");
    }

    #[test]
    fn forward_trace_reaches_the_tests() {
        let (index, graph) = chain();
        let steps = graph.trace_forward(&Id::parse("REQ-001").unwrap(), &index);
        let ids: Vec<String> = steps.iter().map(|s| s.id.to_string()).collect();
        assert!(ids.contains(&"TEST-001".to_string()), "{ids:?}");
    }

    #[test]
    fn each_step_reports_the_relation_it_crossed() {
        // Two different upstream relations in one walk: a step that always
        // reported `DerivedFrom` would look right on a pure chain and be wrong
        // here, where the second hop is `depends-on`.
        let (index, graph) = build_fixture(&[
            ("RULE-001", &[]),
            ("EVD-001", &[]),
            (
                "REQ-001",
                &[("derived-from", "EVD-001"), ("depends-on", "RULE-001")],
            ),
        ]);
        let steps = graph.trace_backward(&Id::parse("REQ-001").unwrap(), &index);
        let relation_of = |id: &str| {
            steps
                .iter()
                .find(|s| s.id.to_string() == id)
                .map(|s| s.relation)
        };
        assert_eq!(
            relation_of("EVD-001"),
            Some(Relation::DerivedFrom),
            "{steps:?}"
        );
        assert_eq!(
            relation_of("RULE-001"),
            Some(Relation::DependsOn),
            "{steps:?}"
        );
    }

    #[test]
    fn trace_terminates_on_cycles() {
        let (_index, graph) = build_fixture(&[
            ("RULE-001", &[("derived-from", "RULE-002")]),
            ("RULE-002", &[("derived-from", "RULE-001")]),
        ]);
        // A cyclic registry must not hang the CLI.
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn impact_separates_direct_from_indirect() {
        let (index, graph) = chain();
        let report = graph.impact(&Id::parse("RULE-001").unwrap(), &index);
        assert_eq!(report.origin.to_string(), "RULE-001");
        let direct: Vec<String> = report
            .entries
            .iter()
            .filter(|e| e.class == ImpactClass::Direct)
            .map(|e| e.id.to_string())
            .collect();
        assert_eq!(direct, vec!["REQ-001".to_string()]);
        let indirect: Vec<String> = report
            .entries
            .iter()
            .filter(|e| e.class == ImpactClass::Indirect)
            .map(|e| e.id.to_string())
            .collect();
        assert!(indirect.contains(&"TEST-001".to_string()), "{indirect:?}");
        assert!(!direct.contains(&"TEST-001".to_string()));
    }

    #[test]
    fn impact_of_a_leaf_is_empty() {
        let (index, graph) = chain();
        let report = graph.impact(&Id::parse("TEST-001").unwrap(), &index);
        assert!(report.is_empty());
    }

    #[test]
    fn mermaid_and_dot_render() {
        let (index, graph) = chain();
        let mermaid = graph.to_mermaid(&index, 100);
        assert!(mermaid.starts_with("flowchart LR"));
        assert!(mermaid.contains("REQ-001"));
        let dot = graph.to_dot(&index);
        assert!(dot.starts_with("digraph opendomain"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn graph_tracks_unresolved_targets() {
        let (_index, graph) = build_fixture(&[("REQ-001", &[("derived-from", "EVD-404")])]);
        let edge = &graph.edges_from(&Id::parse("REQ-001").unwrap())[0];
        assert!(!edge.resolved);
        assert!(graph.contains(&Id::parse("EVD-404").unwrap()));
    }
}
