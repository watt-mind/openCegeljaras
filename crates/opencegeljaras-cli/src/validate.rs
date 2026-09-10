//! The `form validate` command: XSD conformance through the xmllint adapter.
//!
//! Conformance is established in a fixed order: the `form inspect` boundary
//! accepts the bytes, the schema bundle is digest-verified, a supported
//! validator is located, and only then is one child process spawned over a
//! private copy of the accepted bytes. Nothing here reports the operator's
//! path, the bundle directory, or an unredacted document value unless the
//! operator asked for values. A conforming result means the document matches
//! the pinned 2026 graph as one named libxml2 version interprets it: it is
//! not the technical conditions, not a signature check, and not acceptance by
//! any authority.

use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use opencegeljaras_core::form::{self as core_form, Failure, FormStatus, Inspection, Limits};
use serde_json::{Map, Value, json};

use crate::form;
use crate::output::{
    CommandError, CommandResult, failure, output, quoted, success, write_diagnostic, write_json,
    write_text,
};
use crate::schemas::{self, BundleProblem};
use crate::xmllint::{self, AdapterError, DOCUMENT_NAME, VALIDATOR_NAME};

/// The command name reported in envelopes and diagnostics.
const COMMAND: &str = "form validate";
/// The fixed notice carried by every `form validate` envelope.
const NOTICE: &str = "XSD conformance of one form against the pinned 2026 graph as interpreted by the named libxml2 version; not the technical conditions, not a signature check, not acceptance";
/// How long the CLI lets one validator run take.
const TIMEOUT: Duration = Duration::from_secs(60);
/// Findings kept before the list is reported as truncated.
const MAX_FINDINGS: usize = 256;
/// Unicode scalar values kept from one validator message.
const MESSAGE_SCALARS: usize = 512;
/// Unicode scalar values kept from one reported element name.
const ELEMENT_SCALARS: usize = 128;
/// The text that replaces a message that cannot be redacted safely.
const REDACTED_MESSAGE: &str = "message redacted";
/// The text that replaces one single-quoted span carrying a document value.
const REDACTED_SPAN: &str = "'…'";
/// The text introducing a facet name in a validator message.
const FACET_PREFIX: &str = "[facet ";
/// The text introducing a document value in a validator message.
const VALUE_PREFIX: &str = "The value ";
/// The other position libxml2 prints a document value in: after a quoted name.
///
/// `Element '{ns}Kod': '17' is not a valid value of the atomic type
/// 'xs:integer'.` and its `attribute 'a': '17' …` form both put the value
/// straight after an apostrophe, a colon, and a space.
const VALUE_INFIX: &str = "': ";
/// The separator between the element name and the message on a stderr line.
const ERROR_MARKER: &str = ": Schemas validity error : ";
/// The separator between the line number and the element name.
const ELEMENT_MARKER: &str = " element ";
/// libxml2's fixed content-type keywords, which are never document values.
const CONTENT_TYPES: &[&str] = &["element-only", "mixed", "empty", "simple"];
/// The XSD built-in type names, always printed with the `xs:` prefix.
const BUILTIN_TYPES: &[&str] = &[
    "xs:anyType",
    "xs:anySimpleType",
    "xs:string",
    "xs:boolean",
    "xs:decimal",
    "xs:float",
    "xs:double",
    "xs:duration",
    "xs:dateTime",
    "xs:time",
    "xs:date",
    "xs:gYearMonth",
    "xs:gYear",
    "xs:gMonthDay",
    "xs:gDay",
    "xs:gMonth",
    "xs:hexBinary",
    "xs:base64Binary",
    "xs:anyURI",
    "xs:QName",
    "xs:NOTATION",
    "xs:normalizedString",
    "xs:token",
    "xs:language",
    "xs:NMTOKEN",
    "xs:NMTOKENS",
    "xs:Name",
    "xs:NCName",
    "xs:ID",
    "xs:IDREF",
    "xs:IDREFS",
    "xs:ENTITY",
    "xs:ENTITIES",
    "xs:integer",
    "xs:nonPositiveInteger",
    "xs:negativeInteger",
    "xs:long",
    "xs:int",
    "xs:short",
    "xs:byte",
    "xs:nonNegativeInteger",
    "xs:unsignedLong",
    "xs:unsignedInt",
    "xs:unsignedShort",
    "xs:unsignedByte",
    "xs:positiveInteger",
];
/// The XSD facet names whose quoted spans are never document values.
const FACET_NAMES: &[&str] = &[
    "length",
    "minLength",
    "maxLength",
    "pattern",
    "enumeration",
    "whiteSpace",
    "maxInclusive",
    "maxExclusive",
    "minInclusive",
    "minExclusive",
    "totalDigits",
    "fractionDigits",
    "assertion",
    "explicitTimezone",
];

