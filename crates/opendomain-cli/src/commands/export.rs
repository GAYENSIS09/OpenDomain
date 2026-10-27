//! `opendomain export` — la base de connaissances entière, rendue paginée.
//!
//! Le HTML stylé est produit par le core ([`opendomain_core::export`]) ;
//! cette commande ne fait que choisir le graphe, écrire le document, puis
//! l'imprimer en PDF avec le navigateur de la machine en mode headless
//! (`--print-to-pdf`). Rien n'est installé, aucun service n'est appelé, et
//! quand aucun navigateur n'est trouvé — ou que l'impression échoue — le
//! HTML reste et la commande dit pourquoi : la dégradation est honnête,
//! jamais muette. Comme `status` et `graph`, elle ne réécrit aucun registre
//! ; elle n'écrit que son fichier de sortie.

use anyhow::{Context, Result};
use opendomain_core::analysis::graph::Graph;
use opendomain_core::export::{self, ExportDoc, GraphFormat, GRAPH_NODE_LIMIT};
use opendomain_core::report::Style;
use opendomain_core::workspace::{Index, Workspace};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::process::Command as SysCommand;

use super::args::{Cli, ExportArgs, ExportFormat, ExportGraph};
use super::context::{style_ok, to_json};

/// `opendomain export` — toute la base en un document paginé.
pub fn cmd_export(
    args: &ExportArgs,
    ws: &Workspace,
    index: &Index,
    style: Style,
    cli: &Cli,
) -> Result<()> {
    let mut doc = export::collect(ws, index)?;
    apply_graph(args, index, &mut doc);

    let (pdf_target, html_target) = resolve_targets(args, ws, &doc);
    write_html(&html_target, &export::render_document(&doc))?;

    // What actually happened is decided here, then told once — a PDF, or the
    // document instead, with the reason beside it either way.
    let mut out = Outcome::new(args, &doc, &html_target);
    match args.format {
        ExportFormat::Html => {}
        ExportFormat::Pdf => match pdf_target.as_deref() {
            Some(pdf) => match find_browser(args) {
                Some(ref browser) => match print_pdf(browser, &html_target, pdf) {
                    Ok(bytes) => {
                        out.pdf_printed(pdf, bytes, browser, args.keep_html);
                        if !args.keep_html {
                            let _ = std::fs::remove_file(&html_target);
                        }
                    }
                    Err(err) => out.pdf_degraded(format!("{err:#}")),
                },
                None => out.pdf_degraded(
                    "no browser found in the usual places; set OPENDOMAIN_BROWSER or \
                     --browser to print the PDF"
                        .to_string(),
                ),
            },
            // Unreachable while `resolve_targets` pairs one PDF target with
            // every `--format pdf` invocation; the honest message beats a
            // panic all the same.
            None => out.pdf_degraded(
                "no PDF target was chosen for `--format pdf`; this is a bug, \
                 please report it"
                    .to_string(),
            ),
        },
    }

    if cli.json {
        println!("{}", to_json(&out.report)?);
    } else {
        out.print_human(style);
    }
    Ok(())
}

/// Applies the graph choice: Mermaid carried as source, DOT pre-rendered
/// when Graphviz answers, or none. The cover numbers come from the full
/// graph either way, so `--graph none` never hides untraceability.
fn apply_graph(args: &ExportArgs, index: &Index, doc: &mut ExportDoc) {
    let graph = Graph::build(index);
    doc.graph_truncated = graph.node_count() > GRAPH_NODE_LIMIT;
    match args.graph {
        ExportGraph::Mermaid => {
            doc.graph = GraphFormat::Mermaid;
            doc.graph_mermaid = Some(graph.to_mermaid(index, GRAPH_NODE_LIMIT));
            doc.graph_dot = None;
            doc.graph_svg = None;
        }
        ExportGraph::Dot => {
            doc.graph = GraphFormat::Dot;
            let source = graph.to_dot(index);
            doc.graph_svg = render_dot(&source);
            doc.graph_dot = Some(source);
            doc.graph_mermaid = None;
        }
        ExportGraph::None => {
            doc.graph = GraphFormat::None;
            doc.graph_mermaid = None;
            doc.graph_dot = None;
            doc.graph_svg = None;
        }
    }
}

/// Renders DOT to inline SVG with the Graphviz on `PATH`. `None` is not an
/// error: the document then prints the escaped source, which any reviewer
/// can paste into any Graphviz.
fn render_dot(source: &str) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("opd-export-dot-{}", std::process::id()));
    let dot_file = dir.join("graph.dot");
    let svg_file = dir.join("graph.svg");
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(&dot_file, source).ok()?;
    let out = SysCommand::new("dot")
        .arg("-Tsvg")
        .arg(&dot_file)
        .arg("-o")
        .arg(&svg_file)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    std::fs::read_to_string(&svg_file).ok()
}

