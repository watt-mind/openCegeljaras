//! The `dossier inspect` and `dossier preflight` file adapters.
//!
//! Both commands read one local container, report what it says about itself
//! and about the form it carries, and — for preflight — turn that into
//! deterministic findings and, given a verified bundle, one XSD conformance
//! result for the outer dossier. Nothing here verifies a signature or a
//! timestamp, decrypts anything, applies a zip transform, opens a nested
//! dossier, contacts an acknowledgement address, or acknowledges service.
//! Neither command establishes filing or acceptance by any authority.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use opencegeljaras_core::dossier::{
    self, Conformance, DocumentFacts, DossierStatus, Finding, FormOutcome, Inspection,
    XSD_NOT_REQUESTED,
};
use opencegeljaras_core::form::{Failure, FormStatus, Rejection};
use serde_json::{Map, Value, json};

use crate::form;
use crate::output::{
    CommandError, CommandResult, failure, output, quoted, success, write_diagnostic, write_json,
    write_text,
};
use crate::schemas::{self, BundleProblem, VerifyError};
use crate::validate::{self, Findings};
use crate::xmllint::{self, AdapterError};

/// The command name reported by `dossier inspect`.
const INSPECT_COMMAND: &str = "dossier inspect";
/// The command name reported by `dossier preflight`.
const PREFLIGHT_COMMAND: &str = "dossier preflight";

/// Read-only operations on one local company-court e-dossier.
#[derive(Debug, Subcommand)]
pub(crate) enum DossierCommand {
    /// Report what one local dossier says about itself. Not validation.
    Inspect(InspectArgs),
    /// Turn that report into deterministic findings and, with a bundle, XSD.
    Preflight(PreflightArgs),
}

/// Arguments of `opencegeljaras dossier inspect`.
#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    /// Path of the local dossier file to inspect. `-` is an ordinary name.
    path: PathBuf,
    /// Keep declared titles and company-identity values. Personal data.
    #[arg(long)]
    show_values: bool,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Arguments of `opencegeljaras dossier preflight`.
#[derive(Debug, Args)]
pub(crate) struct PreflightArgs {
    /// Path of the local dossier file to check. `-` is an ordinary name.
    path: PathBuf,
    /// Directory mirroring every pinned URL as `<host>/<path>`.
    #[arg(long, value_name = "DIR")]
    bundle: Option<PathBuf>,
    /// Read the manifest from this local file instead of the compiled one.
    #[arg(long, value_name = "FILE", requires = "bundle")]
    manifest: Option<PathBuf>,
    /// Path of the xmllint executable to run instead of the one on `PATH`.
    #[arg(long, value_name = "EXE", requires = "bundle")]
    xmllint: Option<PathBuf>,
    /// Keep declared titles, identity values, and validator values.
    #[arg(long)]
    show_values: bool,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Why `dossier preflight` could not report a passing dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreflightError {
    /// At least one deterministic finding failed.
    Failed,
    /// The bundle's dossier closure did not verify; nothing was spawned.
    BundleUnverified,
    /// A text node is larger than the validator accepts; nothing was spawned.
    PayloadExceedsValidatorLimit,
    /// No supported validator could be located, spawned, or version-checked.
    ValidatorUnavailable,
    /// A supported validator ran but produced no conformance result.
    ValidatorFailed,
}

impl PreflightError {
    /// The stable machine-readable code.
    fn code(self) -> &'static str {
        match self {
            Self::Failed => "preflight_failed",
            Self::BundleUnverified => "bundle_unverified",
            Self::PayloadExceedsValidatorLimit => "payload_exceeds_validator_limit",
            Self::ValidatorUnavailable => "validator_unavailable",
            Self::ValidatorFailed => "validator_failed",
        }
    }

