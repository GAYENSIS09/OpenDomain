//! Scaffold generation and the template set.
//!
//! Templates are embedded in the binary so `opendomain init` works from a single
//! executable with no network access and no template checkout. The same
//! templates are exposed through [`templates::all`] so the docs site and the
//! tests can assert on them.

use crate::error::{OdError, Result};
use crate::model::ids::IdKind;
use crate::model::vocab::{AuthorityModel, Profile};
use crate::registries::config::Config;
use std::path::Path;

pub mod agent_contract;
pub mod templates;

/// Marker files or directories whose presence suggests the project already
/// has an implementation, so `--repository greenfield` is probably wrong.
///
/// Checked by name only, never by content: `opendomain init` must stay fast and
/// must not parse user code to decide what the project is.
const BROWNFIELD_MARKERS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "pom.xml",
    "build.gradle",
    "docker-compose.yml",
    "docker-compose.yaml",
    "prisma/schema.prisma",
    "services",
    "src",
    "docs",
];

/// Names of brownfield markers found under `root`.
///
/// Empty means no implementation signal was detected. Non-empty does not
/// prove the project is brownfield — it only means `opendomain init --repository
/// greenfield` deserves a warning pointing at `--repository brownfield`.
/// The caller decides; this function only reports what exists.
pub fn brownfield_signals(root: &Path) -> Vec<String> {
    BROWNFIELD_MARKERS
        .iter()
        .filter(|m| root.join(m).exists())
        .map(|m| m.to_string())
        .collect()
}

/// Strips `<!-- ... -->` sections, mirroring the parser: template examples
/// live inside comments and must not read as declared artifacts. An
/// unterminated comment discards the tail, exactly as the parser does.
///
/// Used by tests to distinguish a live `## DOM-` heading from the template's
/// commented example.
#[allow(dead_code)]
fn strip_html_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `true` when `text` declares a live (non-comment) heading with `prefix`,
/// e.g. an actual `## DOM-001` rather than the template's commented example.
#[allow(dead_code)]
fn has_live_heading(text: &str, prefix: &str) -> bool {
    strip_html_comments(text)
        .lines()
        .any(|l| l.trim_start().starts_with(prefix))
}

/// One file in the scaffold.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    /// Path relative to the `opendomain/` directory.
    pub path: &'static str,
    /// The literal file contents.
    pub contents: &'static str,
}

/// What `opendomain init` wrote.
///
/// `opendomain init` installs only: registries + `opendomain.toml` + agent contract.
/// Context is completed afterwards by `/opendomain-project` (context-completer agent),
/// never at init time.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct InitReport {
    /// Files created.
    pub created: Vec<String>,
    /// Files that already existed and were left alone.
    pub skipped: Vec<String>,
    /// Agent instruction files created outside `opendomain/`.
    pub agent_files: Vec<String>,
}

impl InitReport {
    /// `true` when nothing was written because the workspace already existed.
    pub fn is_noop(&self) -> bool {
        self.created.is_empty() && self.agent_files.is_empty()
    }
}

/// Writes the scaffold into `opendomain_dir`.
///
/// Existing files are never overwritten: a project that has spent a week
/// curating `domain/glossary.md` must not lose it to a re-run of `opendomain init`.
pub fn scaffold(opendomain_dir: &Path, config: &Config, force: bool) -> Result<InitReport> {
    let mut report = InitReport::default();
    std::fs::create_dir_all(opendomain_dir).map_err(|e| OdError::write(opendomain_dir, e))?;

    let rendered = render_config(config);
    let config_path = opendomain_dir.join("opendomain.toml");
    if force || !config_path.exists() {
        write(&config_path, &rendered)?;
        report.created.push("opendomain.toml".into());
    } else {
        report.skipped.push("opendomain.toml".into());
    }

    for template in templates::for_profile(config.profile) {
        let target = opendomain_dir.join(template.path);
        if target.exists() && !force {
            report.skipped.push(template.path.to_string());
            continue;
        }
        write(&target, template.contents)?;
        report.created.push(template.path.to_string());
    }

    // No context recording here by design: `opendomain init` installs the workspace
    // and the agent contract, nothing more. `/opendomain-project` (context-completer)
    // fills `project.md`, `[project]`, `DOM-001`/`STK-001` from the repository,
    // external research, then user answers.
    write_agent_contract(opendomain_dir, force, &mut report)?;

    Ok(report)
}

