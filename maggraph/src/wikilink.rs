//! Wikilink parsing for MagGraph edges.
//!
//! # Supported syntax
//!
//! | Form | Example | Resolved target |
//! |------|---------|-----------------|
//! | Basic | `[[getting_started]]` | `getting_started` |
//! | Display alias | `[[getting_started\|Start here]]` | `getting_started` (Obsidian-style: target before `\|`) |
//! | Heading anchor | `[[welcome#intro]]` | `welcome` (fragment ignored for graph edges) |
//! | Alias + heading | `[[welcome#intro\|Intro]]` | `welcome` |
//!
//! Wikilinks inside fenced code blocks (triple backticks) are ignored.
//! Inline `` `[[...]]` `` code spans are not stripped in v0.1 (may produce false positives).

use std::collections::HashSet;

/// Raw inner text of a wikilink (before target normalization).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikilinkMatch {
    pub raw: String,
    pub start: usize,
    pub end: usize,
}

/// Normalize wikilink inner text to a link target (id or path stem).
///
/// Strips display aliases (`target|alias`) and heading fragments (`target#heading`).
pub fn normalize_wikilink_target(inner: &str) -> String {
    let without_alias = inner.split('|').next().unwrap_or(inner);
    without_alias
        .split('#')
        .next()
        .unwrap_or(without_alias)
        .trim()
        .to_string()
}

/// Extract all wikilink targets from markdown body text.
///
/// Returns deduplicated targets in document order (first occurrence wins).
pub fn extract_wikilink_targets(body: &str) -> Vec<String> {
    let stripped = strip_fenced_code_blocks(body);
    let mut seen = HashSet::new();
    let mut targets = Vec::new();

    for m in extract_wikilinks(&stripped) {
        let target = normalize_wikilink_target(&m.raw);
        if target.is_empty() {
            continue;
        }
        if seen.insert(target.clone()) {
            targets.push(target);
        }
    }

    targets
}

/// Extract wikilink matches with byte offsets (after stripping fenced blocks).
pub fn extract_wikilinks(body: &str) -> Vec<WikilinkMatch> {
    let mut matches = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;

    while i + 4 <= bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(close) = body[i + 2..].find("]]") {
                let start = i;
                let end = i + 2 + close + 2;
                let inner = &body[i + 2..end - 2];
                matches.push(WikilinkMatch {
                    raw: inner.to_string(),
                    start,
                    end,
                });
                i = end;
                continue;
            }
        }
        i += 1;
    }

    matches
}

fn strip_fenced_code_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"```" {
            if let Some(close) = text[i + 3..].find("```") {
                i += 3 + close + 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_wikilink() {
        assert_eq!(
            extract_wikilink_targets("See [[getting_started]] for details."),
            vec!["getting_started"]
        );
    }

    #[test]
    fn parses_alias_and_heading() {
        assert_eq!(
            extract_wikilink_targets("[[welcome#intro|Back to welcome]]"),
            vec!["welcome"]
        );
    }

    #[test]
    fn deduplicates_targets() {
        assert_eq!(
            extract_wikilink_targets("[[a]] and [[a]] again."),
            vec!["a"]
        );
    }

    #[test]
    fn ignores_fenced_code_blocks() {
        let body = r#"
Normal [[real_link]].

```
[[fake_in_code]]
```
"#;
        assert_eq!(extract_wikilink_targets(body), vec!["real_link"]);
    }

    #[test]
    fn normalize_strips_alias_and_heading() {
        assert_eq!(normalize_wikilink_target("page#section|Display"), "page");
    }
}