    /// The fixed diagnostic text. It never names a path or a document value.
    fn message(self) -> &'static str {
        match self {
            Self::Failed => "at least one preflight finding failed",
            Self::BundleUnverified => {
                "the schema bundle's dossier closure does not match the pinned manifest"
            }
            Self::PayloadExceedsValidatorLimit => {
                "a text node exceeds the validator's 10 000 000-byte limit"
            }
            Self::ValidatorUnavailable => "no supported xmllint validator could be run",
            Self::ValidatorFailed => "the validator did not produce a conformance result",
        }
    }

    /// The exit code: 4 for a performed check that failed, 5 otherwise.
    fn exit(self) -> u8 {
        match self {
            Self::Failed => 4,
            _ => 5,
        }
    }
}

/// Everything the XSD step established, or why it established nothing.
struct Conformed {
    /// The conformance the core findings report.
    conformance: Conformance,
    /// The full `schemas verify` `data` object, or null before it ran.
    bundle: Value,
    /// The validator description, or null before one was located.
    validator: Value,
    /// The parsed validator output, or `None` when none ran.
    findings: Option<Findings>,
    /// The exit-5 condition the step ended in, when it ended in one.
    error: Option<PreflightError>,
}

impl Conformed {
    /// The step's own result when no bundle was named.
    fn not_requested() -> Self {
        Self {
            conformance: Conformance::NotPerformed(XSD_NOT_REQUESTED),
            bundle: Value::Null,
            validator: Value::Null,
            findings: None,
            error: None,
        }
    }

    /// The step's own result when it stopped at one exit-5 condition.
    fn stopped(self, error: PreflightError) -> Self {
        Self {
            conformance: Conformance::NotPerformed(error.code()),
            error: Some(error),
            ..self
        }
    }
}

/// Run one `dossier` subcommand.
pub(crate) fn run(command: &DossierCommand) -> CommandResult {
    match command {
        DossierCommand::Inspect(args) => inspect(args),
        DossierCommand::Preflight(args) => preflight(args),
    }
}

/// Read one local dossier through the dossier boundary.
///
/// The accepted bytes are returned with the inspection so that the validator
/// is handed the very bytes that were inspected, never a second read of the
/// operator's file.
fn read(path: &Path) -> Result<(Vec<u8>, Inspection), Failure> {
    let bytes = form::read_input(path, &dossier::limits())?;
    let inspection = dossier::inspect(&bytes)?;
    Ok((bytes, inspection))
}

/// The fixed diagnostic for one refusal under the dossier profile.
fn refusal_message(code: Rejection) -> &'static str {
    match code {
        Rejection::InputTooLarge => dossier::INPUT_TOO_LARGE,
        other => other.message(),
    }
}

/// Write the single fixed stderr line and choose the exit code.
fn reported(command: &str, code: &str, exit: u8) -> CommandError {
    write_diagnostic(&format!(
        "opencegeljaras: {command}: {code} (exit {exit})\n"
    ));
    CommandError::Reported(exit)
}

/// Inspect one local dossier and report what it says about itself.
fn inspect(args: &InspectArgs) -> CommandResult {
    let inspection = match read(&args.path) {
        Ok((_, inspection)) => inspection,
        Err(rejection) => {
            let code = rejection.code.code();
            if args.json {
                let data = null_data(&rejection, INSPECT_COMMAND);
                output(write_json(&failure(
                    INSPECT_COMMAND,
                    code,
                    refusal_message(rejection.code),
                    data,
                )))?;
            }
            return Err(reported(INSPECT_COMMAND, code, 1));
        }
    };

    let mut data = inspect_data(&inspection, args.show_values);
    data.insert("checks".to_owned(), checks("not_performed"));
    data.insert("notice".to_owned(), json!(dossier::NOTICE));
    let data = Value::Object(data);
    let Some(message) = inspection.dossier.status.message() else {
        return output(if args.json {
            write_json(&success(INSPECT_COMMAND, data))
        } else {
            write_text(&human_report(&inspection, args.show_values))
        });
    };
    let code = inspection.dossier.status.code();
    output(if args.json {
        write_json(&failure(INSPECT_COMMAND, code, message, data))
    } else {
        write_text(&human_report(&inspection, args.show_values))
    })?;
    Err(reported(INSPECT_COMMAND, code, 4))
}

