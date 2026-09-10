//! Unit tests for decoding, limits, and 2026 form classification.

use super::*;

const CK_NAMESPACE: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegbejegyzesiKerelem-20260101#";
const NK_NAMESPACE: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/nevfoglalasiKerelem-20210401#";

fn wrapper(child: &str) -> String {
    format!("<n:Nyomtatvány xmlns:n=\"{WRAPPER_NAMESPACE}\">{child}</n:Nyomtatvány>")
}

fn inspected(text: &str) -> Inspection {
    inspect(text.as_bytes(), &Limits::default()).expect("the document should be inspectable")
}

fn rejected(text: &str) -> Failure {
    inspect(text.as_bytes(), &Limits::default()).expect_err("the document should be refused")
}

#[test]
fn the_default_limits_are_the_contract_values() {
    let limits = Limits::default();
    assert_eq!(limits.input_bytes, 16 * 1024 * 1024);
    assert_eq!(limits.depth, 128);
    assert_eq!(limits.elements, 200_000);
    assert_eq!(limits.text_bytes, 1024 * 1024);
    assert_eq!(limits.attributes, 64);
    assert_eq!(limits.parser_nodes(), 800_000);
}

#[test]
fn every_code_has_a_fixed_message() {
    assert_eq!(Rejection::InputUnreadable.code(), "input_unreadable");
    assert_eq!(
        Rejection::InputTooLarge.message(),
        "the input exceeds the 16 MiB limit"
    );
    assert_eq!(FormStatus::Recognized.code(), "recognized");
    assert_eq!(FormStatus::Recognized.message(), None);
    for status in [
        FormStatus::UnrecognizedVariant,
        FormStatus::UnsupportedVersion,
        FormStatus::UnrecognizedRoot,
    ] {
        assert!(status.message().is_some_and(|text| !text.is_empty()));
    }
    for rejection in [
        Rejection::UnsupportedEncoding,
        Rejection::InvalidEncoding,
        Rejection::ProhibitedConstruct,
        Rejection::LimitExceeded,
        Rejection::MalformedXml,
    ] {
        assert!(!rejection.code().is_empty());
        assert!(!rejection.message().is_empty());
    }
}

#[test]
fn a_registration_form_is_recognized_with_its_sections() {
    let text = wrapper(&format!(
        "<ck:CégbejegyzésiKérelem xmlns:ck=\"{CK_NAMESPACE}\"><ck:Adatlap>x</ck:Adatlap>\
         <ck:ÁltalánosAdatok/><ck:ÁltalánosAdatok/></ck:CégbejegyzésiKérelem>"
    ));
    let inspection = inspected(&text);
    assert_eq!(inspection.form.status, FormStatus::Recognized);
    assert_eq!(inspection.form.profile, Some(PROFILE));
    let variant = inspection.form.variant.expect("a variant is identified");
    assert_eq!(variant.id, "cegbejegyzesi-kerelem-20260101");
    assert_eq!(variant.local_name, "CégbejegyzésiKérelem");
    assert_eq!(variant.namespace.as_deref(), Some(CK_NAMESPACE));
    assert_eq!(
        inspection.form.sections.expect("sections are listed"),
        vec![
            Section {
                local_name: "Adatlap".to_owned(),
                count: 1
            },
            Section {
                local_name: "ÁltalánosAdatok".to_owned(),
                count: 2
            },
        ]
    );
    assert!(!inspection.form.sections_truncated);
    assert_eq!(inspection.root.local_name, "Nyomtatvány");
    assert_eq!(
        inspection.root.namespace.as_deref(),
        Some(WRAPPER_NAMESPACE)
    );
    assert_eq!(inspection.structure.elements, 5);
    assert_eq!(inspection.structure.max_depth, 3);
    assert_eq!(inspection.structure.text_bytes, 1);
    assert_eq!(
        inspection.structure.namespaces,
        vec![CK_NAMESPACE.to_owned(), WRAPPER_NAMESPACE.to_owned()]
    );
    assert!(!inspection.structure.namespaces_truncated);
    assert_eq!(inspection.input.encoding, UTF_8_LABEL);
    assert!(!inspection.input.stylesheet_pi);
    assert!(!inspection.input.schema_location_declared);
}