/// Arguments of `opencegeljaras form validate`.
#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Path of the local form file to validate. `-` is an ordinary name.
    path: PathBuf,
    /// Directory mirroring every pinned URL as `<host>/<path>`.
    #[arg(long, value_name = "DIR")]
    bundle: PathBuf,
    /// Read the manifest from this local file instead of the compiled one.
    #[arg(long, value_name = "FILE")]
    manifest: Option<PathBuf>,
    /// Path of the xmllint executable to run instead of the one on `PATH`.
    #[arg(long, value_name = "EXE")]
    xmllint: Option<PathBuf>,
    /// Keep document values in findings. This can expose personal data.
    #[arg(long)]
    show_values: bool,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// One reported schema violation, already cut and redacted.
struct Finding {
    /// Line of the temporary document copy the validator named.
    line: u64,
    /// The element name the validator named, cut to a fixed length.
    element: String,
    /// The validator's message, redacted unless values were requested.
    message: String,
}

/// Everything parsed out of one validator run's standard error.
struct Findings {
    /// The kept findings, in validator order.
    items: Vec<Finding>,
    /// Whether more findings were reported than are kept.
    truncated: bool,
    /// How many stderr lines did not match the expected format.
    unparsed: usize,
    /// Whether document values were removed from the messages.
    redacted: bool,
}

/// Why `form validate` did not report a conforming document.
#[derive(Debug, Clone, Copy)]
enum ValidateError {
    /// The document is a recognised form and does not conform.
    Nonconformant,
    /// The bundle's form closure did not verify; nothing was spawned.
    BundleUnverified,
    /// No supported validator could be located, spawned, or version-checked.
    ValidatorUnavailable,
    /// A supported validator ran but produced no conformance result.
    ValidatorFailed,
}

impl ValidateError {
    /// The stable machine-readable code.
    fn code(self) -> &'static str {
        match self {
            Self::Nonconformant => "nonconformant",
            Self::BundleUnverified => "bundle_unverified",
            Self::ValidatorUnavailable => "validator_unavailable",
            Self::ValidatorFailed => "validator_failed",
        }
    }

    /// The fixed diagnostic text. It never names a path or a document value.
    fn message(self) -> &'static str {
        match self {
            Self::Nonconformant => "the form does not conform to the pinned schema graph",
            Self::BundleUnverified => {
                "the schema bundle's form closure does not match the pinned manifest"
            }
            Self::ValidatorUnavailable => "no supported xmllint validator could be run",
            Self::ValidatorFailed => "the validator did not produce a conformance result",
        }
    }

    /// The exit code: 4 for a performed check that failed, 5 otherwise.
    fn exit(self) -> u8 {
        match self {
            Self::Nonconformant => 4,
            _ => 5,
        }
    }
}

/// Everything established so far, in the order the contract establishes it.
struct Outcome<'a> {
    /// The completed inspection, absent when the input was refused.
    inspection: Option<&'a Inspection>,
    /// The `input`, `root`, `form`, and `structure` fields of `form inspect`.
    facts: Map<String, Value>,
    /// The full `schemas verify` `data` object, or null before step 2.
    bundle: Value,
    /// The validator description, or null before step 3.
    validator: Value,
    /// The parsed validator output, or `None` before step 6.
    findings: Option<Findings>,
    /// The `xsd_conformance` check result.
    conformance: &'static str,
}

impl<'a> Outcome<'a> {
    /// An outcome carrying only what the `form inspect` boundary established.
    fn inspected(inspection: Option<&'a Inspection>, facts: Map<String, Value>) -> Self {
        Self {
            inspection,
            facts,
            bundle: Value::Null,
            validator: Value::Null,
            findings: None,
            conformance: "not_performed",
        }
    }
}