/// The `checks` object, with the conformance the command established.
fn checks(conformance: &str) -> Value {
    json!({
        "well_formed": "passed",
        "xsd_conformance": conformance,
        "signature": "not_performed",
        "authority_acceptance": "not_performed",
    })
}

/// The `data` object of a refused input, with unestablished facts omitted.
fn null_data(rejection: &Failure, command: &str) -> Value {
    let input = form::rejection_data(rejection)["input"].clone();
    let mut object = Map::new();
    object.insert("input".to_owned(), input);
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
        object.insert((*key).to_owned(), Value::Null);
    }
    if command == PREFLIGHT_COMMAND {
        preflight_nulls(&mut object);
    }
    object.insert(
        "notice".to_owned(),
        json!(if command == PREFLIGHT_COMMAND {
            dossier::PREFLIGHT_NOTICE
        } else {
            dossier::NOTICE
        }),
    );
    Value::Object(object)
}

/// The preflight-only keys, all null until their step has run.
fn preflight_nulls(object: &mut Map<String, Value>) {
    for key in [
        "findings",
        "preflight",
        "bundle",
        "validator",
        "xsd_findings",
        "xsd_findings_truncated",
        "unparsed_lines",
    ] {
        object.insert((*key).to_owned(), Value::Null);
    }
}

/// The `data` fields of one completed dossier inspection.
fn inspect_data(inspection: &Inspection, show_values: bool) -> Map<String, Value> {
    let mut object = Map::new();
    let input = &inspection.input;
    object.insert(
        "input".to_owned(),
        json!({
            "bytes": input.bytes,
            "bom": input.bom,
            "xml_declaration": input.xml_declaration,
            "encoding": input.encoding,
        }),
    );
    object.insert(
        "root".to_owned(),
        json!({
            "local_name": inspection.root.local_name,
            "namespace": inspection.root.namespace,
        }),
    );
    object.insert("dossier".to_owned(), dossier_value(inspection, show_values));
    let listed = inspection.documents.as_ref().map(|documents| {
        Value::Array(
            documents
                .iter()
                .map(|document| document_value(document, show_values))
                .collect(),
        )
    });
    object.insert("documents".to_owned(), listed.unwrap_or(Value::Null));
    object.insert(
        "documents_truncated".to_owned(),
        json!(inspection.documents_truncated),
    );
    object.insert(
        "signatures_present".to_owned(),
        json!(inspection.signatures_present),
    );
    object.insert(
        "timestamps_present".to_owned(),
        json!(inspection.timestamps_present),
    );
    object.insert("values_redacted".to_owned(), json!(!show_values));
    object
}

/// The `dossier` object: only the status when the dossier is not recognised.
fn dossier_value(inspection: &Inspection, show_values: bool) -> Value {
    let facts = &inspection.dossier;
    if facts.status != DossierStatus::Recognized {
        return json!({ "status": facts.status.code() });
    }
    let identity = &facts.identity;
    let mut object = json!({
        "status": facts.status.code(),
        "profile": facts.profile,
        "title": facts.title,
        "title_known": facts.title.is_some(),
        "category": facts.category,
        "category_known": facts.category.is_some(),
        "receipt": facts.receipt,
        "creation_date": facts.creation_date,
        "ack_email_present": facts.ack_email_present,
        "location_present": facts.location_present,
        "identity": {
            "cegforma": identity.cegforma.is_some(),
            "cegjegyzekszam": identity.cegjegyzekszam.is_some(),
            "cegnev": identity.cegnev.is_some(),
            "cegszekhely": identity.cegszekhely.is_some(),
        },
        "documents_id_fixed": facts.documents_id_fixed,
    });
    if show_values {
        object["identity_values"] = json!({
            "cegforma": identity.cegforma,
            "cegjegyzekszam": identity.cegjegyzekszam,
            "cegnev": identity.cegnev,
            "cegszekhely": identity.cegszekhely,
        });
    }
    object
}