/// The browser that prints the PDF: `--browser` overrides the
/// `OPENDOMAIN_BROWSER` environment variable, which overrides detection.
/// Edge is tried first on Windows — it is already there on stock machines.
fn find_browser(args: &ExportArgs) -> Option<PathBuf> {
    if let Some(explicit) = &args.browser {
        return Some(PathBuf::from(explicit));
    }
    if let Some(env) = std::env::var_os("OPENDOMAIN_BROWSER") {
        return Some(PathBuf::from(env));
    }
    browser_candidates().into_iter().find(|p| p.is_file())
}

fn browser_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        out.extend(
            [
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            ]
            .iter()
            .map(PathBuf::from),
        );
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(local).join(r"Google\Chrome\Application\chrome.exe"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        out.extend(
            [
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
            ]
            .iter()
            .map(PathBuf::from),
        );
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        out.extend(
            [
                "/usr/bin/msedge",
                "/opt/google/chrome/chrome",
                "/usr/bin/google-chrome-stable",
                "/usr/bin/google-chrome",
                "/usr/bin/chromium",
                "/usr/bin/chromium-browser",
            ]
            .iter()
            .map(PathBuf::from),
        );
    }
    out
}

/// One headless run over the written HTML, paused only as long as the
/// virtual time budget needs (web fonts, the Mermaid CDN). Returns the
/// PDF's size in bytes, so the caller can refuse a suspicious file.
///
/// The browser returns before its writer does: on this machine the PDF
/// lands a beat after the process exits, so the wait here is bounded but
/// patient — declaring failure while the bytes are still streaming would be
/// a false verdict, the one thing this command may never do.
fn print_pdf(browser: &Path, html: &Path, pdf: &Path) -> Result<u64> {
    let profile = std::env::temp_dir().join(format!("opd-pdf-profile-{}", std::process::id()));
    let output = SysCommand::new(browser)
        .args([
            "--headless",
            "--disable-gpu",
            "--disable-extensions",
            "--no-first-run",
            "--no-pdf-header-footer",
            "--virtual-time-budget=20000",
        ])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--print-to-pdf={}", pdf.display()))
        .arg(html)
        .output()
        .map_err(|e| anyhow::anyhow!("{} cannot be run: {e}", browser.display()))?;
    let _ = std::fs::remove_dir_all(&profile);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail = stderr.lines().last().unwrap_or("").to_string();
        anyhow::bail!(
            "{} exited with {}{}",
            browser.display(),
            output.status,
            if tail.is_empty() {
                String::new()
            } else {
                format!(" — {tail}")
            }
        );
    }
    wait_for_pdf(pdf).map_err(|_| {
        anyhow::anyhow!(
            "{} left no readable PDF at {}",
            browser.display(),
            pdf.display()
        )
    })
}

/// Waits for the PDF to exist behind a credible size and stop growing: two
/// consecutive polls with the same size, within a bounded run. A missing
/// file is not yet a failure — the print may still be streaming; only the
/// timeout is.
fn wait_for_pdf(pdf: &Path) -> Result<u64, ()> {
    let mut last: Option<u64> = None;
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if let Ok(meta) = std::fs::metadata(pdf) {
            let size = meta.len();
            if size >= 500 && last == Some(size) {
                return Ok(size);
            }
            last = Some(size);
        }
    }
    Err(())
}

fn write_html(html: &Path, content: &str) -> Result<()> {
    if let Some(parent) = html.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create the directory {}", parent.display()))?;
    }
    std::fs::write(html, content).with_context(|| format!("cannot write {}", html.display()))
}

/// Where the files go. The default dates itself — `<name>-knowledge-<date>`
/// beside the project — so a re-export the same day replaces its own file
/// instead of growing a pile.
///
/// Both targets come back absolute, because the two writers disagree:
/// `std::fs` happily writes a relative path, but the browser refuses or
/// misplaces a relative `--print-to-pdf`, so `-C .` (whose `project_root`
/// stays relative) would otherwise print nothing while claiming success.
fn resolve_targets(
    args: &ExportArgs,
    ws: &Workspace,
    doc: &ExportDoc,
) -> (Option<PathBuf>, PathBuf) {
    let (pdf, html) = match (&args.format, &args.out) {
        (ExportFormat::Html, Some(out)) => (None, out.clone()),
        (ExportFormat::Html, None) => (None, default_base(ws, doc).with_extension("html")),
        (ExportFormat::Pdf, Some(out)) => (Some(out.clone()), html_sibling(out)),
        (ExportFormat::Pdf, None) => {
            let base = default_base(ws, doc);
            (
                Some(base.with_extension("pdf")),
                base.with_extension("html"),
            )
        }
    };
    (pdf.as_ref().map(|p| absolutize(p)), absolutize(&html))
}

