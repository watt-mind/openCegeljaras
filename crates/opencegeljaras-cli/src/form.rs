//! The `form inspect` file adapter, envelopes, and human report.
//!
//! Nothing here reports a path, a byte of the file, an element name outside
//! the two document-derived string fields, or a parser position. Recognising
//! a form establishes neither schema conformance nor signature validity,
//! filing, or acceptance.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Args;
use opencegeljaras_core::form::{
    self, Failure, Inspection, Limits, NOTICE, PartialInput, Rejection,
};
use serde_json::{Map, Value, json};

use crate::output::{CommandError, CommandResult, failure, output, success, write_diagnostic};
use crate::output::{quoted, write_json, write_text};

/// The command name reported in envelopes and diagnostics.
const COMMAND: &str = "form inspect";

/// Arguments of `opencegeljaras form inspect`.
#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    /// Path of the local form file to inspect. `-` is an ordinary name.
    path: PathBuf,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Inspect one local file and report what it is, without judging it.
pub(crate) fn inspect(args: &InspectArgs) -> CommandResult {
    let limits = Limits::default();
    match read_input(&args.path, &limits).and_then(|bytes| form::inspect(&bytes, &limits)) {
        Ok(inspection) => report(&inspection, args.json),
        Err(rejection) => reject(&rejection, args.json),
    }
}

/// Read the named file under the byte limit, following symlinks.
///
/// The type and size are taken from the opened handle rather than from the
/// path, so that nothing swapped in between a look and an open is read.
///
/// The failure reason is never distinguished: an absent file, a directory, a
/// permission error, and a mid-read error all produce `input_unreadable`.
pub(crate) fn read_input(path: &Path, limits: &Limits) -> Result<Vec<u8>, Failure> {
    let unreadable = || Failure {
        code: Rejection::InputUnreadable,
        input: None,
    };
    let too_large = |bytes: u64| Failure {
        code: Rejection::InputTooLarge,
        input: Some(PartialInput::sized(bytes)),
    };

    let file = fs::File::open(path).map_err(|_| unreadable())?;
    let metadata = file.metadata().map_err(|_| unreadable())?;
    if !metadata.is_file() {
        return Err(unreadable());
    }
    if metadata.len() > limits.input_bytes {
        return Err(too_large(metadata.len()));
    }

    let mut bytes = Vec::new();
    file.take(limits.input_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable())?;
    let read = bytes.len() as u64;
    if read > limits.input_bytes {
        return Err(too_large(read));
    }
    Ok(bytes)
}

/// Report a completed inspection: exit 0 when recognised, otherwise exit 4.
fn report(inspection: &Inspection, json: bool) -> CommandResult {
    let data = inspection_data(inspection);
    let Some(message) = inspection.form.status.message() else {
        return output(if json {
            write_json(&success(COMMAND, data))
        } else {
            write_text(&human_report(inspection))
        });
    };

    let code = inspection.form.status.code();
    let written = if json {
        write_json(&failure(COMMAND, code, message, data))
    } else {
        write_text(&human_report(inspection))
    };
    output(written)?;
    Err(reported(code, 4))
}

/// Report a refused input. Nothing but the envelope is ever written.
fn reject(rejection: &Failure, json: bool) -> CommandResult {
    let code = rejection.code.code();
    if json {
        output(write_json(&failure(
            COMMAND,
            code,
            rejection.code.message(),
            rejection_data(rejection),
        )))?;
    }
    Err(reported(code, 1))
}

/// Write the single fixed stderr line and choose the exit code.
fn reported(code: &str, exit: u8) -> CommandError {
    write_diagnostic(&format!(
        "opencegeljaras: {COMMAND}: {code} (exit {exit})\n"
    ));
    CommandError::Reported(exit)
}

/// The `data` object of a refused input, with unestablished facts omitted.
pub(crate) fn rejection_data(rejection: &Failure) -> Value {
    let input = rejection.input.as_ref().map_or(Value::Null, |input| {
        let mut object = Map::new();
        object.insert("bytes".to_owned(), json!(input.bytes));
        if let Some(bom) = input.bom {
            object.insert("bom".to_owned(), json!(bom));
        }
        if let Some(declaration) = input.xml_declaration {
            object.insert("xml_declaration".to_owned(), json!(declaration));
        }
        if let Some(encoding) = input.encoding {
            object.insert("encoding".to_owned(), json!(encoding));
        }
        Value::Object(object)
    });
    json!({
        "input": input,
        "root": Value::Null,
        "form": Value::Null,
        "structure": Value::Null,
        "checks": Value::Null,
        "notice": NOTICE,
    })
}

