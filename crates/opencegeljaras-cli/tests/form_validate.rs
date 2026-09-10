//! Contract tests for `form validate` over the built binary.
//!
//! Nothing here needs a real schema bundle or a real xmllint: the bundle is
//! the synthetic CC0 mini graph and the validator is the fake built out of
//! this test executable. One `#[ignore]` test uses an operator's own mirror,
//! named by an environment variable, and CI does not run it. Nothing private
//! is read and nothing is fetched.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

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

/// A stored fixture path at the repository root.
fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

/// The path of a stored fixture as a string.
fn text(path: &Path) -> String {
    path.to_str().expect("test paths are UTF-8").to_owned()
}

/// Run `form validate` with the given extra arguments.
fn validate(document: &str, bundle: &Path, extra: &[&str]) -> Output {
    let document = text(&fixture(document));
    let bundle = text(bundle);
    let manifest = text(&fixture("schemas/manifest.json"));
    let mut args = vec![
        "form",
        "validate",
        &document,
        "--bundle",
        &bundle,
        "--manifest",
        &manifest,
    ];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(&args)
        .output()
        .expect("CLI should run")
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
fn assert_single_diagnostic(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit), "{}", stderr(output));
    assert_eq!(
        stderr(output),
        format!("opencegeljaras: form validate: {code} (exit {exit})\n")
    );
}

/// The fields that stay null until the step that establishes them has run.
fn assert_null(data: &Value, keys: &[&str]) {
    for key in keys {
        assert_eq!(data[*key], Value::Null, "{key}");
    }
}

#[test]
fn an_unreadable_input_is_refused_before_anything_is_verified() {
    let output = validate(
        "forms/no-such-form.xml",
        &fixture("schemas/good"),
        &["--json"],
    );
    assert_single_diagnostic(&output, "input_unreadable", 1);
    let envelope = body(&output);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["command"], "form validate");
    assert_eq!(envelope["error"]["code"], "input_unreadable");
    let data = &envelope["data"];
    assert_null(
        data,
        &[
            "input",
            "root",
            "form",
            "structure",
            "bundle",
            "validator",
            "findings",
            "findings_truncated",
            "unparsed_lines",
            "values_redacted",
            "checks",
        ],
    );
    assert!(
        data["notice"]
            .as_str()
            .is_some_and(|notice| notice.contains("not a signature check"))
    );
}

#[test]
fn a_prohibited_construct_is_refused_and_prints_nothing() {
    let output = validate("forms/doctype.xml", &fixture("schemas/good"), &[]);
    assert_single_diagnostic(&output, "prohibited_construct", 1);
    assert!(output.stdout.is_empty());
}

#[test]
fn a_document_that_is_not_a_form_reports_conformance_as_unperformed() {
    let output = validate(
        "forms/unrecognized-root.xml",
        &fixture("schemas/good"),
        &["--json"],
    );
    assert_single_diagnostic(&output, "unrecognized_root", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["form"]["status"], "unrecognized_root");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(data["checks"]["well_formed"], "passed");
    assert_null(
        data,
        &["bundle", "validator", "findings", "values_redacted"],
    );
}

#[test]
fn a_document_that_is_not_a_form_prints_the_unperformed_lines() {
    let output = validate("forms/unrecognized-root.xml", &fixture("schemas/good"), &[]);
    let report = stdout(&output);
    assert!(report.contains("\nbundle: not performed\n"), "{report}");
    assert!(report.contains("\nvalidator: none\n"), "{report}");
    assert!(
        report.contains("\nxsd conformance: not performed\n"),
        "{report}"
    );
    assert!(report.ends_with("verification: not performed\nsubmission: not performed\n"));
}

#[test]
fn an_unreadable_bundle_is_refused() {
    let output = validate(
        "forms/nevfoglalasi-conforming.xml",
        &fixture("schemas/no-such-bundle"),
        &["--json"],
    );
    assert_single_diagnostic(&output, "bundle_unreadable", 1);
    let data = &body(&output)["data"];
    assert_eq!(data["form"]["status"], "recognized");
    assert_null(data, &["bundle", "validator", "findings"]);
}

#[test]
fn an_unverified_bundle_stops_before_any_validator_runs() {
    let output = validate(
        "forms/nevfoglalasi-conforming.xml",
        &fixture("schemas/tampered"),
        &["--json"],
    );
    assert_single_diagnostic(&output, "bundle_unverified", 5);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["bundle"]["form_closure_verified"], false);
    assert_eq!(data["bundle"]["bundle"]["mismatch"], 1);
    assert_eq!(data["bundle"]["bundle"]["missing"], 1);
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_null(data, &["validator", "findings", "unparsed_lines"]);
}

