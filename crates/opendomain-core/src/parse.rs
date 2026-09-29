//! Parser for OpenDomain Markdown registry files.
//!
//! The format is deliberately boring: an `##`-level heading whose first token
//! is a known identifier, an optional fenced `toml` block, then prose. The
//! parser never guesses — an unparseable block is an error, not a silent
//! fallback — because a registry that quietly drops requirements is worse than
//! one that refuses to load.

use crate::artifact::{Artifact, Document, Promotion, Relation};
use crate::error::{OdError, Result};
use crate::ids::{Id, IdKind};
use crate::vocab::ArtifactStatus;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// TOML keys that are lifted into typed fields rather than left as extras.
///
/// Anything not in this list is preserved in [`Artifact::extra`] so a
/// user-defined field never causes data loss.
const TYPED_KEYS: &[&str] = &["status", "epistemic", "confidence", "promotions"];

/// Parses one registry file.
pub fn parse(_path: &Path, relative: PathBuf, text: &str) -> Result<Document> {
    // Worked examples live inside HTML comments in the shipped templates, so
    // stripping comments first is what keeps `od init` from creating a
    // workspace that already contains fabricated requirements.
    let text = strip_html_comments(text);
    let lines: Vec<&str> = text.lines().collect();
    let mut preamble = String::new();
    let mut artifacts: Vec<Artifact> = Vec::new();

    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx];

        // Artifact heading: `## <ID> — <title>` or `## <ID> <title>`
        if let Some(rest) = heading_body(line) {
            let (id_raw, title) = split_id_and_title(rest);
            if !id_raw.is_empty() {
                let id = match Id::parse(id_raw) {
                    Ok(id) => id,
                    // A heading that starts with an uppercase token followed by a
                    // dash is intended to be an artifact; failing loudly is
                    // correct, but a heading like "## Notes" is just prose.
                    Err(_) if looks_like_id_attempt(id_raw) => {
                        return Err(OdError::artifact(
                            &relative,
                            format!("line {}: `{id_raw}` is not a valid identifier", idx + 1),
                        ))
                    }
                    Err(_) => {
                        preamble.push_str(line);
                        preamble.push('\n');
                        idx += 1;
                        continue;
                    }
                };

                let (block, body_start) = read_toml_block(&lines, idx + 1);
                let body_end = find_next_heading(&lines, body_start);
                let body = lines[body_start..body_end].join("\n");

                let mut artifact = build(id, title, &relative, idx + 1, block.as_deref())?;
                artifact.body = body.trim().to_string();
                artifacts.push(artifact);
                idx = body_end;
                continue;
            }
        }

        preamble.push_str(line);
        preamble.push('\n');
        idx += 1;
    }

    check_duplicate_ids(&artifacts, &relative)?;
    Ok(Document {
        path: relative,
        preamble: preamble.trim().to_string(),
        artifacts,
    })
}

/// Blanks out HTML comments while preserving line structure, so every reported
/// line number still points at the real line of the real file.
///
/// Fenced code blocks are respected: a `<!--` inside an example of HTML is not
/// a comment.
fn strip_html_comments(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_comment = false;
    let mut in_fence = false;

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push(line.to_string());
            continue;
        }
        if in_fence {
            out.push(line.to_string());
            continue;
        }

        let mut kept = String::with_capacity(line.len());
        let mut rest = line;

        while !rest.is_empty() {
            if in_comment {
                match rest.find("-->") {
                    Some(end) => {
                        rest = &rest[end + 3..];
                        in_comment = false;
                    }
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(start) => {
                        kept.push_str(&rest[..start]);
                        rest = &rest[start + 4..];
                        in_comment = true;
                    }
                    None => {
                        kept.push_str(rest);
                        break;
                    }
                }
            }
        }

        // Pad to the original width so column-based diagnostics stay aligned.
        let target = line.chars().count();
        let current = kept.chars().count();
        if current < target {
            kept.push_str(&" ".repeat(target - current));
        }
        out.push(kept.trim_end().to_string());
    }

    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

/// Returns the text after `## ` when the line is a level-2 heading.
fn heading_body(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("## ")?;
    // `###` and deeper are sub-sections of the current artifact.
    if rest.starts_with('#') {
        return None;
    }
    Some(rest.trim())
}

