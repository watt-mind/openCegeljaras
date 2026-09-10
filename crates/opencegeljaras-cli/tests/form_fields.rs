//! Contract tests for `form fields` and `form diff` over the built binary.
//!
//! Every fixture used here is synthetic. Generated documents are written to a
//! per-process temporary directory and removed again; nothing private is read.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use serde_json::Value;

/// The value that must never reach standard error, and standard output only
/// when the operator asked for values.
const SENTINEL: &str = "SENTINEL-VALUE-7731";
/// The fixed notice of a completed field listing.
const FIELDS_NOTICE: &str = "field listing of one recognised form; values are shown only on request; not validation, verification, or acceptance";
/// The fixed notice of a completed comparison.
const DIFF_NOTICE: &str =
    "field-level comparison of two recognised forms; not validation, verification, or acceptance";
/// The four lines every completed report ends with.
const BOUNDARY: &str = "well-formed: passed\nxsd conformance: not performed\nverification: not performed\nsubmission: not performed\n";
/// The name-reservation namespace the fixtures use.
const VARIANT_NAMESPACE: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/nevfoglalasiKerelem-20210401#";
/// The address namespace two fixtures share.
const ADDRESS_NAMESPACE: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/common/complex/tagoltCim-20210101#";
/// The largest number of fields one listing reports.
const FIELD_LIMIT: usize = 10_000;
/// The largest number of scalars one emitted value keeps.
const VALUE_LIMIT: usize = 4096;
/// The largest number of scalars one emitted path keeps.
const PATH_LIMIT: usize = 1024;

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
        "opencegeljaras-fields-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory.join(name)
}

/// Remove a scratch directory and everything written into it.
fn discard(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(args)
        .output()
        .expect("CLI should run")
}

/// Run `form fields` over one path.
fn fields(path: &Path, options: &[&str]) -> Output {
    let path = path.to_str().expect("test paths are UTF-8");
    let mut args = vec!["form", "fields", path];
    args.extend_from_slice(options);
    run(&args)
}

/// Run `form diff` over two paths.
fn diff(old: &Path, new: &Path, options: &[&str]) -> Output {
    let old = old.to_str().expect("test paths are UTF-8");
    let new = new.to_str().expect("test paths are UTF-8");
    let mut args = vec!["form", "diff", old, new];
    args.extend_from_slice(options);
    run(&args)
}

fn envelope(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be one JSON object")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("output should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("diagnostics should be UTF-8")
}

/// Every diagnostic is exactly one fixed line naming the code and exit code.
fn assert_single_diagnostic(output: &Output, command: &str, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit));
    assert_eq!(
        stderr(output),
        format!("opencegeljaras: {command}: {code} (exit {exit})\n")
    );
}

/// Write one generated document into a fresh scratch file.
fn generated(name: &str, body: &str) -> PathBuf {
    let path = scratch(name);
    fs::write(&path, wrapped(body)).expect("the scratch file should be writable");
    path
}

/// Wrap one body in the supported wrapper and a name-reservation variant root.
fn wrapped(body: &str) -> String {
    format!(
        "<Nyomtatvány xmlns=\"http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#\" xmlns:nk=\"{VARIANT_NAMESPACE}\"><nk:NévfoglalásiKérelem>{body}</nk:NévfoglalásiKérelem></Nyomtatvány>"
    )
}

/// The listed paths of one JSON field listing.
fn listed_paths(data: &Value) -> Vec<String> {
    data["fields"]
        .as_array()
        .expect("a completed listing carries an array")
        .iter()
        .map(|field| {
            field["path"]
                .as_str()
                .expect("a path is a string")
                .to_owned()
        })
        .collect()
}

/// The `path` values of one diff list.
fn entry_paths(data: &Value, key: &str) -> Vec<String> {
    data[key]
        .as_array()
        .expect("a completed diff carries arrays")
        .iter()
        .map(|entry| {
            entry["path"]
                .as_str()
                .expect("a path is a string")
                .to_owned()
        })
        .collect()
}

