//! The `schemas list` and `schemas verify` adapters over a local mirror.
//!
//! `list` prints the manifest compiled into this build and fetches nothing.
//! `verify` reads only the files the manifest maps to inside one canonical
//! directory, compares their bytes against the pinned digests, and audits the
//! form graph's imports. It walks no directory, opens nothing the manifest
//! does not name, downloads nothing, and writes nothing. A verified bundle is
//! the operator's own copy checked against public digests: it is not a
//! redistribution licence, not XSD validation, and not acceptance.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use opencegeljaras_core::form::Limits;
use opencegeljaras_core::schema::{
    self, LIST_NOTICE, MAX_RESOURCE_BYTES, ResourceStatus, SchemaManifest,
    UNPINNED_MANIFEST_NOTICE, UnpinnedLocations, VERIFY_NOTICE,
};
use serde_json::{Value, json};

use crate::output::{
    CommandError, CommandResult, failure, output, quoted, success, write_diagnostic, write_json,
    write_text,
};

/// The command name reported by `schemas list`.
const LIST_COMMAND: &str = "schemas list";
/// The command name reported in `schemas verify` envelopes and diagnostics.
const VERIFY_COMMAND: &str = "schemas verify";
/// The number of digest characters shown in the human resource lines.
const DIGEST_PREVIEW: usize = 16;

/// Read-only operations on the pinned schema manifest and a local mirror.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemasCommand {
    /// Print the pinned schema manifest compiled into this build.
    List(ListArgs),
    /// Check a local mirror against the pinned digests and imports.
    Verify(VerifyArgs),
}

/// Arguments of `opencegeljaras schemas list`.
#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// Emit one stable JSON object on stdout.
    #[arg(long, conflicts_with = "urls")]
    json: bool,
    /// Print only the pinned URLs, one per line, in manifest order.
    #[arg(long)]
    urls: bool,
}

/// Arguments of `opencegeljaras schemas verify`.
#[derive(Debug, Args)]
pub(crate) struct VerifyArgs {
    /// Directory mirroring every pinned URL as `<host>/<path>`.
    #[arg(long, value_name = "DIR")]
    bundle: PathBuf,
    /// Read the manifest from this local file instead of the compiled one.
    #[arg(long, value_name = "FILE")]
    manifest: Option<PathBuf>,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Run one `schemas` subcommand.
pub(crate) fn run(command: &SchemasCommand) -> CommandResult {
    match command {
        SchemasCommand::List(args) => list(args),
        SchemasCommand::Verify(args) => verify(args),
    }
}

/// The manifest compiled into this build, or the internal failure exit.
fn compiled() -> Result<SchemaManifest, CommandError> {
    schema::compiled_manifest().map_err(|_| {
        write_diagnostic("opencegeljaras: internal schema manifest error (exit 3)\n");
        CommandError::Reported(3)
    })
}

/// Print the pinned manifest. Nothing is fetched, read, or verified.
fn list(args: &ListArgs) -> CommandResult {
    let manifest = compiled()?;
    if args.urls {
        let mut urls = String::new();
        for resource in &manifest.resources {
            urls.push_str(&resource.url);
            urls.push('\n');
        }
        return output(write_text(&urls));
    }

    if args.json {
        let data = json!({
            "snapshot_version": manifest.snapshot_version,
            "retrieved_at": manifest.retrieved_at,
            "purpose": manifest.purpose,
            "roots": manifest.roots,
            "count": manifest.resources.len(),
            "form_closure_count": manifest.form_closure().len(),
            "resources": manifest.resources,
            "notice": LIST_NOTICE,
        });
        return output(write_json(&success(LIST_COMMAND, data)));
    }
    output(write_text(&list_report(&manifest)))
}

/// The human `schemas list` report, one line per pinned resource.
fn list_report(manifest: &SchemaManifest) -> String {
    let mut report = format!("snapshot: {}\nroots:\n", manifest.retrieved_at);
    for root in &manifest.roots {
        report.push_str(&format!("  {root}\n"));
    }
    report.push_str(&format!("resources: {}\n", manifest.resources.len()));
    report.push_str(&format!(
        "form closure: {}\n",
        manifest.form_closure().len()
    ));
    for resource in &manifest.resources {
        let digest: String = resource.sha256.chars().take(DIGEST_PREVIEW).collect();
        let namespace = resource.target_namespace.as_deref().unwrap_or("none");
        report.push_str(&format!(
            "- {} {} bytes {digest} {namespace} {} dependencies\n",
            resource.url,
            resource.bytes,
            resource.dependencies.len()
        ));
    }
    report.push_str(LIST_NOTICE);
    report.push('\n');
    report
}

/// One resource that did not verify, named by its pinned URL only.
struct Problem {
    url: String,
    status: ResourceStatus,
    locations: Vec<String>,
    locations_truncated: bool,
}

/// What a whole bundle turned out to be.
struct Audit {
    verified_count: usize,
    problems: Vec<Problem>,
    closure_digests_complete: bool,
    closure_verified: bool,
    audit_failed: bool,
}

impl Audit {
    /// How many resources carry one status.
    fn count(&self, status: ResourceStatus) -> usize {
        self.problems
            .iter()
            .filter(|problem| problem.status == status)
            .count()
    }

