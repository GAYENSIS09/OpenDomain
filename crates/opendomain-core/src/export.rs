//! `opendomain export` — the whole knowledge base as one paginated document.
//!
//! The registries are the evidence F14/L9 says to commit; a document that
//! mirrors them as they are — no rewriting, no selectivity, the same graph
//! the tool draws — is that evidence in a form a reviewer can open without
//! a terminal. Like every other read path, this runs one pure function over
//! a workspace: the same files always produce the same bytes.
//!
//! The HTML is the source of truth for both formats. The PDF is printed
//! from it by the machine's own browser in headless mode (`--print-to-pdf`),
//! so no new runtime is installed and no external service is called; the
//! only network touch is the Mermaid CDN when the graph is Mermaid, chosen
//! because a bundled copy would have to lag the real one and a wrongly
//! rendered graph is worse than an honest inline fallback. Graphviz
//! (`--graph dot`) renders offline when `dot` is on `PATH`.

use crate::analysis::graph::Graph;
use crate::error::{OdError, Result};
use crate::registries::workspace::{Index, Workspace};
use crate::report;
use std::fmt::Write as _;
use std::path::Path;

/// The Mermaid release the document loads from the CDN. Pinned by exact
/// version rather than a floating tag: a graph that re-renders differently
/// between two prints is not a graph a gate could cite. The legacy build
/// (`dist/mermaid.min.js`) carries a plain `window.mermaid`, which is what a
/// headless print can rely on without ESM gymnastics.
pub const MERMAID_CDN: &str = "https://cdn.jsdelivr.net/npm/mermaid@10.9.3/dist/mermaid.min.js";

/// The node cap of `opendomain graph`, honoured here so an export never
/// prints a sprawling thousand-node page the interactive command refuses
/// to draw.
pub const GRAPH_NODE_LIMIT: usize = 200;

/// How the traceability graph is carried into the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphFormat {
    /// Mermaid source rendered in the browser (headless print included).
    #[default]
    Mermaid,
    /// Graphviz DOT pre-rendered to inline SVG by the `dot` binary.
    Dot,
    /// No diagram — the cover still states the node and link counts.
    None,
}

/// One knowledge file, kept whole: an export mirrors the registries as they
/// are, so a section is the file's text and the file's text alone.
#[derive(Debug, Clone)]
pub struct ExportFile {
    /// `/`-separated path relative to the workspace root.
    pub path: String,
    /// The file's first `#` heading when it has one, its path otherwise.
    pub title: String,
    /// The raw Markdown of the file.
    pub raw: String,
}

/// The numbers the cover quotes, from the same source `status` uses.
#[derive(Debug, Clone)]
pub struct CoverStats {
    /// Project name from the configuration.
    pub name: String,
    /// Active profile.
    pub profile: String,
    /// Active authority model.
    pub authority: String,
    /// Greenfield, brownfield or hybrid.
    pub repository: String,
    /// Total artifacts in the registries.
    pub artifacts: usize,
    /// Knowledge files mirrored into the document.
    pub files: usize,
    /// Nodes of the traceability graph.
    pub graph_nodes: usize,
    /// Links of the traceability graph.
    pub graph_links: usize,
    /// Provenance still unresolved (assumed, unknown or conflicting).
    pub unresolved_provenance: usize,
    /// Conflicts not yet settled.
    pub open_conflicts: usize,
    /// Known gaps still open.
    pub open_uncertainties: usize,
}

/// Everything one export prints, already collected and ordered. The graph
/// fields are filled by the caller, which owns the format choice and the
/// renderer that produced the SVG.
#[derive(Debug, Clone)]
pub struct ExportDoc {
    /// HTML title of the whole document.
    pub title: String,
    /// Generation date, short ISO — the stamp the cover prints.
    pub date: String,
    /// Which graph form was requested.
    pub graph: GraphFormat,
    /// DOT source when `--graph dot` asked for it.
    pub graph_dot: Option<String>,
    /// Pre-rendered SVG when `dot` produced one.
    pub graph_svg: Option<String>,
    /// Mermaid source when `--graph mermaid` asked for it.
    pub graph_mermaid: Option<String>,
    /// Whether the diagram was capped at `GRAPH_NODE_LIMIT` — stated next
    /// to the diagram, so a partial graph never reads as the whole one.
    pub graph_truncated: bool,
    /// Cover numbers.
    pub stats: CoverStats,
    /// Knowledge files, registry order first.
    pub files: Vec<ExportFile>,
}

