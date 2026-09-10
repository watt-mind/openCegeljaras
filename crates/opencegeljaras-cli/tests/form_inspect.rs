//! Contract tests for `form inspect` over the built binary.
//!
//! Every fixture used here is synthetic. Generated inputs are written to a
//! per-process temporary directory; nothing private is read.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;

/// The synthetic fixture directory at the repository root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/forms")
        .join(name)
}

/// A fresh path inside a per-process scratch directory.
fn scratch(name: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "opencegeljaras-form-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory.join(name)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(args)
        .output()
        .expect("CLI should run")
}

fn inspect(path: &Path, json: bool) -> Output {
    let path = path.to_str().expect("test paths are UTF-8");
    if json {
        run(&["form", "inspect", path, "--json"])
    } else {
        run(&["form", "inspect", path])
    }
}

fn body(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be one JSON object")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("diagnostics should be UTF-8")
}

/// Every diagnostic is exactly one fixed line naming the code and exit code.
fn assert_single_diagnostic(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit));
    assert_eq!(
        stderr(output),
        format!("opencegeljaras: form inspect: {code} (exit {exit})\n")
    );
}

/// Write a generated document, inspect it, and remove it again.
fn inspect_generated(name: &str, text: &str) -> Output {
    let path = scratch(name);
    fs::write(&path, text).expect("the scratch file should be writable");
    let output = inspect(&path, true);
    discard(&path);
    output
}

/// Remove a scratch directory and everything written into it.
fn discard(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn every_stored_fixture_reports_its_contract_code() {
    let expectations: [(&str, i32, &str, &str); 17] = [
        ("cegbejegyzesi-minimal-utf8.xml", 0, "", ""),
        ("valtozasbejegyzesi-stylesheet-bom.xml", 0, "", ""),
        ("nevfoglalasi-iso-8859-2.xml", 0, "", ""),
        ("nevfoglalasi-no-declaration.xml", 0, "", ""),
        ("cegbejegyzesi-no-encoding-label.xml", 0, "", ""),
        (
            "wrapper-no-namespace.xml",
            4,
            "unrecognized_root",
            "the root element is not a known company form",
        ),
        (
            "bom-declared-iso-8859-2.xml",
            1,
            "invalid_encoding",
            "the input is not valid in its declared encoding",
        ),
        (
            "wrapper-20240101-unsupported.xml",
            4,
            "unsupported_version",
            "the form wrapper version is not supported",
        ),
        (
            "wrapper-unknown-child.xml",
            4,
            "unrecognized_variant",
            "the 2026 wrapper does not contain exactly one known form variant",
        ),
        (
            "wrapper-two-children.xml",
            4,
            "unrecognized_variant",
            "the 2026 wrapper does not contain exactly one known form variant",
        ),
        (
            "unrecognized-root.xml",
            4,
            "unrecognized_root",
            "the root element is not a known company form",
        ),
        (
            "doctype.xml",
            1,
            "prohibited_construct",
            "DTD and entity declarations are not allowed",
        ),
        (
            "undeclared-entity.xml",
            1,
            "malformed_xml",
            "the input is not well-formed XML",
        ),
        (
            "malformed.xml",
            1,
            "malformed_xml",
            "the input is not well-formed XML",
        ),
        (
            "utf16-bom.xml",
            1,
            "unsupported_encoding",
            "only UTF-8 and ISO-8859-2 XML encodings are supported",
        ),
        (
            "declared-utf8-invalid-bytes.xml",
            1,
            "invalid_encoding",
            "the input is not valid in its declared encoding",
        ),
        (
            "declared-windows-1250.xml",
            1,
            "unsupported_encoding",
            "only UTF-8 and ISO-8859-2 XML encodings are supported",
        ),
    ];

    for (name, exit, code, message) in expectations {
        let output = inspect(&fixture(name), true);
        assert_eq!(output.status.code(), Some(exit), "fixture: {name}");
        if exit == 0 {
            assert!(output.stderr.is_empty(), "fixture: {name}");
            let body = body(&output);
            assert_eq!(body["ok"], true, "fixture: {name}");
            assert!(body.get("error").is_none(), "fixture: {name}");
            assert_eq!(body["data"]["form"]["status"], "recognized");
            continue;
        }
        assert_single_diagnostic(&output, code, exit);
        let body = body(&output);
        assert_eq!(body["ok"], false, "fixture: {name}");
        assert_eq!(body["schema_version"], 1, "fixture: {name}");
        assert_eq!(body["command"], "form inspect", "fixture: {name}");
        assert_eq!(body["error"]["code"], code, "fixture: {name}");
        assert_eq!(body["error"]["message"], message, "fixture: {name}");
        assert_eq!(body["verified"], false, "fixture: {name}");
        assert_eq!(body["submitted"], false, "fixture: {name}");
        if exit == 4 {
            assert_eq!(body["data"]["form"]["status"], code, "fixture: {name}");
        }
    }
}

#[test]
fn a_recognized_form_reports_its_variant_and_boundaries() {
    let output = inspect(&fixture("cegbejegyzesi-minimal-utf8.xml"), true);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let body = body(&output);
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["command"], "form inspect");
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);

    let data = &body["data"];
    assert_eq!(data["input"]["encoding"], "UTF-8");
    assert_eq!(data["input"]["bom"], false);
    assert_eq!(data["input"]["xml_declaration"], true);
    assert_eq!(data["input"]["stylesheet_pi"], false);
    assert_eq!(data["input"]["schema_location_declared"], false);
    assert_eq!(data["root"]["local_name"], "Nyomtatvány");
    assert_eq!(
        data["root"]["namespace"],
        "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#"
    );
    assert_eq!(data["form"]["profile"], "form-2026");
    assert_eq!(
        data["form"]["variant"]["id"],
        "cegbejegyzesi-kerelem-20260101"
    );
    assert_eq!(data["form"]["sections"][0]["local_name"], "Adatlap");
    assert_eq!(data["form"]["sections"][0]["count"], 1);
    assert_eq!(data["form"]["sections_truncated"], false);
    assert_eq!(data["structure"]["namespaces_truncated"], false);
    assert!(
        data["structure"]["elements"]
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    assert_eq!(data["checks"]["well_formed"], "passed");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(data["checks"]["signature"], "not_performed");
    assert_eq!(data["checks"]["authority_acceptance"], "not_performed");
    assert_eq!(
        data["notice"],
        "format recognition only; not schema validation, verification, or acceptance"
    );
}

