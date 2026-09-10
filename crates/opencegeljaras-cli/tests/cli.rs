use std::process::{Command, Output};

use serde_json::Value;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(args)
        .output()
        .expect("CLI should run")
}

fn json_stdout(output: &Output) -> Value {
    assert!(output.stderr.is_empty(), "JSON stderr was not empty");
    serde_json::from_slice(&output.stdout).expect("stdout should be one JSON object")
}

#[test]
fn capabilities_reports_the_read_only_domain_surface() {
    let output = run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let body = json_stdout(&output);
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["ok"], true);
    assert_eq!(body["command"], "capabilities");
    assert_eq!(body["data"]["stage"], "scaffold");
    assert_eq!(
        body["data"]["operations"],
        serde_json::json!([
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
    assert_eq!(
        body["data"]["introspection_commands"],
        serde_json::json!(["capabilities", "sources", "skill"])
    );
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);
}

#[test]
fn capabilities_human_output_states_every_boundary() {
    let output = run(&["capabilities"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(text.contains("stage: scaffold"));
    assert!(text.contains(
        "implemented operations: form inspect, form validate, form fields, form diff, schemas list, schemas verify"
    ));
    assert!(text.contains("verification: not performed"));
    assert!(text.contains("submission: not performed"));
}

#[test]
fn sources_reports_compiled_metadata_without_claiming_fetch_or_submission() {
    let output = run(&["sources", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let body = json_stdout(&output);
    assert_eq!(body["ok"], true);
    assert_eq!(body["command"], "sources");
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);
    assert!(body["data"]["reviewed_at"].is_string());
    assert!(body["data"]["sources"].is_array());
    assert_eq!(
        body["data"]["count"].as_u64(),
        body["data"]["sources"]
            .as_array()
            .map(|items| items.len() as u64)
    );
    assert!(
        body["data"]["notice"]
            .as_str()
            .is_some_and(|notice| notice.contains("no source content was fetched"))
    );
}

#[test]
fn sources_human_output_labels_metadata_as_unfetched() {
    let output = run(&["sources"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(text.contains("Reviewed source metadata: 2026-09-10"));
    assert!(text.contains("Catalogue entries:"));
    assert!(text.contains("- overview [official_guidance; indexed_guidance]"));
    assert!(text.contains("Catalogue metadata only"));
    assert!(text.contains("no source content was fetched or checked"));
}

#[test]
fn skill_supports_raw_and_json_output() {
    let raw = run(&["skill"]);
    assert_eq!(raw.status.code(), Some(0));
    assert!(raw.stderr.is_empty());
    assert_eq!(
        String::from_utf8(raw.stdout).expect("skill should be UTF-8"),
        include_str!("../skills/opencegeljaras/SKILL.md")
    );

    let json = run(&["skill", "--json"]);
    assert_eq!(json.status.code(), Some(0));
    let body = json_stdout(&json);
    assert_eq!(body["command"], "skill");
    assert_eq!(
        body["data"]["document"],
        include_str!("../skills/opencegeljaras/SKILL.md")
    );
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);
}

#[test]
fn proposed_domain_commands_are_rejected() {
    for command in ["preflight", "submit", "status", "validate", "fetch"] {
        let output = run(&[command]);
        assert_eq!(output.status.code(), Some(2), "command: {command}");
        assert!(output.stdout.is_empty(), "command: {command}");
        assert!(!output.stderr.is_empty(), "command: {command}");
    }
}

/// The one JSON object a rejected command line writes on standard output.
const USAGE_JSON: &str = concat!(
    r#"{"schema_version":1,"ok":false,"command":"usage","#,
    r#""error":{"code":"usage.arguments","#,
    r#""message":"Unsupported invocation. Run opencegeljaras --help."},"#,
    r#""data":null,"verified":false,"submitted":false}"#,
    "\n"
);

/// The one stderr line a rejected command line writes without `--json`.
const USAGE_DIAGNOSTIC: &str =
    "opencegeljaras: invalid command line; run opencegeljaras --help (exit 2)\n";

#[test]
fn a_rejected_command_line_with_json_writes_one_usage_envelope() {
    for arguments in [
        vec!["preflight", "--json"],
        vec!["capabilities", "--json", "--bogus"],
        vec!["schemas", "list", "--urls", "--json"],
        vec!["form", "validate", "--json"],
        vec!["--json"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert!(output.stderr.is_empty(), "arguments: {arguments:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            USAGE_JSON,
            "arguments: {arguments:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).lines().count(),
            1,
            "arguments: {arguments:?}"
        );
        let body = json_stdout(&output);
        assert_eq!(body["ok"], false);
        assert_eq!(body["command"], "usage");
        assert_eq!(body["error"]["code"], "usage.arguments");
        assert_eq!(body["data"], Value::Null);
        assert_eq!(body["verified"], false);
        assert_eq!(body["submitted"], false);
    }
}

#[test]
fn an_unknown_command_without_json_keeps_the_fixed_diagnostic() {
    for arguments in [
        vec!["preflight"],
        vec!["preflight", "--", "--json"],
        vec!["capabilities", "--bogus"],
        vec!["schemas", "list", "--urls", "--", "--json"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert!(output.stdout.is_empty(), "arguments: {arguments:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            USAGE_DIAGNOSTIC,
            "arguments: {arguments:?}"
        );
    }
}

#[test]
fn a_rejected_argument_never_reaches_either_stream_in_either_mode() {
    let sentinel = "private-path-sentinel";
    for arguments in [
        vec!["preflight", sentinel, "--json"],
        vec!["preflight", sentinel],
        vec!["--json", sentinel],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains(sentinel),
            "arguments: {arguments:?}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains(sentinel),
            "arguments: {arguments:?}"
        );
    }
}

#[test]
fn help_and_version_are_plain_text_successes() {
    for arguments in [
        vec!["--help"],
        vec!["-h"],
        vec!["--version"],
        vec!["-V"],
        vec!["--help", "--json"],
        vec!["--version", "--json"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(0), "arguments: {arguments:?}");
        assert!(!output.stdout.is_empty(), "arguments: {arguments:?}");
        assert!(output.stderr.is_empty(), "arguments: {arguments:?}");
        assert!(
            !String::from_utf8_lossy(&output.stdout).starts_with('{'),
            "arguments: {arguments:?}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn output_failure_exits_three_without_panicking() {
    use std::fs::File;
    use std::process::Stdio;

    let full = File::options()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full should exist on Unix test hosts");
    let output = Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(["capabilities", "--json"])
        .stdout(Stdio::from(full))
        .output()
        .expect("CLI should run");
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to write output"));
}

#[cfg(target_os = "linux")]
#[test]
fn an_unwritable_usage_envelope_exits_three() {
    use std::fs::File;
    use std::process::Stdio;

    let full = File::options()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full should exist on Unix test hosts");
    let output = Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(["preflight", "--json"])
        .stdout(Stdio::from(full))
        .output()
        .expect("CLI should run");
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to write output"));
}
