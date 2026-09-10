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

/// Scan `text` for prohibited declarations and the pre-parse limits.
///
/// # Errors
///
/// Returns the rejection code for the first prohibited construct or exceeded
/// limit. The counts the scan keeps are an implementation detail; the reported
/// structure comes from the parsed tree.
pub(crate) fn prescan(text: &str, limits: &Limits) -> Result<(), Rejection> {
    scan(text, limits).map(|_| ())
}

/// The counting scan behind [`prescan`], kept separate for its unit tests.
fn scan(text: &str, limits: &Limits) -> Result<Counts, Rejection> {
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
            match scan_tag(bytes, start, usize::MAX) {
                Tag::Ended { next, .. } => next,
                Tag::Unterminated => break,
                Tag::TooManyDeclarations => return Err(Rejection::LimitExceeded),
            }
        } else {
            counts.elements += 1;
            depth += 1;
            counts.max_depth = counts.max_depth.max(depth);
            if counts.elements > limits.elements || depth > limits.depth {
                return Err(Rejection::LimitExceeded);
            }
            match scan_tag(bytes, start, limits.namespace_declarations) {
                Tag::Ended { next, self_closing } => {
                    if self_closing {
                        depth -= 1;
                    }
                    next
                }
                Tag::Unterminated => break,
                Tag::TooManyDeclarations => return Err(Rejection::LimitExceeded),
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

/// What scanning one tag established.
enum Tag {
    /// The tag ended: the index just past `>`, and whether it closed itself.
    Ended {
        /// Index just past the tag's `>`.
        next: usize,
        /// Whether the tag ended with `/>`.
        self_closing: bool,
    },
    /// The tag never closed. The parser reports the malformed input.
    Unterminated,
    /// The tag declares more namespaces than the limit allows.
    TooManyDeclarations,
}

/// Scan one tag, counting the namespace declarations it makes.
///
/// Quoted attribute values may contain `>` and the text `xmlns`, so quoting is
/// tracked and only attribute-name tokens outside quotes are counted. The scan
/// stops at the first declaration past the limit, so a tag with hundreds of
/// thousands of declarations costs no more than the prefix it takes to exceed
/// it: the parser's own namespace handling compares every declaration on an
/// element against all the earlier ones and is quadratic.
fn scan_tag(bytes: &[u8], start: usize, max_declarations: usize) -> Tag {
    let mut quote: Option<u8> = None;
    let mut declarations = 0usize;
    for (index, &byte) in bytes.iter().enumerate().skip(start + 1) {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => {
                return Tag::Ended {
                    next: index + 1,
                    self_closing: bytes[index - 1] == b'/',
                };
            }
            None => {
                if declares_namespace(bytes, index) {
                    declarations += 1;
                    if declarations > max_declarations {
                        return Tag::TooManyDeclarations;
                    }
                }
            }
        }
    }
    Tag::Unterminated
}

/// Whether an `xmlns` or `xmlns:*` attribute name starts at `index`.
fn declares_namespace(bytes: &[u8], index: usize) -> bool {
    const NAME: &[u8] = b"xmlns";
    if !bytes[index..].starts_with(NAME) || !is_xml_space(bytes[index - 1]) {
        return false;
    }
    matches!(
        bytes.get(index + NAME.len()),
        Some(b':' | b'=') | Some(b' ' | b'\t' | b'\r' | b'\n')
    )
}

/// The XML whitespace an attribute name may follow.
fn is_xml_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counted(text: &str) -> Result<Counts, Rejection> {
        scan(text, &Limits::default())
    }

    #[test]
    fn nesting_and_elements_are_counted() {
        let counts = counted("<a><b><c/></b><b/></a>").expect("well-formed input");
        assert_eq!(counts.elements, 4);
        assert_eq!(counts.max_depth, 3);
    }

    #[test]
    fn comments_cdata_and_instructions_are_skipped() {
        let text = "<?xml version='1.0'?><!-- <b><b> --><a><![CDATA[<c><c>]]></a>";
        let counts = counted(text).expect("well-formed input");
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
            assert_eq!(counted(text), Err(Rejection::ProhibitedConstruct), "{text}");
        }
    }

    #[test]
    fn quoted_attribute_values_may_contain_markup_characters() {
        let counts = counted("<a href=\"a>b\" other='c>d'><b/></a>").expect("well-formed input");
        assert_eq!(counts.elements, 2);
        assert_eq!(counts.max_depth, 2);
    }

    #[test]
    fn depth_at_the_limit_passes_and_one_past_it_fails() {
        let limits = Limits::default();
        let at_limit = "<a>".repeat(limits.depth) + &"</a>".repeat(limits.depth);
        assert_eq!(
            scan(&at_limit, &limits)
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
    fn namespace_declarations_at_the_limit_pass_and_one_past_it_fails() {
        let limits = Limits::default();
        let declarations = |count: usize| {
            (0..count)
                .map(|index| format!(" xmlns:n{index}='urn:x:{index}'"))
                .collect::<String>()
        };
        let at_limit = format!("<a{}/>", declarations(limits.namespace_declarations));
        assert!(prescan(&at_limit, &limits).is_ok());
        let past = format!("<a{}/>", declarations(limits.namespace_declarations + 1));
        assert_eq!(prescan(&past, &limits), Err(Rejection::LimitExceeded));
        let default_form = format!("<a{} xmlns='urn:x'/>", declarations(64));
        assert_eq!(
            prescan(&default_form, &limits),
            Err(Rejection::LimitExceeded)
        );
    }

    #[test]
    fn the_text_xmlns_is_only_counted_as_an_attribute_name() {
        let limits = Limits {
            namespace_declarations: 1,
            ..Limits::default()
        };
        for text in [
            "<a href='xmlns xmlns xmlns xmlns'/>",
            "<a xmlnsish='1' bxmlns='2'/>",
            "<a>xmlns xmlns xmlns</a>",
        ] {
            assert!(prescan(text, &limits).is_ok(), "{text}");
        }
        assert_eq!(
            prescan("<a xmlns='urn:x' xmlns:b='urn:y'/>", &limits),
            Err(Rejection::LimitExceeded)
        );
    }

    #[test]
    fn a_hundred_thousand_levels_are_rejected_without_recursion() {
        let deep = "<a>".repeat(100_000);
        assert_eq!(counted(&deep), Err(Rejection::LimitExceeded));
    }

    #[test]
    fn unterminated_constructs_stop_the_scan_for_the_parser() {
        for text in ["<!-- unterminated", "<![CDATA[ open", "<?pi open", "<a"] {
            assert!(counted(text).is_ok(), "{text}");
        }
    }
}