#[test]
fn a_stylesheet_instruction_and_a_byte_order_mark_are_reported_not_resolved() {
    let output = inspect(&fixture("valtozasbejegyzesi-stylesheet-bom.xml"), true);
    assert_eq!(output.status.code(), Some(0));
    let data = body(&output)["data"].clone();
    assert_eq!(data["input"]["bom"], true);
    assert_eq!(data["input"]["stylesheet_pi"], true);
    assert_eq!(
        data["form"]["variant"]["id"],
        "valtozasbejegyzesi-kerelem-20240101"
    );

    let hinted = inspect(&fixture("cegbejegyzesi-no-encoding-label.xml"), true);
    let data = body(&hinted)["data"].clone();
    assert_eq!(data["input"]["schema_location_declared"], true);
    assert_eq!(data["input"]["encoding"], "UTF-8");
    assert_eq!(data["input"]["xml_declaration"], true);

    let iso = inspect(&fixture("nevfoglalasi-iso-8859-2.xml"), true);
    let data = body(&iso)["data"].clone();
    assert_eq!(data["input"]["encoding"], "ISO-8859-2");
    assert_eq!(
        data["form"]["variant"]["id"],
        "nevfoglalasi-kerelem-20210401"
    );

    let plain = inspect(&fixture("nevfoglalasi-no-declaration.xml"), true);
    let data = body(&plain)["data"].clone();
    assert_eq!(data["input"]["xml_declaration"], false);
    assert_eq!(data["input"]["encoding"], "UTF-8");
}

#[test]
fn the_human_report_lists_the_contract_lines_in_order() {
    let output = inspect(&fixture("cegbejegyzesi-minimal-utf8.xml"), false);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).expect("output should be UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "status: recognized");
    assert_eq!(lines[1], "profile: form-2026");
    assert_eq!(lines[2], "variant: cegbejegyzesi-kerelem-20260101");
    assert_eq!(
        lines[3],
        "root: \"Nyomtatvány\" in \
         \"http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#\""
    );
    assert_eq!(lines[4], "encoding: UTF-8");
    assert_eq!(lines[5], "bom: false");
    assert_eq!(lines[6], "xml declaration: true");
    assert_eq!(lines[7], "stylesheet pi: false");
    assert_eq!(lines[8], "schema location declared: false");
    assert_eq!(lines[9], "elements: 7");
    assert_eq!(lines[10], "max depth: 4");
    assert_eq!(lines[11], "attributes: 0");
    assert_eq!(lines[12], "text bytes: 105");
    assert_eq!(
        lines[13],
        "sections: \"Adatlap\"×1, \"ÁltalánosAdatok\"×1, \"CégformátólFüggőAdatok\"×1"
    );
    assert_eq!(lines[14], "namespaces:");
    assert_eq!(
        lines[15],
        "  \"http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/\
         cegbejegyzesiKerelem-20260101#\""
    );
    assert_eq!(lines[lines.len() - 4], "well-formed: passed");
    assert_eq!(lines[lines.len() - 3], "xsd conformance: not performed");
    assert_eq!(lines[lines.len() - 2], "verification: not performed");
    assert_eq!(lines[lines.len() - 1], "submission: not performed");
}

