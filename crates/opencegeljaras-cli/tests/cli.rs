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
fn capabilities_reports_an_empty_domain_surface() {
    let output = run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let body = json_stdout(&output);
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["ok"], true);
    assert_eq!(body["command"], "capabilities");
    assert_eq!(body["data"]["stage"], "scaffold");
    assert_eq!(body["data"]["operations"], serde_json::json!([]));
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
    assert!(text.contains("implemented operations: none"));
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

#[test]
fn usage_errors_are_fixed_diagnostics_even_when_json_is_requested() {
    let sentinel = "private-path-sentinel";
    let output = run(&["preflight", sentinel, "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(sentinel));
}

#[test]
fn help_and_version_are_plain_text_successes() {
    for argument in ["--help", "--version"] {
        let output = run(&[argument]);
        assert_eq!(output.status.code(), Some(0), "argument: {argument}");
        assert!(!output.stdout.is_empty(), "argument: {argument}");
        assert!(output.stderr.is_empty(), "argument: {argument}");
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