/// Writes the agent instruction files, which live at the project root rather
/// than inside `opendomain/`.
///
/// The root is taken as the parent of the workspace directory, which is where
/// `opendomain init` puts it. If the workspace directory has no parent — a bare
/// relative name — the files are written relative to the current directory
/// instead, and this is reported rather than guessed at, because an agent
/// contract written to the wrong place is worse than one not written.
///
/// `AGENTS.md` is treated differently from the rest: it is skipped when it
/// already exists, even under `--force`. The namespaced files cannot collide
/// with anything the project owns; a root-level `AGENTS.md` can, and
/// overwriting instructions this tool did not write is not a decision `opendomain init`
/// should make on the project's behalf. `--force` overwrites the `.opencode/`
/// files, which the tool owns outright.
fn write_agent_contract(opendomain_dir: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    let root = opendomain_dir.parent().unwrap_or_else(|| Path::new("."));

    for template in agent_contract::all() {
        let target = root.join(template.path);
        let exists = target.exists();

        // An existing AGENTS.md belongs to the project. Skipping it is the safe
        // reading of "never overwrite", and the report says so, so a user who
        // expected the framework instructions there learns they were not written
        // rather than assuming they were.
        if exists && (agent_contract::is_root_level(&template) || !force) {
            report.skipped.push(template.path.to_string());
            continue;
        }

        write(&target, template.contents)?;
        report.agent_files.push(template.path.to_string());
    }

    Ok(())
}

/// Renders `opendomain.toml`, header comments included.
///
/// The body comes from the serializer rather than from string formatting, so
/// the config file the CLI writes is guaranteed to be the file the parser
/// accepts.
pub fn render_config(config: &Config) -> String {
    let mut text = String::new();
    text.push_str("# OpenDomain workspace configuration.\n");
    text.push_str("# Every key is optional; the values below are the defaults.\n");
    text.push_str("# Docs: https://opendomain.dev/reference/configuration\n\n");
    text.push_str(&config.render());
    text
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| OdError::write(parent, e))?;
    }
    std::fs::write(path, contents).map_err(|e| OdError::write(path, e))
}

/// The default configuration for a new project, given a name and a choice of
/// profile.
pub fn default_config(name: &str, profile: Profile, authority: AuthorityModel) -> Config {
    Config {
        name: name.to_string(),
        profile,
        authority,
        ..Config::default()
    }
}

