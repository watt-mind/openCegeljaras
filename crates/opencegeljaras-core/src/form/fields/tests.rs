//! Unit tests for the field walk and the field-level comparison.
//!
//! Every document here is written in the test itself. Nothing private is read.

use std::collections::HashSet;

use super::*;
use crate::form::{FormStatus, Limits, list_fields, tree};

/// The wrapper and name-reservation namespaces the fixtures also use.
const WRAPPER: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#";
const VARIANT: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/nevfoglalasiKerelem-20210401#";
const OTHER: &str = "http://example.invalid/other#";

/// Wrap one body in a supported wrapper and a name-reservation variant root.
fn document(root_attributes: &str, body: &str) -> String {
    format!(
        "<w:Nyomtatvány xmlns:w=\"{WRAPPER}\" xmlns=\"{VARIANT}\" xmlns:o=\"{OTHER}\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\
         <NévfoglalásiKérelem {root_attributes}>{body}</NévfoglalásiKérelem></w:Nyomtatvány>"
    )
}

/// List the fields of one generated document.
fn list(text: &str) -> FieldList {
    let limits = Limits::default();
    let parsed = tree::parse(text, &limits).expect("the generated document should parse");
    walk(&parsed)
}

/// Every listed field as `path`, `kind`, `namespace`, `value`.
fn entries(list: &FieldList) -> Vec<(String, &'static str, Option<String>, Option<String>)> {
    (0..list.len())
        .map(|index| {
            let field = &list.fields()[index];
            (
                list.path(index).text,
                field.kind().code(),
                list.namespace(index),
                field.emitted_value(),
            )
        })
        .collect()
}

/// Every listed path, in emission order.
fn paths(list: &FieldList) -> Vec<String> {
    (0..list.len()).map(|index| list.path(index).text).collect()
}

#[test]
fn same_named_siblings_are_indexed_and_singletons_are_not() {
    let list = list(&document(
        "",
        "<A><B>1</B></A><A><B>2</B></A><C><D>3</D></C>",
    ));
    assert_eq!(paths(&list), ["A[1]/B", "A[2]/B", "C/D"]);
}

#[test]
fn siblings_in_different_namespaces_share_one_index_sequence() {
    let list = list(&document("", "<Név>a</Név><o:Név>b</o:Név>"));
    assert_eq!(
        entries(&list),
        [
            (
                String::from("Név[1]"),
                "element",
                Some(String::from(VARIANT)),
                Some(String::from("a")),
            ),
            (
                String::from("Név[2]"),
                "element",
                Some(String::from(OTHER)),
                Some(String::from("b")),
            ),
        ]
    );
}

#[test]
fn a_leaf_beside_a_non_leaf_of_the_same_name_still_carries_an_index() {
    let list = list(&document("", "<A>text</A><A><B>1</B></A>"));
    assert_eq!(paths(&list), ["A[1]", "A[2]/B"]);
}

#[test]
fn root_attributes_come_first_and_exclude_xsi_and_declarations() {
    let list = list(&document(
        "Cégforma=\"Kft\" xsi:schemaLocation=\"urn:a urn:b\" xmlns:extra=\"urn:extra\"",
        "<A>1</A>",
    ));
    assert_eq!(paths(&list), ["@Cégforma", "A"]);
    assert_eq!(list.fields()[0].kind(), FieldKind::Attribute);
    assert_eq!(list.namespace(0), None);
}

#[test]
fn same_local_name_attributes_in_different_namespaces_are_indexed() {
    let list = list(&document("", "<A Típus=\"a\" o:Típus=\"b\">1</A>"));
    assert_eq!(paths(&list), ["A", "A/@Típus[1]", "A/@Típus[2]"]);
    assert_eq!(list.namespace(1), None);
    assert_eq!(list.namespace(2), Some(String::from(OTHER)));
}

#[test]
fn an_element_is_listed_before_its_attributes_and_its_children() {
    let list = list(&document("", "<A x=\"1\"><B y=\"2\">v</B></A>"));
    assert_eq!(paths(&list), ["A/@x", "A/B", "A/B/@y"]);
}

