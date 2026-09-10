//! Adapter tests: the fake validator for mechanics, real xmllint for results.
//!
//! Every bundle used here is the synthetic CC0 mini graph or a copy of it.
//! Nothing private is read and nothing is fetched.

use std::path::{Path, PathBuf};

use super::*;

#[cfg(unix)]
#[allow(dead_code)]
#[path = "../../tests/support/fake.rs"]
mod fake;

/// The marker test the fake launcher re-executes. A no-op otherwise.
#[cfg(unix)]
#[test]
fn fake_validator_entry() {
    fake::dispatch();
}

/// The stored mini-graph fixture directory at the repository root.
fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/schemas")
        .join(relative)
}

/// The canonical path of the mini graph's form root inside a bundle.
#[cfg(target_os = "linux")]
fn mini_root(bundle: &Path) -> PathBuf {
    std::fs::canonicalize(bundle.join("schema.e-cegjegyzek.hu/schema/mini/wrapper-20260101.xsd"))
        .expect("the mini form root should be readable")
}

/// The real xmllint, which must be installed wherever these tests run.
#[cfg(target_os = "linux")]
fn real_xmllint() -> PathBuf {
    locate(None).expect("xmllint must be installed for the real-validator tests")
}

#[test]
fn the_version_line_is_read_from_the_report() {
    assert_eq!(
        parse_version("xmllint: using libxml version 20914\n   compiled with: X"),
        Some(20914)
    );
    assert_eq!(
        parse_version("using libxml version 21299 extra"),
        Some(21299)
    );
    assert_eq!(parse_version("xmllint: using libxml version\n"), None);
    assert_eq!(parse_version("xmllint 2.9.14\n"), None);
    assert_eq!(parse_version(""), None);
}

#[test]
fn an_explicit_validator_path_must_name_a_regular_file() {
    assert_eq!(locate(Some(Path::new("/no/such/xmllint"))), None);
    assert_eq!(locate(Some(&fixture("good"))), None);
    let file = fixture("manifest.json");
    assert_eq!(
        locate(Some(&file)),
        Some(std::fs::canonicalize(&file).expect("the fixture manifest should be readable"))
    );
}

#[cfg(unix)]
mod mechanics {
    use super::*;
    use std::time::Duration;

    /// A generous timeout for a case that is expected to finish at once.
    const PROMPT: Duration = Duration::from_secs(30);

    /// Run one prepared case over a fixed document and a dummy schema root.
    fn run_case(
        name: &str,
        script: &str,
        timeout: Duration,
    ) -> (fake::Case, Result<Run, AdapterError>) {
        let case = fake::case(name, script);
        let root = case.directory.join("root.xsd");
        let outcome = run(&case.exe, &root, b"<document/>", timeout);
        (case, outcome)
    }

    #[test]
    fn the_lowest_established_version_is_accepted() {
        let (_, outcome) = run_case(
            "version-low",
            "version=xmllint: using libxml version 20914\nexit=0\n",
            PROMPT,
        );
        assert_eq!(
            outcome.expect("20914 is supported").version,
            MIN_LIBXML_VERSION
        );
    }

    #[test]
    fn one_below_the_lowest_established_version_is_refused() {
        let (_, outcome) = run_case(
            "version-too-low",
            "version=xmllint: using libxml version 20913\n",
            PROMPT,
        );
        assert_eq!(
            outcome.err(),
            Some(AdapterError::Unavailable {
                version: Some(20913)
            })
        );
    }

    #[test]
    fn the_highest_established_version_is_accepted() {
        let (_, outcome) = run_case(
            "version-high",
            "version=xmllint: using libxml version 21299\nexit=3\n",
            PROMPT,
        );
        let run = outcome.expect("21299 is supported");
        assert_eq!(run.version, MAX_LIBXML_VERSION - 1);
        assert!(!run.conforming);
    }