/// The registry file a new artifact of `kind` should be appended to.
///
/// Returns the path relative to the `opendomain/` directory.
pub fn registry_path_for(kind: IdKind) -> &'static str {
    kind.default_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("opendomain-scaffold-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn full_profile_writes_every_template() {
        let dir = scratch("full");
        let opendomain = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Full, AuthorityModel::SpecAnchored);
        let report = scaffold(&opendomain, &cfg, false).unwrap();
        assert!(!report.created.is_empty());
        for t in templates::all() {
            assert!(
                opendomain.join(t.path).exists(),
                "{} was not written",
                t.path
            );
        }
    }

    #[test]
    fn core_profile_omits_opportunity_registries() {
        let dir = scratch("core");
        let opendomain = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        scaffold(&opendomain, &cfg, false).unwrap();
        assert!(opendomain.join("domain/glossary.md").exists());
        assert!(!opendomain.join("opportunity/concepts.md").exists());
    }

    #[test]
    fn lean_profile_omits_domain_registries() {
        let set = templates::for_profile(Profile::Lean);
        assert!(!set.iter().any(|t| t.path == "domain/glossary.md"));
        assert!(set.iter().any(|t| t.path == "specs/README.md"));
    }

    #[test]
    fn les_arbres_generes_portent_leur_gitignore() {
        let dir = scratch("gitignore-arbres");
        let opendomain = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        let report = scaffold(&opendomain, &cfg, false).unwrap();
        // Les deux arbres générés portent leur propre .gitignore — écrits
        // comme les autres fichiers, sans drapeau ; le .gitignore de la
        // racine du projet n'appartient pas au scaffold.
        assert!(report.created.contains(&".gitignore".to_string()));
        assert!(report
            .agent_files
            .iter()
            .any(|p| p == ".opencode/.gitignore"));
        for tree in [
            opendomain.join(".gitignore"),
            dir.join(".opencode").join(".gitignore"),
        ] {
            let text = fs::read_to_string(&tree).unwrap();
            assert!(
                text.contains('*') && text.contains("!.gitignore"),
                "{tree:?} n'ignore pas l'arbre : {text}"
            );
            assert!(
                text.contains("F14/L9") || text.contains("régénérable"),
                "{tree:?} porte la doctrine : {text}"
            );
        }
        assert!(
            !dir.join(".gitignore").exists(),
            "le .gitignore racine n'appartient pas au scaffold"
        );
        // Idempotent : un re-run ne les réécrit pas.
        let report2 = scaffold(&opendomain, &cfg, false).unwrap();
        assert!(report2.skipped.contains(&".gitignore".to_string()));
        assert!(report2
            .agent_files
            .iter()
            .all(|p| p != ".opencode/.gitignore"));
    }

    #[test]
    fn never_overwrites_without_force() {
        let dir = scratch("no-overwrite");
        let opendomain = dir.join("opendomain");
        let cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        scaffold(&opendomain, &cfg, false).unwrap();
        fs::write(opendomain.join("domain/glossary.md"), "MY CURATED GLOSSARY").unwrap();

        let report = scaffold(&opendomain, &cfg, false).unwrap();
        assert_eq!(
            fs::read_to_string(opendomain.join("domain/glossary.md")).unwrap(),
            "MY CURATED GLOSSARY"
        );
        assert!(report.skipped.contains(&"domain/glossary.md".to_string()));
    }

    #[test]
    fn every_template_is_valid_utf8_and_non_empty() {
        for t in templates::all() {
            assert!(!t.contents.trim().is_empty(), "{} is empty", t.path);
            // Markdown pour les registres et les contrats ; le `.gitignore`
            // porté par l'arbre généré est le seul fichier de versioning du
            // scaffold — volontairement pas du markdown.
            assert!(
                t.path.ends_with(".md") || t.path.ends_with(".gitignore"),
                "{} is neither markdown nor a tree gitignore",
                t.path
            );
        }
    }

    #[test]
    fn template_comments_are_balanced() {
        // An unterminated `<!--` makes the parser discard everything after it,
        // so a registry whose example is never closed also swallows every
        // artifact a user adds. Thirty templates shipped in that state: the
        // example was hidden as intended and so was the entire registry. This
        // test exists so that cannot come back silently.
        for t in templates::all() {
            assert_eq!(
                t.contents.matches("<!--").count(),
                t.contents.matches("-->").count(),
                "{} has unbalanced HTML comments, so its guidance section and any \
                 artifact a user appends will be stripped as comment text",
                t.path
            );
        }
    }

    #[test]
    fn template_guidance_sections_are_not_swallowed() {
        // The complement of the balance test: a balanced comment that is closed
        // in the wrong place still hides the guidance. Confirm that the closing
        // marker precedes the first non-example heading in every template that
        // opens a comment at all.
        for t in templates::all() {
            let Some(open) = t.contents.find("<!--") else {
                continue;
            };
            let body = &t.contents[open..];
            let Some(close) = body.find("-->") else {
                continue; // balance test reports this
            };
            let after = &body[close + 3..];
            let guidance = after
                .lines()
                .find(|l| l.starts_with("## "))
                .unwrap_or_default();
            assert!(
                !guidance.is_empty(),
                "{} closes its comment at the very end, hiding all guidance",
                t.path
            );
        }
    }

    #[test]
    fn agent_files_land_at_the_project_root_not_inside_the_workspace() {
        let dir = scratch("agent-root");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        let report = scaffold(&od_dir, &cfg, false).unwrap();

        assert!(od_dir.join("AGENTS.md").exists() == false);
        assert!(dir.join("AGENTS.md").exists());
        assert!(dir.join(".opencode/skills/opendomain/SKILL.md").exists());
        assert!(report.agent_files.iter().any(|p| p == "AGENTS.md"));

        // The namespaced files must not land inside the workspace, or `opendomain`'s own
        // scanner would read them as registries.
        assert!(!od_dir.join(".opencode").exists());
    }

    #[test]
    fn an_existing_agents_md_is_never_replaced() {
        // The project's own instructions are not the tool's to overwrite, and
        // `--force` does not change that: the flag exists to refresh files `opendomain`
        // wrote, not to claim a file it did not.
        let dir = scratch("agents-existing");
        std::fs::write(dir.join("AGENTS.md"), "# our own instructions\n").unwrap();
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);

        for force in [false, true] {
            scaffold(&od_dir, &cfg, force).unwrap();
            assert_eq!(
                std::fs::read_to_string(dir.join("AGENTS.md")).unwrap(),
                "# our own instructions\n",
                "AGENTS.md was overwritten with force={force}"
            );
        }
    }

    #[test]
    fn re_running_init_adds_missing_agent_files_without_touching_registries() {
        let dir = scratch("agent-rerun");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);

        scaffold(&od_dir, &cfg, false).unwrap();
        std::fs::remove_dir_all(dir.join(".opencode")).unwrap();
        std::fs::write(od_dir.join("project.md"), "edited by a person\n").unwrap();

        let report = scaffold(&od_dir, &cfg, false).unwrap();
        assert!(dir.join(".opencode/command/opendomain-gate.md").exists());
        assert_eq!(
            std::fs::read_to_string(od_dir.join("project.md")).unwrap(),
            "edited by a person\n"
        );
        assert!(!report.is_noop());
    }

    #[test]
    fn every_agent_template_is_non_empty() {
        for t in agent_contract::all() {
            assert!(!t.contents.trim().is_empty(), "{} is empty", t.path);
        }
    }

    #[test]
    fn the_agent_contract_names_only_commands_that_exist() {
        // The contract tells an agent which commands to reach for. A command
        // named here that the CLI does not have is the contract lying, which is
        // worse than the contract being silent: an agent that runs it and finds
        // nothing has been told, authoritatively, that the workspace is
        // self-describing when it is not.
        //
        // The source is scraped rather than linked, because the CLI is a
        // separate crate that depends on this one, not the reverse. The check
        // is deliberately narrow: a clap variant of that name in the `Command`
        // enum.
        for cmd in contract_commands() {
            assert!(
                cli_declares(&cmd),
                "the agent contract names `opendomain {cmd}`, which the CLI does not declare; \
                 the contract's command table must be checked against the `Command` enum"
            );
        }
    }

    #[test]
    fn the_agent_contract_documents_every_command() {
        // The converse of the check above, and the one that was missing. A
        // contract that lists some of the tool's commands and silently omits the
        // rest is the more likely defect, because omission produces no error: a
        // half-documented tool reads as a fully documented one. An agent never
        // learns that `opendomain graph` or `opendomain propose` exist.
        //
        // `help` is clap's own, not a command of ours.
        let documented = contract_commands();
        for cmd in cli_commands() {
            if cmd == "Help" {
                continue;
            }
            assert!(
                documented.contains(&cmd.to_lowercase()),
                "`opendomain {}` exists but the agent contract does not mention it; an agent \
                 reading AGENTS.md cannot discover it",
                cmd.to_lowercase()
            );
        }
    }

    #[test]
    fn the_agent_contract_names_every_gate_and_its_question() {
        // The gate table in the contract answers "what does this stage want?",
        // which is the question an agent has before it runs anything. If the
        // table drifts from `Gate::ALL`, an agent is being told to satisfy a
        // stage by a question the tool no longer asks.
        let contract = agent_contract::AGENTS.contents;
        for g in crate::analysis::gates::Gate::ALL {
            let row = format!("| {} | `{}` | {} |", g.number, g.name, g.question);
            assert!(
                contract.contains(&row),
                "the agent contract's gate table does not carry gate {} `{}` with its \
                 question; the tool's own text is \"{}\"",
                g.number,
                g.name,
                g.question
            );
        }
    }

    #[test]
    fn the_agent_contract_states_the_real_rule_counts() {
        // The contract quotes a number so an agent knows whether `opendomain reference`
        // is truncated. A wrong count makes the reference look incomplete when
        // it is not, which is the same failure as an undocumented command.
        let contract = agent_contract::AGENTS.contents;
        let blocking = crate::analysis::gates::RULES
            .iter()
            .filter(|r| r.severity == crate::analysis::gates::Severity::Blocking)
            .count();
        let total = crate::analysis::gates::RULES.len();
        let expected = format!("**{total} rules**");
        assert!(
            contract.contains(&expected),
            "the agent contract does not state the rule count as {expected}"
        );
        let blocking_expected = format!("{blocking} blocking");
        assert!(
            contract.contains(&blocking_expected),
            "the agent contract does not state the blocking count as {blocking_expected}"
        );
    }

    /// Every command name the shipped agent contract mentions, lowercased.
    ///
    /// Deliberately narrow. A run of text is read as a command only if the
    /// first word after `opendomain` is a plain word: `--help` and `--json` are flags,
    /// and `opendomain init --profile lean` names the profile `lean`, not a command.
    /// Treating those as commands is how a completeness check starts crying
    /// wolf, and a check that cries wolf gets ignored.
    fn contract_commands() -> Vec<String> {
        let contract = agent_contract::AGENTS.contents;
        let mut found: Vec<String> = contract
            .lines()
            .filter_map(|line| {
                line.split("`opendomain ")
                    .skip(1)
                    .filter_map(|rest| rest.split(['`', ' ', '|', ',']).next())
                    .filter(|w| {
                        !w.is_empty()
                            && !w.starts_with('-')
                            && w.chars().all(|c| c.is_ascii_alphanumeric())
                    })
                    .map(str::to_lowercase)
                    .next()
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// The command variants the CLI's `Command` enum declares, in source order.
    ///
    /// The enum body is bounded by its first standalone closing brace (the
    /// outermost `}` at column zero) — not by the first line starting with
    /// `}`: struct-style variants such as
    /// `Spec { #[command(subcommand)] op: Op }` close one of those mid-enum,
    /// and cutting there would hide every variant after it.
    fn cli_commands() -> Vec<String> {
        // Résolu depuis le CWD des tests (crates/opendomain-core), pas depuis
        // le fichier : `..` = crates/. L'enum `Command` vit dans args.rs, et
        // args.rs seul — le contrat ne scrape qu'une source de vérité.
        let src = fs::read_to_string("../opendomain-cli/src/commands/args.rs")
            .expect("the CLI source is part of the workspace");
        let body = src
            .split("enum Command {")
            .nth(1)
            .expect("the CLI declares a `Command` enum");
        let end = body
            .lines()
            .position(|l| l.trim() == "}")
            .expect("the `Command` enum is closed");
        body.lines()
            .take(end)
            .map(str::trim)
            .filter_map(|l| {
                // Both tuple and struct-style variants start with the same
                // capitalised name; the name is everything before the first
                // `(variant-args)` or `{ fields }`. Inner field lines such as
                // `path: String,` start lowercase and are dropped by the
                // name check below.
                let name = l.split(['(', '{']).next()?.trim();
                let name = name.trim_end_matches(',').trim();
                (!name.is_empty()
                    && !name.starts_with('#')
                    && name.chars().next().is_some_and(char::is_uppercase))
                .then(|| name.to_string())
            })
            .collect()
    }

    fn cli_declares(cmd: &str) -> bool {
        let want = capitalise(cmd);
        cli_commands().iter().any(|v| *v == want)
    }

    fn capitalise(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }

    #[test]
    fn emitted_agent_contract_matches_templates() {
        // What `opendomain init` writes must be byte-for-byte the shipped templates:
        // otherwise the framework's own contract drifts from what every other
        // project receives — the exact defect this file set exists to prevent.
        //
        // (An earlier version of this test compared against copies at the
        // repository root, which were never committed, so it failed on every
        // fresh clone. Comparing the emission round-trip instead keeps the
        // invariant without requiring the repo to dogfood its own workspace.)
        let dir = scratch("contract-roundtrip");
        let od_dir = dir.join("opendomain");
        let cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        scaffold(&od_dir, &cfg, false).unwrap();
        for t in agent_contract::all() {
            if agent_contract::is_root_level(&t) {
                continue; // `AGENTS.md` belongs to the project, not the tool.
            }
            let emitted = fs::read_to_string(dir.join(t.path)).unwrap_or_else(|e| {
                panic!("`opendomain init` did not emit {} ({e})", t.path);
            });
            assert_eq!(
                emitted, t.contents,
                "{} as emitted differs from its shipped template",
                t.path
            );
        }
    }

    #[test]
    fn rendered_config_round_trips() {
        let mut cfg = default_config("Acme", Profile::Full, AuthorityModel::SpecAsSource);
        cfg.project.insert("region".into(), "Senegal".into());
        let text = render_config(&cfg);
        let back = Config::parse(&text).expect("rendered config must parse");
        assert_eq!(back.name, "Acme");
        assert_eq!(back.profile, Profile::Full);
        assert_eq!(back.authority, AuthorityModel::SpecAsSource);
        assert_eq!(
            back.project.get("region").map(String::as_str),
            Some("Senegal")
        );
    }

    #[test]
    fn brownfield_signals_detect_existing_implementation() {
        let dir = scratch("signals-empty");
        assert!(brownfield_signals(&dir).is_empty());

        let dir = scratch("signals-code");
        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("services")).unwrap();
        let signals = brownfield_signals(&dir);
        assert!(signals.contains(&"package.json".to_string()));
        assert!(signals.contains(&"services".to_string()));
        assert!(!signals.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn init_installs_only_and_records_no_context() {
        // `opendomain init` installs the workspace + agent contract and records
        // nothing: context is completed by `/opendomain-project` (context-completer),
        // never at init time. Even with `[project]` keys present, no DOM/STK
        // artifact is invented.
        let dir = scratch("init-no-context");
        let opendomain = dir.join("opendomain");
        let mut cfg = default_config("Demo", Profile::Core, AuthorityModel::SpecAnchored);
        cfg.project.insert("purpose".into(), "Coach sellers".into());
        scaffold(&opendomain, &cfg, false).unwrap();
        let map = fs::read_to_string(opendomain.join("domain/domain-map.md")).unwrap();
        assert!(!has_live_heading(&map, "## DOM-"));
        let stk = fs::read_to_string(opendomain.join("domain/stakeholders.md")).unwrap();
        assert!(!has_live_heading(&stk, "## STK-"));
        let project = fs::read_to_string(opendomain.join("project.md")).unwrap();
        assert!(!project.contains("<!-- opendomain-inventory -->"));
    }

    #[test]
    fn context_completer_is_report_bound_and_writes_context() {
        // The context-completer fills project.md + [project] + DOM/STK and
        // asks the user for the rest. Its body must bind it to repo reading,
        // keyword-driven external research, and human-only gaps — otherwise it
        // is a second researcher with no completion contract.
        let body = agent_contract::CONTEXT_COMPLETER.contents;
        assert!(body.contains("Phase 1"), "must read the repository first");
        assert!(
            body.contains("Phase 2"),
            "must run external research from keywords"
        );
        assert!(body.contains("Phase 4"), "must ask the user for the rest");
        assert!(
            body.contains("**Human-only gaps**"),
            "report contract: gaps section"
        );
        assert!(
            body.contains("INFERRED"),
            "code findings are INFERRED at best"
        );
        assert!(
            agent_contract::all()
                .iter()
                .any(|t| t.path.ends_with("context-completer.md")),
            "context-completer must be emitted by `opendomain init`"
        );
        assert!(
            !agent_contract::all()
                .iter()
                .any(|t| t.path.ends_with("brownfield-discoverer.md")),
            "brownfield-discoverer must be gone"
        );
    }

    #[test]
    fn gate_commands_cover_all_nine_gates() {
        // One useful command per gate stage: project(0), research(1),
        // domain(2), requirements(3), opportunity(4), spec(5), plan(6),
        // complete(8). A missing stage command means the
        // framework's full potential is undiscoverable from slash commands.
        for path in [
            ".opencode/command/opendomain-project.md",
            ".opencode/command/opendomain-research.md",
            ".opencode/command/opendomain-domain.md",
            ".opencode/command/opendomain-requirements.md",
            ".opencode/command/opendomain-opportunity.md",
            ".opencode/command/opendomain-spec.md",
            ".opencode/command/opendomain-plan.md",
            ".opencode/command/opendomain-complete.md",
        ] {
            assert!(
                agent_contract::all().iter().any(|t| t.path == path),
                "{path} must be emitted by `opendomain init`"
            );
        }
    }

    #[test]
    fn domain_researcher_fans_out_by_keyword_cluster() {
        // The researcher must delegate in parallel by keyword cluster and
        // triangulate: single-source restatement is not corroboration.
        let body = agent_contract::DOMAIN_RESEARCHER.contents;
        assert!(body.contains("Fan out"), "must fan out in parallel");
        assert!(body.contains("keyword"), "must work from keyword clusters");
        assert!(body.contains("single-source") || body.contains("Single-source"));
        assert!(body.contains("Not claimed"));
    }

    #[test]
    fn focused_skills_exist_and_are_called() {
        // The framework's strength lives in callable skills, not in one
        // generic file nobody loads: epistemics, artifacts, research, gates.
        // Each command/agent states what it calls, so the call graph is
        // reviewable rather than aspirational.
        for path in [
            ".opencode/skills/opendomain-epistemics/SKILL.md",
            ".opencode/skills/opendomain-artifacts/SKILL.md",
            ".opencode/skills/opendomain-research/SKILL.md",
            ".opencode/skills/opendomain-gates/SKILL.md",
        ] {
            assert!(
                agent_contract::all().iter().any(|t| t.path == path),
                "{path} must be emitted by `opendomain init`"
            );
        }
        let index = agent_contract::SKILL.contents;
        assert!(index.contains("opendomain-epistemics"));
        assert!(index.contains("opendomain-artifacts"));
        assert!(index.contains("opendomain-research"));
        assert!(index.contains("opendomain-gates"));

        // Inter-agent calls are declared, not implied.
        assert!(agent_contract::CONTEXT_COMPLETER
            .contents
            .contains("domain-researcher"));

        // Summary-first kills the verbatim-noise: every command states it.
        for t in agent_contract::all() {
            if t.path.starts_with(".opencode/command/") {
                assert!(
                    t.contents.contains("résumé-first") || t.contents.contains("Verdict"),
                    "{} must render summary-first, not verbatim-noise",
                    t.path
                );
            }
        }

        // opendomain-complete routes to opendomain-audit instead of re-rendering the chain.
        assert!(agent_contract::OD_COMPLETE
            .contents
            .contains("/opendomain-audit"));
        assert!(agent_contract::OD_AUDIT.contents.contains("PAS"));
    }
}
