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
    self, Conformance, DossierStatus, Inspection, XSD_NOT_REQUESTED,
};
use opencegeljaras_core::form::{Failure, Rejection};
use serde_json::{Value, json};

use crate::form;
use crate::output::{
    CommandError, CommandResult, failure, output, success, write_diagnostic, write_json, write_text,
};
use crate::schemas::{self, BundleProblem, VerifyError};
use crate::validate::{self, Findings};
use crate::xmllint::{self, AdapterError};

mod report;

use report::{
    boundary_lines, checks, findings_value, human_facts, human_report, insert_xsd_findings,
    inspect_data, null_data, preflight_nulls, preflight_report,
};

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