    #[test]
    fn the_first_unestablished_version_is_refused() {
        let (_, outcome) = run_case(
            "version-ceiling",
            "version=xmllint: using libxml version 21300\n",
            PROMPT,
        );
        assert_eq!(
            outcome.err(),
            Some(AdapterError::Unavailable {
                version: Some(MAX_LIBXML_VERSION)
            })
        );
    }

    #[test]
    fn an_unreadable_version_line_is_refused() {
        let (_, outcome) = run_case("version-absent", "version=\n", PROMPT);
        assert_eq!(
            outcome.err(),
            Some(AdapterError::Unavailable { version: None })
        );
    }

    #[test]
    fn a_missing_validator_is_never_spawned() {
        let outcome = run(
            Path::new("/no/such/xmllint"),
            Path::new("/no/such/root.xsd"),
            b"<document/>",
            PROMPT,
        );
        assert_eq!(
            outcome.err(),
            Some(AdapterError::Unavailable { version: None })
        );
    }

    #[test]
    fn the_arguments_and_working_directory_are_fixed() {
        let (case, outcome) = run_case("argv", "exit=0\n", PROMPT);
        outcome.expect("the fake conforms");
        assert_eq!(case.argv("version"), ["--version"]);
        let root = case
            .directory
            .join("root.xsd")
            .to_str()
            .expect("test paths are UTF-8")
            .to_owned();
        assert_eq!(
            case.argv("run"),
            ["--nonet", "--noout", "--schema", &root, DOCUMENT_NAME]
        );
        let cwd = case.cwd();
        assert_ne!(cwd, case.directory);
        assert!(!cwd.starts_with(&case.directory));
    }

    #[test]
    fn only_the_allowlisted_environment_survives() {
        let (case, outcome) = run_case("environment", "exit=0\n", PROMPT);
        outcome.expect("the fake conforms");
        let names = case.environment();
        assert!(names.iter().any(|name| name == "PATH"), "{names:?}");
        for forbidden in [
            "LD_LIBRARY_PATH",
            "DYLD_LIBRARY_PATH",
            "XML_CATALOG_FILES",
            "SGML_CATALOG_FILES",
            "XML_DEBUG_CATALOG",
            "HOME",
            "CARGO_MANIFEST_DIR",
        ] {
            assert!(!names.iter().any(|name| name == forbidden), "{names:?}");
        }
    }

    #[test]
    fn the_document_copy_is_byte_identical_to_the_input() {
        let case = fake::case("document", "exit=0\n");
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/forms/nevfoglalasi-conforming.xml"),
        )
        .expect("the fixture should be readable");
        let root = case.directory.join("root.xsd");
        run(&case.exe, &root, &bytes, PROMPT).expect("the fake conforms");
        assert_eq!(case.document(), bytes);
    }

    #[test]
    fn exit_codes_are_mapped_to_conformance_or_failure() {
        let (_, conforming) = run_case("exit-0", "exit=0\n", PROMPT);
        assert!(conforming.expect("exit 0 is a result").conforming);
        let (_, nonconforming) = run_case("exit-3", "exit=3\n", PROMPT);
        assert!(!nonconforming.expect("exit 3 is a result").conforming);
        for code in [1, 2, 5, 9] {
            let (_, outcome) = run_case(&format!("exit-{code}"), &format!("exit={code}\n"), PROMPT);
            assert_eq!(
                outcome.err(),
                Some(AdapterError::Failed {
                    version: Some(MIN_LIBXML_VERSION),
                    exit_code: Some(code),
                })
            );
        }
    }

    #[test]
    fn a_run_that_outlasts_its_timeout_is_killed() {
        let (_, outcome) = run_case(
            "timeout",
            "sleep_ms=30000\nexit=0\n",
            Duration::from_millis(2000),
        );
        assert_eq!(
            outcome.err(),
            Some(AdapterError::Failed {
                version: Some(MIN_LIBXML_VERSION),
                exit_code: None,
            })
        );
    }

    #[test]
    fn a_flood_of_standard_error_is_capped() {
        let (_, outcome) = run_case("flood", "flood=3000000\nexit=3\n", PROMPT);
        let run = outcome.expect("a flooding validator still answers");
        assert_eq!(run.stderr.len(), STDERR_CAP);
    }

    #[test]
    fn chosen_standard_error_lines_are_returned() {
        let (_, outcome) = run_case("stderr", "stderr=first\nstderr=second\nexit=3\n", PROMPT);
        let run = outcome.expect("exit 3 is a result");
        assert_eq!(run.stderr, "first\nsecond\n");
        assert_eq!(run.exit_code, 3);
    }
}