#[test]
fn a_recognised_form_lists_every_field_with_its_kind_namespace_and_length() {
    let output = fields(&fixture("fields-repeated-siblings.xml"), &["--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let body = envelope(&output);
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["ok"], true);
    assert_eq!(body["command"], "form fields");
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);

    let data = &body["data"];
    assert_eq!(data["field_count"], 18);
    assert_eq!(data["fields_truncated"], false);
    assert_eq!(data["values_redacted"], true);
    assert_eq!(data["notice"], FIELDS_NOTICE);
    assert_eq!(
        data["form"]["variant"]["id"],
        "nevfoglalasi-kerelem-20210401"
    );
    assert_eq!(data["checks"]["well_formed"], "passed");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(data["checks"]["signature"], "not_performed");
    assert_eq!(data["checks"]["authority_acceptance"], "not_performed");

    assert_eq!(
        listed_paths(data),
        [
            "@Cégforma",
            "Adatlap[1]/@Sorszám",
            "Adatlap[1]/TagoltCím/Hely",
            "Adatlap[1]/TagoltCím/Ország",
            "Adatlap[1]/Név[1]",
            "Adatlap[1]/Név[1]/@Típus[1]",
            "Adatlap[1]/Név[1]/@Típus[2]",
            "Adatlap[1]/Név[2]",
            "Adatlap[1]/Üres",
            "Adatlap[1]/CsakSzóköz",
            "Adatlap[1]/Megjegyzés",
            "Adatlap[1]/Kettévágott",
            "Adatlap[2]/E-mail",
            "Vegyes",
            "Vegyes/Belső",
            "Névfoglalások/Névfoglalás[1]/Név",
            "Névfoglalások/Névfoglalás[2]/Név",
            "Névfoglalások/Névfoglalás[3]/Név",
        ]
    );

    let first = &data["fields"][0];
    assert_eq!(first["kind"], "attribute");
    assert_eq!(first["namespace"], VARIANT_NAMESPACE);
    assert_eq!(first["value_length"], 3);
    assert_eq!(first["value_truncated"], false);
    assert_eq!(first["path_truncated"], false);
    assert!(first.get("value").is_none(), "values are hidden by default");

    let address = &data["fields"][2];
    assert_eq!(address["kind"], "element");
    assert_eq!(address["namespace"], ADDRESS_NAMESPACE);
    assert_eq!(address["value_length"], SENTINEL.chars().count());
}

#[test]
fn every_listed_path_is_unique_within_one_document() {
    let output = fields(&fixture("fields-repeated-siblings.xml"), &["--json"]);
    let paths = listed_paths(&envelope(&output)["data"]);
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), paths.len(), "a path identifies one field");
}

#[test]
fn repeated_siblings_are_indexed_and_singletons_are_not() {
    let path = generated(
        "indices.xml",
        "<nk:A><nk:B>1</nk:B></nk:A><nk:A><nk:B>2</nk:B></nk:A><nk:C><nk:D>3</nk:D></nk:C>",
    );
    let output = fields(&path, &["--json"]);
    discard(&path);
    assert_eq!(
        listed_paths(&envelope(&output)["data"]),
        ["A[1]/B", "A[2]/B", "C/D"]
    );
}

#[test]
fn root_attributes_are_listed_and_xsi_and_declarations_are_not() {
    let output = fields(&fixture("fields-repeated-siblings.xml"), &["--json"]);
    let data = envelope(&output)["data"].clone();
    let paths = listed_paths(&data);
    assert_eq!(paths[0], "@Cégforma");
    assert!(
        !paths.iter().any(|path| path.contains("schemaLocation")),
        "an XSI attribute is not a field"
    );
    assert!(
        !paths.iter().any(|path| path.contains("xmlns")),
        "a namespace declaration is not a field"
    );
    assert_eq!(data["input"]["schema_location_declared"], true);
    assert_eq!(data["structure"]["attributes"], 5);
}

#[test]
fn empty_whitespace_cdata_and_comment_split_values_are_reported_as_written() {
    let output = fields(
        &fixture("fields-repeated-siblings.xml"),
        &["--json", "--show-values"],
    );
    assert_eq!(output.status.code(), Some(0));
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["values_redacted"], false);
    let by_path = |wanted: &str| -> Value {
        data["fields"]
            .as_array()
            .expect("an array")
            .iter()
            .find(|field| field["path"] == wanted)
            .expect("the fixture lists this path")
            .clone()
    };
    assert_eq!(by_path("Adatlap[1]/Üres")["value"], "");
    assert_eq!(by_path("Adatlap[1]/Üres")["value_length"], 0);
    assert_eq!(by_path("Adatlap[1]/CsakSzóköz")["value"], "   ");
    assert_eq!(
        by_path("Adatlap[1]/Megjegyzés")["value"],
        "<nem elem> & jel"
    );
    assert_eq!(by_path("Adatlap[1]/Kettévágott")["value"], "előutó");
    assert_eq!(by_path("Adatlap[1]/TagoltCím/Hely")["value"], SENTINEL);
}