/// Cut an untrusted string to a fixed number of Unicode scalar values.
fn cut(text: &str, limit: usize) -> String {
    let mut kept: String = text.chars().take(limit).collect();
    if text.chars().nth(limit).is_some() {
        kept.push('…');
    }
    kept
}

/// Copy the four `form inspect` fact fields out of an inspect `data` object.
fn facts_of(source: &Value) -> Map<String, Value> {
    let mut facts = Map::new();
    for key in ["input", "root", "form", "structure"] {
        facts.insert(
            key.to_owned(),
            source.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    facts
}

/// Validate one local file against one verified schema bundle.
pub(crate) fn validate(args: &ValidateArgs) -> CommandResult {
    let limits = Limits::default();
    let bytes = match form::read_input(&args.path, &limits) {
        Ok(bytes) => bytes,
        Err(rejection) => return refuse(&rejection, args),
    };
    let inspection = match core_form::inspect(&bytes, &limits) {
        Ok(inspection) => inspection,
        Err(rejection) => return refuse(&rejection, args),
    };

    let facts = facts_of(&form::inspection_data(&inspection));
    let mut outcome = Outcome::inspected(Some(&inspection), facts);
    if inspection.form.status != FormStatus::Recognized {
        let code = inspection.form.status.code();
        let message = inspection
            .form
            .status
            .message()
            .unwrap_or("the document is not a recognised company form");
        return emit(&outcome, args, Some((code, message, 4)));
    }

    conform(&mut outcome, &bytes, args)
}

/// Steps 2 to 7: verify the bundle, locate the validator, and run it once.
fn conform(outcome: &mut Outcome<'_>, bytes: &[u8], args: &ValidateArgs) -> CommandResult {
    let check = match schemas::check_bundle(&args.bundle, args.manifest.as_deref()) {
        Ok(check) => check,
        Err(BundleProblem::Internal) => return Err(CommandError::Reported(3)),
        Err(BundleProblem::Refused(error)) => {
            return emit(outcome, args, Some((error.code(), error.message(), 1)));
        }
    };
    outcome.bundle = check.data;
    let Some(root) = check.form_root.filter(|_| check.form_closure_verified) else {
        return fail(outcome, args, ValidateError::BundleUnverified);
    };

    let Some(exe) = xmllint::locate(args.xmllint.as_deref()) else {
        outcome.validator = validator_value(None, None);
        return fail(outcome, args, ValidateError::ValidatorUnavailable);
    };

    match xmllint::run(&exe, &root, bytes, TIMEOUT) {
        Err(AdapterError::Unavailable { version }) => {
            outcome.validator = validator_value(version, None);
            fail(outcome, args, ValidateError::ValidatorUnavailable)
        }
        Err(AdapterError::Failed { version, exit_code }) => {
            outcome.validator = validator_value(version, exit_code);
            fail(outcome, args, ValidateError::ValidatorFailed)
        }
        Ok(run) => {
            outcome.validator = validator_value(Some(run.version), Some(run.exit_code));
            outcome.findings = Some(collect(&run.stderr, args.show_values));
            if run.conforming {
                outcome.conformance = "passed";
                emit(outcome, args, None)
            } else {
                outcome.conformance = "failed";
                fail(outcome, args, ValidateError::Nonconformant)
            }
        }
    }
}

/// Report one failure that stopped or answered the conformance question.
fn fail(outcome: &Outcome<'_>, args: &ValidateArgs, error: ValidateError) -> CommandResult {
    emit(
        outcome,
        args,
        Some((error.code(), error.message(), error.exit())),
    )
}

/// The `validator` field: the name plus whatever was established about it.
fn validator_value(version: Option<u32>, exit_code: Option<i32>) -> Value {
    json!({ "name": VALIDATOR_NAME, "version": version, "exit_code": exit_code })
}

/// Report a refused input exactly as `form inspect` would, at exit 1.
fn refuse(rejection: &Failure, args: &ValidateArgs) -> CommandResult {
    let facts = facts_of(&form::rejection_data(rejection));
    let outcome = Outcome::inspected(None, facts);
    emit(
        &outcome,
        args,
        Some((rejection.code.code(), rejection.code.message(), 1)),
    )
}

/// Write the envelope or the report, then the one fixed diagnostic line.
fn emit(
    outcome: &Outcome<'_>,
    args: &ValidateArgs,
    error: Option<(&str, &str, u8)>,
) -> CommandResult {
    let data = data(outcome);
    let written = match (args.json, error) {
        (true, None) => write_json(&success(COMMAND, data)),
        (true, Some((code, message, _))) => write_json(&failure(COMMAND, code, message, data)),
        (false, _) => match outcome.inspection {
            None => Ok(()),
            Some(inspection) => write_text(&report(outcome, inspection)),
        },
    };
    output(written)?;
    let Some((code, _, exit)) = error else {
        return Ok(());
    };
    write_diagnostic(&format!(
        "opencegeljaras: {COMMAND}: {code} (exit {exit})\n"
    ));
    Err(CommandError::Reported(exit))
}

/// The `data` object, with every field the contract fixes for this command.
fn data(outcome: &Outcome<'_>) -> Value {
    let mut object = outcome.facts.clone();
    object.insert("bundle".to_owned(), outcome.bundle.clone());
    object.insert("validator".to_owned(), outcome.validator.clone());
    match &outcome.findings {
        None => {
            for key in [
                "findings",
                "findings_truncated",
                "unparsed_lines",
                "values_redacted",
            ] {
                object.insert(key.to_owned(), Value::Null);
            }
        }
        Some(findings) => {
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
            object.insert("findings".to_owned(), Value::Array(listed));
            object.insert("findings_truncated".to_owned(), json!(findings.truncated));
            object.insert("unparsed_lines".to_owned(), json!(findings.unparsed));
            object.insert("values_redacted".to_owned(), json!(findings.redacted));
        }
    }
    let checks = if outcome.inspection.is_none() {
        Value::Null
    } else {
        json!({
            "well_formed": "passed",
            "xsd_conformance": outcome.conformance,
            "signature": "not_performed",
            "authority_acceptance": "not_performed",
        })
    };
    object.insert("checks".to_owned(), checks);
    object.insert("notice".to_owned(), json!(NOTICE));
    Value::Object(object)
}

/// The human report: the inspect facts, the new lines, then the boundaries.
fn report(outcome: &Outcome<'_>, inspection: &Inspection) -> String {
    let mut report = form::human_facts(inspection);
    report.push_str(&format!("bundle: {}\n", bundle_verdict(&outcome.bundle)));
    report.push_str(&format!(
        "validator: {}\n",
        validator_verdict(&outcome.validator)
    ));
    report.push_str(&format!(
        "xsd conformance: {}\n",
        outcome.conformance.replace('_', " ")
    ));
    if let Some(findings) = &outcome.findings {
        for finding in &findings.items {
            report.push_str(&format!(
                "  line {} element {}: {}\n",
                finding.line,
                quoted(&finding.element),
                quoted(&finding.message)
            ));
        }
        if findings.truncated {
            report.push_str("  …\n");
        }
    }
    report.push_str("verification: not performed\nsubmission: not performed\n");
    report
}

/// How a bundle verification turned out, in one word or phrase.
fn bundle_verdict(bundle: &Value) -> &'static str {
    let results = &bundle["bundle"];
    match results.get("verified").and_then(Value::as_bool) {
        None => "not performed",
        Some(true) => "verified",
        Some(false) => {
            if results["form_closure_verified"] == Value::Bool(true) {
                "form closure verified"
            } else {
                "unverified"
            }
        }
    }
}

/// The validator name and version, or `none` when none was established.
fn validator_verdict(validator: &Value) -> String {
    match validator.get("version").and_then(Value::as_u64) {
        None => String::from("none"),
        Some(version) => format!("{VALIDATOR_NAME} {version}"),
    }
}

/// Parse one validator run's standard error into findings.
///
/// The two fixed summary lines the validator ends with, one for each of the
/// exit codes this adapter accepts, are expected output rather than something
/// that could not be interpreted, so neither is counted as unparsed.
fn collect(stderr: &str, show_values: bool) -> Findings {
    let expected = [
        format!("{DOCUMENT_NAME} fails to validate"),
        format!("{DOCUMENT_NAME} validates"),
    ];
    let mut findings = Findings {
        items: Vec::new(),
        truncated: false,
        unparsed: 0,
        redacted: !show_values,
    };
    for raw in stderr.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() || expected.iter().any(|known| known == line) {
            continue;
        }
        let Some((number, element, message)) = parse_line(line) else {
            findings.unparsed += 1;
            continue;
        };
        if findings.items.len() >= MAX_FINDINGS {
            findings.truncated = true;
            continue;
        }
        let message = if show_values {
            message.to_owned()
        } else {
            redact(message)
        };
        findings.items.push(Finding {
            line: number,
            element: cut(element, ELEMENT_SCALARS),
            message: cut(&message, MESSAGE_SCALARS),
        });
    }
    findings
}

