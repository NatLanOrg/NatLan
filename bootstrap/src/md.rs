// A1 Parse: split a markdown document into a tree of sections (deterministic, no LLM).
use crate::model::SectionBody;
use std::collections::{BTreeMap, HashMap};

// Return the reference of the most specific (deepest, by reference length) section
// whose [start_line, end_line) range contains `line`.
#[allow(dead_code)] // available for cursor->section mapping; navigation currently maps via entities
pub fn section_at_line(sections: &BTreeMap<String, SectionBody>, line: usize) -> Option<String> {
    let mut best: Option<(&String, usize)> = None;
    for (reference, s) in sections {
        if line >= s.start_line && line < s.end_line {
            let depth = reference.matches('/').count();
            if best.map(|(_, d)| depth > d).unwrap_or(true) {
                best = Some((reference, depth));
            }
        }
    }
    best.map(|(r, _)| r.clone())
}

// The first line of a section's range, for mapping a node id to an LSP position.
pub fn section_line(sections: &BTreeMap<String, SectionBody>, reference: &str) -> usize {
    sections.get(reference).map(|s| s.start_line).unwrap_or(0)
}

// Locate `needle` as a verbatim substring of `text` (possibly multi-line). Returns the
// 0-based (start_line, start_col, end_line, end_col) in character columns, or None.
// Used to turn an LLM-chosen evidence snippet into a precise editor range.
pub fn locate(text: &str, needle: &str) -> Option<(usize, usize, usize, usize)> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }
    let byte = text.find(needle)?;
    let end = byte + needle.len();
    let (sl, sc) = line_col(text, byte);
    let (el, ec) = line_col(text, end);
    Some((sl, sc, el, ec))
}

// 0-based (line, char column) of a byte offset within `text`.
pub fn line_col(text: &str, byte: usize) -> (usize, usize) {
    let before = &text[..byte.min(text.len())];
    let line = before.matches('\n').count();
    let col = before.rsplit('\n').next().unwrap_or("").chars().count();
    (line, col)
}

// Whole-word, case-insensitive occurrences of `needle` in `text`, returned as
// (line, start_col, len) using 0-based UTF-16-free char columns (good enough for ASCII docs).
pub fn occurrences(text: &str, needle: &str) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    if needle.trim().is_empty() {
        return out;
    }
    let nlow = needle.to_lowercase();
    let nlen = needle.chars().count();
    for (lineno, line) in text.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let lower: String = line.to_lowercase();
        let mut start = 0usize;
        while let Some(byte_idx) = lower[start..].find(&nlow) {
            let abs = start + byte_idx;
            // Translate byte index to char column.
            let col = lower[..abs].chars().count();
            let before_ok = col == 0 || !chars[col - 1].is_alphanumeric();
            let after_idx = col + nlen;
            let after_ok = after_idx >= chars.len() || !chars[after_idx].is_alphanumeric();
            if before_ok && after_ok {
                out.push((lineno, col, nlen));
            }
            start = abs + nlow.len();
            if start > lower.len() {
                break;
            }
        }
    }
    out
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.trim().to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

struct Head {
    level: usize,
    title: String,
    line: usize,
}

pub fn parse_sections(text: &str) -> BTreeMap<String, SectionBody> {
    let lines: Vec<&str> = text.lines().collect();
    let mut heads: Vec<Head> = Vec::new();
    let mut in_code = false;
    for (i, l) in lines.iter().enumerate() {
        if l.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let t = l.trim_start();
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
            heads.push(Head {
                level: hashes,
                title: t[hashes..].trim().to_string(),
                line: i,
            });
        }
    }

    let mut sections: BTreeMap<String, SectionBody> = BTreeMap::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut sibling_counts: HashMap<String, usize> = HashMap::new();
    for (idx, h) in heads.iter().enumerate() {
        while let Some(top) = stack.last() {
            if top.0 >= h.level {
                stack.pop();
            } else {
                break;
            }
        }
        let parent_ref = if stack.is_empty() {
            None
        } else {
            Some(format!("/{}", stack.iter().map(|(_, s)| s.clone()).collect::<Vec<_>>().join("/")))
        };
        let sl = slug(&h.title);
        let path: Vec<String> = stack
            .iter()
            .map(|(_, s)| s.clone())
            .chain(std::iter::once(sl.clone()))
            .collect();
        let reference = format!("/{}", path.join("/"));
        let pkey = parent_ref.clone().unwrap_or_else(|| "/".to_string());
        let order = {
            let c = sibling_counts.entry(pkey).or_insert(0);
            let v = *c;
            *c += 1;
            v
        };
        let end = if idx + 1 < heads.len() {
            heads[idx + 1].line
        } else {
            lines.len()
        };
        let raw = lines[h.line..end].join("\n");
        let kind = if stack.is_empty() && idx == 0 { "root" } else { "heading" };
        sections.insert(
            reference,
            SectionBody {
                title: h.title.clone(),
                kind: kind.to_string(),
                order,
                parent: parent_ref,
                raw,
                start_line: h.line,
                end_line: end,
            },
        );
        stack.push((h.level, sl));
    }
    sections
}
