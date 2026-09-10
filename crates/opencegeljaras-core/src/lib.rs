//! I/O-free capability and source-catalogue model for openCegeljaras.

pub mod form;
pub mod schema;

use serde::{Deserialize, Serialize};

/// Human-facing project name.
pub const PROJECT: &str = "openCegeljaras";
/// Development stage reported by the scaffold.
pub const STAGE: &str = "scaffold";
/// Implemented filing or document-processing operations.
pub const OPERATIONS: &[&str] = &[
    "form inspect",
    "form validate",
    "schemas list",
    "schemas verify",
];
/// Commands that describe the scaffold without performing domain work.
pub const INTROSPECTION_COMMANDS: &[&str] = &["capabilities", "sources", "skill"];

const SOURCE_CATALOG_JSON: &str = include_str!("../../../data/source-catalog.json");

/// Cut an untrusted string to a fixed number of Unicode scalar values.
///
/// Document- and bundle-derived strings are the only untrusted text this crate
/// ever reports, and either can be made arbitrarily long, so every one of them
/// passes through this cut before it reaches an output field.
pub(crate) fn truncate_scalars(text: &str, limit: usize) -> String {
    let mut cut: String = text.chars().take(limit).collect();
    if text.chars().nth(limit).is_some() {
        cut.push('…');
    }
    cut
}

/// The reviewed public metadata compiled into the binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCatalog {
    /// Date on which the catalogue metadata was last reviewed.
    pub reviewed_at: String,
    /// Sources relevant to later research and implementation.
    pub sources: Vec<Source>,
}

/// One catalogue entry. This is metadata; openCegeljaras does not fetch it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub url: String,
    pub kind: String,
    pub status: String,
    pub notes: String,
}

/// Parse the source catalogue compiled into this build.
pub fn source_catalog() -> Result<SourceCatalog, serde_json::Error> {
    serde_json::from_str(SOURCE_CATALOG_JSON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_report_only_read_only_operations() {
        assert_eq!(
            OPERATIONS,
            [
                "form inspect",
                "form validate",
                "schemas list",
                "schemas verify"
            ]
        );
        assert_eq!(INTROSPECTION_COMMANDS, ["capabilities", "sources", "skill"]);
    }

    #[test]
    fn long_strings_are_cut_with_a_marker() {
        assert_eq!(truncate_scalars("abc", 8), "abc");
        assert_eq!(truncate_scalars("abcdefgh", 8), "abcdefgh");
        assert_eq!(truncate_scalars("abcdefghi", 8), "abcdefgh…");
        assert_eq!(truncate_scalars("áéíóöőúüű", 4), "áéíó…");
    }

    #[test]
    fn compiled_source_catalog_is_well_formed() {
        let catalog = source_catalog().expect("compiled source catalogue must be valid JSON");
        assert!(!catalog.reviewed_at.is_empty());
        assert!(catalog.sources.iter().all(|source| {
            !source.id.is_empty()
                && !source.title.is_empty()
                && !source.url.is_empty()
                && !source.kind.is_empty()
                && !source.status.is_empty()
        }));
    }
}