/// Anchors a possibly-relative target onto the process's directory. No
/// `canonicalize`: that would surface `\\?\` prefixes in every message, and
/// a resolved read is not asked for — only an anchor the browser accepts.
fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let mut out = match std::env::current_dir() {
        Ok(dir) => dir,
        // No anchor available: hand the path back and let the writer report
        // its own honest failure.
        Err(_) => return path.to_path_buf(),
    };
    for component in path.components() {
        match component {
            Component::CurDir => {}
            comp => out.push(comp),
        }
    }
    out
}

fn default_base(ws: &Workspace, doc: &ExportDoc) -> PathBuf {
    ws.project_root.join(format!(
        "{}-knowledge-{}",
        sanitize(&doc.stats.name),
        doc.date
    ))
}

/// A sibling the print can source, keeping `--out`'s intention: `report.pdf`
/// prints from `report.html`, an extensionless `report` from `report.html`
/// too, and even `report.doc` keeps the PDF at the requested name — the PDF
/// path itself never changes.
fn html_sibling(pdf: &Path) -> PathBuf {
    match (pdf.parent(), pdf.file_stem()) {
        (Some(parent), Some(stem)) => parent.join(format!("{}.html", stem.to_string_lossy())),
        _ => {
            let mut name = pdf.as_os_str().to_owned();
            name.push(".html");
            PathBuf::from(name)
        }
    }
}

/// Windows-illegal filename characters become `-`, so the dated default
/// name works the same way on every machine.
fn sanitize(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The story the command tells, accumulated as it happens: the JSON report
/// and the plain verdict are rendered from the same final state, so the
/// machine's account and the human's always agree.
struct Outcome<'a> {
    report: ExportReport,
    doc: &'a ExportDoc,
}

impl<'a> Outcome<'a> {
    fn new(args: &ExportArgs, doc: &'a ExportDoc, html: &Path) -> Self {
        Outcome {
            report: ExportReport {
                format: format_label(args.format).to_string(),
                pdf: None,
                pdf_bytes: 0,
                html: Some(html.display().to_string()),
                kept_html: args.format != ExportFormat::Pdf || args.keep_html,
                files: doc.stats.files,
                artifacts: doc.stats.artifacts,
                graph: ExportGraphReport {
                    rendered: graph_label(doc.graph).to_string(),
                    nodes: doc.stats.graph_nodes,
                    links: doc.stats.graph_links,
                    truncated: doc.graph_truncated,
                },
                browser: None,
                note: None,
            },
            doc,
        }
    }

    fn pdf_printed(&mut self, pdf: &Path, bytes: u64, browser: &Path, keep_html: bool) {
        self.report.pdf = Some(pdf.display().to_string());
        self.report.pdf_bytes = bytes;
        self.report.browser = Some(browser.display().to_string());
        self.report.kept_html = keep_html;
        if !keep_html {
            self.report.html = None; // removed after the print
        }
    }

    fn pdf_degraded(&mut self, reason: String) {
        self.report.pdf = None;
        self.report.kept_html = true;
        self.report.note = Some(reason);
    }

    fn print_human(&self, style: Style) {
        let d = self.doc;
        let graph_bit = graph_sentence(d);
        match (&self.report.pdf, &self.report.note) {
            (Some(_), _) => println!(
                "{}",
                style_ok(
                    style,
                    &format!(
                        "exported {} files · {} artifacts as PDF — {}",
                        d.stats.files, d.stats.artifacts, graph_bit
                    )
                )
            ),
            (None, Some(_)) => println!(
                "{}",
                style_ok(
                    style,
                    &format!(
                        "exported {} files · {} artifacts as HTML — the PDF failed: {}",
                        d.stats.files,
                        d.stats.artifacts,
                        self.report.note.as_deref().unwrap_or_default()
                    )
                )
            ),
            (None, None) => println!(
                "{}",
                style_ok(
                    style,
                    &format!(
                        "exported {} files · {} artifacts as HTML — {}",
                        d.stats.files, d.stats.artifacts, graph_bit
                    )
                )
            ),
        }
        if let Some(pdf) = &self.report.pdf {
            println!("  pdf    {} ({})", pdf, human(self.report.pdf_bytes));
        }
        if let Some(html) = &self.report.html {
            if self.report.kept_html {
                println!("  html   {html}");
            } else {
                println!("  html   removed after the print (--keep-html keeps it)");
            }
        }
        if let Some(browser) = &self.report.browser {
            println!("  printed by {browser}");
        }
        if let Some(note) = &self.report.note {
            println!("  note   {note}");
        }
    }
}

