//! Line parsing, value redaction, and truncation, without a child process.
//!
//! Every message below is hand-written in the format libxml2 2.9.14 produces.
//! None of them comes from a real document.

use super::*;

/// One line as the validator writes it, for a chosen element and message.
fn line(number: u32, element: &str, message: &str) -> String {
    format!("{DOCUMENT_NAME}:{number}: element {element}{ERROR_MARKER}{message}")
}

#[test]
fn a_well_formed_violation_line_is_parsed() {
    let text = line(9, "Cegjegyzekszam", "Element '{urn:x}Cegjegyzekszam': bad.");
    let (number, element, message) = parse_line(&text).expect("the line should parse");
    assert_eq!(number, 9);
    assert_eq!(element, "Cegjegyzekszam");
    assert_eq!(message, "Element '{urn:x}Cegjegyzekszam': bad.");
}

#[test]
fn other_lines_are_never_parsed() {
    for text in [
        "",
        "document.xml fails to validate",
        "document.xml:9: element : Schemas validity error : x",
        "document.xml:x: element A: Schemas validity error : x",
        "other.xml:9: element A: Schemas validity error : x",
        "document.xml:9: element A: Schemas parser warning : x",
        "I/O error : Attempt to load network entity",
    ] {
        assert!(parse_line(text).is_none(), "{text}");
    }
}

#[test]
fn the_expected_trailing_lines_are_not_counted() {
    let stderr = format!(
        "{}\n{DOCUMENT_NAME} fails to validate\n",
        line(2, "A", "Element '{urn:x}A': bad.")
    );
    let findings = collect(&stderr, false);
    assert_eq!(findings.items.len(), 1);
    assert_eq!(findings.unparsed, 0);
    assert!(!findings.truncated);
    assert!(findings.redacted);

    let conforming = collect(&format!("{DOCUMENT_NAME} validates\n"), false);
    assert!(conforming.items.is_empty());
    assert_eq!(conforming.unparsed, 0);
}

#[test]
fn unmatched_lines_are_counted_and_never_echoed() {
    let stderr = format!(
        "warning: failed to load external entity \"http://example.invalid/x.xsd\"\n{}\n",
        line(2, "A", "Element '{urn:x}A': bad.")
    );
    let findings = collect(&stderr, false);
    assert_eq!(findings.unparsed, 1);
    assert_eq!(findings.items.len(), 1);
    assert!(!findings.items[0].message.contains("example.invalid"));
}

#[test]
fn a_qualified_name_and_a_facet_name_survive_redaction() {
    let message = "Element '{urn:x}Szam': [facet 'pattern'] The value '01' is not accepted by the pattern '[0-9]{2}'.";
    assert_eq!(
        redact(message),
        "Element '{urn:x}Szam': [facet 'pattern'] The value '…' is not accepted by the pattern '…'."
    );
}

#[test]
fn a_message_without_quotes_is_unchanged() {
    assert_eq!(redact("Missing child element."), "Missing child element.");
}

#[test]
fn an_apostrophe_inside_a_value_redacts_the_whole_message() {
    let message = "Element '{urn:x}Nev': [facet 'pattern'] The value 'O'Brien' is not accepted.";
    assert_eq!(redact(message), REDACTED_MESSAGE);
}

#[test]
fn a_value_shaped_like_a_qualified_name_redacts_the_whole_message() {
    let message = "Element '{urn:x}Nev': The value '{urn:y}secret' is not accepted.";
    assert_eq!(redact(message), REDACTED_MESSAGE);
}

#[test]
fn an_unknown_name_after_a_facet_redacts_the_whole_message() {
    let message = "Element '{urn:x}Nev': [facet 'titok'] rejected.";
    assert_eq!(redact(message), REDACTED_MESSAGE);
}

#[test]
fn only_clark_notation_counts_as_a_qualified_name() {
    assert!(is_qualified_name("{urn:x}Szam"));
    assert!(is_qualified_name("{}Szam"));
    assert!(!is_qualified_name("Szam"));
    assert!(!is_qualified_name("{urn:x"));
    assert!(!is_qualified_name("{urn:{x}Szam"));
    assert!(!is_qualified_name("{urn:x}Sz am"));
}

#[test]
fn requested_values_are_kept_and_reported_as_kept() {
    let stderr = format!("{}\n", line(2, "A", "The value 'secret' is bad."));
    let findings = collect(&stderr, true);
    assert!(!findings.redacted);
    assert_eq!(findings.items[0].message, "The value 'secret' is bad.");
}

#[test]
fn long_element_names_and_messages_are_cut() {
    let element = "N".repeat(200);
    let message = format!("Missing {}.", "x".repeat(600));
    let stderr = format!("{}\n", line(4, &element, &message));
    let findings = collect(&stderr, false);
    let finding = &findings.items[0];
    assert_eq!(finding.element.chars().count(), ELEMENT_SCALARS + 1);
    assert!(finding.element.ends_with('…'));
    assert_eq!(finding.message.chars().count(), MESSAGE_SCALARS + 1);
    assert!(finding.message.ends_with('…'));
}

#[test]
fn strings_at_the_cut_are_left_alone() {
    assert_eq!(cut("abc", 3), "abc");
    assert_eq!(cut("abcd", 3), "abc…");
    assert_eq!(cut("áéíóö", 3), "áéí…");
}

#[test]
fn findings_beyond_the_cap_are_dropped_and_reported() {
    let mut stderr = String::new();
    for number in 1..=(MAX_FINDINGS as u32 + 44) {
        stderr.push_str(&line(number, "A", "Element '{urn:x}A': bad."));
        stderr.push('\n');
    }
    let findings = collect(&stderr, false);
    assert_eq!(findings.items.len(), MAX_FINDINGS);
    assert!(findings.truncated);
    assert_eq!(findings.items[MAX_FINDINGS - 1].line, MAX_FINDINGS as u64);
}

#[test]
fn exactly_the_cap_is_not_reported_as_truncated() {
    let mut stderr = String::new();
    for number in 1..=(MAX_FINDINGS as u32) {
        stderr.push_str(&line(number, "A", "bad."));
        stderr.push('\n');
    }
    assert!(!collect(&stderr, false).truncated);
}

#[test]
fn the_bundle_verdict_names_the_three_states() {
    assert_eq!(bundle_verdict(&Value::Null), "not performed");
    assert_eq!(
        bundle_verdict(&json!({ "bundle": { "verified": true } })),
        "verified"
    );
    assert_eq!(
        bundle_verdict(&json!({ "bundle": { "verified": false, "form_closure_verified": true } })),
        "form closure verified"
    );
    assert_eq!(
        bundle_verdict(&json!({ "bundle": { "verified": false, "form_closure_verified": false } })),
        "unverified"
    );
}

#[test]
fn the_validator_verdict_names_the_version_or_none() {
    assert_eq!(validator_verdict(&Value::Null), "none");
    assert_eq!(validator_verdict(&validator_value(None, None)), "none");
    assert_eq!(
        validator_verdict(&validator_value(Some(20914), Some(3))),
        "xmllint 20914"
    );
}

#[test]
fn every_error_carries_a_fixed_code_message_and_exit() {
    for (error, code, exit) in [
        (ValidateError::Nonconformant, "nonconformant", 4),
        (ValidateError::BundleUnverified, "bundle_unverified", 5),
        (
            ValidateError::ValidatorUnavailable,
            "validator_unavailable",
            5,
        ),
        (ValidateError::ValidatorFailed, "validator_failed", 5),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.exit(), exit);
        assert!(!error.message().is_empty());
        assert!(!error.message().ends_with('.'));
    }
}