    /// Whether every resource of both graphs verified.
    fn verified(&self) -> bool {
        self.problems.is_empty()
    }

    /// Whether the digest comparison ran to a clean result everywhere.
    fn digests_passed(&self) -> bool {
        self.count(ResourceStatus::Missing) == 0
            && self.count(ResourceStatus::Mismatch) == 0
            && self.count(ResourceStatus::Oversize) == 0
    }

    /// How the form closure's import audit turned out.
    ///
    /// A closure resource that never passed its digest was never parsed, so
    /// the audit is reported as unperformed rather than passed.
    fn import_audit(&self) -> &'static str {
        if self.audit_failed {
            "failed"
        } else if self.closure_digests_complete {
            "passed"
        } else {
            "not_performed"
        }
    }
}

/// Check a local mirror against the pinned manifest.
fn verify(args: &VerifyArgs) -> CommandResult {
    let unpinned = args.manifest.is_some();
    let Ok(bundle) = fs::canonicalize(&args.bundle) else {
        return Err(reject(VerifyError::BundleUnreadable, unpinned, args.json));
    };
    if !bundle.is_dir() {
        return Err(reject(VerifyError::BundleUnreadable, unpinned, args.json));
    }

    let manifest = match &args.manifest {
        None => compiled()?,
        Some(path) => match read_manifest(path) {
            Ok(manifest) => manifest,
            Err(error) => return Err(reject(error, unpinned, args.json)),
        },
    };

    let audit = audit_bundle(&bundle, &manifest);
    let source = if unpinned { "file" } else { "compiled" };
    let data = verify_data(&manifest, &audit, source, unpinned);
    if audit.verified() {
        return output(if args.json {
            write_json(&success(VERIFY_COMMAND, data))
        } else {
            write_text(&verify_report(&manifest, &audit, source))
        });
    }

    let code = VerifyError::BundleUnverified;
    output(if args.json {
        write_json(&failure(VERIFY_COMMAND, code.code(), code.message(), data))
    } else {
        write_text(&verify_report(&manifest, &audit, source))
    })?;
    Err(reported(code.code(), 4))
}

/// Why a verification could not produce a bundle result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyError {
    /// The bundle directory could not be canonicalised or is not a directory.
    BundleUnreadable,
    /// The named manifest file is not a readable regular file under the cap.
    ManifestUnreadable,
    /// The manifest text is not a valid manifest under the pinned rules.
    ManifestInvalid,
    /// The bundle was checked and did not match the manifest.
    BundleUnverified,
}

impl VerifyError {
    /// The stable machine-readable code.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::BundleUnreadable => "bundle_unreadable",
            Self::ManifestUnreadable => "manifest_unreadable",
            Self::ManifestInvalid => "manifest_invalid",
            Self::BundleUnverified => "bundle_unverified",
        }
    }

    /// The fixed diagnostic text. It never names a path or a file.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::BundleUnreadable => "the bundle directory could not be read",
            Self::ManifestUnreadable => "the manifest could not be read as a regular file",
            Self::ManifestInvalid => "the manifest does not satisfy the pinned schema rules",
            Self::BundleUnverified => "the bundle does not match the pinned manifest",
        }
    }
}

/// Report a failure that stopped verification before any bundle result.
fn reject(error: VerifyError, unpinned: bool, json: bool) -> CommandError {
    if json {
        let data = json!({
            "manifest": Value::Null,
            "bundle": Value::Null,
            "problems": Value::Null,
            "checks": {
                "digests": "not_performed",
                "import_audit": "not_performed",
                "xsd_conformance": "not_performed",
            },
            "notice": notice(unpinned),
        });
        if write_json(&failure(
            VERIFY_COMMAND,
            error.code(),
            error.message(),
            data,
        ))
        .is_err()
        {
            return CommandError::Output;
        }
    }
    reported(error.code(), 1)
}

/// Write the single fixed stderr line and choose the exit code.
fn reported(code: &str, exit: u8) -> CommandError {
    write_diagnostic(&format!(
        "opencegeljaras: {VERIFY_COMMAND}: {code} (exit {exit})\n"
    ));
    CommandError::Reported(exit)
}