/// Match one `document.xml:LINE: element NAME: Schemas validity error : TEXT`.
///
/// The [`xmllint`] adapter shares this matcher so that the
/// incomplete-graph markers are never looked for inside a line that quotes
/// the operator's own document.
pub(crate) fn parse_line(line: &str) -> Option<(u64, &str, &str)> {
    let rest = line.strip_prefix(DOCUMENT_NAME)?.strip_prefix(':')?;
    let (number, rest) = rest.split_once(':')?;
    let number = number.parse::<u64>().ok()?;
    let rest = rest.strip_prefix(ELEMENT_MARKER)?;
    let marker = rest.find(ERROR_MARKER)?;
    let element = &rest[..marker];
    if element.is_empty() {
        return None;
    }
    Some((number, element, rest[marker + ERROR_MARKER.len()..].trim()))
}

/// How one single-quoted span in a validator message is treated.
enum Span {
    /// Structural text that carries no document value.
    Keep,
    /// A document value, replaced by a fixed placeholder.
    Redact,
    /// Not classifiable; the whole message is replaced instead.
    Ambiguous,
}

/// Remove every document value from one validator message.
///
/// Only two kinds of quoted span survive: a qualified name in Clark notation
/// and the facet name that follows `[facet `. An odd number of apostrophes, or
/// one span that could be either structure or a value, replaces the whole
/// message rather than risk printing personal data.
fn redact(message: &str) -> String {
    let quotes: Vec<usize> = message
        .char_indices()
        .filter(|(_, character)| *character == '\'')
        .map(|(index, _)| index)
        .collect();
    if !quotes.len().is_multiple_of(2) {
        return REDACTED_MESSAGE.to_owned();
    }

    let mut redacted = String::with_capacity(message.len());
    let mut cursor = 0;
    for pair in quotes.as_chunks::<2>().0 {
        let (open, close) = (pair[0], pair[1]);
        redacted.push_str(&message[cursor..open]);
        match classify(&message[..open], &message[open + 1..close]) {
            Span::Keep => redacted.push_str(&message[open..=close]),
            Span::Redact => redacted.push_str(REDACTED_SPAN),
            Span::Ambiguous => return REDACTED_MESSAGE.to_owned(),
        }
        cursor = close + 1;
    }
    redacted.push_str(&message[cursor..]);
    redacted
}

