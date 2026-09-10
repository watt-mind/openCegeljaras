//! Contract tests for `dossier inspect` and `dossier preflight`.
//!
//! The stored dossiers are the synthetic CC0 fixtures; everything else is
//! generated into a scratch directory that the test removes. The validator is
//! the fake built out of this test executable, and one `#[ignore]` test uses
//! an operator's own mirror named by an environment variable. Nothing private
//! is read, nothing is fetched, and no output here establishes acceptance.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

#[cfg(target_os = "linux")]
#[allow(dead_code)]
#[path = "support/fake.rs"]
mod fake;

/// The marker test the fake launcher re-executes. A no-op otherwise.
#[cfg(target_os = "linux")]
#[test]
fn fake_validator_entry() {
    fake::dispatch();
}

#[cfg(not(target_os = "linux"))]
#[test]
#[ignore = "the launcher-based fake validator runs on Linux"]
fn fake_validator_entry() {}

/// The supported dossier namespace.
const NAMESPACE: &str = "http://www.e-cegjegyzek.hu/2026/e-cegeljaras#";
/// The XMLDSig namespace.
const DSIG: &str = "http://www.w3.org/2000/09/xmldsig#";
/// The role of the application form.
const FORM_ROLE: &str = "A kérelem formanyomtatványa";
/// The sentinel suffix of the suffix-role fixture; it must never be printed.
const ROLE_SENTINEL: &str = "SENTINEL-ROLE-4419";
/// The sentinel title of the unknown-role fixture.
const TITLE_SENTINEL: &str = "SENTINEL-TITLE-8802";

/// A stored fixture path at the repository root.
fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

/// The path of a file as a string.
fn text(path: &Path) -> String {
    path.to_str().expect("test paths are UTF-8").to_owned()
}

/// Run the built binary with the given arguments.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(args)
        .output()
        .expect("CLI should run")
}

/// Run `dossier inspect` over one stored fixture.
fn inspect(name: &str, extra: &[&str]) -> Output {
    let path = text(&fixture(&format!("dossiers/{name}")));
    let mut args = vec!["dossier", "inspect", path.as_str()];
    args.extend_from_slice(extra);
    run(&args)
}

/// Run `dossier preflight` over one stored fixture.
fn preflight(name: &str, extra: &[&str]) -> Output {
    let path = text(&fixture(&format!("dossiers/{name}")));
    let mut args = vec!["dossier", "preflight", path.as_str()];
    args.extend_from_slice(extra);
    run(&args)
}

/// Run `dossier inspect` over one generated file.
fn inspect_path(path: &Path, extra: &[&str]) -> Output {
    let path = text(path);
    let mut args = vec!["dossier", "inspect", path.as_str()];
    args.extend_from_slice(extra);
    run(&args)
}

fn body(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be one JSON object")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("output should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("diagnostics should be UTF-8")
}

/// Every diagnostic is exactly one fixed line naming the code and exit code.
fn assert_diagnostic(output: &Output, command: &str, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit), "{}", stderr(output));
    assert_eq!(
        stderr(output),
        format!("opencegeljaras: {command}: {code} (exit {exit})\n")
    );
}