#[test]
fn leaf_values_cover_empty_whitespace_cdata_and_comment_splits() {
    let list = list(&document(
        "",
        "<A/><B>  </B><C><![CDATA[<x> & y]]></C><D>elő<!-- split -->utó</D>",
    ));
    let values: Vec<_> = list
        .fields()
        .iter()
        .map(|field| field.emitted_value().expect("a leaf has a value"))
        .collect();
    assert_eq!(values, ["", "  ", "<x> & y", "előutó"]);
    let lengths: Vec<_> = list
        .fields()
        .iter()
        .map(|field| field.value_length().expect("a leaf has a length"))
        .collect();
    assert_eq!(lengths, [0, 2, 7, 6]);
}

#[test]
fn a_mixed_element_has_no_value_and_its_children_are_still_walked() {
    let list = list(&document("", "<A>text<B>1</B></A><C>\n  <D>2</D>\n</C>"));
    assert_eq!(paths(&list), ["A", "A/B", "C/D"]);
    assert_eq!(list.fields()[0].kind(), FieldKind::Mixed);
    assert_eq!(list.fields()[0].value_length(), None);
    assert_eq!(list.fields()[0].value_truncated(), None);
    assert_eq!(list.fields()[0].emitted_value(), None);
}

#[test]
fn every_listed_path_identifies_exactly_one_field() {
    let list = list(&document(
        "Cégforma=\"Kft\"",
        "<A x=\"1\" o:x=\"2\"><B>1</B><B>2</B></A><A><B>3</B></A>\
         <Név>n</Név><o:Név>m</o:Név><M>t<N>1</N></M>",
    ));
    let unique: HashSet<String> = paths(&list).into_iter().collect();
    assert_eq!(unique.len(), list.len());
    assert!(list.len() > 8);
    assert!(!list.is_empty());
}

#[test]
fn the_field_cap_stops_the_walk_and_reports_the_listed_count() {
    let body = "<A>1</A>".repeat(FIELD_LIMIT + 10);
    let list = list(&document("", &body));
    assert_eq!(list.len(), FIELD_LIMIT);
    assert!(list.truncated());
}

#[test]
fn values_and_paths_are_cut_at_their_own_limits() {
    let long_value = "é".repeat(VALUE_LIMIT + 5);
    let long_name = "N".repeat(NAME_LIMIT + 5);
    let list = list(&document(
        "",
        &format!("<A>{long_value}</A><{long_name}>1</{long_name}>"),
    ));
    let value = list.fields()[0]
        .emitted_value()
        .expect("a leaf has a value");
    assert_eq!(value.chars().count(), VALUE_LIMIT + 1);
    assert!(value.ends_with('…'));
    assert_eq!(list.fields()[0].value_length(), Some(VALUE_LIMIT + 5));
    assert_eq!(list.fields()[0].value_truncated(), Some(true));

    let path = list.path(1);
    assert!(path.truncated);
    assert_eq!(path.text.chars().count(), NAME_LIMIT + 1);
}

#[test]
fn a_path_longer_than_the_path_limit_is_cut_whole() {
    let mut body = String::new();
    for _ in 0..100 {
        body.push_str(&format!("<{}>", "N".repeat(NAME_LIMIT)));
    }
    body.push_str("value");
    for _ in 0..100 {
        body.push_str(&format!("</{}>", "N".repeat(NAME_LIMIT)));
    }
    let list = list(&document("", &body));
    let path = list.path(0);
    assert!(path.truncated);
    assert_eq!(path.text.chars().count(), PATH_LIMIT + 1);
}

#[test]
fn a_deep_chain_keeps_one_arena_entry_per_node_not_per_path_component() {
    let depth = 60;
    let leaves = 400;
    let name = "N".repeat(NAME_LIMIT);
    let mut body = String::new();
    for _ in 0..depth {
        body.push_str(&format!("<{name}>"));
    }
    for _ in 0..leaves {
        body.push_str(&format!("<{name}>v</{name}>"));
    }
    for _ in 0..depth {
        body.push_str(&format!("</{name}>"));
    }
    let list = list(&document("", &body));
    assert_eq!(list.len(), leaves);
    assert!(!list.truncated());
    assert!(
        list.components.len() <= depth + leaves + 1,
        "one arena entry per element, not one per path component"
    );
    assert_eq!(list.namespaces.len(), 1);
}