/// Decide what one quoted span is, from the text before it and its content.
///
/// Structural text is kept only where libxml2 never prints a document value.
/// A value can be shaped like anything, a qualified name and a type name
/// included, so a structural-looking span in a value position is refused as
/// ambiguous rather than printed.
fn classify(before: &str, content: &str) -> Span {
    if before.ends_with(FACET_PREFIX) {
        return if FACET_NAMES.contains(&content) {
            Span::Keep
        } else {
            Span::Ambiguous
        };
    }
    if !is_structural(content) {
        return Span::Redact;
    }
    if before.ends_with(VALUE_PREFIX) || before.ends_with(VALUE_INFIX) {
        Span::Ambiguous
    } else {
        Span::Keep
    }
}

/// Whether one span names structure rather than describing a document value.
fn is_structural(content: &str) -> bool {
    is_qualified_name(content)
        || CONTENT_TYPES.contains(&content)
        || BUILTIN_TYPES.contains(&content)
}

/// Whether one span is a qualified name in Clark notation.
fn is_qualified_name(content: &str) -> bool {
    let Some(rest) = content.strip_prefix('{') else {
        return false;
    };
    let Some((namespace, local)) = rest.split_once('}') else {
        return false;
    };
    !namespace.contains('{') && !local.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests;
