//! Contract tests for `schemas list` and `schemas verify` over the binary.
//!
//! Every bundle used here is synthetic: the stored CC0 mini graph, or a copy
//! of it generated into a per-process scratch directory. Nothing private is
//! read, and no test fetches anything.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use opencegeljaras_core::schema::sha256_hex;
use serde_json::{Value, json};

/// Bundle-relative path of the form root.
const WRAPPER: &str = "schema.e-cegjegyzek.hu/schema/mini/wrapper-20260101.xsd";
/// Bundle-relative path of the first variant, which imports over `http`.
const VARIANT_A: &str = "schema.e-cegjegyzek.hu/schema/mini/variant-a-20260101.xsd";
/// Bundle-relative path of the second variant.
const VARIANT_B: &str = "schema.e-cegjegyzek.hu/schema/mini/variant-b-20260101.xsd";
/// Bundle-relative path of the shared simple-type schema.
const SHARED: &str = "schema.e-cegjegyzek.hu/schema/mini/common/shared-20260101.xsd";
/// Bundle-relative path of the DOCTYPE-bearing dossier resource.
const DOSSIER: &str = "www.e-cegjegyzek.hu/schema/mini/dossier-20260101.xsd";

/// The pinned URL of a bundle-relative fixture path.
fn url(relative: &str) -> String {
    format!("https://{relative}")
}

/// The stored fixture directory at the repository root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/schemas")
        .join(name)
}

/// A fresh per-process scratch directory.
fn scratch(name: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "opencegeljaras-schemas-{}-{}-{name}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencegeljaras"))
        .args(args)
        .output()
        .expect("CLI should run")
}

/// Run `schemas verify` over one bundle and manifest, in JSON or human mode.
fn verify(bundle: &Path, manifest: Option<&Path>, json: bool) -> Output {
    let bundle = bundle.to_str().expect("test paths are UTF-8").to_owned();
    let mut args = vec!["schemas", "verify", "--bundle", &bundle];
    let manifest = manifest.map(|path| path.to_str().expect("test paths are UTF-8").to_owned());
    if let Some(manifest) = &manifest {
        args.extend(["--manifest", manifest]);
    }
    if json {
        args.push("--json");
    }
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
fn assert_single_diagnostic(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit));
    assert_eq!(
        stderr(output),
        format!("opencegeljaras: schemas verify: {code} (exit {exit})\n")
    );
}

/// Assert that no part of a local path reached either stream.
fn assert_no_path_echoed(output: &Output, path: &Path) {
    let path = path.to_str().expect("test paths are UTF-8");
    for stream in [stdout(output), stderr(output)] {
        assert!(
            !stream.contains(path),
            "a local path reached an output stream"
        );
        for component in ["scratch", "tampered", "bundle"] {
            assert!(
                !stream.contains(&format!("/{component}")),
                "a local path component reached an output stream"
            );
        }
    }
}

/// Copy one directory tree, which is how a scratch bundle starts.
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

/// A generated copy of the stored mini graph, with its manifest re-digested.
struct Mini {
    root: PathBuf,
}

impl Mini {
    /// Copy the stored bundle, apply content overrides, and pin what was
    /// written: `None` removes a resource, `Some` replaces its bytes.
    fn new(overrides: &[(&str, Option<&str>)]) -> Self {
        let root = scratch("mini");
        let bundle = root.join("bundle");
        copy_tree(&fixture("good"), &bundle);
        for (relative, content) in overrides {
            let path = bundle.join(relative);
            match content {
                None => fs::remove_file(&path).expect("the fixture copy should be removable"),
                Some(content) => {
                    fs::write(&path, content).expect("the fixture copy should be writable");
                }
            }
        }
        Self { root }.repin()
    }

    /// Rewrite the manifest so that every present file's size and digest match.
    fn repin(self) -> Self {
        let text = fs::read_to_string(fixture("manifest.json"))
            .expect("the fixture manifest should be readable");
        let mut manifest: Value =
            serde_json::from_str(&text).expect("the fixture manifest should be JSON");
        for resource in manifest["resources"]
            .as_array_mut()
            .expect("resources should be an array")
        {
            let pinned = resource["url"]
                .as_str()
                .and_then(|url| url.strip_prefix("https://"))
                .expect("every fixture URL is https")
                .to_owned();
            if let Ok(bytes) = fs::read(self.bundle().join(&pinned)) {
                resource["bytes"] = json!(bytes.len());
                resource["sha256"] = json!(sha256_hex(&bytes));
            }
        }
        fs::write(
            self.manifest(),
            serde_json::to_string(&manifest).expect("the manifest should serialise"),
        )
        .expect("the scratch manifest should be writable");
        self
    }