#[test]
fn an_unrecognized_document_prints_the_report_with_null_form_fields() {
    let output = inspect(&fixture("wrapper-no-namespace.xml"), false);
    assert_single_diagnostic(&output, "unrecognized_root", 4);
    let text = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(text.starts_with("status: unrecognized_root\nprofile: none\nvariant: none\n"));
    assert!(text.contains("root: \"Nyomtatvány\" in none\n"));
    assert!(text.contains("sections: none\n"));
    assert!(text.contains("well-formed: passed\n"));

    let json = inspect(&fixture("wrapper-no-namespace.xml"), true);
    assert_single_diagnostic(&json, "unrecognized_root", 4);
    let data = body(&json)["data"].clone();
    assert_eq!(data["form"]["status"], "unrecognized_root");
    assert!(data["form"]["profile"].is_null());
    assert!(data["form"]["variant"].is_null());
    assert!(data["form"]["sections"].is_null());
    assert!(data["root"]["namespace"].is_null());
    assert_eq!(data["checks"]["well_formed"], "passed");
}

#[test]
fn a_rejected_input_writes_no_report_without_json() {
    let output = inspect(&fixture("malformed.xml"), false);
    assert_single_diagnostic(&output, "malformed_xml", 1);
    assert!(output.stdout.is_empty());
}

#[test]
fn a_rejected_input_reports_only_the_facts_established_before_it() {
    let output = inspect(&fixture("declared-windows-1250.xml"), true);
    let data = body(&output)["data"].clone();
    assert!(data["input"]["bytes"].as_u64().is_some_and(|n| n > 0));
    assert_eq!(data["input"]["bom"], false);
    assert_eq!(data["input"]["xml_declaration"], true);
    assert!(data["input"].get("encoding").is_none());
    for object in ["root", "form", "structure", "checks"] {
        assert!(data[object].is_null(), "object: {object}");
    }
}

#[test]
fn an_unreadable_input_never_names_the_path_or_the_reason() {
    let sentinel = "private-path-sentinel-cegjegyzek";
    let missing = scratch(&format!("{sentinel}.xml"));
    let directory = scratch("directory");
    fs::create_dir_all(&directory).expect("a directory should be creatable");
    let inside = directory.join(sentinel);
    fs::create_dir_all(&inside).expect("a directory should be creatable");

    for path in [missing.clone(), inside] {
        for json in [true, false] {
            let output = inspect(&path, json);
            assert_single_diagnostic(&output, "input_unreadable", 1);
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            assert!(!stdout.contains(sentinel));
            assert!(!stderr(&output).contains(sentinel));
            assert!(!stdout.contains("directory"));
            if json {
                let parsed: Value =
                    serde_json::from_str(&stdout).expect("stdout should be one JSON object");
                assert!(parsed["data"]["input"].is_null());
                assert_eq!(
                    parsed["error"]["message"],
                    "the input could not be read as a regular file"
                );
            } else {
                assert!(stdout.is_empty());
            }
        }
    }
    discard(&missing);
    discard(&directory);
}

#[test]
fn an_oversized_input_is_refused_from_its_metadata() {
    let path = scratch("oversized.xml");
    let limit = 16 * 1024 * 1024;
    let mut text = String::from("<a>");
    text.push_str(&"x".repeat(limit));
    text.push_str("</a>");
    fs::write(&path, &text).expect("the scratch file should be writable");
    let output = inspect(&path, true);
    discard(&path);
    assert_single_diagnostic(&output, "input_too_large", 1);
    let data = body(&output)["data"].clone();
    assert_eq!(data["input"]["bytes"], text.len() as u64);
    assert!(data["input"].get("bom").is_none());
    assert_eq!(
        body(&output)["error"]["message"],
        "the input exceeds the 16 MiB limit"
    );
}

#[test]
fn an_input_exactly_at_the_byte_limit_is_accepted() {
    let path = scratch("at-limit.xml");
    let limit = 16 * 1024 * 1024;
    let frame = "<a><!----></a>";
    let mut text = String::from("<a><!--");
    text.push_str(&"x".repeat(limit - frame.len()));
    text.push_str("--></a>");
    assert_eq!(text.len(), limit);
    fs::write(&path, &text).expect("the scratch file should be writable");
    let output = inspect(&path, true);
    discard(&path);
    assert_single_diagnostic(&output, "unrecognized_root", 4);
    assert_eq!(body(&output)["data"]["input"]["bytes"], limit as u64);
}