/// One listed document's own object.
fn document_value(document: &DocumentFacts, show_values: bool) -> Value {
    let mime = &document.mime;
    let mut object = json!({
        "index": document.index,
        "role": document.role,
        "role_known": document.role.is_some(),
        "category": document.category,
        "category_known": document.category.is_some(),
        "mime": {
            "type": mime.type_name,
            "subtype": mime.subtype,
            "extension": mime.extension,
            "charset_present": mime.charset_present,
        },
        "source_size": document.source_size,
        "transforms": document.transforms,
        "object_present": document.object_present,
        "nested_dossier": document.nested_dossier,
        "signatures": document.signatures,
        "timestamps": document.timestamps,
        "payload": document.payload.code(),
        "form": document.form.as_ref().map_or(Value::Null, form_value),
    });
    if show_values {
        object["title"] = json!(document.title);
    }
    object
}

/// The `form` object of one inspected payload.
fn form_value(outcome: &FormOutcome) -> Value {
    let facts = match outcome {
        Err(code) => return json!({ "status": "rejected", "code": code.code() }),
        Ok(facts) => facts,
    };
    if facts.status != FormStatus::Recognized {
        return json!({
            "status": facts.status.code(),
            "profile": facts.profile,
            "variant": Value::Null,
            "sections": Value::Null,
        });
    }
    let sections: Vec<Value> = facts
        .sections
        .iter()
        .flatten()
        .map(|section| json!({ "local_name": section.local_name, "count": section.count }))
        .collect();
    json!({
        "status": facts.status.code(),
        "profile": facts.profile,
        "variant": facts.variant.as_ref().map(|variant| json!({
            "id": variant.id,
            "local_name": variant.local_name,
            "namespace": variant.namespace,
        })),
        "sections": sections,
    })
}

/// The human dossier facts, up to and including the timestamp count.
fn human_facts(inspection: &Inspection, show_values: bool) -> String {
    let facts = &inspection.dossier;
    let namespace = inspection
        .root
        .namespace
        .as_deref()
        .map_or_else(|| String::from("none"), quoted);
    let mut report = format!(
        "root: {} in {namespace}\nencoding: {}\nbom: {}\nxml declaration: {}\n",
        quoted(&inspection.root.local_name),
        inspection.input.encoding,
        inspection.input.bom,
        inspection.input.xml_declaration
    );
    report.push_str(&format!("status: {}\n", facts.status.code()));
    report.push_str(&format!("profile: {}\n", facts.profile.unwrap_or("none")));
    report.push_str(&format!("title: {}\n", optional(facts.title)));
    report.push_str(&format!("category: {}\n", optional(facts.category)));
    report.push_str(&format!("receipt: {}\n", facts.receipt));
    report.push_str(&format!(
        "creation date: {}\n",
        optional(facts.creation_date.as_deref())
    ));
    report.push_str(&format!(
        "ack email: {}\nlocation: {}\n",
        presence(facts.ack_email_present),
        presence(facts.location_present)
    ));
    let identity = &facts.identity;
    report.push_str(&format!(
        "identity: cegforma {}, cegjegyzekszam {}, cegnev {}, cegszekhely {}\n",
        presence(identity.cegforma.is_some()),
        presence(identity.cegjegyzekszam.is_some()),
        presence(identity.cegnev.is_some()),
        presence(identity.cegszekhely.is_some())
    ));
    report.push_str(&format!(
        "documents id fixed: {}\n",
        facts.documents_id_fixed
    ));
    let documents = inspection.documents();
    let truncated = if inspection.documents_truncated {
        " (truncated)"
    } else {
        ""
    };
    report.push_str(&format!("documents: {}{truncated}\n", documents.len()));
    for document in documents {
        report.push_str(&human_document(document, show_values));
    }
    report.push_str(&format!(
        "signatures present: {}\ntimestamps present: {}\n",
        inspection.signatures_present, inspection.timestamps_present
    ));
    report
}