    /// The generated bundle directory.
    fn bundle(&self) -> PathBuf {
        self.root.join("bundle")
    }

    /// The generated manifest file.
    fn manifest(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    /// Verify the generated bundle with its own manifest, as JSON.
    fn verify(&self) -> Output {
        verify(&self.bundle(), Some(&self.manifest()), true)
    }
}

impl Drop for Mini {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// One schema body in the fixture's shared namespace, with a chosen import.
fn variant_with_import(namespace: &str, location: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:opencegeljaras:fixture:variant-a"
           elementFormDefault="qualified">
  <xs:import namespace="{namespace}" schemaLocation="{location}"/>
</xs:schema>
"#
    )
}

#[test]
fn list_reports_the_compiled_manifest_without_fetching_anything() {
    let output = run(&["schemas", "list", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let body = body(&output);
    assert_eq!(body["command"], "schemas list");
    assert_eq!(body["ok"], true);
    assert_eq!(body["verified"], false);
    assert_eq!(body["submitted"], false);

    let data = &body["data"];
    assert_eq!(data["snapshot_version"], 1);
    assert_eq!(data["retrieved_at"], "2026-09-10");
    assert_eq!(data["count"], 149);
    assert_eq!(data["form_closure_count"], 139);
    assert_eq!(
        data["notice"],
        "pinned research metadata; nothing was fetched or verified"
    );
    let resources = data["resources"].as_array().expect("resources is an array");
    assert_eq!(resources.len(), 149);
    assert_eq!(resources[0]["url"], data["roots"][0]);
    for resource in resources {
        assert_eq!(
            resource["sha256"]
                .as_str()
                .expect("a digest is a string")
                .len(),
            64
        );
        assert!(resource["bytes"].is_u64());
        assert!(resource["dependencies"].is_array());
        assert!(resource["roots"].is_array());
    }
}

#[test]
fn list_human_output_names_the_snapshot_roots_and_every_resource() {
    let output = run(&["schemas", "list"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.starts_with("snapshot: 2026-09-10\nroots:\n"));
    assert!(text.contains("\nresources: 149\n"));
    assert!(text.contains("\nform closure: 139\n"));
    assert!(text.contains(
        "- https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd 1368 bytes 6f727249d5d3c7d2 "
    ));
    assert!(text.ends_with("pinned research metadata; nothing was fetched or verified\n"));
    assert_eq!(
        text.lines().filter(|line| line.starts_with("- ")).count(),
        149
    );
}

#[test]
fn urls_prints_only_the_pinned_urls_and_refuses_json() {
    let output = run(&["schemas", "list", "--urls"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 149);
    assert_eq!(
        lines[0],
        "https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd"
    );
    assert!(lines.iter().all(|line| line.starts_with("https://")));

    let refused = run(&["schemas", "list", "--urls", "--json"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert_eq!(
        stderr(&refused),
        "opencegeljaras: invalid command line; run opencegeljaras --help (exit 2)\n"
    );
}

#[test]
fn the_stored_good_bundle_verifies_including_the_doctype_resource() {
    let bundle = fixture("good");
    let manifest = fixture("manifest.json");
    let output = verify(&bundle, Some(&manifest), true);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let data = &body(&output)["data"];
    assert_eq!(data["manifest"]["source"], "file");
    assert_eq!(data["manifest"]["count"], 5);
    assert_eq!(data["manifest"]["form_closure_count"], 4);
    assert_eq!(data["bundle"]["verified"], true);
    assert_eq!(data["bundle"]["form_closure_verified"], true);
    assert_eq!(data["bundle"]["verified_count"], 5);
    assert_eq!(data["problems"], json!([]));
    assert_eq!(data["checks"]["digests"], "passed");
    assert_eq!(data["checks"]["import_audit"], "passed");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(
        data["notice"],
        "digest and import audit of a local mirror; not a licence, not validation; unpinned manifest"
    );
    assert_eq!(body(&output)["verified"], false);

    // The dossier resource carries a DOCTYPE the bounded parser refuses, so
    // its verified status can only have come from the digest comparison.
    let dossier = fs::read_to_string(bundle.join(DOSSIER)).expect("the fixture is readable");
    assert!(dossier.contains("<!DOCTYPE"));
}

#[test]
fn the_good_bundle_human_report_states_every_boundary() {
    let output = verify(&fixture("good"), Some(&fixture("manifest.json")), false);
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    assert!(
        text.starts_with(
            "manifest: file, retrieved 2026-09-10, 5 resources, 4 in the form closure\n"
        )
    );
    assert!(text.contains("\nbundle: verified\n"));
    assert!(text.contains("\nform closure: verified\n"));
    assert!(text.contains(
        "\ncounts: verified 5, missing 0, mismatch 0, oversize 0, unparseable 0, unpinned import 0\n"
    ));
    assert!(text.contains("\nproblems: none\n"));
    assert!(text.contains("\ndigests: passed\n"));
    assert!(text.contains("\nimport audit: passed\n"));
    assert!(text.ends_with(
        "xsd conformance: not performed\nverification: not performed\nsubmission: not performed\n"
    ));
}

#[test]
fn the_stored_tampered_bundle_reports_mismatch_and_missing_by_url() {
    let bundle = fixture("tampered");
    let output = verify(&bundle, Some(&fixture("manifest.json")), true);
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    assert_no_path_echoed(&output, &bundle);
    let envelope = body(&output);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "bundle_unverified");
    assert_eq!(
        envelope["error"]["message"],
        "the bundle does not match the pinned manifest"
    );
    let data = &envelope["data"];
    assert_eq!(data["bundle"]["verified"], false);
    assert_eq!(data["bundle"]["form_closure_verified"], false);
    assert_eq!(data["bundle"]["verified_count"], 3);
    assert_eq!(data["bundle"]["mismatch"], 1);
    assert_eq!(data["bundle"]["missing"], 1);
    assert_eq!(data["checks"]["digests"], "failed");
    assert_eq!(data["checks"]["import_audit"], "not_performed");
    assert_eq!(
        data["problems"],
        json!([
            {
                "url": url(VARIANT_A),
                "status": "mismatch",
                "locations": [],
                "locations_truncated": false,
            },
            {
                "url": url(VARIANT_B),
                "status": "missing",
                "locations": [],
                "locations_truncated": false,
            },
        ])
    );
}

#[test]
fn a_form_closure_resource_importing_something_unpinned_lists_the_location() {
    let mini = Mini::new(&[(
        VARIANT_A,
        Some(&variant_with_import(
            "urn:opencegeljaras:fixture:shared",
            "../elsewhere/shared-20260101.xsd",
        )),
    )]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["unpinned_import"], 1);
    assert_eq!(data["bundle"]["form_closure_verified"], false);
    assert_eq!(data["checks"]["digests"], "passed");
    assert_eq!(data["checks"]["import_audit"], "failed");
    assert_eq!(
        data["problems"],
        json!([{
            "url": url(VARIANT_A),
            "status": "unpinned_import",
            "locations": ["https://schema.e-cegjegyzek.hu/schema/elsewhere/shared-20260101.xsd"],
            "locations_truncated": false,
        }])
    );
}

#[test]
fn an_import_naming_the_wrong_namespace_is_unpinned() {
    let mini = Mini::new(&[(
        VARIANT_A,
        Some(&variant_with_import(
            "urn:opencegeljaras:fixture:other",
            "common/shared-20260101.xsd",
        )),
    )]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["unpinned_import"], 1);
    assert_eq!(data["problems"][0]["locations"], json!([url(SHARED)]));
}

#[test]
fn an_http_import_on_an_allowed_host_normalises_to_the_pinned_url() {
    let mini = Mini::new(&[(
        VARIANT_A,
        Some(&variant_with_import(
            "urn:opencegeljaras:fixture:shared",
            &format!("http://{SHARED}"),
        )),
    )]);
    let output = mini.verify();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(body(&output)["data"]["bundle"]["verified"], true);
}

#[test]
fn a_form_closure_resource_the_parser_refuses_is_unparseable() {
    let mini = Mini::new(&[(
        VARIANT_A,
        Some("<!DOCTYPE xs:schema><xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"/>"),
    )]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["unparseable"], 1);
    assert_eq!(data["checks"]["digests"], "passed");
    assert_eq!(data["checks"]["import_audit"], "failed");
    assert_eq!(data["problems"][0]["status"], "unparseable");
    assert_eq!(data["problems"][0]["locations"], json!([]));
}

#[test]
fn more_offending_locations_than_the_cap_are_reported_as_truncated() {
    let mut imports = String::new();
    for index in 0..70 {
        imports.push_str(&format!(
            r#"<xs:import namespace="urn:opencegeljaras:fixture:shared" schemaLocation="/unpinned-{index}.xsd"/>"#
        ));
    }
    let variant = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:opencegeljaras:fixture:variant-a"
           elementFormDefault="qualified">{imports}</xs:schema>
"#
    );
    let mini = Mini::new(&[(VARIANT_A, Some(&variant))]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let problem = &body(&output)["data"]["problems"][0];
    assert_eq!(problem["status"], "unpinned_import");
    assert_eq!(problem["locations_truncated"], true);
    assert_eq!(
        problem["locations"]
            .as_array()
            .expect("locations is an array")
            .len(),
        64
    );
}

#[test]
fn a_missing_form_root_is_reported_by_its_pinned_url() {
    let mini = Mini::new(&[(WRAPPER, None)]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["missing"], 1);
    assert_eq!(data["bundle"]["form_closure_verified"], false);
    assert_eq!(data["bundle"]["verified_count"], 4);
    assert_eq!(data["problems"][0]["url"], url(WRAPPER));
    assert_eq!(data["problems"][0]["status"], "missing");
    assert_eq!(data["checks"]["import_audit"], "not_performed");
}

#[test]
fn a_resource_larger_than_the_read_cap_is_oversize() {
    let oversize = "a".repeat(4 * 1024 * 1024 + 1);
    let mini = Mini::new(&[(DOSSIER, Some(&oversize))]);
    let output = mini.verify();
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["bundle"]["oversize"], 1);
    assert_eq!(data["bundle"]["form_closure_verified"], true);
    assert_eq!(data["checks"]["digests"], "failed");
    assert_eq!(data["checks"]["import_audit"], "passed");
    assert_eq!(data["problems"][0]["url"], url(DOSSIER));
    assert_eq!(data["problems"][0]["status"], "oversize");
}

#[test]
fn an_unreadable_bundle_directory_fails_closed() {
    let root = scratch("unreadable");
    let missing = root.join("absent");
    let file = root.join("a-file");
    fs::write(&file, b"not a directory").expect("the scratch file should be writable");
    for path in [&missing, &file] {
        let output = verify(path, None, true);
        assert_single_diagnostic(&output, "bundle_unreadable", 1);
        assert_no_path_echoed(&output, path);
        let envelope = body(&output);
        assert_eq!(envelope["ok"], false);
        assert_eq!(
            envelope["error"]["message"],
            "the bundle directory could not be read"
        );
        assert_eq!(envelope["data"]["manifest"], Value::Null);
        assert_eq!(envelope["data"]["bundle"], Value::Null);
        assert_eq!(envelope["data"]["problems"], Value::Null);
        assert_eq!(envelope["data"]["checks"]["digests"], "not_performed");
        assert_eq!(envelope["verified"], false);
    }
    let human = verify(&missing, None, false);
    assert_single_diagnostic(&human, "bundle_unreadable", 1);
    assert!(human.stdout.is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_manifest_file_fails_closed() {
    let root = scratch("no-manifest");
    let missing = root.join("absent.json");
    let output = verify(&fixture("good"), Some(&missing), true);
    assert_single_diagnostic(&output, "manifest_unreadable", 1);
    assert_no_path_echoed(&output, &missing);
    assert_eq!(
        body(&output)["error"]["message"],
        "the manifest could not be read as a regular file"
    );
    assert_eq!(
        body(&output)["data"]["notice"],
        "digest and import audit of a local mirror; not a licence, not validation; unpinned manifest"
    );

    let directory = verify(&fixture("good"), Some(&root), true);
    assert_single_diagnostic(&directory, "manifest_unreadable", 1);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_manifest_breaking_any_pinned_rule_is_invalid() {
    let text = fs::read_to_string(fixture("manifest.json")).expect("the fixture is readable");
    let root = scratch("manifests");
    let edits: [(&str, String); 5] = [
        ("unknown-field", text.replacen('{', r#"{"extra": 1,"#, 1)),
        (
            "dot-segments",
            text.replace("schema/mini/common/", "schema/mini/../common/"),
        ),
        (
            "other-host",
            text.replace("https://www.e-cegjegyzek.hu", "https://example.invalid"),
        ),
        (
            "http-scheme",
            text.replace(
                "https://schema.e-cegjegyzek.hu",
                "http://schema.e-cegjegyzek.hu",
            ),
        ),
        ("not-json", String::from("{")),
    ];
    for (name, edited) in edits {
        let path = root.join(format!("{name}.json"));
        fs::write(&path, &edited).expect("the scratch manifest should be writable");
        let output = verify(&fixture("good"), Some(&path), true);
        assert_single_diagnostic(&output, "manifest_invalid", 1);
        assert_no_path_echoed(&output, &path);
        assert_eq!(
            body(&output)["error"]["message"],
            "the manifest does not satisfy the pinned schema rules",
            "{name}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_compiled_manifest_verifies_nothing_without_a_bundle_of_its_own() {
    // The stored mini bundle mirrors five URLs the compiled manifest does not
    // pin, so verifying it against the compiled manifest reports every pinned
    // resource as missing and never reads anything outside the manifest.
    let output = verify(&fixture("good"), None, true);
    assert_single_diagnostic(&output, "bundle_unverified", 4);
    let data = &body(&output)["data"];
    assert_eq!(data["manifest"]["source"], "compiled");
    assert_eq!(data["bundle"]["missing"], 149);
    assert_eq!(data["bundle"]["verified_count"], 0);
    assert_eq!(data["checks"]["import_audit"], "not_performed");
    assert_eq!(
        data["notice"],
        "digest and import audit of a local mirror; not a licence, not validation"
    );
}

#[test]
fn verification_output_is_byte_identical_across_runs() {
    let first = verify(&fixture("good"), Some(&fixture("manifest.json")), true);
    let second = verify(&fixture("good"), Some(&fixture("manifest.json")), true);
    assert_eq!(first.stdout, second.stdout);
    let first = verify(&fixture("tampered"), Some(&fixture("manifest.json")), false);
    let second = verify(&fixture("tampered"), Some(&fixture("manifest.json")), false);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[cfg(unix)]
#[test]
fn symlinked_resources_are_followed_and_a_symlinked_directory_is_canonicalised() {
    use std::os::unix::fs::symlink;

    let mini = Mini::new(&[]);
    let stored = fixture("good").join(SHARED);
    let mirrored = mini.bundle().join(SHARED);
    fs::remove_file(&mirrored).expect("the copied resource should be removable");
    symlink(
        fs::canonicalize(&stored).expect("the fixture path canonicalises"),
        &mirrored,
    )
    .expect("a symlink should be creatable");
    let output = mini.verify();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(body(&output)["data"]["bundle"]["verified"], true);

    let link = mini.root.join("link");
    symlink(mini.bundle(), &link).expect("a directory symlink should be creatable");
    let output = verify(&link, Some(&mini.manifest()), true);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(body(&output)["data"]["bundle"]["verified_count"], 5);
    assert_no_path_echoed(&output, &link);
}

#[test]
#[ignore = "needs a real mirror named by OPENCEGELJARAS_SCHEMA_BUNDLE"]
fn the_real_bundle_verifies_against_the_compiled_manifest() {
    let bundle = std::env::var("OPENCEGELJARAS_SCHEMA_BUNDLE")
        .expect("set OPENCEGELJARAS_SCHEMA_BUNDLE to a local mirror of the pinned schema bundle");
    let output = verify(Path::new(&bundle), None, true);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let data = &body(&output)["data"];
    assert_eq!(data["manifest"]["source"], "compiled");
    assert_eq!(data["bundle"]["verified"], true);
    assert_eq!(data["bundle"]["form_closure_verified"], true);
    assert_eq!(data["bundle"]["verified_count"], 149);
    assert_eq!(data["checks"]["digests"], "passed");
    assert_eq!(data["checks"]["import_audit"], "passed");
    assert_eq!(data["checks"]["xsd_conformance"], "not_performed");
    assert_eq!(data["problems"], json!([]));
}
