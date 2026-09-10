//! Linear pre-parse scan for prohibited declarations, depth, and element count.
//!
//! `roxmltree` builds its tree recursively and has no nesting limit, so a
//! deeply nested document has to be rejected before it runs. The scan is a
//! single pass over the decoded text that skips comments, CDATA sections, and
//! processing instructions. It only has to be exact for well-formed input:
//! anything it miscounts on malformed input is rejected by the parser
//! afterwards, and it stops at the first construct it cannot follow.

use crate::form::{Limits, Rejection};

/// What the scan counted before it stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Counts {
    /// Element start tags seen.
    pub(crate) elements: usize,
    /// Deepest element nesting, with the root counted as one.
    pub(crate) max_depth: usize,
}

/// Scan `text` for prohibited declarations and the depth and element limits.
pub(crate) fn prescan(text: &str, limits: &Limits) -> Result<Counts, Rejection> {
    let bytes = text.as_bytes();
    let mut counts = Counts::default();
    let mut depth = 0usize;
    let mut cursor = 0usize;

    while let Some(offset) = bytes[cursor..].iter().position(|&byte| byte == b'<') {
        let start = cursor + offset;
        let rest = &bytes[start..];
        cursor = if rest.starts_with(b"<!--") {
            match skip_to(bytes, start + 4, b"-->") {
                Some(next) => next,
                None => break,
            }
        } else if rest.starts_with(b"<![CDATA[") {
            match skip_to(bytes, start + 9, b"]]>") {
                Some(next) => next,
                None => break,
            }
        } else if rest.starts_with(b"<!") {
            return Err(Rejection::ProhibitedConstruct);
        } else if rest.starts_with(b"<?") {
            match skip_to(bytes, start + 2, b"?>") {
                Some(next) => next,
                None => break,
            }
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            match scan_tag(bytes, start) {
                Some((next, _)) => next,
                None => break,
            }
        } else {
            counts.elements += 1;
            depth += 1;
            counts.max_depth = counts.max_depth.max(depth);
            if counts.elements > limits.elements || depth > limits.depth {
                return Err(Rejection::LimitExceeded);
            }
            match scan_tag(bytes, start) {
                Some((next, self_closing)) => {
                    if self_closing {
                        depth -= 1;
                    }
                    next
                }
                None => break,
            }
        };
    }

    Ok(counts)
}

/// Index just past the next occurrence of `needle` at or after `from`.
fn skip_to(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from > bytes.len() {
        return None;
    }
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset + needle.len())
}

/// Index just past a tag's `>`, and whether the tag closed itself.
///
/// Quoted attribute values may contain `>`, so quoting is tracked.
fn scan_tag(bytes: &[u8], start: usize) -> Option<(usize, bool)> {
    let mut quote: Option<u8> = None;
    for (index, &byte) in bytes.iter().enumerate().skip(start + 1) {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => {
                return Some((index + 1, bytes[index - 1] == b'/'));
            }
            None => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &str) -> Result<Counts, Rejection> {
        prescan(text, &Limits::default())
    }

    #[test]
    fn nesting_and_elements_are_counted() {
        let counts = scan("<a><b><c/></b><b/></a>").expect("well-formed input");
        assert_eq!(counts.elements, 4);
        assert_eq!(counts.max_depth, 3);
    }

    #[test]
    fn comments_cdata_and_instructions_are_skipped() {
        let text = "<?xml version='1.0'?><!-- <b><b> --><a><![CDATA[<c><c>]]></a>";
        let counts = scan(text).expect("well-formed input");
        assert_eq!(counts.elements, 1);
        assert_eq!(counts.max_depth, 1);
    }

    #[test]
    fn a_doctype_is_a_prohibited_construct() {
        for text in [
            "<!DOCTYPE a><a/>",
            "<!ENTITY x 'y'><a/>",
            "<a><!DOCTYPE b/></a>",
        ] {
            assert_eq!(scan(text), Err(Rejection::ProhibitedConstruct), "{text}");
        }
    }

    #[test]
    fn quoted_attribute_values_may_contain_markup_characters() {
        let counts = scan("<a href=\"a>b\" other='c>d'><b/></a>").expect("well-formed input");
        assert_eq!(counts.elements, 2);
        assert_eq!(counts.max_depth, 2);
    }

    #[test]
    fn depth_at_the_limit_passes_and_one_past_it_fails() {
        let limits = Limits::default();
        let at_limit = "<a>".repeat(limits.depth) + &"</a>".repeat(limits.depth);
        assert_eq!(
            prescan(&at_limit, &limits)
                .expect("the limit itself is allowed")
                .max_depth,
            limits.depth
        );
        let past = "<a>".repeat(limits.depth + 1) + &"</a>".repeat(limits.depth + 1);
        assert_eq!(prescan(&past, &limits), Err(Rejection::LimitExceeded));
    }

    #[test]
    fn element_count_at_the_limit_passes_and_one_past_it_fails() {
        let limits = Limits {
            elements: 8,
            ..Limits::default()
        };
        assert!(prescan(&"<a/>".repeat(8), &limits).is_ok());
        assert_eq!(
            prescan(&"<a/>".repeat(9), &limits),
            Err(Rejection::LimitExceeded)
        );
    }

    #[test]
    fn a_hundred_thousand_levels_are_rejected_without_recursion() {
        let deep = "<a>".repeat(100_000);
        assert_eq!(scan(&deep), Err(Rejection::LimitExceeded));
    }

    #[test]
    fn unterminated_constructs_stop_the_scan_for_the_parser() {
        for text in ["<!-- unterminated", "<![CDATA[ open", "<?pi open", "<a"] {
            assert!(scan(text).is_ok(), "{text}");
        }
    }
}