/// The `data` object of a completed inspection.
pub(crate) fn inspection_data(inspection: &Inspection) -> Value {
    let input = &inspection.input;
    let structure = &inspection.structure;
    let form = &inspection.form;
    let mut form_object = Map::new();
    form_object.insert("status".to_owned(), json!(form.status.code()));
    form_object.insert("profile".to_owned(), json!(form.profile));
    form_object.insert(
        "variant".to_owned(),
        form.variant.as_ref().map_or(Value::Null, |variant| {
            json!({
                "id": variant.id,
                "local_name": variant.local_name,
                "namespace": variant.namespace,
            })
        }),
    );
    match &form.sections {
        None => {
            form_object.insert("sections".to_owned(), Value::Null);
        }
        Some(sections) => {
            let listed: Vec<Value> = sections
                .iter()
                .map(|section| json!({ "local_name": section.local_name, "count": section.count }))
                .collect();
            form_object.insert("sections".to_owned(), Value::Array(listed));
            form_object.insert(
                "sections_truncated".to_owned(),
                json!(form.sections_truncated),
            );
        }
    }

    json!({
        "input": {
            "bytes": input.bytes,
            "bom": input.bom,
            "xml_declaration": input.xml_declaration,
            "encoding": input.encoding,
            "stylesheet_pi": input.stylesheet_pi,
            "schema_location_declared": input.schema_location_declared,
        },
        "root": {
            "local_name": inspection.root.local_name,
            "namespace": inspection.root.namespace,
        },
        "form": Value::Object(form_object),
        "structure": {
            "elements": structure.elements,
            "max_depth": structure.max_depth,
            "attributes": structure.attributes,
            "text_bytes": structure.text_bytes,
            "namespaces": structure.namespaces,
            "namespaces_truncated": structure.namespaces_truncated,
        },
        "checks": {
            "well_formed": "passed",
            "xsd_conformance": "not_performed",
            "signature": "not_performed",
            "authority_acceptance": "not_performed",
        },
        "notice": NOTICE,
    })
}

/// One fact per line, in the order fixed by the form inspection contract.
fn human_report(inspection: &Inspection) -> String {
    let mut report = human_facts(inspection);
    report.push_str("xsd conformance: not performed\n");
    report.push_str("verification: not performed\n");
    report.push_str("submission: not performed\n");
    report
}

/// The document facts alone, up to and including the well-formedness line.
///
/// `form validate` prints the same lines and then reports what it established
/// beyond them, so the two commands cannot drift apart.
pub(crate) fn human_facts(inspection: &Inspection) -> String {
    let input = &inspection.input;
    let structure = &inspection.structure;
    let form = &inspection.form;
    let root_namespace = inspection
        .root
        .namespace
        .as_deref()
        .map_or_else(|| String::from("none"), quoted);

    let mut report = String::new();
    report.push_str(&format!("status: {}\n", form.status.code()));
    report.push_str(&format!("profile: {}\n", form.profile.unwrap_or("none")));
    report.push_str(&format!(
        "variant: {}\n",
        form.variant.as_ref().map_or("none", |variant| variant.id)
    ));
    report.push_str(&format!(
        "root: {} in {root_namespace}\n",
        quoted(&inspection.root.local_name)
    ));
    report.push_str(&format!("encoding: {}\n", input.encoding));
    report.push_str(&format!("bom: {}\n", input.bom));
    report.push_str(&format!("xml declaration: {}\n", input.xml_declaration));
    report.push_str(&format!("stylesheet pi: {}\n", input.stylesheet_pi));
    report.push_str(&format!(
        "schema location declared: {}\n",
        input.schema_location_declared
    ));
    report.push_str(&format!("elements: {}\n", structure.elements));
    report.push_str(&format!("max depth: {}\n", structure.max_depth));
    report.push_str(&format!("attributes: {}\n", structure.attributes));
    report.push_str(&format!("text bytes: {}\n", structure.text_bytes));
    report.push_str(&format!("sections: {}\n", human_sections(inspection)));
    report.push_str("namespaces:\n");
    for namespace in &structure.namespaces {
        report.push_str(&format!("  {}\n", quoted(namespace)));
    }
    if structure.namespaces_truncated {
        report.push_str("  …\n");
    }
    report.push_str("well-formed: passed\n");
    report
}

/// The comma-separated section list, or `none`.
fn human_sections(inspection: &Inspection) -> String {
    let Some(sections) = &inspection.form.sections else {
        return String::from("none");
    };
    if sections.is_empty() {
        return String::from("none");
    }
    let mut listed: Vec<String> = sections
        .iter()
        .map(|section| format!("{}×{}", quoted(&section.local_name), section.count))
        .collect();
    if inspection.form.sections_truncated {
        listed.push(String::from("…"));
    }
    listed.join(", ")
}