/// One document's indented block.
fn human_document(document: &DocumentFacts, show_values: bool) -> String {
    let mime = &document.mime;
    let transforms = if document.transforms.is_empty() {
        String::from("none")
    } else {
        document.transforms.join(",")
    };
    let mut block = format!("  [{}] role: {}\n", document.index, optional(document.role));
    block.push_str(&format!(
        "      category: {}\n",
        optional(document.category)
    ));
    block.push_str(&format!(
        "      mime: {}/{} {}\n",
        optional(mime.type_name.as_deref()),
        optional(mime.subtype.as_deref()),
        optional(mime.extension.as_deref())
    ));
    block.push_str(&format!("      transforms: {transforms}\n"));
    block.push_str(&format!("      payload: {}\n", document.payload.code()));
    block.push_str(&format!("      form: {}\n", human_form(document)));
    block.push_str(&format!(
        "      signatures: {} timestamps: {}\n",
        document.signatures, document.timestamps
    ));
    if show_values {
        block.push_str(&format!(
            "      title: {}\n",
            optional(document.title.as_deref())
        ));
    }
    block
}

/// One document's `form` line: the status and, when known, the variant.
fn human_form(document: &DocumentFacts) -> String {
    match &document.form {
        None => String::from("none"),
        Some(Err(code)) => format!("rejected {}", code.code()),
        Some(Ok(facts)) => match &facts.variant {
            None => facts.status.code().to_owned(),
            Some(variant) => format!("{} {}", facts.status.code(), variant.id),
        },
    }
}

/// A document-derived string as an escaped literal, or the word `none`.
fn optional(value: Option<&str>) -> String {
    value.map_or_else(|| String::from("none"), quoted)
}

/// The word used for a presence flag.
fn presence(present: bool) -> &'static str {
    if present { "present" } else { "absent" }
}

/// The full human `dossier inspect` report.
fn human_report(inspection: &Inspection, show_values: bool) -> String {
    let mut report = human_facts(inspection, show_values);
    report.push_str(form::BOUNDARY_LINES);
    report
}

/// The four boundary lines with the conformance the command established.
fn boundary_lines(conformance: &str) -> String {
    format!(
        "well-formed: passed\nxsd conformance: {}\nverification: not performed\nsubmission: not performed\n",
        conformance.replace('_', " ")
    )
}

/// Preflight one local dossier: findings and, with a bundle, XSD conformance.
fn preflight(args: &PreflightArgs) -> CommandResult {
    let (bytes, inspection) = match read(&args.path) {
        Ok(read) => read,
        Err(rejection) => {
            let code = rejection.code.code();
            if args.json {
                let data = null_data(&rejection, PREFLIGHT_COMMAND);
                output(write_json(&failure(
                    PREFLIGHT_COMMAND,
                    code,
                    refusal_message(rejection.code),
                    data,
                )))?;
            }
            return Err(reported(PREFLIGHT_COMMAND, code, 1));
        }
    };
    if inspection.dossier.status != DossierStatus::Recognized {
        return unrecognized(&inspection, args);
    }

    let conformed = conform(&inspection, &bytes, args)?;
    drop(bytes);
    emit(&inspection, &conformed, args)
}

/// Report a dossier that is not a recognised 2026 company dossier, at exit 4.
fn unrecognized(inspection: &Inspection, args: &PreflightArgs) -> CommandResult {
    let mut object = inspect_data(inspection, args.show_values);
    preflight_nulls(&mut object);
    object.insert("checks".to_owned(), checks("not_performed"));
    object.insert("notice".to_owned(), json!(dossier::PREFLIGHT_NOTICE));
    let code = inspection.dossier.status.code();
    let message = inspection
        .dossier
        .status
        .message()
        .unwrap_or("the document is not a recognised company dossier");
    output(if args.json {
        write_json(&failure(
            PREFLIGHT_COMMAND,
            code,
            message,
            Value::Object(object),
        ))
    } else {
        let mut report = human_facts(inspection, args.show_values);
        report.push_str(&boundary_lines("not_performed"));
        write_text(&report)
    })?;
    Err(reported(PREFLIGHT_COMMAND, code, 4))
}

