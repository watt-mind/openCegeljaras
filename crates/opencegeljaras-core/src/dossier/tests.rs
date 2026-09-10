//! Core unit tests for dossier recognition, payloads, and findings.
//!
//! Every dossier here is built inline from the public element and namespace
//! names of the pinned schema. Nothing private is read and nothing is fetched.

use super::*;
use crate::form::{FormStatus, Limits, Rejection};

/// A minimal recognised form, as bytes, for use as a payload.
const FORM: &str = concat!(
    "<Nyomtatvány xmlns=\"http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/",
    "cegnyomtatvany-20260101#\">",
    "<NévfoglalásiKérelem xmlns=\"http://schema.e-cegjegyzek.hu/schema/",
    "cegnyomtatvany/nevfoglalasiKerelem-20210401#\"/></Nyomtatvány>"
);

/// The standard base64 of one text, built without the decoder under test.
fn encode(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    let mut encoded = String::new();
    for block in bytes.chunks(3) {
        let mut packed = 0u32;
        for index in 0..3 {
            packed = (packed << 8) | u32::from(block.get(index).copied().unwrap_or_default());
        }
        for index in 0..4 {
            if index <= block.len() {
                let sextet = (packed >> (18 - 6 * index)) & 0x3f;
                encoded.push(char::from(ALPHABET[sextet as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

/// A dossier carrying one document with the given profile body and object.
fn dossier(profile: &str, document: &str) -> Vec<u8> {
    format!(
        "<es:Dossier xmlns:es=\"{NAMESPACE}\" \
         xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">\
         <es:DossierProfile>{profile}</es:DossierProfile>\
         <es:Documents Id=\"Object0\">{document}</es:Documents></es:Dossier>"
    )
    .into_bytes()
}

/// The conforming name-reservation profile body.
fn profile() -> String {
    String::from(
        "<es:Title>Névfoglalási kérelem</es:Title>\
         <es:E-category>electronic dossier</es:E-category>\
         <es:CreationDate>2026-01-01T00:00:00Z</es:CreationDate>",
    )
}

/// One document with the form role, a media type, a chain, and an object.
fn document(mime: &str, chain: &str, object: &str) -> String {
    format!(
        "<es:Document><es:DocumentProfile>\
         <es:Title>A kérelem formanyomtatványa</es:Title>\
         <es:Format>{mime}</es:Format>\
         <es:SourceSize sizeValue=\"12\" sizeUnit=\"B\"/>\
         <es:BaseTransform>{chain}</es:BaseTransform>\
         </es:DocumentProfile>{object}</es:Document>"
    )
}

/// The XML media type every payload test declares.
const XML_MIME: &str = "<es:MIME-Type type=\"text\" subtype=\"xml\" extension=\"xml\"/>";
/// The plain base64 chain every payload test declares.
const PLAIN: &str = "<es:Transform Algorithm=\"base64\"/>";

/// Inspect one built dossier, expecting it to pass the boundary.
fn inspected(bytes: &[u8]) -> Inspection {
    inspect(bytes).expect("the built dossier should pass the boundary")
}

/// The payload reason of the first listed document.
fn payload(bytes: &[u8]) -> PayloadStatus {
    inspected(bytes).documents()[0].payload
}

#[test]
fn the_two_limit_profiles_are_the_documented_constants() {
    let form = Limits::default();
    assert_eq!(form.input_bytes, 16 * 1024 * 1024);
    assert_eq!(form.depth, 128);
    assert_eq!(form.elements, 200_000);
    assert_eq!(form.text_bytes, 1024 * 1024);
    assert_eq!(form.attributes, 64);
    assert_eq!(form.namespace_declarations, 256);
    assert_eq!(form.parser_nodes(), 800_000);

    let dossier = limits();
    assert_eq!(dossier.input_bytes, 64 * 1024 * 1024);
    assert_eq!(dossier.depth, 128);
    assert_eq!(dossier.elements, 1_000_000);
    assert_eq!(dossier.text_bytes, 64 * 1024 * 1024);
    assert_eq!(dossier.attributes, 64);
    assert_eq!(dossier.namespace_declarations, 256);
    assert_eq!(dossier.parser_nodes(), 4_000_000);
}

#[test]
fn recognition_separates_the_year_the_root_and_the_namespace() {
    let recognized = inspected(&dossier(&profile(), ""));
    assert_eq!(recognized.dossier.status, DossierStatus::Recognized);
    assert_eq!(recognized.dossier.profile, Some(PROFILE));
    assert_eq!(recognized.dossier.title, Some("Névfoglalási kérelem"));
    assert_eq!(recognized.dossier.category, Some("electronic dossier"));
    assert!(!recognized.dossier.receipt);
    assert_eq!(
        recognized.dossier.creation_date.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    assert!(recognized.dossier.documents_id_fixed);

    for (document, status) in [
        (
            "<es:Dossier xmlns:es=\"http://www.e-cegjegyzek.hu/2014/e-cegeljaras#\"/>",
            DossierStatus::UnsupportedVersion,
        ),
        (
            "<es:Dossier xmlns:es=\"http://www.e-cegjegyzek.hu/20261/e-cegeljaras#\"/>",
            DossierStatus::UnrecognizedRoot,
        ),
        (
            "<Dossier xmlns=\"https://www.microsec.hu/ds/e-szigno30#\"/>",
            DossierStatus::UnrecognizedRoot,
        ),
        (
            &format!("<es:Documents xmlns:es=\"{NAMESPACE}\"/>"),
            DossierStatus::UnrecognizedRoot,
        ),
        ("<Dossier/>", DossierStatus::UnrecognizedRoot),
    ] {
        let inspection = inspected(document.as_bytes());
        assert_eq!(inspection.dossier.status, status, "{document}");
        assert!(inspection.documents.is_none(), "{document}");
        assert!(status.message().is_some(), "{document}");
    }
}

#[test]
fn every_payload_reason_is_decided_in_the_contract_order() {
    let object = format!("<ds:Object>{}</ds:Object>", encode(FORM));
    assert_eq!(
        payload(&dossier(&profile(), &document(XML_MIME, PLAIN, ""))),
        PayloadStatus::ObjectMissing
    );
    assert_eq!(
        payload(&dossier(
            &profile(),
            &document(XML_MIME, PLAIN, "<ds:Object><ds:X/></ds:Object>")
        )),
        PayloadStatus::ObjectNotText
    );
    assert_eq!(
        payload(&dossier(
            &profile(),
            &document(
                XML_MIME,
                "<es:Transform Algorithm=\"zip\"/><es:Transform Algorithm=\"base64\"/>",
                &object
            )
        )),
        PayloadStatus::TransformNotPlainBase64
    );
    assert_eq!(
        payload(&dossier(
            &profile(),
            &document(
                "<es:MIME-Type type=\"application\" subtype=\"pdf\" extension=\"pdf\"/>",
                PLAIN,
                &object
            )
        )),
        PayloadStatus::NotXml
    );
    assert_eq!(
        payload(&dossier(
            &profile(),
            &document(XML_MIME, PLAIN, "<ds:Object>a</ds:Object>")
        )),
        PayloadStatus::Base64Invalid
    );
    assert_eq!(
        payload(&dossier(&profile(), &document(XML_MIME, PLAIN, &object))),
        PayloadStatus::Inspected
    );
}

#[test]
fn an_inspected_payload_carries_the_form_inspect_outcome() {
    let object = format!("<ds:Object>{}</ds:Object>", encode(FORM));
    let inspection = inspected(&dossier(&profile(), &document(XML_MIME, PLAIN, &object)));
    let facts = inspection.documents()[0]
        .form
        .as_ref()
        .expect("an inspected payload carries a form outcome")
        .as_ref()
        .expect("the synthetic payload is a recognised form");
    assert_eq!(facts.status, FormStatus::Recognized);
    assert_eq!(
        facts.variant.as_ref().map(|variant| variant.id),
        Some("nevfoglalasi-kerelem-20210401")
    );

    let refused = format!("<ds:Object>{}</ds:Object>", encode("<a"));
    let inspection = inspected(&dossier(&profile(), &document(XML_MIME, PLAIN, &refused)));
    assert_eq!(
        inspection.documents()[0].form,
        Some(Err(Rejection::MalformedXml))
    );
}

#[test]
fn a_title_reaches_the_facts_but_a_role_is_the_pattern_text() {
    let listed = "<es:Document><es:DocumentProfile>\
         <es:Title>Hiteles cégaláírási nyilatkozat - SENTINEL-ROLE-4419</es:Title>\
         </es:DocumentProfile></es:Document>";
    let inspection = inspected(&dossier(&profile(), listed));
    let document = &inspection.documents()[0];
    assert_eq!(document.role, Some("Hiteles cégaláírási nyilatkozat"));
    assert_eq!(
        document.title.as_deref(),
        Some("Hiteles cégaláírási nyilatkozat - SENTINEL-ROLE-4419")
    );
    assert!(!document.role.expect("a role matched").contains("SENTINEL"));
}

#[test]
fn a_creation_date_is_reported_only_when_it_is_lexically_a_date_time() {
    for (declared, reported) in [
        ("2026-01-01T00:00:00Z", true),
        ("2026-01-01T00:00:00", true),
        ("2026-01-01T00:00:00.5+02:00", true),
        ("-20260-01-01T00:00:00Z", true),
        ("2026-01-01", false),
        ("2026-1-01T00:00:00Z", false),
        ("2026-01-01T00:00Z", false),
        ("tegnap", false),
    ] {
        let body = format!("<es:CreationDate>{declared}</es:CreationDate>");
        let inspection = inspected(&dossier(&body, ""));
        assert_eq!(
            inspection.dossier.creation_date.is_some(),
            reported,
            "{declared}"
        );
    }
}

#[test]
fn a_receipt_makes_every_filing_finding_inapplicable() {
    let body = "<es:Title>Egyéb beadvány</es:Title>\
                <es:E-category>electronic acknowledgement</es:E-category>";
    let inspection = inspected(&dossier(body, ""));
    assert!(inspection.dossier.receipt);
    let findings = findings(&inspection, Conformance::NotPerformed(XSD_NOT_REQUESTED));
    assert_eq!(findings.len(), 11);
    for finding in &findings[..10] {
        assert_eq!(
            finding.status,
            FindingStatus::NotApplicable,
            "{}",
            finding.code
        );
        assert_eq!(finding.detail, "the dossier is a receipt");
    }
    assert_eq!(findings[10].code, "xsd_conformance");
    assert_eq!(verdict(&findings, true), Preflight::NotApplicable);
    assert_eq!(verdict(&findings, false), Preflight::Incomplete);
}

#[test]
fn the_filing_findings_report_presence_and_never_validity() {
    let object = format!("<ds:Object>{}</ds:Object>", encode(FORM));
    let inspection = inspected(&dossier(&profile(), &document(XML_MIME, PLAIN, &object)));
    let findings = findings(&inspection, Conformance::Passed);
    let status = |code: &str| {
        findings
            .iter()
            .find(|finding| finding.code == code)
            .map(|finding| finding.status)
    };
    assert_eq!(status("procedure_title"), Some(FindingStatus::Passed));
    assert_eq!(status("dossier_category"), Some(FindingStatus::Passed));
    assert_eq!(status("documents_id"), Some(FindingStatus::Passed));
    assert_eq!(status("form_document_present"), Some(FindingStatus::Passed));
    assert_eq!(status("form_recognized"), Some(FindingStatus::Passed));
    assert_eq!(
        status("form_matches_procedure"),
        Some(FindingStatus::Passed)
    );
    assert_eq!(status("document_roles_known"), Some(FindingStatus::Passed));
    assert_eq!(status("transforms_supported"), Some(FindingStatus::Passed));
    assert_eq!(status("signature_present"), Some(FindingStatus::Failed));
    assert_eq!(status("timestamp_present"), Some(FindingStatus::Failed));
    assert_eq!(status("xsd_conformance"), Some(FindingStatus::Passed));
    assert_eq!(verdict(&findings, false), Preflight::Failed);
}

#[test]
fn the_dossier_profile_accepts_a_text_node_the_form_profile_refuses() {
    let object = "A".repeat(2 * 1024 * 1024);
    let bytes = dossier(
        &profile(),
        &document(XML_MIME, PLAIN, &format!("<ds:Object>{object}</ds:Object>")),
    );
    assert!(inspect(&bytes).is_ok());
    assert_eq!(
        crate::form::inspect(&bytes, &Limits::default())
            .expect_err("the form profile caps one text node at 1 MiB")
            .code,
        Rejection::LimitExceeded
    );
}