/// Collects the whole workspace for export: the cover statistics, the graph
/// numbers, and every knowledge file in registry order. Read-only. Returns
/// the document with the graph format at its default; the caller applies the
/// requested format and fills the matching source field.
pub fn collect(ws: &Workspace, index: &Index) -> Result<ExportDoc> {
    let stats = report::status("opendomain", &ws.config, index);
    let graph = Graph::build(index);

    let scanned = ws.registry_files()?;
    let mut paths: Vec<String> = scanned
        .iter()
        .map(|p| ws.relative(p).to_string_lossy().to_string())
        .collect();
    walk_markdown(ws, Path::new(""), &scanned, &mut paths)?;
    paths.sort_by(|a, b| section_order(a).cmp(&section_order(b)).then(a.cmp(b)));
    paths.dedup();

    let files = paths
        .into_iter()
        .map(|p| -> Result<ExportFile> {
            let file = ws.path(&p);
            let raw = std::fs::read_to_string(&file).map_err(|e| OdError::read(&file, e))?;
            let title = raw
                .lines()
                .find_map(|l| l.strip_prefix("# ").filter(|t| !t.trim().is_empty()))
                .map(|t| t.trim().to_string())
                .unwrap_or_else(|| p.clone());
            Ok(ExportFile {
                path: p,
                title,
                raw,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // `status` echoes the name it is handed, not the project's; the cover
    // quotes the configuration's own name, falling back to the directory
    // that carries the workspace.
    let mut name = ws.config.name.clone();
    if name.trim().is_empty() {
        name = ws
            .project_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".into());
    }

    Ok(ExportDoc {
        title: format!("{name} — OpenDomain knowledge base"),
        date: today(),
        graph: GraphFormat::default(),
        graph_dot: None,
        graph_svg: None,
        graph_mermaid: None,
        graph_truncated: false,
        stats: CoverStats {
            name,
            profile: stats.profile.clone(),
            authority: stats.authority.clone(),
            repository: stats.repository.clone(),
            artifacts: stats.total,
            files: files.len(),
            graph_nodes: graph.node_count(),
            graph_links: graph.edge_count(),
            unresolved_provenance: stats.unresolved_provenance,
            open_conflicts: stats.open_conflicts,
            open_uncertainties: stats.open_uncertainties,
        },
        files,
    })
}

/// Files under the workspace that the configuration does not scan as
/// registries: `project.md`, the workspace README, the protocol prose.
/// The recursion stays inside the workspace and skips what `registry_files`
/// already covers, so a file enters the document exactly once.
fn walk_markdown(
    ws: &Workspace,
    rel_dir: &Path,
    scanned: &[std::path::PathBuf],
    out: &mut Vec<String>,
) -> Result<()> {
    let dir = if rel_dir.as_os_str().is_empty() {
        ws.root.clone()
    } else {
        ws.root.join(rel_dir)
    };
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| OdError::read(&dir, e))?
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let rel = ws.relative(&p).to_string_lossy().to_string();
        if p.is_dir() {
            walk_markdown(ws, Path::new(&rel), scanned, out)?;
        } else if p.extension().is_some_and(|x| x == "md")
            && !scanned
                .iter()
                .any(|s| ws.relative(s).to_string_lossy() == rel)
        {
            out.push(rel);
        }
    }
    Ok(())
}

/// The canonical order of the document: identity first (project state, the
/// workspace README), then the registries in gate order, then the change
/// history. A fixed order is what makes two exports comparable file by file.
fn section_order(rel: &str) -> usize {
    if rel == "project.md" {
        0
    } else if rel == "README.md" {
        1
    } else {
        match rel.split('/').next().unwrap_or("") {
            "domain" => 2,
            "research" => 3,
            "requirements" => 4,
            "opportunity" => 5,
            "specs" => 6,
            "plans" => 7,
            "tasks" => 8,
            "validation" => 9,
            "changes" => 10,
            _ => 11,
        }
    }
}

/// Renders the complete standalone document: cover, contents, graph, then
/// every knowledge file. Same input, same bytes — which is what lets an
/// export be regenerated and diffed instead of trusted on sight.
pub fn render_document(doc: &ExportDoc) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    let _ = write!(
        out,
        "<title>{}</title>\n{}\n</head>\n<body>\n",
        escape_html(&doc.title),
        DOCUMENT_CSS
    );
    render_cover(&doc.stats, &doc.date, &mut out);
    render_toc(&doc.files, &mut out);
    render_graph(doc, &mut out);
    for file in &doc.files {
        let _ = write!(
            out,
            "<section class=\"file\" id=\"{}\">\n<div class=\"file-path\">{}</div>\n{}\n</section>\n",
            slug(&file.path),
            escape_html(&file.path),
            markdown_to_html(&file.raw)
        );
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn render_cover(stats: &CoverStats, date: &str, out: &mut String) {
    let _ = write!(
        out,
        "<header class=\"cover\">\n<div class=\"cover-rule\"></div>\n<h1>{}</h1>\n\
         <p class=\"cover-sub\">OpenDomain knowledge base — generated by <code>opendomain export</code> on {}</p>\n\
         <dl class=\"cover-stats\">\n\
         <div><dt>profile</dt><dd>{}</dd></div>\n\
         <div><dt>authority</dt><dd>{}</dd></div>\n\
         <div><dt>repository</dt><dd>{}</dd></div>\n\
         <div><dt>artifacts</dt><dd>{}</dd></div>\n\
         <div><dt>files</dt><dd>{}</dd></div>\n\
         <div><dt>graph</dt><dd>{} nodes · {} links</dd></div>\n\
         <div><dt>unresolved provenance</dt><dd>{}</dd></div>\n\
         <div><dt>open conflicts</dt><dd>{}</dd></div>\n\
         <div><dt>open uncertainties</dt><dd>{}</dd></div>\n\
         </dl>\n\
         <p class=\"cover-note\">This document mirrors the registries as they are; gates were not run. \
         The current verdicts are <code>opendomain status</code> and <code>opendomain gate --through 8</code> — \
         a stale printout must never be cited against a fresh registry.</p>\n</header>\n",
        escape_html(&stats.name),
        escape_html(date),
        escape_html(&stats.profile),
        escape_html(&stats.authority),
        escape_html(&stats.repository),
        stats.artifacts,
        stats.files,
        stats.graph_nodes,
        stats.graph_links,
        stats.unresolved_provenance,
        stats.open_conflicts,
        stats.open_uncertainties,
    );
}

fn render_toc(files: &[ExportFile], out: &mut String) {
    out.push_str("<nav class=\"toc\"><h2>Contents</h2>\n<ol>\n");
    for f in files {
        let _ = write!(
            out,
            "<li><a href=\"#{}\">{}</a> <span class=\"toc-path\">{}</span></li>\n",
            slug(&f.path),
            escape_html(&f.title),
            escape_html(&f.path)
        );
    }
    out.push_str("</ol>\n</nav>\n");
}

fn render_graph(doc: &ExportDoc, out: &mut String) {
    if doc.graph == GraphFormat::None {
        return;
    }
    out.push_str(
        "<section class=\"file graph\" id=\"traceability-graph\">\n\
        <div class=\"file-path\">graph</div>\n<h2>Traceability graph</h2>\n",
    );
    if doc.graph_truncated {
        out.push_str("<p class=\"graph-note\">Capped at 200 nodes — a large workspace prints truncated rather than unreadable. Raise the limit with <code>opendomain graph --limit</code> when you need the whole graph.</p>\n");
    }
    match doc.graph {
        GraphFormat::None => {}
        GraphFormat::Dot => match &doc.graph_svg {
            Some(svg) => {
                let _ = write!(out, "<div class=\"graph-svg\">{svg}</div>\n");
            }
            None => {
                // An honest fallback: the source a reviewer can paste into
                // any Graphviz, instead of a blank promise of a diagram.
                out.push_str(
                    "<p class=\"graph-note\">The DOT source is printed below; \
                     no SVG was pre-rendered for it.</p>\n",
                );
                if let Some(src) = &doc.graph_dot {
                    let _ = write!(
                        out,
                        "<pre class=\"code\" data-lang=\"dot\"><code>{}</code></pre>\n",
                        escape_html(src)
                    );
                }
            }
        },
        GraphFormat::Mermaid => {
            if let Some(src) = &doc.graph_mermaid {
                let _ = write!(
                    out,
                    "<pre class=\"mermaid\">{}</pre>\n<script src=\"{}\"></script>\n\
                     <script>mermaid.initialize({{ startOnLoad: true, theme: \"neutral\", \
                     flowchart: {{ htmlLabels: false, padding: 6 }} }});</script>\n",
                    escape_html(src),
                    MERMAID_CDN
                );
            }
        }
    }
    out.push_str("</section>\n");
}

/// Renders registry Markdown to HTML. The registries are machine-checked
/// Markdown with a frozen shape — `##` headings, fenced `toml` records, prose
/// that is never parsed — so the renderer handles that shape exactly and
/// escapes everything else rather than executing it.
pub(crate) fn markdown_to_html(md: &str) -> String {
    let mut out = String::new();
    let mut lines = md.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("```") {
            let lang = rest.trim();
            let mut body: Vec<&str> = Vec::new();
            for l in lines.by_ref() {
                if l.trim_end().starts_with("```") {
                    break;
                }
                body.push(l);
            }
            if lang == "toml" {
                out.push_str(&toml_record(&body.join("\n")));
            } else if lang == "mermaid" {
                let _ = write!(
                    out,
                    "<pre class=\"code\" data-lang=\"mermaid\"><code>{}</code></pre>\n",
                    escape_html(&body.join("\n"))
                );
            } else {
                let _ = write!(
                    out,
                    "<pre class=\"code\"{}><code>{}</code></pre>\n",
                    lang_label(lang),
                    escape_html(&body.join("\n"))
                );
            }
            continue;
        }
        if trimmed.starts_with("#### ") {
            let _ = write!(out, "<h5>{}</h5>\n", inline(trimmed[5..].trim()));
            continue;
        }
        if trimmed.starts_with("### ") {
            let _ = write!(
                out,
                "<h4 id=\"{}\">{}</h4>\n",
                slug(trimmed[4..].trim()),
                inline(trimmed[4..].trim())
            );
            continue;
        }
        if trimmed.starts_with("## ") {
            let _ = write!(
                out,
                "<h3 id=\"{}\">{}</h3>\n",
                slug(trimmed[3..].trim()),
                inline(trimmed[3..].trim())
            );
            continue;
        }
        if trimmed.starts_with("# ") {
            let _ = write!(
                out,
                "<h2 id=\"{}\">{}</h2>\n",
                slug(trimmed[2..].trim()),
                inline(trimmed[2..].trim())
            );
            continue;
        }
        if trimmed.starts_with('|') {
            let mut peeked: Vec<String> = Vec::new();
            let mut separator_seen = false;
            while let Some(n) = lines.peek() {
                let n = n.trim_end();
                if !n.starts_with('|') {
                    break;
                }
                if table_row_is_separator(n) {
                    separator_seen = true;
                }
                peeked.push(n.to_string());
                lines.next();
            }
            if separator_seen && peeked.len() >= 2 {
                let header = trimmed.to_string();
                let mut rows = vec![header, peeked[0].clone()];
                rows.extend(peeked.into_iter().skip(1));
                out.push_str(&markdown_table(&rows));
            } else {
                out.push_str(&paragraph_block(peeked_pipe_paragraph(trimmed, peeked)));
            }
            continue;
        }
        if trimmed.starts_with("> ") || trimmed == ">" {
            let mut quote: Vec<String> = vec![trimmed.trim_start_matches('>').trim().to_string()];
            while let Some(n) = lines.peek() {
                let n = n.trim_end();
                if n.starts_with("> ") || n == ">" {
                    quote.push(n.trim_start_matches('>').trim().to_string());
                    lines.next();
                } else {
                    break;
                }
            }
            let _ = write!(
                out,
                "<blockquote><p>{}</p></blockquote>\n",
                inline(quote.join(" ").trim())
            );
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            out.push_str("<hr>\n");
            continue;
        }
        if is_list_item(trimmed) {
            let mut items: Vec<(usize, bool, String)> = Vec::new();
            items.push(list_entry(trimmed));
            while let Some(n) = lines.peek() {
                let n = n.trim_end();
                if is_list_item(n) {
                    items.push(list_entry(n));
                    lines.next();
                } else if n.trim().is_empty()
                    && lines
                        .clone()
                        .next()
                        .is_some_and(|l| is_list_item(l.trim_end()))
                {
                    lines.next();
                } else {
                    break;
                }
            }
            out.push_str(&list_block(&items));
            continue;
        }
        if trimmed.trim().is_empty() {
            continue;
        }
        // A paragraph: consecutive plain lines joined with a space — registry
        // prose is hard-wrapped, and the wrap is not content.
        let mut para: Vec<String> = vec![trimmed.to_string()];
        while let Some(n) = lines.peek() {
            let n = n.trim_end();
            let structural = n.trim().is_empty()
                || n.starts_with("```")
                || n.starts_with('#')
                || n.starts_with('|')
                || n.starts_with("> ")
                || is_list_item(n)
                || n.trim() == "---"
                || n.trim() == "***";
            if structural {
                break;
            }
            para.push(n.to_string());
            lines.next();
        }
        out.push_str(&paragraph_block(para));
    }
    out
}

fn peeked_pipe_paragraph(header: &str, rest: Vec<String>) -> Vec<String> {
    let mut para = vec![header.to_string()];
    para.extend(rest);
    para
}

/// The delimiter row of a GFM table: per cell `---`, `:--`, `--:` or `:-:`,
/// now and then with padding.
fn table_row_is_separator(row: &str) -> bool {
    let cells: Vec<&str> = row
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    !cells.is_empty()
        && cells.iter().all(|c| {
            let body = c.trim_matches(':');
            !body.is_empty() && body.chars().all(|ch| ch == '-')
        })
}

fn paragraph_block(lines: Vec<String>) -> String {
    let joined: String = lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.trim().is_empty() {
        return String::new();
    }
    format!("<p>{}</p>\n", inline(joined.trim()))
}

fn lang_label(lang: &str) -> String {
    if lang.is_empty() {
        String::new()
    } else {
        format!(" data-lang=\"{}\"", escape_html(lang))
    }
}

/// One fenced `toml` record as a key/value table — the machine-checked block
/// the gates cite, and the part a reviewer squints at most when it stays
/// fenced. A record that does not parse is printed escaped and flagged: the
/// document mirrors the file even where the file is broken.
fn toml_record(body: &str) -> String {
    match toml::from_str::<toml::Value>(body).map_err(|_| ()) {
        Ok(v) => match v {
            toml::Value::Table(map) => {
                let mut out = String::from("<table class=\"toml\">\n");
                for (k, val) in map {
                    let _ = write!(
                        out,
                        "<tr><th scope=\"row\"><code>{}</code></th><td><code>{}</code></td></tr>\n",
                        escape_html(&k),
                        escape_html(&toml_value_display(&val))
                    );
                }
                out.push_str("</table>\n");
                out
            }
            _ => escaped_toml(body),
        },
        Err(_) => escaped_toml(body),
    }
}

fn escaped_toml(body: &str) -> String {
    format!(
        "<pre class=\"code\" data-lang=\"toml\" data-unparsed=\"1\"><code>{}</code></pre>\n",
        escape_html(body)
    )
}

/// Values rendered for reviewers: arrays on one line, inline tables flattened,
/// strings verbatim. This is a print discipline, not a parse discipline — the
/// registries stay the authority.
fn toml_value_display(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Array(items) => items
            .iter()
            .map(toml_value_display)
            .collect::<Vec<_>>()
            .join(", "),
        toml::Value::Table(t) => t
            .iter()
            .map(|(k, val)| format!("{k} = {}", toml_value_display(val)))
            .collect::<Vec<_>>()
            .join("; "),
        other => other.to_string(),
    }
}

fn markdown_table(rows: &[String]) -> String {
    let cells = |row: &str| -> Vec<String> {
        let r = row.trim();
        let r = r.strip_prefix('|').unwrap_or(r);
        let r = r.strip_suffix('|').unwrap_or(r);
        r.split('|').map(|c| c.trim().to_string()).collect()
    };
    let header = cells(&rows[0]);
    let mut out = String::from("<table class=\"md\">\n<tr>");
    for h in &header {
        let _ = write!(out, "<th scope=\"col\">{}</th>", inline(h));
    }
    out.push_str("</tr>\n");
    for row in rows.iter().skip(2) {
        out.push_str("<tr>");
        for c in cells(row) {
            let _ = write!(out, "<td>{}</td>", inline(&c));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n");
    out
}

/// A list block from already-collected entries: `(depth, ordered, text)`.
/// Nesting is whatever the indentation says; each nested list inherits the
/// marker of its own first item.
fn list_block(items: &[(usize, bool, String)]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut at = 0;
    build_list(items, items[0].0, &mut at)
}

fn build_list(items: &[(usize, bool, String)], level: usize, at: &mut usize) -> String {
    if *at >= items.len() || items[*at].0 != level {
        return String::new();
    }
    let ordered = items[*at].1;
    let mut out = if ordered { "<ol>\n" } else { "<ul>\n" }.to_string();
    while *at < items.len() && items[*at].0 == level {
        let text = items[*at].2.clone();
        *at += 1;
        let nested = if *at < items.len() && items[*at].0 > level {
            let deeper = items[*at].0;
            build_list(items, deeper, at)
        } else {
            String::new()
        };
        if nested.is_empty() {
            let _ = write!(out, "<li>{}</li>\n", inline(&text));
        } else {
            let _ = write!(out, "<li>{}\n{nested}</li>\n", inline(&text));
        }
    }
    out.push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
    out
}

/// Reads one list line into `(indent level, ordered?, text after the marker)`.
fn list_entry(line: &str) -> (usize, bool, String) {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && (t[digits..].starts_with(". ") || t[digits..].starts_with(") ")) {
        let mut after = &t[digits..];
        after = &after[1..];
        after = after.strip_prefix(' ').unwrap_or(after);
        ((indent / 2).min(3), true, after.trim().to_string())
    } else {
        let text = t.trim_start_matches(['-', '*']).trim_start_matches(' ');
        ((indent / 2).min(3), false, text.trim().to_string())
    }
}

fn is_list_item(line: &str) -> bool {
    let t = line.trim_start();
    if t.is_empty() {
        return false;
    }
    if t.starts_with("- ") || t.starts_with("* ") {
        return true;
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && (t[digits..].starts_with(". ") || t[digits..].starts_with(") "))
}

/// Inline formatting: inline code, bold, links — escaped first, so no raw
/// HTML can pass through. The registries' prose is reviewer-facing; the
/// renderer deliberately supports only what the registries use.
fn inline(text: &str) -> String {
    let escaped = escape_html(text);
    let mut out = String::with_capacity(escaped.len() + 16);
    let chars: Vec<char> = escaped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            if let Some(end) = (i + 1..chars.len()).find(|&j| chars[j] == '`') {
                let code: String = chars[i + 1..end].iter().collect();
                let _ = write!(out, "<code>{}</code>", code.trim());
                i = end + 1;
                continue;
            }
        }
        if chars[i] == '*' && chars.get(i + 1) == Some(&'*') {
            let mut end = None;
            let mut j = i + 2;
            while j + 1 < chars.len() {
                if chars[j] == '*' && chars[j + 1] == '*' {
                    end = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(end) = end {
                let bold: String = chars[i + 2..end].iter().collect();
                if !bold.trim().is_empty() {
                    let _ = write!(out, "<strong>{}</strong>", bold);
                    i = end + 2;
                    continue;
                }
            }
        }
        if chars[i] == '[' {
            if let Some(close) = (i + 1..chars.len()).find(|&j| chars[j] == ']') {
                if close + 1 < chars.len() && chars[close + 1] == '(' {
                    let mut parens = None;
                    let mut k = close + 2;
                    while k < chars.len() {
                        if chars[k] == ')' {
                            parens = Some(k);
                            break;
                        }
                        k += 1;
                    }
                    if let Some(parens) = parens {
                        let label: String = chars[i + 1..close].iter().collect();
                        let url: String = chars[close + 2..parens].iter().collect();
                        if !label.trim().is_empty()
                            && !url.contains(' ')
                            && !url.contains(['<', '>'])
                        {
                            let _ = write!(out, "<a href=\"{}\">{}</a>", url, &label);
                            i = parens + 1;
                            continue;
                        }
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// A stable HTML id for a heading or a file path.
fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = true;
    for c in text.chars() {
        let c = if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else {
            '-'
        };
        if c == '-' && prev_dash {
            continue;
        }
        prev_dash = c == '-';
        if out.len() > 80 {
            break;
        }
        out.push(c);
    }
    let mut s = out;
    while s.starts_with('-') {
        s.remove(0);
    }
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() {
        "id".to_string()
    } else {
        s
    }
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Today's date, short ISO — the generation stamp the cover prints.
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// The document's stylesheet: quasi-monochrome, one accent, the same tokens
/// the docs site uses, so a printed workspace reads as part of the same
/// project rather than as a third-party tool's output.
const DOCUMENT_CSS: &str = r#"<style>
:root { --paper:#F8F8F7; --ink:#1B1B19; --ink-soft:#57544E; --ink-faint:#8B8880;
  --accent:#9E3D2B; --accent-soft:rgba(158,61,43,.07); --rule:#E2E1DC; }
* { box-sizing: border-box; }
@page { size: A4; margin: 17mm 15mm 18mm; }
html { background: var(--paper); }
body { margin: 0; color: var(--ink); background: var(--paper);
  font: 10.5pt/1.55 ui-sans-serif, "Segoe UI", "Helvetica Neue", Arial, sans-serif;
  font-kerning: normal; text-rendering: optimizeLegibility; }
h1, h2, h3, h4, h5 { line-height: 1.2; }
code, pre, table.toml th, table.toml td, .file-path, .toc-path {
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace; }
code { font-size: .82em; background: var(--accent-soft); padding: 0 3px;
  border-radius: 2px; color: var(--ink); }
a { color: var(--accent); text-decoration: none; }
pre, table, blockquote { orphans: 3; widows: 3; }

.cover { min-height: 95%; display: flex; flex-direction: column;
  justify-content: center; break-after: page; page-break-after: always; }
.cover-rule { width: 100%; height: 3px; background: var(--ink); margin-bottom: 26pt; }
.cover h1 { font-size: 27pt; font-weight: 650; margin: 0 0 6pt; letter-spacing: -.015em; }
.cover-sub { color: var(--ink-soft); margin: 0 0 22pt; font-size: 9.5pt; }
.cover-sub code { font-size: .9em; }
.cover-stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8pt; margin: 0 0 18pt; }
.cover-stats > div { border-top: 1.5px solid var(--ink); padding-top: 5pt; }
.cover-stats dt { font-size: 7.5pt; letter-spacing: .06em; text-transform: uppercase;
  color: var(--ink-faint); margin: 0 0 1pt; }
.cover-stats dd { margin: 0; font-weight: 600; font-size: 10.5pt; }
.cover-note { color: var(--ink-soft); font-size: 8.5pt; border-left: 2px solid var(--accent);
  padding: 4pt 0 4pt 8pt; margin: 10pt 0 0; }
.cover-note code { background: none; color: inherit; font-size: .95em; }

.toc { break-after: page; page-break-after: always; }
.toc h2 { font-size: 13pt; margin: 0 0 8pt; }
.toc ol { columns: 2; column-gap: 14pt; padding-left: 16pt; margin: 0; }
.toc li { margin: 1.5pt 0; break-inside: avoid; font-size: 8.6pt; }
.toc li a { color: var(--ink); }
.toc-path { color: var(--ink-faint); font-size: 7.5pt; }

.graph { break-after: page; page-break-after: always; }
.graph-note { color: var(--ink-soft); font-size: 8.5pt; }
.mermaid, .mermaid * {
  font-family: ui-sans-serif, "Segoe UI", Arial, sans-serif !important; }
pre.mermaid { background: none; border: none; padding: 0; }
.graph-svg svg { max-width: 100%; height: auto; }
.graph-svg p { font-size: 8.5pt; color: var(--ink-faint); margin: 2pt 0; }

section.file { break-before: page; page-break-before: always; }
.file-path { color: var(--ink-faint); font-size: 7pt; letter-spacing: .05em;
  text-transform: lowercase; }
section.file h2 { font-size: 16pt; font-weight: 650; margin: 4pt 0 10pt; }
section.file h3 { font-size: 11pt; margin: 12pt 0 4pt; border-bottom: 1px solid var(--rule);
  padding-bottom: 2pt; break-after: avoid; page-break-after: avoid; }
section.file h4, section.file h5 { font-size: 10pt; margin: 9pt 0 3pt;
  break-after: avoid; page-break-after: avoid; }
p { margin: 3pt 0; }
ul, ol { margin: 3pt 0 3pt 14pt; padding: 0; }
li { margin: 1.5pt 0; }
hr { border: none; border-top: 1px solid var(--rule); margin: 10pt 0; }
table { border-collapse: collapse; width: 100%; margin: 5pt 0; }
th, td { border: 1px solid var(--rule); padding: 2.5pt 5pt; text-align: left;
  vertical-align: top; font-size: 9pt; }
th { background: #EFEFEC; font-size: 8.6pt; }
pre.code { background: #FFFFFF; border: 1px solid var(--rule); border-radius: 3px;
  padding: 6pt 8pt; font-size: 8.4pt; line-height: 1.45; overflow-wrap: anywhere;
  white-space: pre-wrap; }
table.toml { font-size: 8.6pt; }
table.toml th { background: none; border: none; width: 22%; font-weight: 600;
  color: var(--ink-soft); padding: 2pt 6pt; }
table.toml td { border-left: none; border-right: none; padding: 2pt 6pt; }
table.toml tr { border-bottom: 1px solid var(--rule); }
table.toml td { word-break: break-word; }
blockquote { border-left: 2px solid var(--rule); margin: 5pt 0; padding: 1pt 0 1pt 8pt;
  color: var(--ink-soft); }
</style>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> String {
        let mut md = String::from(
            "# Requirements\n\n## REQ-004 — Reject an over-receipt\n\n```toml\ntype = \"FUNCTIONAL\"\nstatus = \"VALIDATED\"\nepistemic = \"EXPLICIT\"\nderived-from = [\"RULE-001\", \"EVD-014\"]\n```\n\n**Statement**\n\nThe system SHALL reject a receipt whose accepted quantity exceeds the\nremaining quantity of the referenced line.\n\n| Fact | Value |\n|---|---|\n| cycle | 7 days |\n\n- first, read the registry\n- then, run the gate\n\n1. one\n2. two\n",
        );
        md.push_str("\nInspired by [the paper](https://example.com/paper).\n");
        md
    }

    #[test]
    fn a_record_renders_as_a_readable_table_and_toml() {
        let html = markdown_to_html(&sample_record());
        assert!(html.contains("<h2 id=\"requirements\">Requirements</h2>"));
        // The heading slug keeps the whole title; consecutive separators
        // collapse so the TOC id stays readable.
        assert!(
            html.contains("<h3 id=\"req-004-reject-an-over-receipt\">REQ-004"),
            "{html}"
        );
        // The record's toml block renders as a table: keys as rows, arrays joined.
        assert!(html.contains("<table class=\"toml\">"), "{html}");
        assert!(html.contains("derived-from"));
        assert!(html.contains("RULE-001, EVD-014"));
        // The prose keeps its statement, with markdown bold translated.
        assert!(html.contains("<strong>Statement</strong>"), "{html}");
        assert!(html.contains("The system SHALL reject"));
        // The GFM table renders as an HTML table with header cells.
        assert!(html.contains("<th scope=\"col\">Fact</th>"), "{html}");
        assert!(html.contains("<td>7 days</td>"), "{html}");
        // Bullet and numbered lists render as their own list kinds.
        assert!(html.contains("<ul>"), "{html}");
        assert!(html.contains("<li>first, read the registry</li>"), "{html}");
        assert!(html.contains("<ol>"), "{html}");
        assert!(html.contains("<li>one</li>"), "{html}");
        // The link survives with its URL preserved.
        assert!(html.contains("<a href=\"https://example.com/paper\">the paper</a>"));
    }

    #[test]
    fn raw_html_never_passes_through() {
        // A registry file with stray HTML (a real-world typo) must be escaped,
        // never executed in the printed page.
        let html = markdown_to_html("<style>bad</style>");
        assert!(
            !html.contains("<style>bad"),
            "raw HTML must not pass through"
        );
        assert!(html.contains("&lt;style&gt;"), "{html}");
    }

    #[test]
    fn the_section_order_is_gate_order_not_alphabetical() {
        // project and README first, registries in gate order, changes last,
        // and anything unrecognised after — this is what makes two exports
        // of the same workspace comparable file by file.
        assert!(section_order("project.md") < section_order("README.md"));
        assert!(section_order("README.md") < section_order("domain/domain.md"));
        assert!(section_order("domain/domain.md") < section_order("research/markdown.md"));
        assert!(
            section_order("requirements/requirements.md")
                < section_order("changes/2026-01-01-a.md")
        );
        assert!(section_order("changes/change.md") < section_order("elsewhere/notes.md"));
    }

    #[test]
    fn a_broken_toml_block_is_displayed_and_flagged() {
        let html = markdown_to_html("```toml\nnot = = valid\n```");
        assert!(html.contains("data-unparsed=\"1\""), "{html}");
        assert!(html.contains("not = = valid"), "{html}");
    }

    #[test]
    fn the_document_carries_cover_graph_and_every_file_in_gate_order() {
        // The order `collect()` produces; the renderer must keep it as given.
        let files = vec![
            ExportFile {
                path: "project.md".into(),
                title: "Project".into(),
                raw: "# Project\n\npurpose\n".into(),
            },
            ExportFile {
                path: "README.md".into(),
                title: "README".into(),
                raw: "# README\n".into(),
            },
            ExportFile {
                path: "requirements/requirements.md".into(),
                title: "Requirements".into(),
                raw: sample_record(),
            },
            ExportFile {
                path: "validation/audit.md".into(),
                title: "Audit".into(),
                raw: "# Audit\n\nbody\n".into(),
            },
        ];
        let doc = ExportDoc {
            title: "Demo — OpenDomain knowledge base".into(),
            date: "2026-10-26".into(),
            graph: GraphFormat::Mermaid,
            graph_dot: None,
            graph_svg: None,
            graph_mermaid: Some("flowchart LR\n  n_evd_001[\"EVD-001: source\"]\n".into()),
            graph_truncated: false,
            stats: CoverStats {
                name: "Demo".into(),
                profile: "FULL".into(),
                authority: "SPEC_ANCHORED".into(),
                repository: "GREENFIELD".into(),
                artifacts: 3,
                files: files.len(),
                graph_nodes: 1,
                graph_links: 0,
                unresolved_provenance: 0,
                open_conflicts: 0,
                open_uncertainties: 1,
            },
            files,
        };
        let html = render_document(&doc);
        // project.md precedes README, which precedes requirements, which
        // precedes validation — gate order, not alphabetical accident.
        let project = html.find("id=\"project-md\"").expect("project section");
        let readme = html.find("id=\"readme-md\"").expect("readme section");
        let reqs = html
            .find("id=\"requirements-requirements-md\"")
            .expect("requirements section");
        let audit = html
            .find("id=\"validation-audit-md\"")
            .expect("audit section");
        assert!(project < readme && readme < reqs && reqs < audit);
        // The cover quotes the same numbers `opendomain status` reports.
        assert!(html.contains("SPEC_ANCHORED"), "{html}");
        assert!(html.contains("open uncertainties</dt><dd>1</dd>"), "{html}");
        // The graph rides Mermaid from the pinned CDN, escaped.
        assert!(html.contains(MERMAID_CDN), "{html}");
        assert!(html.contains("flowchart LR"));
        // The TOC links every section.
        assert!(html.contains("#requirements-requirements-md"), "{html}");
        // Standalone by construction.
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.trim_end().ends_with("</html>"));
    }

    #[test]
    fn a_truncated_graph_says_so_next_to_the_diagram() {
        let doc = ExportDoc {
            title: "t".into(),
            date: "d".into(),
            graph: GraphFormat::Mermaid,
            graph_dot: None,
            graph_svg: None,
            graph_mermaid: Some("flowchart LR\n".into()),
            graph_truncated: true,
            stats: CoverStats {
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
        let html = render_document(&doc);
        assert!(html.contains("Capped at 200 nodes"), "{html}");
    }

    #[test]
    fn dot_without_a_pre_render_falls_back_to_the_source() {
        let doc = ExportDoc {
            title: "t".into(),
            date: "d".into(),
            graph: GraphFormat::Dot,
            graph_dot: Some(String::from("digraph opendomain {}\n")),
            graph_svg: None,
            graph_mermaid: None,
            graph_truncated: false,
            stats: CoverStats {
                name: "x".into(),
                profile: "p".into(),
                authority: "a".into(),
                repository: "r".into(),
                artifacts: 0,
                files: 0,
                graph_nodes: 0,
                graph_links: 0,
                unresolved_provenance: 0,
                open_conflicts: 0,
                open_uncertainties: 0,
            },
            files: vec![],
        };
        let html = render_document(&doc);
        assert!(html.contains("no SVG was pre-rendered"), "{html}");
        assert!(html.contains("digraph opendomain"), "{html}");
    }

    #[test]
    fn with_no_graph_the_document_has_mermaid_nowhere() {
        let doc = ExportDoc {
            title: "t".into(),
            date: "d".into(),
            graph: GraphFormat::None,
            graph_dot: None,
            graph_svg: None,
            graph_mermaid: None,
            graph_truncated: false,
            stats: CoverStats {
                name: "x".into(),
                profile: "p".into(),
                authority: "a".into(),
                repository: "r".into(),
                artifacts: 0,
                files: 1,
                graph_nodes: 3,
                graph_links: 2,
                unresolved_provenance: 0,
                open_conflicts: 0,
                open_uncertainties: 0,
            },
            files: vec![ExportFile {
                path: "a.md".into(),
                title: "A".into(),
                raw: "# A\nx\n".into(),
            }],
        };
        let html = render_document(&doc);
        assert!(!html.contains(MERMAID_CDN), "{html}");
        assert!(!html.contains("class=\"mermaid\""), "{html}");
    }

    #[test]
    fn slugs_are_stable_and_unique_enough_for_the_toc() {
        assert_eq!(
            slug("requirements/requirements.md"),
            "requirements-requirements-md"
        );
        assert_eq!(slug("project.md"), "project-md");
        assert_eq!(slug("---"), "id");
    }

    #[test]
    fn list_parsing_never_slices_across_a_multibyte_character() {
        // `é` is not a digit: the line stays a bullet, and no byte offset
        // computed from a char count ever lands inside a character.
        let (_, ordered, text) = list_entry("é10. two");
        assert!(!ordered, "é is not a digit, the line stays a bullet");
        assert_eq!(text, "é10. two");
        // A real numbered marker kept whole.
        let (_, ordered, text) = list_entry("10. two");
        assert!(ordered);
        assert_eq!(text, "two");
    }
}