mod real {
    use super::*;
    use std::time::Duration;

    /// The timeout used by every real-validator test.
    const TIMEOUT: Duration = Duration::from_secs(60);

    /// Copy the stored good mini bundle into a fresh directory.
    #[cfg(target_os = "linux")]
    fn mirror(into: &Path) {
        for relative in [
            "schema.e-cegjegyzek.hu/schema/mini/wrapper-20260101.xsd",
            "schema.e-cegjegyzek.hu/schema/mini/variant-a-20260101.xsd",
            "schema.e-cegjegyzek.hu/schema/mini/variant-b-20260101.xsd",
            "schema.e-cegjegyzek.hu/schema/mini/common/shared-20260101.xsd",
            "www.e-cegjegyzek.hu/schema/mini/dossier-20260101.xsd",
        ] {
            let target = into.join(relative);
            std::fs::create_dir_all(target.parent().expect("a mirrored file has a parent"))
                .expect("the mirror directory should be creatable");
            std::fs::copy(fixture("good").join(relative), target)
                .expect("the fixture should be copyable");
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned xmllint is checked on Linux"
    )]
    fn the_real_validator_accepts_a_conforming_mini_instance() {
        #[cfg(target_os = "linux")]
        {
            let bundle = fixture("good");
            let document = std::fs::read(fixture("instance-conforming.xml"))
                .expect("the fixture should be readable");
            let run = run(&real_xmllint(), &mini_root(&bundle), &document, TIMEOUT)
                .expect("the mini graph should load");
            assert!(run.conforming);
            assert_eq!(run.exit_code, 0);
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned xmllint is checked on Linux"
    )]
    fn the_real_validator_rejects_a_nonconforming_mini_instance() {
        #[cfg(target_os = "linux")]
        {
            let bundle = fixture("good");
            let document = std::fs::read(fixture("instance-nonconforming.xml"))
                .expect("the fixture should be readable");
            let run = run(&real_xmllint(), &mini_root(&bundle), &document, TIMEOUT)
                .expect("the mini graph should load");
            assert!(!run.conforming);
            assert_eq!(run.exit_code, 3);
            assert!(
                run.stderr.contains("Schemas validity error"),
                "{}",
                run.stderr
            );
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned xmllint is checked on Linux"
    )]
    fn a_bundle_path_with_uri_characters_still_resolves_its_imports() {
        #[cfg(target_os = "linux")]
        {
            let directory = fake_scratch("uri-characters").join("bundle#?%dir");
            std::fs::create_dir_all(&directory).expect("the odd directory should be creatable");
            mirror(&directory);
            let document = std::fs::read(fixture("instance-nonconforming.xml"))
                .expect("the fixture should be readable");
            let run = run(&real_xmllint(), &mini_root(&directory), &document, TIMEOUT)
                .expect("the mini graph should load from an odd path");
            assert_eq!(run.exit_code, 3);
            assert!(
                run.stderr.contains("[facet 'pattern']"),
                "the shared import must have resolved: {}",
                run.stderr
            );
        }
    }
}

/// A fresh per-process scratch directory for the real-validator tests.
#[cfg(target_os = "linux")]
fn fake_scratch(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "opencegeljaras-xmllint-{}-{}-{name}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}