#[test]
fn a_mixed_element_carries_no_value_and_its_children_are_still_listed() {
    let output = fields(
        &fixture("fields-repeated-siblings.xml"),
        &["--json", "--show-values"],
    );
    let data = envelope(&output)["data"].clone();
    let mixed = data["fields"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|field| field["path"] == "Vegyes")
        .expect("the fixture carries a mixed element")
        .clone();
    assert_eq!(mixed["kind"], "mixed");
    assert!(mixed.get("value").is_none());
    assert!(mixed.get("value_length").is_none());
    assert!(mixed.get("value_truncated").is_none());
    assert!(
        listed_paths(&data)
            .iter()
            .any(|path| path == "Vegyes/Belső")
    );
}

#[test]
fn a_value_longer_than_the_limit_is_cut_and_flagged() {
    let long = "é".repeat(VALUE_LIMIT + 7);
    let path = generated("long-value.xml", &format!("<nk:A>{long}</nk:A>"));
    let output = fields(&path, &["--json", "--show-values"]);
    discard(&path);
    let field = envelope(&output)["data"]["fields"][0].clone();
    assert_eq!(field["value_length"], VALUE_LIMIT + 7);
    assert_eq!(field["value_truncated"], true);
    let value = field["value"].as_str().expect("a value is a string");
    assert_eq!(value.chars().count(), VALUE_LIMIT + 1);
    assert!(value.ends_with('…'));
}

#[test]
fn a_path_longer_than_the_limit_is_cut_and_flagged() {
    let name = format!("nk:{}", "N".repeat(120));
    let mut document = String::new();
    for _ in 0..40 {
        document.push_str(&format!("<{name}>"));
    }
    document.push_str("érték");
    for _ in 0..40 {
        document.push_str(&format!("</{name}>"));
    }
    let path = generated("long-path.xml", &document);
    let output = fields(&path, &["--json"]);
    discard(&path);
    let field = envelope(&output)["data"]["fields"][0].clone();
    assert_eq!(field["path_truncated"], true);
    let listed = field["path"].as_str().expect("a path is a string");
    assert_eq!(listed.chars().count(), PATH_LIMIT + 1);
    assert!(listed.ends_with('…'));
}

#[test]
fn more_fields_than_the_cap_stop_the_walk_and_are_reported_as_truncated() {
    let body_text = "<nk:A>1</nk:A>".repeat(FIELD_LIMIT + 25);
    let path = generated("many-fields.xml", &body_text);
    let output = fields(&path, &["--json"]);
    let human = fields(&path, &[]);
    discard(&path);
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["field_count"], FIELD_LIMIT);
    assert_eq!(data["fields_truncated"], true);
    assert_eq!(
        data["fields"].as_array().expect("an array").len(),
        FIELD_LIMIT
    );
    assert!(stdout(&human).contains(&format!("fields: {FIELD_LIMIT} (truncated)\n")));
}

#[test]
fn a_deep_chain_over_many_leaves_stays_bounded_in_time_and_output() {
    let name = format!("nk:{}", "N".repeat(120));
    let mut document = String::new();
    for _ in 0..110 {
        document.push_str(&format!("<{name}>"));
    }
    for _ in 0..FIELD_LIMIT {
        document.push_str("<nk:L>v</nk:L>");
    }
    for _ in 0..110 {
        document.push_str(&format!("</{name}>"));
    }
    let path = generated("deep-chain.xml", &document);
    let size = fs::metadata(&path).expect("the document exists").len();

    let started = Instant::now();
    let output = fields(&path, &["--json"]);
    let elapsed = started.elapsed();
    discard(&path);

    assert_eq!(output.status.code(), Some(0));
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["field_count"], FIELD_LIMIT);
    assert_eq!(data["fields_truncated"], false);
    assert_eq!(data["fields"][0]["path_truncated"], true);
    assert!(
        output.stdout.len() < 32 * 1024 * 1024,
        "cut paths keep the output bounded: {} bytes from {size} bytes of input",
        output.stdout.len()
    );
    assert!(elapsed.as_secs() < 60, "the listing took {elapsed:?}");
}