/// The fixed notice, marking a manifest that is not the pinned one.
fn notice(unpinned: bool) -> String {
    if unpinned {
        format!("{VERIFY_NOTICE}{UNPINNED_MANIFEST_NOTICE}")
    } else {
        VERIFY_NOTICE.to_owned()
    }
}

/// Read one manifest file under the same cap as a bundle resource.
fn read_manifest(path: &Path) -> Result<SchemaManifest, VerifyError> {
    let bytes = read_bounded(path).ok_or(VerifyError::ManifestUnreadable)?;
    if bytes.len() as u64 > MAX_RESOURCE_BYTES {
        return Err(VerifyError::ManifestUnreadable);
    }
    let text = String::from_utf8(bytes).map_err(|_| VerifyError::ManifestInvalid)?;
    SchemaManifest::parse(&text).map_err(|_| VerifyError::ManifestInvalid)
}

/// Read at most the resource cap from a regular file, following symlinks.
///
/// The file type is taken from the opened handle rather than from the path, so
/// that nothing swapped in between a look and an open is read. Symlinks are
/// followed, which is what mirrors laid out by hand need; a bundle directory
/// therefore has to be one the caller trusts.
///
/// `None` covers every reason a file is not readable. A `Some` longer than the
/// cap is possible by one byte, which is how the caller detects an oversized
/// file without reading it all.
fn read_bounded(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_RESOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

/// Check every manifest resource against the bundle, in manifest order.
fn audit_bundle(bundle: &Path, manifest: &SchemaManifest) -> Audit {
    let limits = Limits::default();
    let closure = manifest.form_closure();
    let mut audit = Audit {
        verified_count: 0,
        problems: Vec::new(),
        closure_digests_complete: true,
        closure_verified: true,
        audit_failed: false,
    };

    for resource in &manifest.resources {
        let in_closure = closure.contains(resource.url.as_str());
        let (status, locations) = check_resource(bundle, manifest, resource, in_closure, &limits);
        if status == ResourceStatus::Verified {
            audit.verified_count += 1;
            continue;
        }
        if in_closure {
            audit.closure_verified = false;
            match status {
                ResourceStatus::Unparseable | ResourceStatus::UnpinnedImport => {
                    audit.audit_failed = true;
                }
                _ => audit.closure_digests_complete = false,
            }
        }
        audit.problems.push(Problem {
            url: resource.url.clone(),
            status,
            locations: locations.locations,
            locations_truncated: locations.truncated,
        });
    }
    audit
}

/// Check one mirrored resource, auditing imports only inside the form closure.
fn check_resource(
    bundle: &Path,
    manifest: &SchemaManifest,
    resource: &schema::SchemaResource,
    in_closure: bool,
    limits: &Limits,
) -> (ResourceStatus, UnpinnedLocations) {
    let clean = UnpinnedLocations::default;
    let Some(path) = resource_path(bundle, &resource.url) else {
        return (ResourceStatus::Missing, clean());
    };
    let Some(bytes) = read_bounded(&path) else {
        return (ResourceStatus::Missing, clean());
    };
    if bytes.len() as u64 > MAX_RESOURCE_BYTES {
        return (ResourceStatus::Oversize, clean());
    }
    if bytes.len() as u64 != resource.bytes || schema::sha256_hex(&bytes) != resource.sha256 {
        return (ResourceStatus::Mismatch, clean());
    }
    if !in_closure {
        return (ResourceStatus::Verified, clean());
    }

    let Ok(edges) = schema::edges(&bytes, &resource.url, limits) else {
        return (ResourceStatus::Unparseable, clean());
    };
    let unpinned = schema::unpinned_locations(manifest, &edges);
    if unpinned.locations.is_empty() {
        (ResourceStatus::Verified, clean())
    } else {
        (ResourceStatus::UnpinnedImport, unpinned)
    }
}

/// Map one pinned URL to its file inside the bundle directory.
///
/// The manifest path rules have already refused every `.` and `..` segment and
/// every separator, so the joined path cannot leave the bundle.
fn resource_path(bundle: &Path, url: &str) -> Option<PathBuf> {
    let mapped = schema::bundle_path(url)?;
    let mut path = bundle.join(mapped.host);
    for segment in mapped.segments {
        path.push(segment);
    }
    Some(path)
}

/// The `data` object of a completed verification.
fn verify_data(manifest: &SchemaManifest, audit: &Audit, source: &str, unpinned: bool) -> Value {
    let problems: Vec<Value> = audit
        .problems
        .iter()
        .map(|problem| {
            json!({
                "url": problem.url,
                "status": problem.status.code(),
                "locations": problem.locations,
                "locations_truncated": problem.locations_truncated,
            })
        })
        .collect();
    json!({
        "manifest": {
            "source": source,
            "retrieved_at": manifest.retrieved_at,
            "count": manifest.resources.len(),
            "form_closure_count": manifest.form_closure().len(),
        },
        "bundle": {
            "verified": audit.verified(),
            "form_closure_verified": audit.closure_verified,
            "verified_count": audit.verified_count,
            "missing": audit.count(ResourceStatus::Missing),
            "mismatch": audit.count(ResourceStatus::Mismatch),
            "oversize": audit.count(ResourceStatus::Oversize),
            "unparseable": audit.count(ResourceStatus::Unparseable),
            "unpinned_import": audit.count(ResourceStatus::UnpinnedImport),
        },
        "problems": problems,
        "checks": {
            "digests": if audit.digests_passed() { "passed" } else { "failed" },
            "import_audit": audit.import_audit(),
            "xsd_conformance": "not_performed",
        },
        "notice": notice(unpinned),
    })
}

/// The human `schemas verify` report. The bundle directory never appears.
fn verify_report(manifest: &SchemaManifest, audit: &Audit, source: &str) -> String {
    let mut report = format!(
        "manifest: {source}, retrieved {}, {} resources, {} in the form closure\n",
        manifest.retrieved_at,
        manifest.resources.len(),
        manifest.form_closure().len()
    );
    report.push_str(&format!("bundle: {}\n", verdict(audit.verified())));
    report.push_str(&format!(
        "form closure: {}\n",
        verdict(audit.closure_verified)
    ));
    report.push_str(&format!(
        "counts: verified {}, missing {}, mismatch {}, oversize {}, unparseable {}, unpinned import {}\n",
        audit.verified_count,
        audit.count(ResourceStatus::Missing),
        audit.count(ResourceStatus::Mismatch),
        audit.count(ResourceStatus::Oversize),
        audit.count(ResourceStatus::Unparseable),
        audit.count(ResourceStatus::UnpinnedImport),
    ));
    if audit.problems.is_empty() {
        report.push_str("problems: none\n");
    } else {
        report.push_str("problems:\n");
        for problem in &audit.problems {
            report.push_str(&format!("  {} {}", problem.status.code(), problem.url));
            for location in &problem.locations {
                report.push(' ');
                report.push_str(&quoted(location));
            }
            if problem.locations_truncated {
                report.push_str(" …");
            }
            report.push('\n');
        }
    }
    report.push_str(&format!(
        "digests: {}\n",
        if audit.digests_passed() {
            "passed"
        } else {
            "failed"
        }
    ));
    report.push_str(&format!(
        "import audit: {}\n",
        audit.import_audit().replace('_', " ")
    ));
    report.push_str("xsd conformance: not performed\n");
    report.push_str("verification: not performed\n");
    report.push_str("submission: not performed\n");
    report
}

/// The word used for a verified or unverified graph.
fn verdict(verified: bool) -> &'static str {
    if verified { "verified" } else { "unverified" }
}