/// The story `--json` tells: what was written, at what size, in which graph
/// form, by which browser — and the honest note when no PDF happened.
#[derive(Serialize)]
struct ExportReport {
    format: String,
    /// Where the PDF went; null when no browser printed it.
    pdf: Option<String>,
    pdf_bytes: u64,
    /// Where the print source lives: null when `--keep-html` was not passed
    /// and the print succeeded, since the file is then deliberately gone.
    html: Option<String>,
    kept_html: bool,
    files: usize,
    artifacts: usize,
    graph: ExportGraphReport,
    browser: Option<String>,
    /// The fallback reason when the PDF did not happen.
    note: Option<String>,
}

#[derive(Serialize)]
struct ExportGraphReport {
    rendered: String,
    nodes: usize,
    links: usize,
    truncated: bool,
}

fn graph_label(graph: GraphFormat) -> &'static str {
    match graph {
        GraphFormat::Mermaid => "mermaid",
        GraphFormat::Dot => "dot",
        GraphFormat::None => "none",
    }
}

fn format_label(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Pdf => "pdf",
        ExportFormat::Html => "html",
    }
}

/// The graph clause of the verdict: what was rendered and how much of it,
/// so a truncated diagram is stated rather than silent.
fn graph_sentence(doc: &ExportDoc) -> String {
    let rendered = match doc.graph {
        GraphFormat::Mermaid => "Mermaid graph",
        GraphFormat::Dot => "DOT graph",
        GraphFormat::None => "graph not rendered (its size is on the cover)",
    };
    let cap = if doc.graph_truncated {
        format!(" (capped at {GRAPH_NODE_LIMIT} nodes)")
    } else {
        String::new()
    };
    format!(
        "{rendered}, {} nodes · {} links{cap}",
        doc.stats.graph_nodes, doc.stats.graph_links
    )
}

fn human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_sibling_keeps_the_intended_stem() {
        assert_eq!(
            html_sibling(Path::new("dir/report.pdf")),
            PathBuf::from("dir/report.html")
        );
        // Extensionless and odd extensions: the PDF keeps the requested
        // name, the print source is a plain `.html` beside it either way.
        assert_eq!(
            html_sibling(Path::new("report")),
            PathBuf::from("report.html")
        );
        assert_eq!(
            html_sibling(Path::new("report.doc")),
            PathBuf::from("report.html")
        );
    }

    #[test]
    fn relative_targets_are_anchored_to_the_process_directory() {
        // An anchor the caller already made absolute passes through untouched.
        let anchored = if cfg!(windows) {
            PathBuf::from(r"C:\somewhere\x.html")
        } else {
            PathBuf::from("/somewhere/x.html")
        };
        assert_eq!(absolutize(&anchored), anchored);

        // Dot-prefixed or bare, a relative target ends up beside the
        // process, keeping its own name — what makes `--print-to-pdf` land.
        for given in [PathBuf::from(".").join("x.html"), PathBuf::from("x.html")] {
            let out = absolutize(&given);
            assert!(
                out.is_absolute(),
                "{} stayed relative: {}",
                given.display(),
                out.display()
            );
            assert_eq!(out.file_name(), given.file_name());
        }
    }

    #[test]
    fn forbidden_name_characters_become_dashes() {
        assert_eq!(sanitize("My: Project?"), "My- Project");
        assert_eq!(sanitize("---"), "project");
        assert_eq!(sanitize("OpenDomain"), "OpenDomain");
    }

    #[test]
    fn sizes_are_reported_honestly() {
        assert_eq!(human(999), "1 KB");
        assert_eq!(human(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn labels_are_stable_in_the_json_report() {
        assert_eq!(format_label(ExportFormat::Pdf), "pdf");
        assert_eq!(format_label(ExportFormat::Html), "html");
        assert_eq!(graph_label(GraphFormat::Mermaid), "mermaid");
        assert_eq!(graph_label(GraphFormat::Dot), "dot");
        assert_eq!(graph_label(GraphFormat::None), "none");
    }

    #[test]
    fn the_graph_sentence_states_truncation() {
        let doc = ExportDoc {
            title: "t".into(),
            date: "d".into(),
            graph: GraphFormat::Mermaid,
            graph_dot: None,
            graph_svg: None,
            graph_mermaid: None,
            graph_truncated: true,
            stats: opendomain_core::export::CoverStats {
                name: "x".into(),
                profile: "p".into(),
                authority: "a".into(),
                repository: "r".into(),
                artifacts: 0,
                files: 0,
                graph_nodes: GRAPH_NODE_LIMIT,
                graph_links: 0,
                unresolved_provenance: 0,
                open_conflicts: 0,
                open_uncertainties: 0,
            },
            files: vec![],
        };
        let sentence = graph_sentence(&doc);
        assert!(sentence.contains("capped at 200 nodes"), "{sentence}");
        assert!(sentence.contains("200 nodes · 0 links"), "{sentence}");
    }
}