/// The standard base64 of some bytes, written here rather than depended on.
fn encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for block in bytes.chunks(3) {
        let mut packed = 0u32;
        for index in 0..3 {
            packed = (packed << 8) | u32::from(block.get(index).copied().unwrap_or_default());
        }
        for index in 0..4 {
            if index <= block.len() {
                encoded.push(char::from(
                    ALPHABET[((packed >> (18 - 6 * index)) & 0x3f) as usize],
                ));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

/// A generated dossier with one profile body and one document list.
fn dossier(profile: &str, documents: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <es:Dossier xmlns:es=\"{NAMESPACE}\" xmlns:ds=\"{DSIG}\">\
         <es:DossierProfile>{profile}</es:DossierProfile>\
         <es:Documents Id=\"Object0\">{documents}</es:Documents></es:Dossier>\n"
    )
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
        "<es:Document><es:DocumentProfile><es:Title>{FORM_ROLE}</es:Title>\
         <es:Format>{mime}</es:Format>\
         <es:SourceSize sizeValue=\"1\" sizeUnit=\"B\"/>\
         <es:BaseTransform>{chain}</es:BaseTransform></es:DocumentProfile>\
         {object}</es:Document>"
    )
}

/// The XML media type declaration.
const XML_MIME: &str = "<es:MIME-Type type=\"text\" subtype=\"xml\" extension=\"xml\"/>";
/// The plain base64 transform chain.
const PLAIN: &str = "<es:Transform Algorithm=\"base64\"/>";

/// Write one generated file into a scratch directory.
fn written(directory: &TempDir, name: &str, text: &str) -> PathBuf {
    let path = directory.path().join(name);
    fs::write(&path, text).expect("the generated dossier should be writable");
    path
}

/// A scratch directory removed when the test ends.
fn scratch(name: &str) -> TempDir {
    TempDir::with_prefix(format!("opencegeljaras-dossier-{name}-"))
        .expect("a scratch directory should be creatable")
}

#[test]
fn the_conforming_fixture_reports_the_documented_json_shape() {
    let output = inspect("nevfoglalas-2026.es3", &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let envelope = body(&output);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "dossier inspect");
    assert_eq!(envelope["verified"], false);
    assert_eq!(envelope["submitted"], false);

    let data = &envelope["data"];
    assert_eq!(data["input"]["encoding"], "UTF-8");
    assert_eq!(data["input"]["bom"], false);
    assert_eq!(data["input"]["xml_declaration"], true);
    assert_eq!(data["root"]["local_name"], "Dossier");
    assert_eq!(data["root"]["namespace"], NAMESPACE);

    let dossier = &data["dossier"];
    assert_eq!(dossier["status"], "recognized");
    assert_eq!(dossier["profile"], "dossier-2026");
    assert_eq!(dossier["title"], "Névfoglalási kérelem");
    assert_eq!(dossier["title_known"], true);
    assert_eq!(dossier["category"], "electronic dossier");
    assert_eq!(dossier["category_known"], true);
    assert_eq!(dossier["receipt"], false);
    assert_eq!(dossier["creation_date"], "2026-01-01T00:00:00Z");
    assert_eq!(dossier["ack_email_present"], false);
    assert_eq!(dossier["location_present"], false);
    assert_eq!(dossier["identity"]["cegnev"], false);
    assert_eq!(dossier["documents_id_fixed"], true);
    assert_eq!(dossier["identity_values"], Value::Null);

    let document = &data["documents"][0];
    assert_eq!(document["index"], 0);
    assert_eq!(document["role"], FORM_ROLE);
    assert_eq!(document["role_known"], true);
    assert_eq!(document["category"], "electronic data");
    assert_eq!(document["mime"]["type"], "text");
    assert_eq!(document["mime"]["subtype"], "xml");
    assert_eq!(document["mime"]["extension"], "xml");
    assert_eq!(document["mime"]["charset_present"], false);
    assert_eq!(document["source_size"], 1209);
    assert_eq!(document["transforms"], json!(["base64"]));
    assert_eq!(document["object_present"], true);
    assert_eq!(document["nested_dossier"], false);
    assert_eq!(document["signatures"], 0);
    assert_eq!(document["timestamps"], 0);
    assert_eq!(document["payload"], "inspected");
    assert_eq!(document["title"], Value::Null);

    assert_eq!(data["documents_truncated"], false);
    assert_eq!(data["signatures_present"], 0);
    assert_eq!(data["timestamps_present"], 0);
    assert_eq!(data["values_redacted"], true);
    assert_eq!(data["checks"]["well_formed"], "passed");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(data["checks"]["signature"], "not_performed");
    assert_eq!(data["checks"]["authority_acceptance"], "not_performed");
    assert_eq!(
        data["notice"],
        "structural account of one dossier and of the form it carries; nothing was verified, decrypted, or accepted"
    );
}

#[test]
fn the_form_object_is_the_form_inspect_result_over_the_payload() {
    let inspected = inspect("nevfoglalas-2026.es3", &["--json"]);
    let carried = body(&inspected)["data"]["documents"][0]["form"].clone();

    let path = text(&fixture("forms/nevfoglalasi-conforming.xml"));
    let alone = run(&["form", "inspect", path.as_str(), "--json"]);
    assert_eq!(alone.status.code(), Some(0), "{}", stderr(&alone));
    let mut expected = body(&alone)["data"]["form"].clone();
    expected
        .as_object_mut()
        .expect("the form object is an object")
        .remove("sections_truncated");
    assert_eq!(carried, expected);
}

#[test]
fn recognition_separates_the_year_the_root_and_the_namespace() {
    let output = inspect("dossier-2023.es3", &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "unsupported_version", 4);
    let envelope = body(&output);
    assert_eq!(
        envelope["error"]["message"],
        "the dossier namespace version is not supported"
    );
    let data = &envelope["data"];
    assert_eq!(data["dossier"], json!({ "status": "unsupported_version" }));
    assert_eq!(data["documents"], Value::Null);
    assert_eq!(data["root"]["local_name"], "Dossier");

    let output = inspect("microsec-default.es3", &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "unrecognized_root", 4);
    assert_eq!(
        body(&output)["error"]["message"],
        "the root element is not a company dossier"
    );

    let directory = scratch("recognition");
    let older = written(
        &directory,
        "2014.es3",
        "<Dossier xmlns=\"http://www.e-cegjegyzek.hu/2014/e-cegeljaras#\"/>",
    );
    let output = inspect_path(&older, &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "unsupported_version", 4);

    let other = written(
        &directory,
        "other.es3",
        "<Documents xmlns=\"http://www.e-cegjegyzek.hu/2026/e-cegeljaras#\"/>",
    );
    let output = inspect_path(&other, &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "unrecognized_root", 4);
}

#[test]
fn a_refused_input_reports_the_form_inspect_codes_and_null_facts() {
    let directory = scratch("refused");
    let missing = directory.path().join("no-such-dossier.es3");
    let output = inspect_path(&missing, &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "input_unreadable", 1);
    let data = &body(&output)["data"];
    for key in [
        "root",
        "dossier",
        "documents",
        "documents_truncated",
        "signatures_present",
        "timestamps_present",
        "values_redacted",
        "checks",
    ] {
        assert_eq!(data[key], Value::Null, "{key}");
    }

    let malformed = written(&directory, "malformed.es3", "<es:Dossier");
    let output = inspect_path(&malformed, &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "malformed_xml", 1);

    let doctype = written(&directory, "doctype.es3", "<!DOCTYPE Dossier []><Dossier/>");
    let output = inspect_path(&doctype, &["--json"]);
    assert_diagnostic(&output, "dossier inspect", "prohibited_construct", 1);
    assert_eq!(
        body(&output)["error"]["message"],
        "DTD and entity declarations are not allowed"
    );
}

#[test]
fn every_payload_reason_is_reported_in_precedence_order() {
    let directory = scratch("payload");
    let form = fs::read(fixture("forms/nevfoglalasi-conforming.xml")).expect("fixture is readable");
    let object = format!("<ds:Object>{}</ds:Object>", encode(&form));
    let pdf = "<es:MIME-Type type=\"application\" subtype=\"pdf\" extension=\"pdf\"/>";
    let zipped = "<es:Transform Algorithm=\"zip\"/><es:Transform Algorithm=\"base64\"/>";
    let oversized = format!("<ds:Object>{}</ds:Object>", "A".repeat(22_369_626));

    for (name, listed, reason) in [
        ("missing", document(XML_MIME, PLAIN, ""), "object_missing"),
        (
            "element",
            document(XML_MIME, PLAIN, "<ds:Object><ds:X/></ds:Object>"),
            "object_not_text",
        ),
        (
            "zipped",
            document(XML_MIME, zipped, &object),
            "transform_not_plain_base64",
        ),
        ("pdf", document(pdf, PLAIN, &object), "not_xml"),
        (
            "oversized",
            document(XML_MIME, PLAIN, &oversized),
            "payload_too_large",
        ),
        (
            "invalid",
            document(XML_MIME, PLAIN, "<ds:Object>a</ds:Object>"),
            "base64_invalid",
        ),
        ("good", document(XML_MIME, PLAIN, &object), "inspected"),
    ] {
        let path = written(
            &directory,
            &format!("{name}.es3"),
            &dossier(&profile(), &listed),
        );
        let output = inspect_path(&path, &["--json"]);
        assert_eq!(output.status.code(), Some(0), "{name}");
        let data = &body(&output)["data"];
        assert_eq!(data["documents"][0]["payload"], reason, "{name}");
        if reason != "inspected" {
            assert_eq!(data["documents"][0]["form"], Value::Null, "{name}");
        }
    }
}

#[test]
fn a_refused_payload_is_reported_as_rejected_with_its_code() {
    let directory = scratch("rejected");
    let listed = document(
        XML_MIME,
        PLAIN,
        &format!("<ds:Object>{}</ds:Object>", encode(b"<a")),
    );
    let path = written(&directory, "rejected.es3", &dossier(&profile(), &listed));
    let data = &body(&inspect_path(&path, &["--json"]))["data"];
    assert_eq!(
        data["documents"][0]["form"],
        json!({ "status": "rejected", "code": "malformed_xml" })
    );

    let listed = document(
        XML_MIME,
        PLAIN,
        &format!("<ds:Object>{}</ds:Object>", encode(b"<Egyeb/>")),
    );
    let path = written(
        &directory,
        "unrecognized.es3",
        &dossier(&profile(), &listed),
    );
    let data = &body(&inspect_path(&path, &["--json"]))["data"];
    assert_eq!(
        data["documents"][0]["form"],
        json!({
            "status": "unrecognized_root",
            "profile": Value::Null,
            "variant": Value::Null,
            "sections": Value::Null,
        })
    );
}

#[test]
fn declared_values_reach_the_output_only_with_show_values() {
    for extra in [vec!["--json"], vec![]] {
        let output = inspect("suffix-role.es3", &extra);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(!stdout(&output).contains(ROLE_SENTINEL));
        assert!(!stderr(&output).contains(ROLE_SENTINEL));
    }
    let output = inspect("suffix-role.es3", &["--json", "--show-values"]);
    let data = &body(&output)["data"];
    assert_eq!(data["values_redacted"], false);
    assert_eq!(
        data["documents"][1]["role"],
        "Hiteles cégaláírási nyilatkozat"
    );
    assert_eq!(
        data["documents"][1]["title"],
        format!("Hiteles cégaláírási nyilatkozat - {ROLE_SENTINEL}")
    );
    assert!(stderr(&output).is_empty());

    let output = inspect("unknown-role.es3", &["--json"]);
    let data = &body(&output)["data"];
    assert_eq!(data["documents"][1]["role"], Value::Null);
    assert_eq!(data["documents"][1]["role_known"], false);
    assert!(!stdout(&output).contains(TITLE_SENTINEL));
    let output = inspect("unknown-role.es3", &["--json", "--show-values"]);
    assert_eq!(
        body(&output)["data"]["documents"][1]["title"],
        TITLE_SENTINEL
    );
}

#[test]
fn identity_values_are_reported_as_presence_and_only_then_as_values() {
    let directory = scratch("identity");
    let body_text = format!(
        "{}<es:Cégnév Custom=\"true\">{TITLE_SENTINEL}</es:Cégnév>",
        profile()
    );
    let path = written(&directory, "identity.es3", &dossier(&body_text, ""));
    let output = inspect_path(&path, &["--json"]);
    let data = &body(&output)["data"];
    assert_eq!(data["dossier"]["identity"]["cegnev"], true);
    assert_eq!(data["dossier"]["identity"]["cegforma"], false);
    assert!(!stdout(&output).contains(TITLE_SENTINEL));

    let output = inspect_path(&path, &["--json", "--show-values"]);
    let identity = &body(&output)["data"]["dossier"]["identity_values"];
    assert_eq!(identity["cegnev"], TITLE_SENTINEL);
    assert_eq!(identity["cegforma"], Value::Null);
    assert_eq!(identity["cegjegyzekszam"], Value::Null);
    assert_eq!(identity["cegszekhely"], Value::Null);
}

#[test]
fn a_nested_dossier_is_flagged_and_never_opened() {
    let data = &body(&inspect("nested-dossier.es3", &["--json"]))["data"];
    assert_eq!(data["documents"][1]["nested_dossier"], true);
    assert_eq!(data["documents"][1]["payload"], "not_xml");
    assert_eq!(data["documents"][0]["nested_dossier"], false);
}

#[test]
fn an_unknown_transform_algorithm_is_named_unknown() {
    let data = &body(&inspect("encrypted-form.es3", &["--json"]))["data"];
    assert_eq!(
        data["documents"][0]["transforms"],
        json!(["encrypt", "base64"])
    );
    assert_eq!(
        data["documents"][1]["transforms"],
        json!(["unknown", "unknown"])
    );
    assert_eq!(
        data["documents"][0]["payload"],
        "transform_not_plain_base64"
    );
}

#[test]
fn an_iso_8859_2_dossier_is_read_under_its_declared_encoding() {
    let output = inspect("iso-8859-2.es3", &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let data = &body(&output)["data"];
    assert_eq!(data["input"]["encoding"], "ISO-8859-2");
    assert_eq!(data["dossier"]["title"], "Névfoglalási kérelem");
    assert_eq!(data["documents"][0]["role"], FORM_ROLE);
}

#[test]
fn documents_past_the_cap_are_cut_and_marked_truncated() {
    let directory = scratch("truncated");
    let listed = document(XML_MIME, PLAIN, "").repeat(257);
    let path = written(&directory, "many.es3", &dossier(&profile(), &listed));
    let data = &body(&inspect_path(&path, &["--json"]))["data"];
    assert_eq!(
        data["documents"].as_array().expect("a document list").len(),
        256
    );
    assert_eq!(data["documents_truncated"], true);

    let output = run(&["dossier", "preflight", text(&path).as_str(), "--json"]);
    let findings = &body(&output)["data"]["findings"];
    let roles = findings
        .as_array()
        .expect("findings are listed")
        .iter()
        .find(|finding| finding["code"] == "document_roles_known")
        .expect("the roles finding is reported");
    assert_eq!(roles["status"], "not_performed");
}

#[test]
fn the_human_report_prints_the_documented_lines_in_order() {
    let output = inspect("nevfoglalas-2026.es3", &["--show-values"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report = stdout(&output);
    let expected = [
        "root: \"Dossier\" in ",
        "\nencoding: UTF-8\n",
        "\nbom: false\n",
        "\nxml declaration: true\n",
        "\nstatus: recognized\n",
        "\nprofile: dossier-2026\n",
        "\ncategory: \"electronic dossier\"\n",
        "\nreceipt: false\n",
        "\ncreation date: \"2026-01-01T00:00:00Z\"\n",
        "\nack email: absent\n",
        "\nlocation: absent\n",
        "\nidentity: cegforma absent, cegjegyzekszam absent, cegnev absent, cegszekhely absent\n",
        "\ndocuments id fixed: true\n",
        "\ndocuments: 1\n",
        "\n      transforms: base64\n",
        "\n      payload: inspected\n",
        "\n      form: recognized nevfoglalasi-kerelem-20210401\n",
        "\n      signatures: 0 timestamps: 0\n",
        "\nsignatures present: 0\n",
        "\ntimestamps present: 0\n",
        "\nwell-formed: passed\nxsd conformance: not performed\nverification: not performed\nsubmission: not performed\n",
    ];
    let mut cursor = 0;
    for line in expected {
        let found = report[cursor..]
            .find(line)
            .unwrap_or_else(|| panic!("{line:?} should follow in\n{report}"));
        cursor += found + line.len() - 1;
    }
    assert!(
        report.contains("\n  [0] role: \"A k\\u00e9relem"),
        "{report}"
    );
}

#[test]
fn the_preflight_findings_reach_each_reachable_status() {
    let status = |fixture: &str, code: &str| {
        let output = preflight(fixture, &["--json"]);
        body(&output)["data"]["findings"]
            .as_array()
            .expect("findings are listed")
            .iter()
            .find(|finding| finding["code"] == code)
            .map(|finding| finding["status"].as_str().unwrap_or_default().to_owned())
            .unwrap_or_default()
    };
    assert_eq!(status("nevfoglalas-2026.es3", "procedure_title"), "passed");
    assert_eq!(
        status(
            "nevfoglalas-2026-bad-procedure-title.es3",
            "procedure_title"
        ),
        "failed"
    );
    assert_eq!(status("receipt.es3", "dossier_category"), "not_applicable");
    assert_eq!(status("two-forms.es3", "form_document_present"), "failed");
    assert_eq!(status("two-forms.es3", "form_recognized"), "not_performed");
    assert_eq!(status("zip-form.es3", "form_recognized"), "not_performed");
    assert_eq!(
        status("mismatched-variant.es3", "form_matches_procedure"),
        "failed"
    );
    assert_eq!(status("unknown-role.es3", "document_roles_known"), "failed");
    assert_eq!(
        status("encrypted-form.es3", "transforms_supported"),
        "failed"
    );
    assert_eq!(
        status("nevfoglalas-2026.es3", "signature_present"),
        "failed"
    );
    assert_eq!(
        status("nevfoglalas-2026-signed-shape.es3", "timestamp_present"),
        "passed"
    );
    assert_eq!(
        status("nevfoglalas-2026.es3", "xsd_conformance"),
        "not_performed"
    );
}

#[test]
fn a_procedure_without_a_fixed_variant_makes_the_match_inapplicable() {
    let directory = scratch("egyeb");
    let form = fs::read(fixture("forms/nevfoglalasi-conforming.xml")).expect("fixture is readable");
    let listed = document(
        XML_MIME,
        PLAIN,
        &format!("<ds:Object>{}</ds:Object>", encode(&form)),
    );
    let body_text = "<es:Title>Egyéb beadvány</es:Title>\
                     <es:E-category>elektronikus akta</es:E-category>";
    let path = written(&directory, "egyeb.es3", &dossier(body_text, &listed));
    let output = run(&["dossier", "preflight", text(&path).as_str(), "--json"]);
    let findings = body(&output)["data"]["findings"].clone();
    let status = |code: &str| {
        findings
            .as_array()
            .expect("findings are listed")
            .iter()
            .find(|finding| finding["code"] == code)
            .map(|finding| finding["status"].clone())
            .expect("the finding is reported")
    };
    assert_eq!(status("form_matches_procedure"), "not_applicable");
    assert_eq!(status("dossier_category"), "passed");
}

#[test]
fn a_receipt_is_recognised_and_never_acknowledged() {
    let output = preflight("receipt.es3", &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let envelope = body(&output);
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["dossier"]["receipt"], true);
    assert_eq!(data["preflight"], json!({ "status": "not_applicable" }));
    for finding in data["findings"].as_array().expect("findings are listed") {
        if finding["code"] == "xsd_conformance" {
            continue;
        }
        assert_eq!(finding["status"], "not_applicable", "{finding}");
        assert_eq!(finding["detail"], "the dossier is a receipt");
    }
    let notice = data["notice"].as_str().expect("a notice is reported");
    assert!(
        notice.ends_with("; receipt recognised, not acknowledged"),
        "{notice}"
    );
}

#[test]
fn a_failing_finding_exits_four_and_a_complete_run_exits_zero() {
    let output = preflight("nevfoglalas-2026.es3", &["--json"]);
    assert_diagnostic(&output, "dossier preflight", "preflight_failed", 4);
    let envelope = body(&output);
    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["message"],
        "at least one preflight finding failed"
    );
    assert_eq!(envelope["data"]["preflight"], json!({ "status": "failed" }));

    let output = preflight("nevfoglalas-2026-signed-shape.es3", &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let data = &body(&output)["data"];
    assert_eq!(data["preflight"], json!({ "status": "incomplete" }));
    for key in [
        "bundle",
        "validator",
        "xsd_findings",
        "xsd_findings_truncated",
        "unparsed_lines",
    ] {
        assert_eq!(data[key], Value::Null, "{key}");
    }

    let output = preflight("dossier-2023.es3", &["--json"]);
    assert_diagnostic(&output, "dossier preflight", "unsupported_version", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["findings"], Value::Null);
    assert_eq!(data["preflight"], Value::Null);
}

#[test]
fn the_preflight_human_report_prints_findings_and_a_verdict() {
    let output = preflight("nevfoglalas-2026-signed-shape.es3", &[]);
    let report = stdout(&output);
    assert!(report.contains("\nfindings:\n"), "{report}");
    assert!(
        report.contains("\n  procedure_title: passed (xsd-2026)\n"),
        "{report}"
    );
    assert!(
        report.contains("\n  dossier_category: passed (openCegeljaras)\n"),
        "{report}"
    );
    assert!(
        report.contains("\n  form_recognized: passed (form-2026)\n"),
        "{report}"
    );
    assert!(report.contains("\npreflight: incomplete\n"), "{report}");
    assert!(
        report.ends_with("well-formed: passed\nxsd conformance: not performed\nverification: not performed\nsubmission: not performed\n"),
        "{report}"
    );
}

#[test]
fn a_text_node_past_the_validator_ceiling_stops_before_the_validator() {
    let directory = scratch("ceiling");
    let form = fs::read(fixture("forms/nevfoglalasi-conforming.xml")).expect("fixture is readable");
    let listed = document(
        XML_MIME,
        PLAIN,
        &format!("<ds:Object>{}</ds:Object>", encode(&form)),
    );
    let padding = format!("<es:TimeStamp>{}</es:TimeStamp>", "A".repeat(10_000_004));
    let path = written(
        &directory,
        "huge.es3",
        &dossier(&profile(), &listed).replace(
            "</es:Dossier>",
            &format!("<ds:Signature/>{padding}</es:Dossier>"),
        ),
    );
    let output = run(&[
        "dossier",
        "preflight",
        text(&path).as_str(),
        "--bundle",
        text(&fixture("schemas/good")).as_str(),
        "--manifest",
        text(&fixture("schemas/manifest.json")).as_str(),
        "--xmllint",
        "/no/such/xmllint",
        "--json",
    ]);
    assert_diagnostic(
        &output,
        "dossier preflight",
        "payload_exceeds_validator_limit",
        5,
    );
    let envelope = body(&output);
    assert_eq!(
        envelope["error"]["message"],
        "a text node exceeds the validator's 10 000 000-byte limit"
    );
    let data = &envelope["data"];
    assert_eq!(data["validator"], Value::Null);
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    let xsd = data["findings"]
        .as_array()
        .expect("findings are listed")
        .iter()
        .find(|finding| finding["code"] == "xsd_conformance")
        .expect("the xsd finding is reported");
    assert_eq!(xsd["status"], "not_performed");
    assert_eq!(xsd["detail"], "payload_exceeds_validator_limit");
}

/// Copy one fixture tree into a scratch directory.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the scratch tree should be creatable");
    for entry in fs::read_dir(from).expect("the fixture tree should be readable") {
        let entry = entry.expect("the fixture entry should be readable");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("the fixture file should be copyable");
        }
    }
}

#[test]
fn an_unverified_dossier_closure_leaves_the_conformance_unperformed() {
    let directory = scratch("dossier-closure");
    let bundle = directory.path().join("bundle");
    copy_tree(&fixture("schemas/good"), &bundle);
    fs::write(
        bundle.join("www.e-cegjegyzek.hu/schema/mini/dossier-20260101.xsd"),
        "<!-- tampered -->",
    )
    .expect("the copied dossier root should be writable");

    let output = preflight(
        "nevfoglalas-2026-signed-shape.es3",
        &[
            "--bundle",
            text(&bundle).as_str(),
            "--manifest",
            text(&fixture("schemas/manifest.json")).as_str(),
            "--json",
        ],
    );
    assert_diagnostic(&output, "dossier preflight", "bundle_unverified", 5);
    let envelope = body(&output);
    assert_eq!(
        envelope["error"]["message"],
        "the schema bundle's dossier closure does not match the pinned manifest"
    );
    let data = &envelope["data"];
    assert_eq!(data["bundle"]["bundle"]["dossier_closure_verified"], false);
    assert_eq!(data["validator"], Value::Null);
    assert_eq!(data["preflight"], json!({ "status": "incomplete" }));
}

#[test]
fn a_missing_validator_leaves_the_conformance_unperformed() {
    let output = preflight(
        "nevfoglalas-2026-signed-shape.es3",
        &[
            "--bundle",
            text(&fixture("schemas/good")).as_str(),
            "--manifest",
            text(&fixture("schemas/manifest.json")).as_str(),
            "--xmllint",
            "/no/such/xmllint",
            "--json",
        ],
    );
    assert_diagnostic(&output, "dossier preflight", "validator_unavailable", 5);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["bundle"]["dossier_closure_verified"], true);
    assert_eq!(data["validator"]["name"], "xmllint");
    assert_eq!(data["validator"]["version"], Value::Null);
}

#[test]
fn a_manifest_without_a_dossier_root_is_invalid() {
    let directory = scratch("one-root");
    let text_manifest =
        fs::read_to_string(fixture("schemas/manifest.json")).expect("the manifest is readable");
    let mut manifest: Value = serde_json::from_str(&text_manifest).expect("the manifest is JSON");
    let roots = manifest["roots"]
        .as_array()
        .expect("roots are listed")
        .first()
        .cloned()
        .expect("one root");
    manifest["roots"] = json!([roots]);
    let path = written(&directory, "manifest.json", &manifest.to_string());

    let output = preflight(
        "nevfoglalas-2026-signed-shape.es3",
        &[
            "--bundle",
            text(&fixture("schemas/good")).as_str(),
            "--manifest",
            text(&path).as_str(),
            "--json",
        ],
    );
    assert_diagnostic(&output, "dossier preflight", "manifest_invalid", 1);
    let data = &body(&output)["data"];
    assert_eq!(data["findings"], Value::Null);
    assert_eq!(data["dossier"]["status"], "recognized");
}

#[test]
fn output_is_byte_identical_across_runs() {
    for arguments in [
        vec!["--json"],
        vec!["--json", "--show-values"],
        vec!["--show-values"],
    ] {
        let first = inspect("suffix-role.es3", &arguments);
        let second = inspect("suffix-role.es3", &arguments);
        assert_eq!(first.stdout, second.stdout);
        assert_eq!(first.stderr, second.stderr);

        let first = preflight("suffix-role.es3", &arguments);
        let second = preflight("suffix-role.es3", &arguments);
        assert_eq!(first.stdout, second.stdout);
        assert_eq!(first.stderr, second.stderr);
    }
}

#[test]
fn capabilities_lists_both_dossier_operations_last() {
    let output = run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let operations = body(&output)["data"]["operations"].clone();
    assert_eq!(
        operations,
        json!([
            "form inspect",
            "form validate",
            "form fields",
            "form diff",
            "schemas list",
            "schemas verify",
            "dossier inspect",
            "dossier preflight"
        ])
    );
}

/// Nothing here reads or writes the private corpus.
#[test]
fn every_stored_fixture_this_suite_names_exists_and_none_is_private() {
    for relative in [
        "dossiers/nevfoglalas-2026.es3",
        "dossiers/nevfoglalas-2026-bad-procedure-title.es3",
        "dossiers/nevfoglalas-2026-signed-shape.es3",
        "dossiers/nested-dossier.es3",
        "dossiers/encrypted-form.es3",
        "dossiers/iso-8859-2.es3",
        "dossiers/suffix-role.es3",
        "dossiers/microsec-default.es3",
        "dossiers/dossier-2023.es3",
        "dossiers/mismatched-variant.es3",
        "dossiers/two-forms.es3",
        "dossiers/zip-form.es3",
        "dossiers/unknown-role.es3",
        "dossiers/receipt.es3",
        "dossiers/bad-base64.es3",
        "forms/nevfoglalasi-conforming.xml",
        "schemas/manifest.json",
        "schemas/good",
        "schemas/tampered",
    ] {
        assert!(fs::metadata(fixture(relative)).is_ok(), "{relative}");
        assert!(!relative.contains("samples"), "{relative}");
    }
}

#[cfg(target_os = "linux")]
mod faked {
    use super::*;

    /// One violation line the fake validator can print.
    const VIOLATION: &str = "document.xml:4: element Title: Schemas validity error : Element '{http://www.e-cegjegyzek.hu/2026/e-cegeljaras#}Title': [facet 'enumeration'] The value 'SENTINEL-VALUE' is not an element of the set {'a'}.";

    /// Run `dossier preflight` against the good mini bundle and one fake case.
    fn with_fake(name: &str, script: &str, extra: &[&str]) -> Output {
        let case = fake::case(name, script);
        let exe = text(&case.exe);
        let mut args = vec!["--bundle", "", "--manifest", "", "--xmllint", exe.as_str()];
        let bundle = text(&fixture("schemas/good"));
        let manifest = text(&fixture("schemas/manifest.json"));
        args[1] = bundle.as_str();
        args[3] = manifest.as_str();
        args.extend_from_slice(extra);
        preflight("nevfoglalas-2026-signed-shape.es3", &args)
    }

    #[test]
    fn a_conforming_dossier_passes_the_xsd_finding() {
        let output = with_fake("dossier-conforming", "exit=0\n", &["--json"]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(output.stderr.is_empty());
        let data = &body(&output)["data"];
        assert_eq!(data["checks"]["xsd_conformance"], "passed");
        assert_eq!(data["preflight"], json!({ "status": "passed" }));
        assert_eq!(data["validator"]["version"], 20914);
        assert_eq!(data["xsd_findings"], json!([]));
        assert_eq!(data["xsd_findings_truncated"], false);
        assert_eq!(data["unparsed_lines"], 0);
        assert_eq!(data["values_redacted"], true);
    }

    #[test]
    fn a_nonconforming_dossier_fails_the_xsd_finding_with_redacted_messages() {
        let script = format!("exit=3\nstderr={VIOLATION}\nstderr=document.xml fails to validate\n");
        let output = with_fake("dossier-nonconforming", &script, &["--json"]);
        assert_diagnostic(&output, "dossier preflight", "preflight_failed", 4);
        let data = &body(&output)["data"];
        assert_eq!(data["checks"]["xsd_conformance"], "failed");
        assert_eq!(data["preflight"], json!({ "status": "failed" }));
        let message = data["xsd_findings"][0]["message"]
            .as_str()
            .expect("a message is reported");
        assert!(!message.contains("SENTINEL-VALUE"), "{message}");
        assert!(message.contains("'…'"), "{message}");
        assert!(!stdout(&output).contains("SENTINEL-VALUE"));
    }

    #[test]
    fn the_human_report_prints_the_xsd_findings_after_the_findings() {
        let script = format!("exit=3\nstderr={VIOLATION}\n");
        let output = with_fake("dossier-human", &script, &[]);
        let report = stdout(&output);
        assert!(report.contains("\nxsd findings:\n"), "{report}");
        assert!(
            report.contains("\n  line 4 element \"Title\": "),
            "{report}"
        );
        assert!(report.contains("\npreflight: failed\n"), "{report}");
        assert!(report.contains("\nxsd conformance: failed\n"), "{report}");
    }

    #[test]
    fn a_failing_validator_leaves_the_conformance_unperformed() {
        let output = with_fake("dossier-failed", "exit=5\n", &["--json"]);
        assert_diagnostic(&output, "dossier preflight", "validator_failed", 5);
        let data = &body(&output)["data"];
        assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
        assert_eq!(data["validator"]["exit_code"], 5);
        assert_eq!(data["xsd_findings"], Value::Null);
    }
}

#[cfg(not(target_os = "linux"))]
mod faked {
    //! Ignored stubs keeping the Linux test names visible off Linux.

    macro_rules! stubs {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                #[ignore = "the launcher-based fake validator runs on Linux"]
                fn $name() {}
            )*
        };
    }

    stubs!(
        a_conforming_dossier_passes_the_xsd_finding,
        a_nonconforming_dossier_fails_the_xsd_finding_with_redacted_messages,
        the_human_report_prints_the_xsd_findings_after_the_findings,
        a_failing_validator_leaves_the_conformance_unperformed,
    );
}

