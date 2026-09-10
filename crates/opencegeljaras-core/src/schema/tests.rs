//! Unit tests for the manifest rules, the compiled manifest, and the audit.

use super::*;
use crate::form::Limits;

/// The wrapper the form graph is rooted at.
const FORM_ROOT: &str =
    "https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd";
/// The dossier graph's own root.
const DOSSIER_ROOT: &str = "https://www.e-cegjegyzek.hu/schema/2026/e-cegeljaras2026.xsd";
/// The dossier resource carrying a DOCTYPE and a `www` host copy of `xml.xsd`.
const DOSSIER_XML: &str = "https://www.e-cegjegyzek.hu/schema/2026/xml.xsd";
/// The dossier resource carrying a DOCTYPE and an entity in its namespace.
const DOSSIER_XMLDSIG: &str = "https://www.e-cegjegyzek.hu/schema/2026/xmldsig-core-schema.xsd";

/// A minimal valid manifest text with the given resource list.
fn manifest_text(resources: &str, roots: &str) -> String {
    format!(
        r#"{{"snapshot_version":1,"retrieved_at":"2026-09-10","purpose":"test",
            "roots":[{roots}],"resources":[{resources}]}}"#
    )
}

/// One resource entry with a valid digest.
fn resource(url: &str, dependencies: &str) -> String {
    format!(
        r#"{{"url":"{url}","bytes":1,
            "sha256":"0000000000000000000000000000000000000000000000000000000000000000",
            "target_namespace":null,"dependencies":[{dependencies}],"roots":[]}}"#
    )
}

#[test]
fn the_compiled_manifest_is_valid_and_pins_the_reviewed_graph() {
    let manifest = compiled_manifest().expect("the compiled manifest must be valid");
    assert_eq!(manifest.snapshot_version, 1);
    assert_eq!(manifest.retrieved_at, "2026-09-10");
    assert_eq!(manifest.resources.len(), 149);
    assert_eq!(manifest.roots.first().map(String::as_str), Some(FORM_ROOT));

    let closure = manifest.form_closure();
    assert_eq!(closure.len(), 139);
    assert!(closure.contains(FORM_ROOT));
    for dossier in [DOSSIER_XML, DOSSIER_XMLDSIG] {
        assert!(
            manifest.resource(dossier).is_some(),
            "{dossier} must be pinned"
        );
        assert!(
            !closure.contains(dossier),
            "{dossier} carries a DOCTYPE and must stay outside the form closure"
        );
    }
}

#[test]
fn the_dossier_closure_is_ten_resources_disjoint_from_the_form_closure() {
    let manifest = compiled_manifest().expect("the compiled manifest must be valid");
    assert_eq!(
        manifest.roots.get(1).map(String::as_str),
        Some(DOSSIER_ROOT)
    );

    let dossier = manifest.dossier_closure();
    assert_eq!(dossier.len(), 10);
    assert!(dossier.contains(DOSSIER_ROOT));
    for resource in [DOSSIER_XML, DOSSIER_XMLDSIG] {
        assert!(
            dossier.contains(resource),
            "{resource} carries a DOCTYPE and is verified by digest inside the dossier closure"
        );
    }
    let form = manifest.form_closure();
    assert!(
        form.is_disjoint(&dossier),
        "the two graphs share no resource"
    );
    assert_eq!(form.len() + dossier.len(), manifest.resources.len());
}

#[test]
fn a_manifest_without_a_second_root_has_an_empty_dossier_closure() {
    let text = manifest_text(
        &resource("https://www.e-cegjegyzek.hu/a.xsd", ""),
        r#""https://www.e-cegjegyzek.hu/a.xsd""#,
    );
    let manifest = SchemaManifest::parse(&text).expect("the built manifest is valid");
    assert_eq!(manifest.form_closure().len(), 1);
    assert!(manifest.dossier_closure().is_empty());
}

#[test]
fn every_compiled_resource_url_maps_inside_a_bundle() {
    let manifest = compiled_manifest().expect("the compiled manifest must be valid");
    for resource in &manifest.resources {
        let path = bundle_path(&resource.url).expect("every pinned URL must map to a bundle path");
        assert!(ALLOWED_HOSTS.contains(&path.host));
        assert!(!path.segments.is_empty());
        assert!(!path.segments.contains(&".."));
    }
}