/// Establish XSD conformance, when a bundle was named and nothing stops it.
fn conform(
    inspection: &Inspection,
    bytes: &[u8],
    args: &PreflightArgs,
) -> Result<Conformed, CommandError> {
    let Some(bundle) = &args.bundle else {
        return Ok(Conformed::not_requested());
    };
    let check = match schemas::check_bundle(bundle, args.manifest.as_deref()) {
        Ok(check) => check,
        Err(BundleProblem::Internal) => return Err(CommandError::Reported(3)),
        Err(BundleProblem::Refused(error)) => {
            return Err(refused(inspection, args, error));
        }
    };
    if check.roots < 2 {
        return Err(refused(inspection, args, VerifyError::ManifestInvalid));
    }
    let started = Conformed {
        bundle: check.data,
        ..Conformed::not_requested()
    };
    let Some(root) = check
        .dossier_root
        .filter(|_| check.dossier_closure_verified)
    else {
        return Ok(started.stopped(PreflightError::BundleUnverified));
    };
    if inspection.exceeds_validator_limit() {
        return Ok(started.stopped(PreflightError::PayloadExceedsValidatorLimit));
    }
    let Some(exe) = xmllint::locate(args.xmllint.as_deref()) else {
        let stopped = Conformed {
            validator: validate::validator_value(None, None),
            ..started
        };
        return Ok(stopped.stopped(PreflightError::ValidatorUnavailable));
    };

    Ok(validated(started, &exe, &root, bytes, args))
}

/// Run the validator once and turn its outcome into the step's result.
fn validated(
    started: Conformed,
    exe: &Path,
    root: &Path,
    bytes: &[u8],
    args: &PreflightArgs,
) -> Conformed {
    match xmllint::run(exe, root, bytes, validate::TIMEOUT) {
        Err(AdapterError::Unavailable { version }) => {
            let stopped = Conformed {
                validator: validate::validator_value(version, None),
                ..started
            };
            stopped.stopped(PreflightError::ValidatorUnavailable)
        }
        Err(AdapterError::Failed { version, exit_code }) => {
            let stopped = Conformed {
                validator: validate::validator_value(version, exit_code),
                ..started
            };
            stopped.stopped(PreflightError::ValidatorFailed)
        }
        Ok(run) => Conformed {
            conformance: if run.conforming {
                Conformance::Passed
            } else {
                Conformance::Failed
            },
            validator: validate::validator_value(Some(run.version), Some(run.exit_code)),
            findings: Some(validate::collect(&run.stderr, args.show_values)),
            ..started
        },
    }
}

/// Report a bundle or manifest the verification refused, at exit 1.
fn refused(inspection: &Inspection, args: &PreflightArgs, error: VerifyError) -> CommandError {
    let mut object = inspect_data(inspection, args.show_values);
    preflight_nulls(&mut object);
    object.insert("checks".to_owned(), checks("not_performed"));
    object.insert("notice".to_owned(), json!(dossier::PREFLIGHT_NOTICE));
    if args.json
        && write_json(&failure(
            PREFLIGHT_COMMAND,
            error.code(),
            error.message(),
            Value::Object(object),
        ))
        .is_err()
    {
        return CommandError::Output;
    }
    reported(PREFLIGHT_COMMAND, error.code(), 1)
}