#[test]
fn a_name_reservation_form_is_recognized() {
    let text = wrapper(&format!(
        "<nk:NévfoglalásiKérelem xmlns:nk=\"{NK_NAMESPACE}\"><nk:Adatlap/><nk:Névfoglalások/>\
         </nk:NévfoglalásiKérelem>"
    ));
    let inspection = inspected(&text);
    assert_eq!(inspection.form.status, FormStatus::Recognized);
    assert_eq!(
        inspection.form.variant.expect("a variant is identified").id,
        "nevfoglalasi-kerelem-20210401"
    );
}

#[test]
fn a_wrapper_without_exactly_one_known_child_is_an_unrecognized_variant() {
    for child in [
        String::new(),
        "<Ismeretlen/>".to_owned(),
        format!("<ck:CégbejegyzésiKérelem xmlns:ck=\"{CK_NAMESPACE}\"/><Másik/>"),
        "<CégbejegyzésiKérelem/>".to_owned(),
    ] {
        let inspection = inspected(&wrapper(&child));
        assert_eq!(inspection.form.status, FormStatus::UnrecognizedVariant);
        assert_eq!(inspection.form.profile, Some(PROFILE));
        assert!(inspection.form.variant.is_none());
        assert!(inspection.form.sections.is_none());
    }
}

#[test]
fn other_wrapper_versions_and_roots_fail_closed() {
    let older = "<n:Nyomtatvány xmlns:n=\"http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/\
                 cegnyomtatvany-20240101#\"/>";
    let inspection = inspected(older);
    assert_eq!(inspection.form.status, FormStatus::UnsupportedVersion);
    assert!(inspection.form.profile.is_none());

    for text in [
        "<Nyomtatvány/>",
        "<n:Nyomtatvány xmlns:n=\"http://example.invalid/other#\"/>",
        "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"/>",
        "<ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\"/>",
    ] {
        let inspection = inspected(text);
        assert_eq!(
            inspection.form.status,
            FormStatus::UnrecognizedRoot,
            "{text}"
        );
        assert!(inspection.form.profile.is_none());
        assert!(inspection.form.sections.is_none());
    }
    assert!(inspected("<Nyomtatvány/>").root.namespace.is_none());
}

#[test]
fn stylesheet_instructions_and_schema_hints_are_reported_but_never_resolved() {
    let text = format!(
        "<?xml-stylesheet type=\"text/xsl\" href=\"http://example.invalid/x.xsl\"?>\
         <n:Nyomtatvány xmlns:n=\"{WRAPPER_NAMESPACE}\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"{WRAPPER_NAMESPACE} x.xsd\"/>"
    );
    let inspection = inspected(&text);
    assert!(inspection.input.stylesheet_pi);
    assert!(inspection.input.schema_location_declared);
    assert_eq!(inspection.structure.attributes, 1);
}

#[test]
fn prohibited_and_malformed_inputs_are_refused_with_their_facts() {
    for (text, code) in [
        (
            "<!DOCTYPE a [<!ENTITY x 'y'>]><a>&x;</a>",
            Rejection::ProhibitedConstruct,
        ),
        ("<a>&foo;</a>", Rejection::MalformedXml),
        ("<a><b></a>", Rejection::MalformedXml),
        ("", Rejection::MalformedXml),
    ] {
        let failure = rejected(text);
        assert_eq!(failure.code, code, "{text}");
        let input = failure.input.expect("input facts are established");
        assert_eq!(input.bytes, text.len() as u64);
        assert_eq!(input.encoding, Some(UTF_8_LABEL));
    }
}

#[test]
fn an_oversized_input_reports_only_its_size() {
    let limits = Limits {
        input_bytes: 4,
        ..Limits::default()
    };
    let failure = inspect(b"<a/><b/>", &limits).expect_err("the size limit applies");
    assert_eq!(failure.code, Rejection::InputTooLarge);
    let input = failure.input.expect("the size is established");
    assert_eq!(input.bytes, 8);
    assert_eq!(input.bom, None);
    assert_eq!(input.xml_declaration, None);
    assert_eq!(input.encoding, None);
}