#[test]
fn a_hundred_thousand_siblings_are_listed_within_the_time_bound() {
    let mut document = String::with_capacity(3 * 1024 * 1024);
    for index in 0..100_000u32 {
        document.push_str(&format!("<nk:Tétel>{index}</nk:Tétel>"));
    }
    let path = generated("wide.xml", &document);

    let started = Instant::now();
    let output = fields(&path, &["--json"]);
    let listing = started.elapsed();
    let compared = Instant::now();
    let comparison = diff(&path, &path, &["--json"]);
    let comparing = compared.elapsed();
    discard(&path);

    assert_eq!(output.status.code(), Some(0));
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["field_count"], FIELD_LIMIT);
    assert_eq!(data["fields_truncated"], true);
    assert!(listing.as_secs() < 60, "the listing took {listing:?}");

    assert_eq!(comparison.status.code(), Some(0));
    let data = envelope(&comparison)["data"].clone();
    assert_eq!(
        data["identical"], false,
        "a truncated side is never identical"
    );
    assert_eq!(data["old"]["fields_truncated"], true);
    assert_eq!(data["unchanged"], FIELD_LIMIT);
    assert!(
        comparing.as_secs() < 60,
        "the comparison took {comparing:?}"
    );
}

#[test]
fn both_commands_are_byte_identical_across_runs() {
    let old = fixture("fields-repeated-siblings.xml");
    let new = fixture("fields-repeated-siblings-edited.xml");
    for options in [
        vec!["--json"],
        vec!["--json", "--show-values"],
        vec![],
        vec!["--show-values"],
    ] {
        let first = fields(&old, &options);
        let second = fields(&old, &options);
        assert_eq!(first.stdout, second.stdout, "form fields {options:?}");
        assert_eq!(first.stderr, second.stderr, "form fields {options:?}");

        let first = diff(&old, &new, &options);
        let second = diff(&old, &new, &options);
        assert_eq!(first.stdout, second.stdout, "form diff {options:?}");
        assert_eq!(first.stderr, second.stderr, "form diff {options:?}");
    }
}

#[test]
fn the_human_listing_sits_between_the_namespaces_and_the_boundary_lines() {
    let path = fixture("fields-repeated-siblings.xml");
    let hidden = fields(&path, &[]);
    assert_eq!(hidden.status.code(), Some(0));
    assert!(hidden.stderr.is_empty());
    let report = stdout(&hidden);
    let (before, after) = report
        .split_once("fields: 18\n")
        .expect("the count line follows the namespace list");
    assert!(before.ends_with("#\"\n"), "the namespace list comes first");
    assert!(before.contains("namespaces:\n"));
    assert!(after.ends_with(BOUNDARY));
    assert!(after.contains("  \"@C\\u00e9gforma\" (3 chars)\n"));
    assert!(after.contains("  \"Vegyes\" (mixed content)\n"));
    assert!(!after.contains(" = "), "values are hidden by default");

    let shown = stdout(&fields(&path, &["--show-values"]));
    assert!(shown.contains("  \"@C\\u00e9gforma\" = \"Kft\"\n"));
    assert!(shown.contains("  \"Vegyes\" (mixed content)\n"));
    assert!(shown.ends_with(BOUNDARY));
}

#[test]
fn an_unrecognised_or_refused_document_is_reported_as_form_inspect_would() {
    let output = fields(&fixture("unrecognized-root.xml"), &["--json"]);
    assert_single_diagnostic(&output, "form fields", "unrecognized_root", 4);
    let body = envelope(&output);
    assert_eq!(body["command"], "form fields");
    assert_eq!(
        body["error"]["message"],
        "the root element is not a known company form"
    );
    let data = &body["data"];
    assert_eq!(data["form"]["status"], "unrecognized_root");
    assert_eq!(data["fields"], Value::Null);
    assert_eq!(data["field_count"], Value::Null);
    assert_eq!(data["fields_truncated"], Value::Null);
    assert_eq!(data["values_redacted"], true);
    assert_eq!(data["notice"], FIELDS_NOTICE);
    let human = fields(&fixture("unrecognized-root.xml"), &[]);
    assert_eq!(stdout(&human), stdout(&inspect("unrecognized-root.xml")));

    let refused = fields(&fixture("malformed.xml"), &["--json"]);
    assert_single_diagnostic(&refused, "form fields", "malformed_xml", 1);
    let data = envelope(&refused)["data"].clone();
    assert_eq!(data["checks"], Value::Null);
    assert_eq!(data["fields"], Value::Null);
    assert_eq!(data["values_redacted"], true);
    let quiet = fields(&fixture("malformed.xml"), &[]);
    assert!(quiet.stdout.is_empty(), "exit 1 prints nothing as text");
}