/// Splits `REQ-001 — Title` or `REQ-001 Title` into id and title.
fn split_id_and_title(rest: &str) -> (&str, String) {
    let trimmed = rest.trim_start();
    let mut end = 0;
    for (i, c) in trimmed.char_indices() {
        if c.is_whitespace() {
            end = i;
            break;
        }
        end = i + c.len_utf8();
    }
    let (id_raw, title) = trimmed.split_at(end);
    // Whitespace first, then the separator, then whitespace again: the space
    // between `REQ-001` and the dash would otherwise defeat the separator strip
    // and leave the title as "- Title".
    let title = title
        .trim()
        .trim_start_matches(['—', '–', '-', ':', '|'])
        .trim()
        .to_string();
    (id_raw, title)
}

/// Heuristic for "this heading was meant to hold an identifier".
///
/// Two shapes count: `REQ-FOO` (a prefix with a malformed number) and a bare
/// known prefix like `REQ` with the number simply missing. Prose headings such
/// as `## Overview` match neither, because `OVERVIEW` is not a prefix and does
/// not contain a dash.
fn looks_like_id_attempt(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if let Some((prefix, rest)) = token.split_once('-') {
        return !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_uppercase())
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    }
    IdKind::from_prefix(token).is_ok()
}

/// Reads the fenced TOML block starting at `start`, returning it and the index
/// of the first line after the block.
fn read_toml_block(lines: &[&str], start: usize) -> (Option<String>, usize) {
    let mut i = start;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    if i >= lines.len() {
        return (None, lines.len());
    }
    let fence = lines[i].trim();
    if !(fence == "```toml" || fence == "``` toml") {
        return (None, start);
    }
    let mut buf = String::new();
    let mut j = i + 1;
    while j < lines.len() && lines[j].trim() != "```" {
        buf.push_str(lines[j]);
        buf.push('\n');
        j += 1;
    }
    // If the fence is never closed, treat the whole remainder as the block so
    // the TOML error message points at the real problem.
    if j < lines.len() {
        (Some(buf), j + 1)
    } else {
        (Some(buf), lines.len())
    }
}

fn find_next_heading(lines: &[&str], start: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(start) {
        if line.starts_with("## ") {
            return i;
        }
    }
    lines.len()
}

fn build(id: Id, title: String, file: &Path, line: usize, block: Option<&str>) -> Result<Artifact> {
    let mut kind_fields: BTreeMap<String, String> = BTreeMap::new();
    let mut links: BTreeMap<Relation, Vec<Id>> = BTreeMap::new();
    let mut extra: BTreeMap<String, toml::Value> = BTreeMap::new();
    let mut status = ArtifactStatus::Draft;
    let mut epistemic = None;
    let mut confidence = None;
    let mut promotions: Vec<Promotion> = Vec::new();

    if let Some(text) = block {
        let value: toml::Value = toml::from_str(text).map_err(|e| {
            OdError::artifact(file, format!("line {line}: invalid TOML block — {e}"))
        })?;
        let table = value.as_table().ok_or_else(|| {
            OdError::artifact(file, format!("line {line}: TOML block must be a table"))
        })?;

        for (key, val) in table {
            match key.as_str() {
                "id" => {
                    // A declared `id` must agree with the heading, otherwise
                    // `od trace REQ-001` would resolve to a differently named
                    // artifact depending on which field you read.
                    if let Some(declared) = val.as_str() {
                        let declared = Id::parse(declared)
                            .map_err(|e| OdError::artifact(file, format!("line {line}: {e}")))?;
                        if declared != id {
                            return Err(OdError::artifact(
                                file,
                                format!(
                                    "line {line}: TOML `id` is `{declared}` but the heading says `{id}`"
                                ),
                            ));
                        }
                    }
                }
                "status" => {
                    let raw = expect_string(file, line, key, val)?;
                    status = raw.parse().map_err(|e: String| {
                        OdError::artifact(file, format!("line {line}: {e}"))
                    })?;
                }
                "epistemic" => {
                    let raw = expect_string(file, line, key, val)?;
                    epistemic = Some(raw.parse().map_err(|e: String| {
                        OdError::artifact(file, format!("line {line}: {e}"))
                    })?);
                }
                "confidence" => {
                    let raw = expect_string(file, line, key, val)?;
                    confidence = Some(raw.parse().map_err(|e: String| {
                        OdError::artifact(file, format!("line {line}: {e}"))
                    })?);
                }
                "promotions" => {
                    promotions = parse_promotions(file, line, val)?;
                }
                _ => {
                    if let Some(rel) = Relation::parse(key) {
                        let ids = parse_id_list(file, line, key, val)?;
                        if !ids.is_empty() {
                            links.entry(rel).or_default().extend(ids);
                        }
                    } else if TYPED_KEYS.contains(&key.as_str()) {
                        kind_fields.insert(key.clone(), to_display(val));
                    } else {
                        kind_fields.insert(key.clone(), to_display(val));
                        extra.insert(key.clone(), val.clone());
                    }
                }
            }
        }
    }

    Ok(Artifact {
        id,
        title,
        file: file.to_path_buf(),
        line,
        kind_fields,
        status,
        epistemic,
        confidence,
        links,
        promotions,
        extra,
        body: String::new(),
    })
}

