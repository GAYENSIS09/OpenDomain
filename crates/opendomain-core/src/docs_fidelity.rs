//! Fidélité de la documentation au CLI réel — la loi de la doc est le code.
//!
//! La dérive la plus triviale entre un outil et sa documentation est le
//! drapeau mort : le CLI évolue, un article qui renomme `--llm` en
//! `--offline` ou retire `--executor` laisse derrière lui des pages qui
//! « documentent » des commandes qui n'existent plus. Un agent qui lit cette
//! page l'exécute et trouve une erreur typée — la doc a menti
//! autoritativement. Ce module épingle les surfaces documentaires au
//! vocabulaire réel, en compilation (`include_str!`) :
//!
//! 1. `--executor` n'existe plus nulle part — zéro occurrence tolérée ;
//! 2. `--llm` n'est légal que derrière `repatch` (le méta-agent de re-plan) —
//!    toute autre occurrence est une dérive.
//!
//! Quand une surface légitime doit nommer le passé (historique des
//! milestones), la formulation « the executor is opencode » le fait sans le
//! drapeau mort.

const README: &str = include_str!("../../../readme.md");
const SITE_INDEX: &str = include_str!("../../../docs-site/index.html");
const SITE_README: &str = include_str!("../../../docs-site/README.md");
const DOCS_HARNESS: &str = include_str!("../../../docs-site/docs/harness.html");
const DOCS_GETTING_STARTED: &str = include_str!("../../../docs-site/docs/getting-started.html");
const DOCS_BROWNFIELD: &str = include_str!("../../../docs-site/docs/brownfield.html");
const DOCS_CLI_REFERENCE: &str = include_str!("../../../docs-site/docs/cli-reference.html");
const DOCS_GATES: &str = include_str!("../../../docs-site/docs/gates.html");
const DOCS_CONCEPTS: &str = include_str!("../../../docs-site/docs/concepts.html");
const DOCS_ARTIFACTS: &str = include_str!("../../../docs-site/docs/artifacts.html");
const DOCS_OPENCODE: &str = include_str!("../../../docs-site/docs/opencode.html");
const DOCS_THEORY: &str = include_str!("../../../docs-site/docs/theory.html");
const SITE_JS: &str = include_str!("../../../docs-site/js/main.js");
const TPL_AGENTS: &str = include_str!("../templates/agent/AGENTS.md");
const TPL_SKILL_HARNESS: &str =
    include_str!("../templates/agent/.opencode/skills/opendomain-harness/SKILL.md");
const TPL_HARNAIS: &str = include_str!("../templates/agent/.opencode/agent/opendomain-harnais.md");
const TPL_SPEC_SAMPLE: &str = include_str!("../templates/harness-spec-sample.md");

/// Les surfaces documentaires épinglées (nom lisible → contenu).
pub const SURFACES: &[(&str, &str)] = &[
    ("readme.md", README),
    ("docs-site/index.html", SITE_INDEX),
    ("docs-site/README.md", SITE_README),
    ("docs-site/docs/harness.html", DOCS_HARNESS),
    ("docs-site/docs/getting-started.html", DOCS_GETTING_STARTED),
    ("docs-site/docs/brownfield.html", DOCS_BROWNFIELD),
    ("docs-site/docs/cli-reference.html", DOCS_CLI_REFERENCE),
    ("docs-site/docs/gates.html", DOCS_GATES),
    ("docs-site/docs/concepts.html", DOCS_CONCEPTS),
    ("docs-site/docs/artifacts.html", DOCS_ARTIFACTS),
    ("docs-site/docs/opencode.html", DOCS_OPENCODE),
    ("docs-site/docs/theory.html", DOCS_THEORY),
    ("docs-site/js/main.js", SITE_JS),
    ("templates/agent/AGENTS.md", TPL_AGENTS),
    (
        "templates/agent/.opencode/skills/opendomain-harness/SKILL.md",
        TPL_SKILL_HARNESS,
    ),
    (
        "templates/agent/.opencode/agent/opendomain-harnais.md",
        TPL_HARNAIS,
    ),
    ("templates/harness-spec-sample.md", TPL_SPEC_SAMPLE),
];

/// Les occurrences de `needle` qui **n'admettent pas** `allowed` sur leur ligne.
#[cfg(test)]
fn violations(content: &str, needle: &str, allowed: Option<&str>) -> Vec<String> {
    content
        .lines()
        .filter(|l| l.contains(needle))
        .filter(|l| !allowed.map(|a| l.contains(a)).unwrap_or(false))
        .map(|l| l.trim().chars().take(120).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_drapeau_mort_executor_ne_figurait_plus_nulle_part() {
        for (name, content) in SURFACES {
            let v = violations(content, "--executor", None);
            assert!(
                v.is_empty(),
                "`--executor` est un drapeau mort (le run n'a qu'un exécuteur : \
opencode, le mock est l'auto-vérification interne du bench) — {name} le mentionne \
encore sur {} ligne(s) : {v:?}",
                v.len()
            );
        }
    }

    #[test]
    fn le_drapeau_llm_ne_vit_que_derriere_repatch() {
        for (name, content) in SURFACES {
            let v = violations(content, "--llm", Some("repatch"));
            assert!(
                v.is_empty(),
                "`--llm` n'est légal que derrière `repatch` (le plan passe par opencode \
par défaut, `--offline` est le repli demandé explicitement) — {name} : {v:?}"
            );
        }
    }

    #[test]
    fn les_surfaces_epinglées_existent_toutes() {
        // un include_str sur un chemin faux ne compile pas : ce test épingle
        // aussi la liste elle-même (une surface retirée du dépôt casse la
        // compilation — c'est voulu).
        assert!(SURFACES.len() >= 17);
        assert!(README.contains("Executing under the harness"));
        assert!(DOCS_HARNESS.contains("lifecycle"));
        assert!(DOCS_THEORY.contains("gates"));
        assert!(TPL_AGENTS.contains("opendomain kit add"));
    }
}