/// Run `form inspect` over one fixture, for the unchanged-behaviour checks.
fn inspect(name: &str) -> Output {
    let path = fixture(name);
    let path = path.to_str().expect("test paths are UTF-8");
    run(&["form", "inspect", path])
}

#[test]
fn a_comparison_reports_added_removed_changed_and_unchanged_entries() {
    let output = diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("fields-repeated-siblings-edited.xml"),
        &["--json"],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let body = envelope(&output);
    assert_eq!(body["ok"], true);
    assert_eq!(body["command"], "form diff");

    let data = &body["data"];
    assert_eq!(data["side"], Value::Null);
    assert_eq!(data["same_variant"], true);
    assert_eq!(data["identical"], false);
    assert_eq!(data["unchanged"], 14);
    assert_eq!(data["values_redacted"], true);
    assert_eq!(data["notice"], DIFF_NOTICE);
    assert_eq!(data["old"]["field_count"], 18);
    assert_eq!(data["old"]["fields_truncated"], false);
    assert_eq!(data["new"]["field_count"], 18);
    assert_eq!(
        data["old"]["form"]["variant"]["id"],
        "nevfoglalasi-kerelem-20210401"
    );
    assert_eq!(data["checks"]["well_formed"], "passed");

    assert_eq!(entry_paths(data, "added"), ["Adatlap[2]/@Megye"]);
    assert_eq!(
        entry_paths(data, "removed"),
        ["Névfoglalások/Névfoglalás[3]/Név"]
    );
    assert_eq!(
        entry_paths(data, "changed"),
        [
            "Adatlap[1]/TagoltCím/Hely",
            "Névfoglalások/Névfoglalás[1]/Név",
            "Névfoglalások/Névfoglalás[2]/Név",
        ]
    );

    let added = &data["added"][0];
    assert_eq!(added["kind"], "attribute");
    assert_eq!(added["namespace"], VARIANT_NAMESPACE);
    assert_eq!(added["value_length"], 5);
    assert_eq!(added["value_truncated"], false);
    assert!(added.get("value").is_none());

    let changed = &data["changed"][0];
    assert_eq!(changed["old_kind"], "element");
    assert_eq!(changed["new_kind"], "element");
    assert_eq!(changed["old_namespace"], ADDRESS_NAMESPACE);
    assert_eq!(changed["new_namespace"], ADDRESS_NAMESPACE);
    assert_eq!(changed["old_value_length"], 19);
    assert_eq!(changed["new_value_length"], 16);
    assert_eq!(changed["old_value_truncated"], false);
    assert_eq!(changed["new_value_truncated"], false);
    assert!(changed.get("old_value").is_none());
}

#[test]
fn requested_values_reach_every_kind_of_entry() {
    let output = diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("fields-repeated-siblings-edited.xml"),
        &["--json", "--show-values"],
    );
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["values_redacted"], false);
    assert_eq!(data["added"][0]["value"], "Példa");
    assert_eq!(data["removed"][0]["value"], "Harmadik Példa Kft");
    assert_eq!(data["changed"][0]["old_value"], SENTINEL);
    assert_eq!(data["changed"][0]["new_value"], "PLACEHOLDER-0002");

    let human = stdout(&diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("fields-repeated-siblings-edited.xml"),
        &["--show-values"],
    ));
    assert!(human.contains("  \"Adatlap[2]/@Megye\" = \"P\\u00e9lda\"\n"));
    assert!(human.contains(&format!(
        "  \"Adatlap[1]/TagoltC\\u00edm/Hely\" \"{SENTINEL}\" -> \"PLACEHOLDER-0002\"\n"
    )));
}