/// Reads the `promotions` array: an ordered list of epistemic promotions.
///
/// A malformed entry is an error rather than a skipped row. A promotion record
/// that silently fails to load is indistinguishable from no promotion having
/// happened, which is precisely the ambiguity the record exists to remove.
fn parse_promotions(file: &Path, line: usize, val: &toml::Value) -> Result<Vec<Promotion>> {
    let items = val.as_array().ok_or_else(|| {
        OdError::artifact(
            file,
            format!(
                "line {line}: `promotions` must be an array of tables, found {}",
                val.type_str()
            ),
        )
    })?;

    let mut out = Vec::with_capacity(items.len());
    for (n, item) in items.iter().enumerate() {
        let at = line + 1;
        let table = item.as_table().ok_or_else(|| {
            OdError::artifact(
                file,
                format!("line {line}: `promotions[{n}]` must be a table"),
            )
        })?;

        let from = match table.get("from") {
            None => None,
            Some(v) => {
                let raw = expect_string(file, at, &format!("promotions[{n}].from"), v)?;
                Some(raw.parse().map_err(|e: String| {
                    OdError::artifact(file, format!("line {line}: `promotions[{n}].from`: {e}"))
                })?)
            }
        };

        let to_raw = table.get("to").ok_or_else(|| {
            OdError::artifact(
                file,
                format!("line {line}: `promotions[{n}]` must declare `to`"),
            )
        })?;
        let to: ArtifactStatus = expect_string(file, at, &format!("promotions[{n}].to"), to_raw)?
            .parse()
            .map_err(|e: String| {
                OdError::artifact(file, format!("line {line}: `promotions[{n}].to`: {e}"))
            })?;

        let evidence = match table.get("evidence") {
            None => Vec::new(),
            Some(v) => parse_id_list(file, at, &format!("promotions[{n}].evidence"), v)?,
        };

        let date = match table.get("date") {
            None => None,
            Some(v) => Some(expect_string(
                file,
                at,
                &format!("promotions[{n}].date"),
                v,
            )?),
        };

        out.push(Promotion {
            from,
            to,
            evidence,
            date,
        });
    }
    Ok(out)
}

fn expect_string(file: &Path, line: usize, key: &str, val: &toml::Value) -> Result<String> {
    match val {
        toml::Value::String(s) => Ok(s.clone()),
        other => Err(OdError::artifact(
            file,
            format!(
                "line {line}: `{key}` must be a string, found {}",
                other.type_str()
            ),
        )),
    }
}