#[test]
fn the_path_rules_refuse_everything_outside_the_pinned_shape() {
    for url in [
        "http://schema.e-cegjegyzek.hu/a.xsd",
        "https://example.invalid/a.xsd",
        "https://schema.e-cegjegyzek.hu",
        "https://schema.e-cegjegyzek.hu/",
        "https://schema.e-cegjegyzek.hu/../a.xsd",
        "https://schema.e-cegjegyzek.hu/./a.xsd",
        "https://schema.e-cegjegyzek.hu//a.xsd",
        "https://schema.e-cegjegyzek.hu/a.xsd?v=1",
        "https://schema.e-cegjegyzek.hu/a.xsd#top",
        "https://schema.e-cegjegyzek.hu/a%2e.xsd",
        "https://schema.e-cegjegyzek.hu/a\\b.xsd",
        "https://schema.e-cegjegyzek.hu/a:b.xsd",
        "https://schema.e-cegjegyzek.hu/á.xsd",
        "https://schema.e-cegjegyzek.hu/a\0.xsd",
    ] {
        assert!(bundle_path(url).is_none(), "{url} must be refused");
    }
    let long = format!("https://schema.e-cegjegyzek.hu/{}.xsd", "a".repeat(512));
    assert!(bundle_path(&long).is_none());

    let path = bundle_path("https://www.e-cegjegyzek.hu/schema/2026/a-b_c.1.xsd")
        .expect("a pinned shape must be accepted");
    assert_eq!(path.host, "www.e-cegjegyzek.hu");
    assert_eq!(path.segments, ["schema", "2026", "a-b_c.1.xsd"]);
}

#[test]
fn manifest_rule_violations_are_reported_by_kind() {
    let good = resource("https://schema.e-cegjegyzek.hu/a.xsd", "");
    let root = r#""https://schema.e-cegjegyzek.hu/a.xsd""#;
    assert!(SchemaManifest::parse(&manifest_text(&good, root)).is_ok());

    for (text, problem) in [
        (String::from("{"), ManifestProblem::Syntax),
        (
            manifest_text(&good, root).replace(r#""purpose":"test""#, r#""extra":1"#),
            ManifestProblem::Syntax,
        ),
        (
            manifest_text(&resource("https://example.invalid/a.xsd", ""), root),
            ManifestProblem::ResourceUrl,
        ),
        (
            manifest_text(&format!("{good},{good}"), root),
            ManifestProblem::DuplicateUrl,
        ),
        (
            manifest_text(
                &resource(
                    "https://schema.e-cegjegyzek.hu/a.xsd",
                    r#""https://schema.e-cegjegyzek.hu/b.xsd""#,
                ),
                root,
            ),
            ManifestProblem::UnknownDependency,
        ),
        (
            manifest_text(&good.replace("0000", "000A"), root),
            ManifestProblem::Digest,
        ),
        (
            manifest_text(&good, r#""https://schema.e-cegjegyzek.hu/b.xsd""#),
            ManifestProblem::Roots,
        ),
        (manifest_text(&good, ""), ManifestProblem::Roots),
    ] {
        assert_eq!(SchemaManifest::parse(&text), Err(problem), "{text}");
    }
}

#[test]
fn the_form_closure_follows_dependencies_only_from_the_first_root() {
    let resources = [
        resource(
            "https://schema.e-cegjegyzek.hu/a.xsd",
            r#""https://schema.e-cegjegyzek.hu/b.xsd""#,
        ),
        resource(
            "https://schema.e-cegjegyzek.hu/b.xsd",
            r#""https://schema.e-cegjegyzek.hu/a.xsd""#,
        ),
        resource("https://schema.e-cegjegyzek.hu/c.xsd", ""),
    ]
    .join(",");
    let roots = r#""https://schema.e-cegjegyzek.hu/a.xsd","https://schema.e-cegjegyzek.hu/c.xsd""#;
    let manifest =
        SchemaManifest::parse(&manifest_text(&resources, roots)).expect("the manifest is valid");
    let closure = manifest.form_closure();
    assert_eq!(closure.len(), 2);
    assert!(!closure.contains("https://schema.e-cegjegyzek.hu/c.xsd"));
}

#[test]
fn http_locations_on_the_two_hosts_normalise_to_https() {
    assert_eq!(
        normalise_scheme("http://schema.e-cegjegyzek.hu/a.xsd"),
        "https://schema.e-cegjegyzek.hu/a.xsd"
    );
    assert_eq!(
        normalise_scheme("http://www.e-cegjegyzek.hu/a.xsd"),
        "https://www.e-cegjegyzek.hu/a.xsd"
    );
    for unchanged in [
        "http://www.w3.org/2001/03/xml.xsd",
        "https://schema.e-cegjegyzek.hu/a.xsd",
        "http://schema.e-cegjegyzek.hu.example.invalid/a.xsd",
    ] {
        assert_eq!(normalise_scheme(unchanged), unchanged);
    }
}

#[test]
fn a_known_digest_is_reported_in_lowercase_hexadecimal() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn resource_statuses_have_stable_codes() {
    for (status, code) in [
        (ResourceStatus::Verified, "verified"),
        (ResourceStatus::Missing, "missing"),
        (ResourceStatus::Oversize, "oversize"),
        (ResourceStatus::Mismatch, "mismatch"),
        (ResourceStatus::Unparseable, "unparseable"),
        (ResourceStatus::UnpinnedImport, "unpinned_import"),
    ] {
        assert_eq!(status.code(), code);
    }
}

/// A two-resource manifest whose second resource carries a namespace.
fn audit_manifest() -> SchemaManifest {
    let resources = format!(
        r#"{},{{"url":"https://schema.e-cegjegyzek.hu/schema/b.xsd","bytes":1,
            "sha256":"0000000000000000000000000000000000000000000000000000000000000000",
            "target_namespace":"urn:b","dependencies":[],"roots":[]}}"#,
        resource(
            "https://schema.e-cegjegyzek.hu/schema/a.xsd",
            r#""https://schema.e-cegjegyzek.hu/schema/b.xsd""#
        )
    );
    SchemaManifest::parse(&manifest_text(
        &resources,
        r#""https://schema.e-cegjegyzek.hu/schema/a.xsd""#,
    ))
    .expect("the audit manifest is valid")
}