#[test]
fn a_document_compared_with_itself_is_identical() {
    let path = fixture("fields-repeated-siblings.xml");
    let output = diff(&path, &path, &["--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["identical"], true);
    assert_eq!(data["same_variant"], true);
    assert_eq!(data["unchanged"], 18);
    assert_eq!(data["added"], serde_json::json!([]));
    assert_eq!(data["removed"], serde_json::json!([]));
    assert_eq!(data["changed"], serde_json::json!([]));
}

#[test]
fn two_variants_are_compared_by_path_and_report_a_different_variant() {
    let output = diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("fields-other-variant.xml"),
        &["--json"],
    );
    assert_eq!(output.status.code(), Some(0));
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["same_variant"], false);
    assert_eq!(data["identical"], false);
    assert_eq!(
        data["new"]["form"]["variant"]["id"],
        "cegbejegyzesi-kerelem-20260101"
    );
    assert_eq!(data["unchanged"], 1);
    assert_eq!(entry_paths(&data, "added"), ["Adatlap[2]/Azonosító"]);
    assert!(
        entry_paths(&data, "changed").contains(&String::from("@Cégforma")),
        "the same path in another namespace is a change"
    );
}

#[test]
fn the_human_comparison_reports_counts_entries_and_the_boundary_lines() {
    let report = stdout(&diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("fields-repeated-siblings-edited.xml"),
        &[],
    ));
    assert!(report.starts_with(
        "old: nevfoglalasi-kerelem-20210401, 18 fields\nnew: nevfoglalasi-kerelem-20210401, 18 fields\nsame variant: true\nidentical: false\nadded: 1\n"
    ));
    assert!(report.contains("  \"Adatlap[2]/@Megye\"\n"));
    assert!(report.contains("removed: 1\n"));
    assert!(report.contains("changed: 3\n"));
    assert!(report.contains("unchanged: 14\n"));
    assert!(report.ends_with(BOUNDARY));
    assert!(!report.contains(" = "), "values are hidden by default");
}

#[test]
fn an_unrecognised_old_document_stops_the_comparison_before_new_is_opened() {
    let missing = scratch("absent.xml");
    let output = diff(&fixture("unrecognized-root.xml"), &missing, &["--json"]);
    discard(&missing);
    assert_single_diagnostic(&output, "form diff", "old_not_recognized", 4);
    let body = envelope(&output);
    assert_eq!(
        body["error"]["message"],
        "the OLD document is not a recognised form"
    );
    let data = &body["data"];
    assert_eq!(data["side"], "old");
    assert_eq!(data["new"], Value::Null);
    assert_eq!(data["old"]["form"]["status"], "unrecognized_root");
    assert!(data["old"].get("field_count").is_none());
    for key in [
        "same_variant",
        "identical",
        "added",
        "removed",
        "changed",
        "unchanged",
    ] {
        assert_eq!(data[key], Value::Null, "key: {key}");
    }
    assert_eq!(data["checks"]["well_formed"], "passed");
    assert_eq!(data["values_redacted"], true);
    assert_eq!(data["notice"], DIFF_NOTICE);

    let human = stdout(&diff(&fixture("unrecognized-root.xml"), &missing, &[]));
    assert_eq!(
        human,
        format!("old: unrecognized_root\nnew: none\n{BOUNDARY}")
    );
}

#[test]
fn an_unrecognised_new_document_keeps_the_old_summary() {
    let output = diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("wrapper-unknown-child.xml"),
        &["--json"],
    );
    assert_single_diagnostic(&output, "form diff", "new_not_recognized", 4);
    let body = envelope(&output);
    assert_eq!(
        body["error"]["message"],
        "the NEW document is not a recognised form"
    );
    let data = &body["data"];
    assert_eq!(data["side"], "new");
    assert_eq!(data["old"]["field_count"], 18);
    assert_eq!(data["old"]["fields_truncated"], false);
    assert_eq!(
        data["old"]["form"]["variant"]["id"],
        "nevfoglalasi-kerelem-20210401"
    );
    assert_eq!(data["new"]["form"]["status"], "unrecognized_variant");
    assert_eq!(data["identical"], Value::Null);

    let human = stdout(&diff(
        &fixture("fields-repeated-siblings.xml"),
        &fixture("wrapper-unknown-child.xml"),
        &[],
    ));
    assert_eq!(
        human,
        format!(
            "old: nevfoglalasi-kerelem-20210401, 18 fields\nnew: unrecognized_variant\n{BOUNDARY}"
        )
    );
}

