//! Commandes utilitaires de la CLI.

use anyhow::{anyhow, Result};

/// `opendomain validate` : contrôle statique complet d'un `graph.json`.
pub fn validate(path: &str) -> Result<()> {
    use opendomain_core::harness::{Dag, GraphFile};

    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("lecture de `{path}` impossible : {e}"))?;
    let file: GraphFile = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("`{path}` n'est pas un graph.json valide : {e}"))?;
    let dag = Dag::build(&file)
        .map_err(|e| anyhow!("`{path}` invalide : {e}\n  → corriger puis relancer `opendomain validate`"))?;

    let edges: usize = dag.nodes.values().map(|n| n.depends_on.len()).sum();
    println!("OK `{path}` : {} nœud(s), {edges} arête(s) — DAG acyclique", dag.nodes.len());
    let order = dag.topological_order()?;
    println!("  ordre topologique : {}", order.join(" -> "));
    println!("  boucle canonique  : {}", opendomain_core::harness::LOOP);
    for id in &order {
        let n = &dag.nodes[id];
        let deps = if n.depends_on.is_empty() {
            "-".to_string()
        } else {
            n.depends_on.join(", ")
        };
        let tools = if n.tools.is_empty() {
            "-".to_string()
        } else {
            n.tools.join(", ")
        };
        let contract = if n.contract.output_schema.is_null() {
            "ATTENTION: pas de schéma de sortie"
        } else {
            "contrat OK"
        };
        println!("  - {id} | outils [{tools}] | deps: {deps} | {contract}");
        println!("      role: {}", n.role);
    }
    Ok(())
}

