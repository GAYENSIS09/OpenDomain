//! Les commandes de registre de `opendomain` — tout ce qui n'est ni le harnais
//! ([`crate::harness`]) ni le sous-système de vérification
//! ([`crate::verification`]).
//!
//! Une commande = une fonction mince : découvrir le workspace, déléguer à
//! `opendomain_core`, rendre. Rien ici ne réécrit un registre, sauf
//! [`registry`] et [`changes`] — et ils n'écrivent qu'append-only.
//!
//! | module | commandes | responsabilité |
//! |---|---|---|
//! | [`args`] | (toutes) | la définition clap : commandes, arguments, conversions |
//! | [`context`] | — | répertoire projet, découverte, parsing, style, JSON |
//! | [`init`] | `opendomain init` | créer `opendomain/` + les contrats agent |
//! | [`reference`] | `opendomain reference` | les catalogues : gates, kinds, rules |
//! | [`status`] | `opendomain status`, `audit` | la vue d'ensemble et la prochaine étape |
//! | [`gates`] | `opendomain gate` | exécuter une gate (0-8) et rendre les findings |
//! | [`inspect`] | `trace`, `impact`, `graph`, `show` | lire la traçabilité |
//! | [`registry`] | `link`, `new`, `advance` | écrire dans un registre (append-only) |
//! | [`changes`] | `propose`, `apply`, `archive` | le cycle de change (layout OpenSpec) |

pub mod args;
pub mod changes;
pub mod context;
pub mod gates;
pub mod init;
pub mod inspect;
pub mod reference;
pub mod registry;
pub mod status;
