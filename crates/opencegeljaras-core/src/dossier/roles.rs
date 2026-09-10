//! Compiling the 194 document-role patterns and matching one declared title.
//!
//! The patterns are XSD `pattern` facets, not literal titles. Only literal
//! text, `( … )` groups, `.`, `.*`, `?`, and `\)` occur in them, which a unit
//! test asserts, so they are compiled as fully anchored regular expressions
//! whose meaning under XSD and under the `regex` crate agrees. Each pattern is
//! compiled once, at first use, under a fixed compiled-program size limit.
//!
//! A matched role is the pattern's own fixed text, never the declared title:
//! reporting it reveals only that the document carries a public schema value.

use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};

use super::patterns::{FORM_ROLE, ROLE_PATTERNS};

/// Largest compiled program accepted for one role pattern, in bytes.
const PATTERN_SIZE_LIMIT: usize = 1024 * 1024;
/// The optional free suffix the two suffix patterns end in.
const SUFFIX: &str = "( - .*)?";

/// One compiled role pattern and the fixed text it is reported as.
struct Role {
    /// The anchored matcher for the whole declared title.
    matcher: Regex,
    /// The pattern's fixed text, which is what `role` reports.
    text: String,
}

/// The compiled role table, built once on first use.
fn table() -> &'static [Role] {
    static TABLE: OnceLock<Vec<Role>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ROLE_PATTERNS
            .iter()
            .filter_map(|pattern| {
                Some(Role {
                    matcher: compile(pattern)?,
                    text: fixed_text(pattern),
                })
            })
            .collect()
    })
}

/// Compile one XSD pattern as a fully anchored regular expression.
///
/// A pattern that does not compile under the size limit is dropped rather than
/// panicking: a title it would have matched is then simply unknown. The unit
/// test asserts that every shipped pattern compiles, so this is unreachable in
/// a published build.
fn compile(pattern: &str) -> Option<Regex> {
    RegexBuilder::new(&format!("^(?:{pattern})$"))
        .size_limit(PATTERN_SIZE_LIMIT)
        .build()
        .ok()
}

/// The fixed text one pattern is reported as.
///
/// For the two patterns ending in `( - .*)?` it is the prefix before the
/// separator, so an arbitrary declared suffix can never reach the output. For
/// every other pattern it is the literal title below.
fn fixed_text(pattern: &str) -> String {
    match pattern.strip_suffix(SUFFIX) {
        Some(prefix) => prefix.to_owned(),
        None => literal(pattern),
    }
}

/// The literal title of one pattern: groups kept, `.*` removed, `\)` a `)`.
pub(crate) fn literal(pattern: &str) -> String {
    let mut literal = String::with_capacity(pattern.len());
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => literal.extend(characters.next()),
            '(' | ')' | '?' => {}
            '.' if characters.peek() == Some(&'*') => {
                characters.next();
            }
            _ => literal.push(character),
        }
    }
    literal
}

/// The fixed text of the role whose pattern matches one declared title.
///
/// The patterns are tried in schema order and the first match wins; two of the
/// 194 texts are duplicates, so the order fixes which one is reported.
#[must_use]
pub(crate) fn role_for(title: &str) -> Option<&'static str> {
    table()
        .iter()
        .find(|role| role.matcher.is_match(title))
        .map(|role| role.text.as_str())
}

/// The fixed text of every role pattern, in schema order.
pub(crate) fn texts() -> Vec<&'static str> {
    table().iter().map(|role| role.text.as_str()).collect()
}

/// Whether one matched role is the application form's own role.
#[must_use]
pub(crate) fn is_form_role(role: &str) -> bool {
    role == FORM_ROLE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character a regular expression could give a special meaning.
    const METACHARACTERS: &str = "\\^$.|?*+()[]{}";
    /// The only regular-expression constructs the shipped patterns may use.
    const ALLOWED: [char; 6] = ['(', ')', '.', '*', '?', '\\'];
    /// The separator before the free suffix of the two suffix patterns.
    const SUFFIX_SEPARATOR: &str = " - ";

    #[test]
    fn every_pattern_uses_only_the_reviewed_construct_subset() {
        for pattern in ROLE_PATTERNS {
            let characters: Vec<char> = pattern.chars().collect();
            for (index, character) in characters.iter().enumerate() {
                if !METACHARACTERS.contains(*character) {
                    continue;
                }
                assert!(ALLOWED.contains(character), "{pattern}");
                if *character == '\\' {
                    assert_eq!(characters.get(index + 1), Some(&')'), "{pattern}");
                }
                if *character == '*' {
                    assert_eq!(characters.get(index - 1), Some(&'.'), "{pattern}");
                }
                if *character == ')' && index > 0 {
                    assert_ne!(characters[index - 1], '(', "{pattern}");
                }
            }
        }
    }

    #[test]
    fn every_pattern_compiles_and_matches_its_literal_title() {
        assert_eq!(table().len(), ROLE_PATTERNS.len());
        assert_eq!(ROLE_PATTERNS.len(), 194);
        for pattern in ROLE_PATTERNS {
            let literal = literal(pattern);
            assert!(role_for(&literal).is_some(), "{pattern}");
        }
    }

    #[test]
    fn the_two_suffix_patterns_report_only_their_prefix() {
        let suffixed: Vec<&str> = ROLE_PATTERNS
            .iter()
            .copied()
            .filter(|pattern| pattern.ends_with(SUFFIX))
            .collect();
        assert_eq!(suffixed.len(), 2);
        for pattern in suffixed {
            let text = fixed_text(pattern);
            assert!(!text.contains(SUFFIX_SEPARATOR), "{text}");
            let declared = format!("{text}{SUFFIX_SEPARATOR}SENTINEL-ROLE-4419");
            assert_eq!(role_for(&declared), Some(text.as_str()));
        }
    }

    #[test]
    fn a_group_is_removed_and_an_escaped_parenthesis_is_kept() {
        assert_eq!(literal("A telephely(ek) haszn"), "A telephelyek haszn");
        assert_eq!(literal(r"b\) pontja"), "b) pontja");
        assert_eq!(literal("x( - .*)?"), "x - ");
    }

    #[test]
    fn the_form_role_is_the_first_listed_pattern() {
        assert!(is_form_role(&fixed_text(ROLE_PATTERNS[0])));
        assert_eq!(role_for(FORM_ROLE), Some(FORM_ROLE));
        assert_eq!(role_for("SENTINEL-ROLE-4419"), None);
    }
}