#[test]
#[ignore = "needs a real mirror named by OPENCEGELJARAS_SCHEMA_BUNDLE and a real xmllint"]
fn the_evidence_dossiers_agree_with_the_recorded_evidence() {
    let bundle = std::env::var("OPENCEGELJARAS_SCHEMA_BUNDLE")
        .expect("set OPENCEGELJARAS_SCHEMA_BUNDLE to a local mirror of the pinned schema bundle");

    let conforming = preflight(
        "nevfoglalas-2026.es3",
        &["--bundle", bundle.as_str(), "--json"],
    );
    assert_diagnostic(&conforming, "dossier preflight", "preflight_failed", 4);
    let data = &body(&conforming)["data"];
    assert_eq!(data["checks"]["xsd_conformance"], "passed");
    assert_eq!(data["bundle"]["bundle"]["dossier_closure_verified"], true);
    assert_eq!(data["xsd_findings"], json!([]));
    assert!(!stdout(&conforming).contains(&bundle));

    let rejected = preflight(
        "nevfoglalas-2026-bad-procedure-title.es3",
        &["--bundle", bundle.as_str(), "--json"],
    );
    assert_diagnostic(&rejected, "dossier preflight", "preflight_failed", 4);
    let data = &body(&rejected)["data"];
    assert_eq!(data["checks"]["xsd_conformance"], "failed");
    let message = data["xsd_findings"][0]["message"]
        .as_str()
        .expect("a message is reported");
    assert!(message.contains("[facet 'enumeration']"), "{message}");
}