#[test]
fn text_and_attribute_limits_apply_at_the_limit_and_one_past_it() {
    let limits = Limits {
        text_bytes: 8,
        attributes: 2,
        ..Limits::default()
    };
    assert!(inspect("<a>12345678</a>".as_bytes(), &limits).is_ok());
    assert_eq!(
        inspect("<a>123456789</a>".as_bytes(), &limits)
            .expect_err("one byte past the text limit")
            .code,
        Rejection::LimitExceeded
    );
    assert!(inspect("<a x='1' y='2'/>".as_bytes(), &limits).is_ok());
    assert_eq!(
        inspect("<a x='1' y='2' z='3'/>".as_bytes(), &limits)
            .expect_err("one attribute past the limit")
            .code,
        Rejection::LimitExceeded
    );
    assert!(
        inspect(
            "<a xmlns='urn:x' xmlns:b='urn:y' x='1' y='2'/>".as_bytes(),
            &limits
        )
        .is_ok(),
        "namespace declarations are not attributes"
    );
}

#[test]
fn deep_documents_are_refused_before_the_parser_runs() {
    let deep = "<a>".repeat(100_000) + &"</a>".repeat(100_000);
    let failure = rejected(&deep);
    assert_eq!(failure.code, Rejection::LimitExceeded);
}

#[test]
fn document_strings_are_cut_to_a_fixed_length() {
    let long = "N".repeat(200);
    let text = format!("<{long}/>");
    let inspection = inspected(&text);
    assert_eq!(inspection.root.local_name.chars().count(), NAME_LIMIT + 1);
    assert!(inspection.root.local_name.ends_with('…'));
    assert_eq!(truncate_name("Adatlap"), "Adatlap");
    assert_eq!(truncate_name(&"á".repeat(128)).chars().count(), 128);
}

#[test]
fn iso_8859_2_bytes_are_decoded_and_classified() {
    let mut bytes = b"<?xml version=\"1.0\" encoding=\"ISO-8859-2\"?>".to_vec();
    let text = wrapper(&format!(
        "<nk:NévfoglalásiKérelem xmlns:nk=\"{NK_NAMESPACE}\"><nk:Adatlap/></nk:NévfoglalásiKérelem>"
    ));
    let (encoded, _, had_errors) = encoding_rs::ISO_8859_2.encode(&text);
    assert!(!had_errors, "the fixture text must be representable");
    bytes.extend_from_slice(&encoded);
    let inspection = inspect(&bytes, &Limits::default()).expect("the document is inspectable");
    assert_eq!(inspection.input.encoding, "ISO-8859-2");
    assert!(inspection.input.xml_declaration);
    assert!(!inspection.input.bom);
    assert_eq!(inspection.form.status, FormStatus::Recognized);
    assert_eq!(inspection.input.bytes, bytes.len() as u64);
}

#[test]
fn section_and_namespace_lists_are_capped() {
    let mut children = String::new();
    for index in 0..70 {
        children.push_str(&format!("<ck:S{index}/>"));
    }
    let text = wrapper(&format!(
        "<ck:CégbejegyzésiKérelem xmlns:ck=\"{CK_NAMESPACE}\">{children}</ck:CégbejegyzésiKérelem>"
    ));
    let inspection = inspected(&text);
    let sections = inspection.form.sections.expect("sections are listed");
    assert_eq!(sections.len(), 64);
    assert!(inspection.form.sections_truncated);

    let mut wide = String::from("<r xmlns=\"urn:x:0\">");
    for index in 1..70 {
        wide.push_str(&format!("<e xmlns=\"urn:x:{index:03}\"/>"));
    }
    wide.push_str("</r>");
    let inspection = inspected(&wide);
    assert_eq!(inspection.structure.namespaces.len(), 64);
    assert!(inspection.structure.namespaces_truncated);
}