/// The offending locations of one schema body, audited against
/// [`audit_manifest`].
fn audit(body: &str) -> Vec<String> {
    audit_all(body).locations
}

/// The full audit result of one schema body, including its truncation flag.
fn audit_all(body: &str) -> UnpinnedLocations {
    let manifest = audit_manifest();
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
           <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#
    );
    let found = edges(
        text.as_bytes(),
        "https://schema.e-cegjegyzek.hu/schema/a.xsd",
        &Limits::default(),
    )
    .expect("the test schema parses");
    unpinned_locations(&manifest, &found)
}

#[test]
fn a_pinned_import_with_the_recorded_namespace_audits_clean() {
    assert!(audit(r#"<xs:import namespace="urn:b" schemaLocation="b.xsd"/>"#).is_empty());
    assert!(audit(r#"<xs:import namespace="urn:b" schemaLocation="http://schema.e-cegjegyzek.hu/schema/b.xsd"/>"#).is_empty());
    assert!(audit(r#"<xs:include schemaLocation="./b.xsd"/>"#).is_empty());
    assert!(audit("<xs:element name=\"a\"/>").is_empty());
}

#[test]
fn an_unpinned_or_mismatched_import_lists_its_resolved_location() {
    assert_eq!(
        audit(r#"<xs:import namespace="urn:b" schemaLocation="../elsewhere/b.xsd"/>"#),
        ["https://schema.e-cegjegyzek.hu/elsewhere/b.xsd"]
    );
    assert_eq!(
        audit(r#"<xs:import namespace="urn:c" schemaLocation="b.xsd"/>"#),
        ["https://schema.e-cegjegyzek.hu/schema/b.xsd"]
    );
    assert_eq!(
        audit(
            r#"<xs:import namespace="urn:b" schemaLocation="http://www.w3.org/2001/03/xml.xsd"/>"#
        ),
        ["http://www.w3.org/2001/03/xml.xsd"]
    );
    assert_eq!(audit(r#"<xs:import namespace="urn:b"/>"#), [""]);
    assert_eq!(
        audit(
            r#"<xs:import namespace="urn:c" schemaLocation="b.xsd"/>
               <xs:import namespace="urn:d" schemaLocation="b.xsd"/>"#
        ),
        ["https://schema.e-cegjegyzek.hu/schema/b.xsd"],
        "one offending location is listed once"
    );
}

#[test]
fn more_offending_locations_than_the_cap_are_truncated() {
    let mut body = String::new();
    for index in 0..70 {
        body.push_str(&format!(
            r#"<xs:import namespace="urn:b" schemaLocation="/unpinned-{index}.xsd"/>"#
        ));
    }
    // The same offender repeated must not consume a slot or set the flag.
    body.push_str(r#"<xs:import namespace="urn:b" schemaLocation="/unpinned-0.xsd"/>"#);
    let found = audit_all(&body);
    assert_eq!(found.locations.len(), LOCATION_CAP);
    assert!(found.truncated);
    assert_eq!(
        found.locations[0], "https://schema.e-cegjegyzek.hu/unpinned-0.xsd",
        "the first offender is listed first"
    );
    assert_eq!(
        found.locations[LOCATION_CAP - 1],
        "https://schema.e-cegjegyzek.hu/unpinned-63.xsd",
        "the listed offenders keep first-appearance order"
    );

    let mut body = String::new();
    for index in 0..LOCATION_CAP {
        body.push_str(&format!(
            r#"<xs:import namespace="urn:b" schemaLocation="/unpinned-{index}.xsd"/>"#
        ));
    }
    let found = audit_all(&body);
    assert_eq!(found.locations.len(), LOCATION_CAP);
    assert!(!found.truncated, "exactly the cap is not a truncation");
}

#[test]
fn a_long_location_is_cut_before_it_is_reported() {
    let long = "a".repeat(200);
    let offending = audit(&format!(
        r#"<xs:import namespace="urn:b" schemaLocation="/{long}"/>"#
    ));
    assert_eq!(offending.len(), 1);
    assert_eq!(offending[0].chars().count(), LOCATION_LIMIT + 1);
    assert!(offending[0].ends_with('…'));
}

#[test]
fn a_schema_the_bounded_entry_point_refuses_is_not_audited() {
    let limits = Limits::default();
    for refused in [
        "<!DOCTYPE schema><xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"/>",
        "<xs:schema",
        "",
    ] {
        assert!(
            edges(
                refused.as_bytes(),
                "https://schema.e-cegjegyzek.hu/schema/a.xsd",
                &limits
            )
            .is_err(),
            "{refused}"
        );
    }
}