#[test]
fn a_missing_validator_is_reported_as_unavailable() {
    let output = validate(
        "forms/nevfoglalasi-conforming.xml",
        &fixture("schemas/good"),
        &["--json", "--xmllint", "/no/such/xmllint"],
    );
    assert_single_diagnostic(&output, "validator_unavailable", 5);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["bundle"]["verified"], true);
    assert_eq!(data["validator"]["name"], "xmllint");
    assert_eq!(data["validator"]["version"], Value::Null);
    assert_eq!(data["validator"]["exit_code"], Value::Null);
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
}

#[test]
fn identical_input_produces_identical_output() {
    let first = validate(
        "forms/nevfoglalasi-conforming.xml",
        &fixture("schemas/good"),
        &["--json", "--xmllint", "/no/such/xmllint"],
    );
    let second = validate(
        "forms/nevfoglalasi-conforming.xml",
        &fixture("schemas/good"),
        &["--json", "--xmllint", "/no/such/xmllint"],
    );
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[cfg(target_os = "linux")]
mod faked {
    use super::*;

    /// A violation line for the mini graph's shared pattern.
    const VIOLATION: &str = "document.xml:6: element Szam: Schemas validity error : Element '{urn:opencegeljaras:fixture:variant-b}Szam': [facet 'pattern'] The value 'nem-szam' is not accepted by the pattern '[0-9]{2}-[0-9]{2}-[0-9]{6}'.";

    /// Run `form validate` against the good mini bundle and one fake case.
    fn with_fake(name: &str, script: &str, extra: &[&str]) -> Output {
        let case = fake::case(name, script);
        let exe = text(&case.exe);
        let mut args = vec!["--xmllint", exe.as_str()];
        args.extend_from_slice(extra);
        validate(
            "forms/nevfoglalasi-conforming.xml",
            &fixture("schemas/good"),
            &args,
        )
    }

    #[test]
    fn a_conforming_document_reports_a_passed_check() {
        let output = with_fake("cli-conforming", "exit=0\n", &["--json"]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(output.stderr.is_empty());
        let envelope = body(&output);
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["verified"], false);
        assert_eq!(envelope["submitted"], false);
        let data = &envelope["data"];
        assert_eq!(data["checks"]["xsd_conformance"], "passed");
        assert_eq!(data["checks"]["signature"], "not_performed");
        assert_eq!(data["checks"]["authority_acceptance"], "not_performed");
        assert_eq!(data["validator"]["version"], 20914);
        assert_eq!(data["validator"]["exit_code"], 0);
        assert_eq!(data["findings"], serde_json::json!([]));
        assert_eq!(data["findings_truncated"], false);
        assert_eq!(data["unparsed_lines"], 0);
        assert_eq!(data["values_redacted"], true);
    }

    #[test]
    fn a_conforming_document_prints_the_bundle_and_validator_lines() {
        let output = with_fake("cli-conforming-human", "exit=0\n", &[]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let report = stdout(&output);
        assert!(report.contains("\nbundle: verified\n"), "{report}");
        assert!(report.contains("\nvalidator: xmllint 20914\n"), "{report}");
        assert!(report.contains("\nxsd conformance: passed\n"), "{report}");
    }

    #[test]
    fn a_nonconforming_document_reports_redacted_findings() {
        let script = format!("exit=3\nstderr={VIOLATION}\nstderr=document.xml fails to validate\n");
        let output = with_fake("cli-nonconforming", &script, &["--json"]);
        assert_single_diagnostic(&output, "nonconformant", 4);
        let data = &body(&output)["data"];
        assert_eq!(data["checks"]["xsd_conformance"], "failed");
        assert_eq!(data["unparsed_lines"], 0);
        assert_eq!(data["values_redacted"], true);
        let finding = &data["findings"][0];
        assert_eq!(finding["line"], 6);
        assert_eq!(finding["element"], "Szam");
        let message = finding["message"].as_str().expect("a message is reported");
        assert!(message.contains("[facet 'pattern']"), "{message}");
        assert!(message.contains("'{urn:opencegeljaras:fixture:variant-b}Szam'"));
        assert!(!message.contains("nem-szam"), "{message}");
        assert!(message.contains("'…'"), "{message}");
    }

    #[test]
    fn a_nonconforming_document_prints_one_line_per_finding() {
        let script = format!("exit=3\nstderr={VIOLATION}\n");
        let output = with_fake("cli-nonconforming-human", &script, &[]);
        assert_single_diagnostic(&output, "nonconformant", 4);
        let report = stdout(&output);
        assert!(report.contains("\nxsd conformance: failed\n"), "{report}");
        assert!(
            report.contains("\n  line 6 element \"Szam\": \""),
            "{report}"
        );
        assert!(!report.contains("nem-szam"), "{report}");
    }

    #[test]
    fn requested_values_are_kept_in_the_findings() {
        let script = format!("exit=3\nstderr={VIOLATION}\n");
        let output = with_fake("cli-show-values", &script, &["--json", "--show-values"]);
        assert_single_diagnostic(&output, "nonconformant", 4);
        let data = &body(&output)["data"];
        assert_eq!(data["values_redacted"], false);
        assert!(
            data["findings"][0]["message"]
                .as_str()
                .is_some_and(|message| message.contains("nem-szam"))
        );
    }

    #[test]
    fn unexpected_stderr_lines_are_counted_and_never_echoed() {
        let script = format!(
            "exit=3\nstderr=xmllint: a note in an unexpected shape about example.invalid\nstderr={VIOLATION}\n"
        );
        let output = with_fake("cli-unparsed", &script, &["--json"]);
        assert_single_diagnostic(&output, "nonconformant", 4);
        let data = &body(&output)["data"];
        assert_eq!(data["unparsed_lines"], 1);
        assert!(!stdout(&output).contains("example.invalid"));
    }

    #[test]
    fn an_unsupported_version_reports_the_version_it_read() {
        let output = with_fake(
            "cli-old-version",
            "version=xmllint: using libxml version 20913\n",
            &["--json"],
        );
        assert_single_diagnostic(&output, "validator_unavailable", 5);
        let data = &body(&output)["data"];
        assert_eq!(data["validator"]["version"], 20913);
        assert_eq!(data["validator"]["exit_code"], Value::Null);
        assert_null(data, &["findings", "unparsed_lines"]);
    }

    #[test]
    fn a_warning_about_the_schema_graph_is_a_failed_validator() {
        let output = with_fake(
            "cli-warning",
            "exit=0\nstderr=warning: failed to load external entity \"http://x\"\n",
            &["--json"],
        );
        assert_single_diagnostic(&output, "validator_failed", 5);
        let data = &body(&output)["data"];
        assert_eq!(data["validator"]["exit_code"], 0);
        assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
        assert!(!stdout(&output).contains("http://x"));
    }

    #[test]
    fn any_other_exit_code_is_a_failed_validator() {
        let output = with_fake("cli-failed", "exit=5\n", &["--json"]);
        assert_single_diagnostic(&output, "validator_failed", 5);
        let data = &body(&output)["data"];
        assert_eq!(data["validator"]["version"], 20914);
        assert_eq!(data["validator"]["exit_code"], 5);
        assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
        assert_null(data, &["findings", "values_redacted"]);
    }
}

/// Validate one real fixture against an operator's own mirror.
fn validate_real(document: &str, bundle: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args([
            "form",
            "validate",
            &text(&fixture(document)),
            "--bundle",
            bundle,
            "--json",
        ])
        .output()
        .expect("CLI should run")
}

#[test]
#[ignore = "needs a real mirror named by OPENCEGELJARAS_SCHEMA_BUNDLE and a real xmllint"]
fn the_real_fixtures_agree_with_the_recorded_evidence() {
    let bundle = std::env::var("OPENCEGELJARAS_SCHEMA_BUNDLE")
        .expect("set OPENCEGELJARAS_SCHEMA_BUNDLE to a local mirror of the pinned schema bundle");

    let conforming = validate_real("forms/nevfoglalasi-conforming.xml", &bundle);
    assert_eq!(conforming.status.code(), Some(0), "{}", stderr(&conforming));
    assert!(conforming.stderr.is_empty());
    let data = &body(&conforming)["data"];
    assert_eq!(data["checks"]["xsd_conformance"], "passed");
    assert_eq!(data["bundle"]["bundle"]["verified"], true);
    assert_eq!(data["findings"], serde_json::json!([]));
    assert_eq!(data["validator"]["name"], "xmllint");

    let nonconforming = validate_real("forms/nevfoglalasi-bad-registry-number.xml", &bundle);
    assert_single_diagnostic(&nonconforming, "nonconformant", 4);
    let data = &body(&nonconforming)["data"];
    assert_eq!(data["checks"]["xsd_conformance"], "failed");
    let message = data["findings"][0]["message"]
        .as_str()
        .expect("a message is reported");
    assert!(message.contains("[facet 'pattern']"), "{message}");
    assert!(!message.contains("99-99-999999"), "{message}");
    assert!(!stdout(&nonconforming).contains(&bundle));
}

/// Nothing here reads or writes the fixture tree; this keeps `fs` honest.
#[test]
fn every_stored_fixture_this_suite_names_exists() {
    for relative in [
        "forms/nevfoglalasi-conforming.xml",
        "forms/nevfoglalasi-bad-registry-number.xml",
        "schemas/manifest.json",
        "schemas/good",
        "schemas/tampered",
    ] {
        assert!(fs::metadata(fixture(relative)).is_ok(), "{relative}");
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
        a_conforming_document_reports_a_passed_check,
        a_conforming_document_prints_the_bundle_and_validator_lines,
        a_nonconforming_document_reports_redacted_findings,
        a_nonconforming_document_prints_one_line_per_finding,
        requested_values_are_kept_in_the_findings,
        unexpected_stderr_lines_are_counted_and_never_echoed,
        an_unsupported_version_reports_the_version_it_read,
        any_other_exit_code_is_a_failed_validator,
        a_warning_about_the_schema_graph_is_a_failed_validator,
    );
}
