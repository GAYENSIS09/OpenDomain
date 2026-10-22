//! M4 — registres d'agents métier : `opendomain workflow new <kind> <slug>`
//! pré-remplit une spec avec les priors du domaine (migration, refactor,
//! tests, doc) — point de départ, pas doctrine : le planificateur reste maître
//! du DAG et le harnais des gates.

use anyhow::{Result, anyhow};

use crate::harness::common::SPEC_DIR;

/// Les registres de priors métier disponibles — un point de départ léger
/// par domaine.
const WORKFLOWS: &[(&str, &str)] = &[
    ("migration", SPEC_MIGRATION),
    ("refactor", SPEC_REFACTOR),
    ("tests", SPEC_TESTS),
    ("doc", SPEC_DOC),
];

/// `opendomain workflow new <kind> <slug>`.
pub fn run(kind: &str, slug: &str) -> Result<()> {
    let template = WORKFLOWS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, t)| *t)
        .ok_or_else(|| {
            anyhow!(
                "workflow inconnu : {kind} — disponibles : {}",
                WORKFLOWS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")
            )
        })?;

    let dir = std::path::Path::new(SPEC_DIR).join(slug);
    let spec_file = dir.join("spec.md");
    anyhow::ensure!(!slug.is_empty(), "slug requis : opendomain workflow new migration 001-x");
    if spec_file.exists() {
        anyhow::bail!("existe déjà : {}", spec_file.display());
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&spec_file, template)?;
    println!(
        "OK {kind} → {}\n  → puis : opendomain ground {slug} && opendomain plan {slug} && opendomain build {slug}\n  → gates et sessions : opendomain run {slug} (mock pour un dry-run)",
        spec_file.display()
    );
    Ok(())
}

// ---------------------------------------------------------------- gabarits --

const SPEC_MIGRATION: &str = r#"# Spec — migration de <API/module/source> vers <cible>

> Workflow `migration` : plan de portage vérifié. Le harnais exige un
> inventaire exhaustif AVANT toute écriture — pas de migration « au feeling ».

## Objectif
Porter <origin> vers <target> sans régression observable.

## Contraintes
- inventaire complet des points d'appel avant modification (grep, arbre)
- chaque unité migrée : équivalence de comportement prouvée par un test existant ou ajouté
- interdiction : points morts silencieux sans ticket de suivi
- pas de suppression de l'ancien code tant que la suite verte ne le prouve pas

## Grounding à collecter
- signatures publiques actuelles, usages internes/externes
- version cible : changelog, breaking changes
- stratégie de compat (shims ? feature flags ?)

## Exemples entrée → sortie
- avant : fonctions x() y() ; après : api x2() y2() + shim retiré

## Critères d'acceptance
- [ ] inventaire dans `grounding/` référencé par l'analyste
- [ ] `verificateur` émet PASS/FAIL motivé par critère
- [ ] tests verts (`cargo test` / équivalent)
"#;

const SPEC_REFACTOR: &str = r#"# Spec — refactor <cible> : <but mesurable>

> Workflow `refactor` : le comportement n'est PAS un livrable négociable ;
> seules les structures bougent, les gates restent stricts.

## Objectif
<forme visée : ex. réduire la dette, retirer les unwrap(), découpler modules>

## Contraintes
- zéro changement de comportement externe (tests existants = oracle)
- étapes en diamant : petites, nommées, réversibles
- interdiction : reformat simultané + refactor (diffs incompressibles)

## Grounding à collecter
- métrique de départ (taille, couplage, unwraps)
- règles de style du repo (lint, fmt)

## Critères d'acceptance
- [ ] métrique de départ vs métrique finale consignées par le vérificateur
- [ ] tests verts + lint propre
- [ ] commits alignés sur les étapes nommées
"#;

const SPEC_TESTS: &str = r#"# Spec — couverture de tests pour <module/cas>

> Workflow `tests` : les cascritiques deviennent des unités de test
> traçables ; le harnais n'accepte pas de test élastique.

## Objectif
Couvrir <comportements ciblés> avec <N> tests exécutables.

## Contraintes
- un test = un comportement documenté (pas de snapshot « parce que »)
- cas limites listés AVANT codage (liste vide, seuil 0, overflow…)
- interdiction : non-déterminisme (temps, réseau) hors sandbox

## Grounding à collecter
- surface publique réelle (API, CLI, IPC)
- corrélats de bugs historiques (issues, TODO)

## Critères d'acceptance
- [ ] chaque cas limite de l'analyste a SON test
- [ ] suite déterministe (aucun ordre implicite)
- [ ] verdict PASS/FAIL motivé du vérificateur
"#;

const SPEC_DOC: &str = r#"# Spec — documentation <produit/module>

> Workflow `doc` : la doc est du code contractuel — vérifiée comme lui,
> jamais "au mieux à la fin".

## Objectif
Documenter <cible> pour <audience> (readme, docstrings, exemples).

## Contraintes
- verbatim des signatures / comportements réels (pas du roman)
- un exemple exécutable par section majeure
- la langue est celle du repo (fr EN ? — trancher ICI)

## Grounding à collecter
- vraies signatures + doctests existants
- FAQ issues du tracker

## Critères d'acceptance
- [ ] chaque interface publique documentée ET illustrée
- [ ] les exemples exécutables passent effectivement en sandbox
- [ ] revue du vérificateur : PASS/FAIL motivé
"#;
