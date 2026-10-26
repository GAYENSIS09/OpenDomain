//! `opendomain-cli` ne vend volontairement pas de bibliothèque : tout ce qui
//! est réutilisable — ids, artefacts, graphe de traçabilité, gates,
//! scaffolding, sous-système de vérification (§9/§10) — vit dans
//! [`opendomain_core`](https://docs.rs/opendomain-core), l'unique dépendance
//! interne de ce crate.
//!
//! Cette cible `lib` minimale existe pour que docs.rs puisse construire et
//! héberger ces pages (`rustdoc --lib` ne trouve de cible que là) ; la
//! commande `opendomain` elle-même documente son usage dans le readme du
//! dépôt et `--help`.