#[test]
fn a_single_dash_is_an_ordinary_file_name_not_standard_input() {
    let output = run(&["form", "inspect", "-", "--json"]);
    assert_single_diagnostic(&output, "input_unreadable", 1);
    assert_eq!(body(&output)["error"]["code"], "input_unreadable");
}

#[test]
fn depth_is_accepted_at_the_limit_and_refused_one_past_it() {
    let at_limit = "<a>".repeat(128) + &"</a>".repeat(128);
    let output = inspect_generated("depth-128.xml", &at_limit);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(body(&output)["data"]["structure"]["max_depth"], 128);

    let past = "<a>".repeat(129) + &"</a>".repeat(129);
    let output = inspect_generated("depth-129.xml", &past);
    assert_single_diagnostic(&output, "limit_exceeded", 1);
    assert_eq!(
        body(&output)["error"]["message"],
        "the document exceeds a fixed depth, node, text, or attribute limit"
    );
}

#[test]
fn a_hundred_thousand_levels_are_refused_without_a_stack_overflow() {
    let deep = "<a>".repeat(100_000) + &"</a>".repeat(100_000);
    let output = inspect_generated("depth-100000.xml", &deep);
    assert_single_diagnostic(&output, "limit_exceeded", 1);
}

#[test]
fn element_counts_are_accepted_at_the_limit_and_refused_one_past_it() {
    let at_limit = format!("<r>{}</r>", "<e/>".repeat(199_999));
    let output = inspect_generated("elements-200000.xml", &at_limit);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(body(&output)["data"]["structure"]["elements"], 200_000);

    let past = format!("<r>{}</r>", "<e/>".repeat(200_000));
    let output = inspect_generated("elements-200001.xml", &past);
    assert_single_diagnostic(&output, "limit_exceeded", 1);
}

#[test]
fn text_and_attribute_limits_apply_at_the_limit_and_one_past_it() {
    let text_limit = 1024 * 1024;
    let at_limit = format!("<a>{}</a>", "x".repeat(text_limit));
    let output = inspect_generated("text-1mib.xml", &at_limit);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(body(&output)["data"]["structure"]["text_bytes"], text_limit);

    let past = format!("<a>{}</a>", "x".repeat(text_limit + 1));
    let output = inspect_generated("text-1mib-plus.xml", &past);
    assert_single_diagnostic(&output, "limit_exceeded", 1);

    let attributes = (0..64)
        .map(|index| format!(" a{index}=\"1\""))
        .collect::<String>();
    let output = inspect_generated("attributes-64.xml", &format!("<a{attributes}/>"));
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(body(&output)["data"]["structure"]["attributes"], 64);

    let output = inspect_generated("attributes-65.xml", &format!("<a{attributes} b=\"1\"/>"));
    assert_single_diagnostic(&output, "limit_exceeded", 1);
}

#[test]
fn an_oversized_declaration_is_malformed() {
    let padding = " ".repeat(300);
    let text = format!("<?xml version=\"1.0\"{padding}encoding=\"UTF-8\"?><a/>");
    let output = inspect_generated("long-declaration.xml", &text);
    assert_single_diagnostic(&output, "malformed_xml", 1);
}

#[test]
fn output_is_byte_identical_across_runs() {
    for name in ["cegbejegyzesi-minimal-utf8.xml", "wrapper-two-children.xml"] {
        for json in [true, false] {
            let first = inspect(&fixture(name), json);
            let second = inspect(&fixture(name), json);
            assert_eq!(first.stdout, second.stdout, "fixture: {name}");
            assert_eq!(first.stderr, second.stderr, "fixture: {name}");
            assert_eq!(first.status.code(), second.status.code());
        }
    }
}

#[test]
fn the_form_group_keeps_the_existing_usage_error_behaviour() {
    let path = fixture("cegbejegyzesi-minimal-utf8.xml");
    let path = path.to_str().expect("test paths are UTF-8");
    for arguments in [
        vec!["form"],
        vec!["form", "validate", path],
        vec!["form", "inspect"],
        vec!["form", "inspect", path, path],
        vec!["form", "inspect", "--limit", "1", path],
        vec!["preflight"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert!(output.stdout.is_empty(), "arguments: {arguments:?}");
        assert_eq!(
            stderr(&output),
            "opencegeljaras: invalid command line; run opencegeljaras --help (exit 2)\n"
        );
    }
}