/// Evaluate the findings, write the report, and choose the exit code.
fn emit(inspection: &Inspection, conformed: &Conformed, args: &PreflightArgs) -> CommandResult {
    let findings = dossier::findings(inspection, conformed.conformance);
    let verdict = dossier::verdict(&findings, inspection.dossier.receipt);
    let conformance = match conformed.conformance {
        Conformance::Passed => "passed",
        Conformance::Failed => "failed",
        Conformance::NotPerformed(_) => "not_performed",
    };
    // A required check that could not be performed is the stronger signal, as
    // it is for `form validate`: it is reported at exit 5 even when a
    // deterministic finding also failed, so that the operator learns the XSD
    // question was never answered rather than believing it was.
    let error = conformed.error.or_else(|| {
        findings
            .iter()
            .any(|finding| finding.status == dossier::FindingStatus::Failed)
            .then_some(PreflightError::Failed)
    });

    let mut object = inspect_data(inspection, args.show_values);
    object.insert("findings".to_owned(), findings_value(&findings));
    object.insert("preflight".to_owned(), json!({ "status": verdict.code() }));
    object.insert("bundle".to_owned(), conformed.bundle.clone());
    object.insert("validator".to_owned(), conformed.validator.clone());
    insert_xsd_findings(&mut object, conformed.findings.as_ref());
    object.insert("checks".to_owned(), checks(conformance));
    let notice = if inspection.dossier.receipt {
        format!("{}{}", dossier::PREFLIGHT_NOTICE, dossier::RECEIPT_NOTICE)
    } else {
        dossier::PREFLIGHT_NOTICE.to_owned()
    };
    object.insert("notice".to_owned(), json!(notice));

    let data = Value::Object(object);
    let written = match (args.json, error) {
        (true, None) => write_json(&success(PREFLIGHT_COMMAND, data)),
        (true, Some(error)) => write_json(&failure(
            PREFLIGHT_COMMAND,
            error.code(),
            error.message(),
            data,
        )),
        (false, _) => write_text(&preflight_report(
            inspection,
            conformed,
            &findings,
            verdict.code(),
            conformance,
            args.show_values,
        )),
    };
    output(written)?;
    match error {
        None => Ok(()),
        Some(error) => Err(reported(PREFLIGHT_COMMAND, error.code(), error.exit())),
    }
}

/// The `findings` array, in the contract's order.
fn findings_value(findings: &[Finding]) -> Value {
    Value::Array(
        findings
            .iter()
            .map(|finding| {
                json!({
                    "code": finding.code,
                    "status": finding.status.code(),
                    "source": finding.source.code(),
                    "detail": finding.detail,
                })
            })
            .collect(),
    )
}

/// The validator-finding keys, null when the validator did not run.
fn insert_xsd_findings(object: &mut Map<String, Value>, findings: Option<&Findings>) {
    let Some(findings) = findings else {
        for key in ["xsd_findings", "xsd_findings_truncated", "unparsed_lines"] {
            object.insert((*key).to_owned(), Value::Null);
        }
        return;
    };
    let listed: Vec<Value> = findings
        .items
        .iter()
        .map(|finding| {
            json!({
                "line": finding.line,
                "element": finding.element,
                "message": finding.message,
            })
        })
        .collect();
    object.insert("xsd_findings".to_owned(), Value::Array(listed));
    object.insert(
        "xsd_findings_truncated".to_owned(),
        json!(findings.truncated),
    );
    object.insert("unparsed_lines".to_owned(), json!(findings.unparsed));
}

/// The human `dossier preflight` report.
fn preflight_report(
    inspection: &Inspection,
    conformed: &Conformed,
    findings: &[Finding],
    verdict: &str,
    conformance: &str,
    show_values: bool,
) -> String {
    let mut report = human_facts(inspection, show_values);
    report.push_str("findings:\n");
    for finding in findings {
        report.push_str(&format!(
            "  {}: {} ({})\n",
            finding.code,
            finding.status.code(),
            finding.source.code()
        ));
    }
    if let Some(collected) = &conformed.findings {
        report.push_str("xsd findings:\n");
        for finding in &collected.items {
            report.push_str(&format!(
                "  line {} element {}: {}\n",
                finding.line,
                quoted(&finding.element),
                quoted(&finding.message)
            ));
        }
        if collected.truncated {
            report.push_str("  …\n");
        }
    }
    report.push_str(&format!("preflight: {verdict}\n"));
    report.push_str(&boundary_lines(conformance));
    report
}