#[test]
fn a_wide_parent_counts_its_siblings_once() {
    let width = 20_000;
    let body = "<A>v</A>".repeat(width);
    let list = list(&document("", &body));
    assert_eq!(list.len(), FIELD_LIMIT);
    assert_eq!(list.path(0).text, "A[1]");
    assert_eq!(list.path(FIELD_LIMIT - 1).text, format!("A[{FIELD_LIMIT}]"));
}

#[test]
fn the_diff_reports_added_removed_changed_and_unchanged_by_path() {
    let old = list(&document("", "<A>1</A><A>2</A><A>3</A><K>same</K>"));
    let new = list(&document("", "<A>2</A><A>3</A><K>same</K><Új>4</Új>"));
    let diff = diff(&old, &new);
    assert_eq!(diff.unchanged, 1);
    assert_eq!(
        diff.changed
            .iter()
            .map(|(from, to)| (old.path(*from).text, new.path(*to).text))
            .collect::<Vec<_>>(),
        [
            (String::from("A[1]"), String::from("A[1]")),
            (String::from("A[2]"), String::from("A[2]")),
        ]
    );
    assert_eq!(
        diff.removed
            .iter()
            .map(|index| old.path(*index).text)
            .collect::<Vec<_>>(),
        [String::from("A[3]")]
    );
    assert_eq!(
        diff.added
            .iter()
            .map(|index| new.path(*index).text)
            .collect::<Vec<_>>(),
        [String::from("Új")]
    );
}

#[test]
fn a_different_kind_or_namespace_at_one_path_is_a_change() {
    let old = list(&document("", "<A>text<B>1</B></A><Név>n</Név>"));
    let new = list(&document("", "<A><B>1</B></A><o:Név>n</o:Név>"));
    let diff = diff(&old, &new);
    assert_eq!(diff.unchanged, 1);
    assert_eq!(diff.added.len(), 0);
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(old.path(diff.removed[0]).text, "A");
    assert_eq!(diff.changed.len(), 1);
    let (from, to) = diff.changed[0];
    assert_eq!(new.path(to).text, "Név");
    assert_eq!(old.raw_namespace(from), Some(VARIANT));
    assert_eq!(new.raw_namespace(to), Some(OTHER));
}

#[test]
fn values_are_compared_beyond_the_emitted_cut() {
    let base = "é".repeat(VALUE_LIMIT);
    let old = list(&document("", &format!("<A>{base}old</A>")));
    let new = list(&document("", &format!("<A>{base}new</A>")));
    let diff = diff(&old, &new);
    assert_eq!(diff.changed.len(), 1);
    assert_eq!(diff.unchanged, 0);
    let (from, to) = diff.changed[0];
    assert_eq!(
        old.fields()[from].emitted_value(),
        new.fields()[to].emitted_value()
    );
    assert_eq!(old.fields()[from].value_truncated(), Some(true));
}

#[test]
fn an_empty_diff_of_one_listing_with_itself_reports_only_unchanged() {
    let list = list(&document("x=\"1\"", "<A>1</A><A>2</A><M>t<N>1</N></M>"));
    let diff = diff(&list, &list);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.changed.is_empty());
    assert_eq!(diff.unchanged, list.len());
}

#[test]
fn a_recognised_document_is_listed_and_an_unrecognised_one_is_not() {
    let limits = Limits::default();
    let recognised = document("", "<A>1</A>");
    let listing = list_fields(recognised.as_bytes(), &limits).expect("the document parses");
    assert_eq!(listing.inspection.form.status, FormStatus::Recognized);
    assert_eq!(
        listing.fields.expect("a recognised form is listed").len(),
        1
    );

    let listing = list_fields(b"<Egyeb/>", &limits).expect("the document parses");
    assert_eq!(listing.inspection.form.status, FormStatus::UnrecognizedRoot);
    assert!(listing.fields.is_none());
}

#[test]
fn field_kinds_report_stable_codes() {
    assert_eq!(FieldKind::Element.code(), "element");
    assert_eq!(FieldKind::Attribute.code(), "attribute");
    assert_eq!(FieldKind::Mixed.code(), "mixed");
}