#[test]
fn a_refused_side_is_reported_at_exit_one_with_the_other_side_null() {
    let good = fixture("fields-repeated-siblings.xml");
    let output = diff(&fixture("malformed.xml"), &good, &["--json"]);
    assert_single_diagnostic(&output, "form diff", "malformed_xml", 1);
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["side"], "old");
    assert_eq!(data["new"], Value::Null);
    assert_eq!(data["checks"], Value::Null);
    assert_eq!(data["old"]["form"], Value::Null);
    assert!(data["old"]["input"]["bytes"].as_u64().is_some());
    assert_eq!(data["values_redacted"], true);
    assert!(
        diff(&fixture("malformed.xml"), &good, &[])
            .stdout
            .is_empty(),
        "exit 1 prints nothing as text"
    );

    let missing = scratch("absent.xml");
    let output = diff(&good, &missing, &["--json"]);
    discard(&missing);
    assert_single_diagnostic(&output, "form diff", "input_unreadable", 1);
    let data = envelope(&output)["data"].clone();
    assert_eq!(data["side"], "new");
    assert_eq!(data["old"], Value::Null);
    assert_eq!(data["new"]["input"], Value::Null);
}

#[test]
fn the_new_document_is_never_opened_when_the_old_one_fails() {
    let unreadable = scratch("never-opened.xml");
    for old in ["malformed.xml", "unrecognized-root.xml"] {
        let output = diff(&fixture(old), &unreadable, &["--json"]);
        assert!(
            output.status.code() != Some(0),
            "an old failure is reported, not a missing new file"
        );
        let code = envelope(&output)["error"]["code"]
            .as_str()
            .expect("a code is a string")
            .to_owned();
        assert_ne!(code, "input_unreadable", "NEW was opened after OLD failed");
    }
    discard(&unreadable);
}

#[test]
fn the_sentinel_value_reaches_standard_output_only_when_values_are_requested() {
    let old = fixture("fields-repeated-siblings.xml");
    let new = fixture("fields-repeated-siblings-edited.xml");
    let quiet = [
        fields(&old, &["--json"]),
        fields(&old, &[]),
        diff(&old, &new, &["--json"]),
        diff(&old, &new, &[]),
        diff(&fixture("malformed.xml"), &old, &["--json"]),
        fields(&fixture("unrecognized-root.xml"), &["--json"]),
    ];
    for output in &quiet {
        assert!(!stdout(output).contains(SENTINEL), "a value was printed");
        assert!(!stderr(output).contains(SENTINEL), "a value was diagnosed");
    }

    let loud = [
        fields(&old, &["--json", "--show-values"]),
        fields(&old, &["--show-values"]),
        diff(&old, &new, &["--json", "--show-values"]),
        diff(&old, &new, &["--show-values"]),
    ];
    for output in &loud {
        assert!(stdout(output).contains(SENTINEL), "a value was withheld");
        assert!(stderr(output).is_empty(), "nothing is written to stderr");
    }
}

#[test]
fn capabilities_lists_both_operations_and_the_existing_ones_are_unchanged() {
    let output = run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let operations = envelope(&output)["data"]["operations"].clone();
    assert_eq!(
        operations,
        serde_json::json!([
            "form inspect",
            "form validate",
            "form fields",
            "form diff",
            "schemas list",
            "schemas verify",
        ])
    );

    let text = stdout(&run(&["capabilities"]));
    assert!(text.contains("form fields, form diff"));

    let inspected = inspect("fields-repeated-siblings.xml");
    assert_eq!(inspected.status.code(), Some(0));
    assert!(
        !stdout(&inspected).contains("fields:"),
        "form inspect still reports only its own facts"
    );
}

#[test]
fn a_rejected_command_line_is_still_a_usage_error() {
    let output = run(&["form", "diff", "one.xml", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(envelope(&output)["error"]["code"], "usage.arguments");
    assert!(output.stderr.is_empty());

    let output = run(&["form", "fields", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(envelope(&output)["error"]["code"], "usage.arguments");
}