/// Accepts `"REQ-001"` or `["REQ-001", "REQ-002"]` for a relation key.
fn parse_id_list(file: &Path, line: usize, key: &str, val: &toml::Value) -> Result<Vec<Id>> {
    let mut out = Vec::new();
    match val {
        toml::Value::String(s) => {
            let id =
                Id::parse(s).map_err(|e| OdError::artifact(file, format!("line {line}: {e}")))?;
            out.push(id);
        }
        toml::Value::Array(items) => {
            for item in items {
                let s = item.as_str().ok_or_else(|| {
                    OdError::artifact(
                        file,
                        format!("line {line}: `{key}` must contain identifier strings"),
                    )
                })?;
                let id = Id::parse(s)
                    .map_err(|e| OdError::artifact(file, format!("line {line}: {e}")))?;
                out.push(id);
            }
        }
        other => {
            return Err(OdError::artifact(
                file,
                format!(
                    "line {line}: `{key}` must be a string or an array of strings, found {}",
                    other.type_str()
                ),
            ))
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn to_display(val: &toml::Value) -> String {
    match val {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(items) => items.iter().map(to_display).collect::<Vec<_>>().join(", "),
        toml::Value::Table(t) => t
            .iter()
            .map(|(k, v)| format!("{k}={}", to_display(v)))
            .collect::<Vec<_>>()
            .join(", "),
        toml::Value::Datetime(d) => d.to_string(),
    }
}

fn check_duplicate_ids(artifacts: &[Artifact], file: &Path) -> Result<()> {
    let mut seen: BTreeSet<Id> = BTreeSet::new();
    for a in artifacts {
        if !seen.insert(a.id.clone()) {
            return Err(OdError::artifact(
                file,
                format!("line {}: duplicate identifier `{}`", a.line, a.id),
            ));
        }
    }
    Ok(())
}

/// Collects every identifier that appears anywhere in `text`.
///
/// Used by `od link --check` to catch dangling references in prose and tables
/// that the TOML block does not cover.
pub fn scan_ids(text: &str) -> Vec<Id> {
    let mut found = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        // An identifier starts with 2+ uppercase letters.
        if !(bytes[i].is_ascii_uppercase()) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_uppercase() {
            i += 1;
        }
        if i - start < 2 {
            continue;
        }
        // Consume `-` segments while they look like identifier parts.
        let token_start = start;
        let mut j = i;
        while j < bytes.len() && bytes[j] == '-' {
            let seg_start = j + 1;
            let mut k = seg_start;
            while k < bytes.len() && (bytes[k].is_ascii_uppercase() || bytes[k].is_ascii_digit()) {
                k += 1;
            }
            if k == seg_start || !bytes[seg_start].is_ascii_alphanumeric() {
                break;
            }
            j = k;
        }
        // A valid identifier ends in a number, so test the last character
        // consumed rather than the character that follows it: `REQ-001,` ends at
        // the comma, and the digit sits at `j - 1`.
        if j > i && bytes[j - 1].is_ascii_digit() {
            let token: String = bytes[token_start..j].iter().collect();
            if let Ok(id) = Id::parse(&token) {
                found.push(id);
            }
        }
        i = j.max(i + 1);
    }
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::IdKind;
    use crate::vocab::Stage;
    use crate::vocab::{Confidence, KnowledgeType};

    fn doc(text: &str) -> Result<Document> {
        parse(
            Path::new("domain/business-rules.md"),
            PathBuf::from("domain/business-rules.md"),
            text,
        )
    }

    #[test]
    fn parses_a_full_artifact() {
        let text = r#"
# Business rules

## RULE-007 — Stock cannot go negative

```toml
type = "OPERATIONAL"
status = "VALIDATED"
epistemic = "EXPLICIT"
confidence = "HIGH"
scope = "CORE"
derived-from = ["EVD-014"]
owned-by = "DOM-002"
```

**Statement**

A confirmed receipt may never drive available stock below zero.
"#;
        let d = doc(text).unwrap();
        assert_eq!(d.title(), Some("Business rules"));
        assert_eq!(d.artifacts.len(), 1);
        let a = &d.artifacts[0];
        assert_eq!(a.id.kind(), IdKind::Rule);
        assert_eq!(a.id.number(), 7);
        assert_eq!(a.title, "Stock cannot go negative");
        assert_eq!(a.status, ArtifactStatus::Validated);
        assert_eq!(a.epistemic, Some(KnowledgeType::Explicit));
        assert_eq!(a.confidence, Some(Confidence::High));
        assert_eq!(a.field("type"), Some("OPERATIONAL"));
        assert_eq!(a.field("scope"), Some("CORE"));
        // `derived-from` and `owned-by` are both upstream: they state what the
        // rule rests on, not what it produces.
        assert_eq!(
            a.upstream(),
            vec![Id::parse("EVD-014").unwrap(), Id::parse("DOM-002").unwrap()]
        );
        assert!(a.downstream().is_empty());
        assert!(a.body.contains("available stock below zero"));
    }

    #[test]
    fn accepts_scoped_headings_and_dashes() {
        let text = "## REQ-INVENTORY-012 - Consult movement history\n\nbody\n";
        let d = doc(text).unwrap();
        let a = &d.artifacts[0];
        assert_eq!(a.id.to_string(), "REQ-INVENTORY-012");
        assert_eq!(a.title, "Consult movement history");
        assert_eq!(a.id.stage(), Stage::Specification);
    }

    #[test]
    fn ignores_level_three_headings() {
        let text = "## REQ-001 — Title\n\n### Scenario: ok\n\nbody\n\n## REQ-002 — Other\n\nbody\n";
        let d = doc(text).unwrap();
        assert_eq!(d.artifacts.len(), 2);
        assert!(d.artifacts[0].body.contains("### Scenario: ok"));
    }

    #[test]
    fn rejects_heading_and_toml_id_mismatch() {
        let text = "## REQ-001 — Title\n\n```toml\nid = \"REQ-002\"\n```\n";
        let err = doc(text).unwrap_err();
        assert!(format!("{err}").contains("heading says"), "{err}");
    }

    #[test]
    fn rejects_invalid_status() {
        let text = "## REQ-001 — Title\n\n```toml\nstatus = \"FINISHED\"\n```\n";
        let err = doc(text).unwrap_err();
        assert!(format!("{err}").contains("unknown status"), "{err}");
    }

    #[test]
    fn rejects_malformed_relation_values() {
        let text = "## REQ-001 — Title\n\n```toml\nderived-from = 42\n```\n";
        let err = doc(text).unwrap_err();
        assert!(format!("{err}").contains("must be a string"), "{err}");
    }

    #[test]
    fn rejects_dangling_relation_ids() {
        let text = "## REQ-001 — Title\n\n```toml\nderived-from = [\"nope\"]\n```\n";
        assert!(doc(text).is_err());
    }

    #[test]
    fn rejects_duplicate_ids_in_one_file() {
        let text = "## REQ-001 — A\n\nx\n\n## REQ-001 — B\n\ny\n";
        let err = doc(text).unwrap_err();
        assert!(format!("{err}").contains("duplicate identifier"), "{err}");
    }

    #[test]
    fn defaults_missing_status_to_draft() {
        let d = doc("## REQ-001 — Title\n\nno block\n").unwrap();
        assert_eq!(d.artifacts[0].status, ArtifactStatus::Draft);
    }

    #[test]
    fn treats_prose_headings_as_preamble() {
        let d = doc("# Title\n\n## Overview\n\nSome text\n").unwrap();
        assert!(d.artifacts.is_empty());
        assert!(d.preamble.contains("## Overview"));
    }

    #[test]
    fn reports_bad_identifier_headings() {
        let err = doc("## REQ — Missing number\n\nbody\n").unwrap_err();
        assert!(format!("{err}").contains("not a valid identifier"), "{err}");
    }

    #[test]
    fn scans_ids_in_prose() {
        let ids = scan_ids("See REQ-001 and EVD-014, plus AC-INVENTORY-012-01. Not FOO-1 or BAR.");
        assert!(ids.contains(&Id::parse("REQ-001").unwrap()));
        assert!(ids.contains(&Id::parse("EVD-014").unwrap()));
        assert!(ids.contains(&Id::parse("AC-INVENTORY-012-01").unwrap()));
        assert_eq!(ids.len(), 3, "got {ids:?}");
    }

    #[test]
    fn preserves_unknown_keys_as_extras() {
        let text = "## HYP-001 — Users will adopt\n\n```toml\ncategory = \"USER\"\nmetric = \"retention\"\n```\n";
        let d = doc(text).unwrap();
        let a = &d.artifacts[0];
        assert_eq!(a.field("category"), Some("USER"));
        assert_eq!(a.extra.len(), 2);
    }

    #[test]
    fn html_commented_examples_are_ignored() {
        let text = r#"
## REQ-001 — Real

```toml
derived-from = ["RULE-001"]
```

body

<!--
## REQ-999 — Example only

```toml
derived-from = ["NOPE"]
```
-->

## REQ-002 — Also real

```toml
derived-from = ["RULE-001"]
```
"#;
        let d = doc(text).unwrap();
        let ids: Vec<String> = d.artifacts.iter().map(|a| a.id.to_string()).collect();
        assert_eq!(ids, vec!["REQ-001", "REQ-002"], "got {ids:?}");
    }

    #[test]
    fn html_inside_a_fence_is_not_a_comment() {
        let text = "## REQ-001 — T\n\n```html\n<!-- a real comment -->\n```\n\nbody\n";
        let d = doc(text).unwrap();
        assert!(
            d.artifacts[0].body.contains("<!-- a real comment -->"),
            "{:?}",
            d.artifacts[0].body
        );
    }

    #[test]
    fn multi_line_comments_are_handled() {
        let text = "## REQ-001 — T\n\nx\n\n<!--\nline one\nline two\n-->\n\n## REQ-002 — U\n\ny\n";
        let d = doc(text).unwrap();
        let ids: Vec<String> = d.artifacts.iter().map(|a| a.id.to_string()).collect();
        assert_eq!(ids, vec!["REQ-001", "REQ-002"]);
    }

    #[test]
    fn unterminated_comment_blanks_the_rest() {
        let text = "## REQ-001 — T\n\nx\n\n<!-- never closed\n## REQ-999 — Nope\n";
        let d = doc(text).unwrap();
        assert_eq!(d.artifacts.len(), 1);
    }

    #[test]
    fn comment_stripping_preserves_line_numbers() {
        // The heading sits on line 6 of the original, after a five-line comment.
        let text = "a\n<!--\nxx\nxx\n-->\n## REQ-001 — T\n\nbody\n";
        let d = doc(text).unwrap();
        assert_eq!(
            d.artifacts[0].line, 6,
            "line numbers must survive stripping"
        );
    }
}