/// A bundle verification performed on behalf of another command.
///
/// `form validate` needs the same digest comparison and import audit before it
/// spawns anything, and it reports the identical `data` object, so the check
/// is run here rather than reimplemented.
pub(crate) struct BundleCheck {
    /// The full `schemas verify` `data` object.
    pub(crate) data: Value,
    /// Whether every form-closure resource verified.
    pub(crate) form_closure_verified: bool,
    /// The canonical path of the form root inside the bundle, when readable.
    pub(crate) form_root: Option<PathBuf>,
}

/// Why a bundle check produced no result at all.
pub(crate) enum BundleProblem {
    /// The bundle directory or the named manifest file was refused.
    Refused(VerifyError),
    /// The compiled manifest is unusable; the diagnostic is already written.
    Internal,
}

/// Verify one bundle for another command and report the form root's path.
pub(crate) fn check_bundle(
    bundle: &Path,
    manifest_path: Option<&Path>,
) -> Result<BundleCheck, BundleProblem> {
    let unpinned = manifest_path.is_some();
    let bundle = fs::canonicalize(bundle)
        .ok()
        .filter(|bundle| bundle.is_dir())
        .ok_or(BundleProblem::Refused(VerifyError::BundleUnreadable))?;
    let manifest = match manifest_path {
        None => compiled().map_err(|_| BundleProblem::Internal)?,
        Some(path) => read_manifest(path).map_err(BundleProblem::Refused)?,
    };

    let audit = audit_bundle(&bundle, &manifest);
    let source = if unpinned { "file" } else { "compiled" };
    let form_root = manifest
        .roots
        .first()
        .and_then(|root| resource_path(&bundle, root))
        .and_then(|path| fs::canonicalize(path).ok());
    Ok(BundleCheck {
        data: verify_data(&manifest, &audit, source, unpinned),
        form_closure_verified: audit.closure_verified,
        form_root,
    })
}
