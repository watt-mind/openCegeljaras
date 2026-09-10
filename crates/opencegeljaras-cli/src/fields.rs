//! The `form fields` and `form diff` commands: envelopes and human output.
//!
//! Both run the `form inspect` boundary first and list nothing for a document
//! that boundary refuses or does not recognise. Field values are personal and
//! company data: they reach standard output only with `--show-values`, never
//! standard error, and never a diagnostic. Value lengths are structure and are
//! always reported. Listing or comparing fields establishes neither schema
//! conformance nor signature validity, filing, or acceptance.

use std::path::{Path, PathBuf};

use clap::Args;
use opencegeljaras_core::form::{
    self as core_form, DIFF_NOTICE, Diff, FIELDS_NOTICE, FieldKind, FieldList, Inspection, Limits,
    Listing,
};
use serde_json::{Map, Value, json};

use crate::form;
use crate::output::{
    CommandError, CommandResult, failure, output, quoted, success, write_diagnostic, write_json,
    write_text,
};
use crate::validate::facts_of;

/// The command name reported in `form fields` envelopes and diagnostics.
const FIELDS_COMMAND: &str = "form fields";
/// The command name reported in `form diff` envelopes and diagnostics.
const DIFF_COMMAND: &str = "form diff";
/// The fixed code and message for an unrecognised OLD document.
const OLD_NOT_RECOGNIZED: (&str, &str) = (
    "old_not_recognized",
    "the OLD document is not a recognised form",
);
/// The fixed code and message for an unrecognised NEW document.
const NEW_NOT_RECOGNIZED: (&str, &str) = (
    "new_not_recognized",
    "the NEW document is not a recognised form",
);
/// How a field with no value is printed where a value would go.
const MIXED_MARKER: &str = "(mixed content)";

/// Arguments of `opencegeljaras form fields`.
#[derive(Debug, Args)]
pub(crate) struct FieldsArgs {
    /// Path of the local form file to list. `-` is an ordinary name.
    path: PathBuf,
    /// Print field values. This can expose personal and company data.
    #[arg(long)]
    show_values: bool,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Arguments of `opencegeljaras form diff`.
#[derive(Debug, Args)]
pub(crate) struct DiffArgs {
    /// Path of the earlier local form file. It is read first.
    old: PathBuf,
    /// Path of the later local form file. It is never opened when OLD fails.
    new: PathBuf,
    /// Print field values. This can expose personal and company data.
    #[arg(long)]
    show_values: bool,
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

/// Read and list one local file under the shared bounded limits.
fn load(path: &Path, limits: &Limits) -> Result<Listing, core_form::Failure> {
    form::read_input(path, limits).and_then(|bytes| core_form::list_fields(&bytes, limits))
}

/// Write the single fixed stderr line and choose the exit code.
///
/// No path, document value, name, or namespace ever reaches this line.
fn reported(command: &str, code: &str, exit: u8) -> CommandError {
    write_diagnostic(&format!(
        "opencegeljaras: {command}: {code} (exit {exit})\n"
    ));
    CommandError::Reported(exit)
}

/// List the fields of one local form file.
pub(crate) fn fields(args: &FieldsArgs) -> CommandResult {
    let limits = Limits::default();
    let listing = match load(&args.path, &limits) {
        Ok(listing) => listing,
        Err(rejection) => {
            let code = rejection.code.code();
            if args.json {
                let data = listing_data(form::rejection_data(&rejection), None, args.show_values);
                output(write_json(&failure(
                    FIELDS_COMMAND,
                    code,
                    rejection.code.message(),
                    data,
                )))?;
            }
            return Err(reported(FIELDS_COMMAND, code, 1));
        }
    };

    let inspection = &listing.inspection;
    let data = listing_data(
        form::inspection_data(inspection),
        listing.fields.as_ref(),
        args.show_values,
    );
    let Some(list) = listing.fields.as_ref() else {
        let status = inspection.form.status;
        let code = status.code();
        let message = status
            .message()
            .unwrap_or("the document is not a recognised company form");
        output(if args.json {
            write_json(&failure(FIELDS_COMMAND, code, message, data))
        } else {
            write_text(&form::human_report(inspection))
        })?;
        return Err(reported(FIELDS_COMMAND, code, 4));
    };

    output(if args.json {
        write_json(&success(FIELDS_COMMAND, data))
    } else {
        write_text(&fields_report(inspection, list, args.show_values))
    })
}

/// The `form fields` `data` object: the inspect structure plus the listing.
fn listing_data(base: Value, list: Option<&FieldList>, show_values: bool) -> Value {
    let mut object = match base {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    match list {
        None => {
            for key in ["fields", "field_count", "fields_truncated"] {
                object.insert(key.to_owned(), Value::Null);
            }
        }
        Some(list) => {
            let listed: Vec<Value> = (0..list.len())
                .map(|index| field_entry(list, index, show_values, false))
                .collect();
            object.insert("fields".to_owned(), Value::Array(listed));
            object.insert("field_count".to_owned(), json!(list.len()));
            object.insert("fields_truncated".to_owned(), json!(list.truncated()));
        }
    }
    object.insert("values_redacted".to_owned(), json!(!show_values));
    object.insert("notice".to_owned(), json!(FIELDS_NOTICE));
    Value::Object(object)
}

/// One field entry.
///
/// A mixed element has no value at all: `form fields` omits the value keys,
/// and `form diff` reports them as null so that every entry has the same
/// shape.
fn field_entry(list: &FieldList, index: usize, show_values: bool, keep_null: bool) -> Value {
    let field = &list.fields()[index];
    let path = list.path(index);
    let mut object = Map::new();
    object.insert("path".to_owned(), json!(path.text));
    object.insert("path_truncated".to_owned(), json!(path.truncated));
    object.insert("kind".to_owned(), json!(field.kind().code()));
    object.insert("namespace".to_owned(), json!(list.namespace(index)));
    if keep_null || field.kind() != FieldKind::Mixed {
        object.insert("value_length".to_owned(), json!(field.value_length()));
        object.insert("value_truncated".to_owned(), json!(field.value_truncated()));
        if show_values {
            object.insert("value".to_owned(), json!(field.emitted_value()));
        }
    }
    Value::Object(object)
}

/// One changed entry: the same path with a different kind, namespace or value.
fn change_entry(
    old: &FieldList,
    new: &FieldList,
    pair: (usize, usize),
    show_values: bool,
) -> Value {
    let (from, to) = pair;
    let path = new.path(to);
    let mut object = Map::new();
    object.insert("path".to_owned(), json!(path.text));
    object.insert("path_truncated".to_owned(), json!(path.truncated));
    for (prefix, list, index) in [("old", old, from), ("new", new, to)] {
        let field = &list.fields()[index];
        object.insert(format!("{prefix}_kind"), json!(field.kind().code()));
        object.insert(format!("{prefix}_namespace"), json!(list.namespace(index)));
        object.insert(
            format!("{prefix}_value_length"),
            json!(field.value_length()),
        );
        object.insert(
            format!("{prefix}_value_truncated"),
            json!(field.value_truncated()),
        );
        if show_values {
            object.insert(format!("{prefix}_value"), json!(field.emitted_value()));
        }
    }
    Value::Object(object)
}

/// The human `form fields` report: inspect's facts, the listing, the boundary.
fn fields_report(inspection: &Inspection, list: &FieldList, show_values: bool) -> String {
    let mut report = form::human_document_facts(inspection);
    let truncated = if list.truncated() { " (truncated)" } else { "" };
    report.push_str(&format!("fields: {}{truncated}\n", list.len()));
    for index in 0..list.len() {
        let field = &list.fields()[index];
        let path = quoted(&list.path(index).text);
        let line = match (show_values, field.emitted_value()) {
            (_, None) => format!("{path} {MIXED_MARKER}"),
            (true, Some(value)) => format!("{path} = {}", quoted(&value)),
            (false, Some(_)) => format!(
                "{path} ({} chars)",
                field.value_length().unwrap_or_default()
            ),
        };
        report.push_str(&format!("  {line}\n"));
    }
    report.push_str(form::BOUNDARY_LINES);
    report
}

/// Which side of a comparison a failure belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Old,
    New,
}

impl Side {
    /// The stable machine-readable side name.
    fn code(self) -> &'static str {
        match self {
            Self::Old => "old",
            Self::New => "new",
        }
    }
}

/// Compare two local form files field by field.
pub(crate) fn diff(args: &DiffArgs) -> CommandResult {
    let limits = Limits::default();
    let old = match load(&args.old, &limits) {
        Ok(listing) => listing,
        Err(rejection) => return refuse(&rejection, Side::Old, args),
    };
    let Some(old_list) = old.fields.as_ref() else {
        return unrecognized(&old, Side::Old, None, args);
    };

    let new = match load(&args.new, &limits) {
        Ok(listing) => listing,
        Err(rejection) => return refuse(&rejection, Side::New, args),
    };
    let Some(new_list) = new.fields.as_ref() else {
        return unrecognized(&new, Side::New, Some(&old), args);
    };

    let difference = core_form::diff(old_list, new_list);
    let same_variant = variant(&old.inspection) == variant(&new.inspection);
    let identical = difference.added.is_empty()
        && difference.removed.is_empty()
        && difference.changed.is_empty()
        && same_variant
        && !old_list.truncated()
        && !new_list.truncated();

    let data = diff_data(
        &old,
        &new,
        &difference,
        (same_variant, identical),
        args.show_values,
    );
    output(if args.json {
        write_json(&success(DIFF_COMMAND, data))
    } else {
        write_text(&diff_report(
            &old,
            &new,
            &difference,
            (same_variant, identical),
            args.show_values,
        ))
    })
}

/// The variant identifier of one inspection, or `none`.
fn variant(inspection: &Inspection) -> &str {
    inspection
        .form
        .variant
        .as_ref()
        .map_or("none", |variant| variant.id)
}

/// Report one side's refused input at exit 1. Nothing is printed as text.
fn refuse(rejection: &core_form::Failure, side: Side, args: &DiffArgs) -> CommandResult {
    let code = rejection.code.code();
    if args.json {
        let facts = Value::Object(facts_of(&form::rejection_data(rejection)));
        let mut object = side_data(side, facts, None, args.show_values);
        object.insert("checks".to_owned(), Value::Null);
        output(write_json(&failure(
            DIFF_COMMAND,
            code,
            rejection.code.message(),
            Value::Object(object),
        )))?;
    }
    Err(reported(DIFF_COMMAND, code, 1))
}

/// Report one side that is well-formed but not a recognised form, at exit 4.
fn unrecognized(
    listing: &Listing,
    side: Side,
    other: Option<&Listing>,
    args: &DiffArgs,
) -> CommandResult {
    let (code, message) = match side {
        Side::Old => OLD_NOT_RECOGNIZED,
        Side::New => NEW_NOT_RECOGNIZED,
    };
    let inspect = form::inspection_data(&listing.inspection);
    let written = if args.json {
        let facts = Value::Object(facts_of(&inspect));
        let mut object = side_data(side, facts, other, args.show_values);
        object.insert("checks".to_owned(), inspect["checks"].clone());
        write_json(&failure(DIFF_COMMAND, code, message, Value::Object(object)))
    } else {
        write_text(&unrecognized_report(listing, other))
    };
    output(written)?;
    Err(reported(DIFF_COMMAND, code, 4))
}

/// The `data` object of a failing comparison, with the other side's summary.
fn side_data(
    side: Side,
    failing: Value,
    other: Option<&Listing>,
    show_values: bool,
) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("side".to_owned(), json!(side.code()));
    let established = other.map_or(Value::Null, side_summary);
    match side {
        Side::Old => {
            object.insert("old".to_owned(), failing);
            object.insert("new".to_owned(), established);
        }
        Side::New => {
            object.insert("old".to_owned(), established);
            object.insert("new".to_owned(), failing);
        }
    }
    for key in [
        "same_variant",
        "identical",
        "added",
        "removed",
        "changed",
        "unchanged",
    ] {
        object.insert(key.to_owned(), Value::Null);
    }
    object.insert("values_redacted".to_owned(), json!(!show_values));
    object.insert("notice".to_owned(), json!(DIFF_NOTICE));
    object
}

/// One recognised side's summary: the inspect facts and its field counts.
fn side_summary(listing: &Listing) -> Value {
    let mut object = facts_of(&form::inspection_data(&listing.inspection));
    let (count, truncated) = listing
        .fields
        .as_ref()
        .map_or((Value::Null, Value::Null), |list| {
            (json!(list.len()), json!(list.truncated()))
        });
    object.insert("field_count".to_owned(), count);
    object.insert("fields_truncated".to_owned(), truncated);
    Value::Object(object)
}

/// The `data` object of a completed comparison.
fn diff_data(
    old: &Listing,
    new: &Listing,
    difference: &Diff,
    verdict: (bool, bool),
    show_values: bool,
) -> Value {
    let old_list = old.fields.as_ref().expect("a compared side is listed");
    let new_list = new.fields.as_ref().expect("a compared side is listed");
    let added: Vec<Value> = difference
        .added
        .iter()
        .map(|index| field_entry(new_list, *index, show_values, true))
        .collect();
    let removed: Vec<Value> = difference
        .removed
        .iter()
        .map(|index| field_entry(old_list, *index, show_values, true))
        .collect();
    let changed: Vec<Value> = difference
        .changed
        .iter()
        .map(|pair| change_entry(old_list, new_list, *pair, show_values))
        .collect();

    json!({
        "side": Value::Null,
        "old": side_summary(old),
        "new": side_summary(new),
        "same_variant": verdict.0,
        "identical": verdict.1,
        "added": added,
        "removed": removed,
        "changed": changed,
        "unchanged": difference.unchanged,
        "values_redacted": !show_values,
        "checks": form::inspection_data(&new.inspection)["checks"].clone(),
        "notice": DIFF_NOTICE,
    })
}

/// One side's human line: its variant and how many fields it carries.
fn side_line(name: &str, listing: &Listing) -> String {
    let Some(list) = listing.fields.as_ref() else {
        return format!("{name}: {}\n", listing.inspection.form.status.code());
    };
    let truncated = if list.truncated() { " (truncated)" } else { "" };
    format!(
        "{name}: {}, {} fields{truncated}\n",
        variant(&listing.inspection),
        list.len()
    )
}

/// The human report of a comparison one side did not reach.
fn unrecognized_report(listing: &Listing, other: Option<&Listing>) -> String {
    let mut report = match other {
        None => format!("{}new: none\n", side_line("old", listing)),
        Some(old) => format!("{}{}", side_line("old", old), side_line("new", listing)),
    };
    report.push_str(form::BOUNDARY_LINES);
    report
}

/// How one entry's value is printed when values were requested.
fn value_text(list: &FieldList, index: usize) -> String {
    list.fields()[index]
        .emitted_value()
        .map_or_else(|| String::from(MIXED_MARKER), |value| quoted(&value))
}

/// The human report of a completed comparison.
fn diff_report(
    old: &Listing,
    new: &Listing,
    difference: &Diff,
    verdict: (bool, bool),
    show_values: bool,
) -> String {
    let old_list = old.fields.as_ref().expect("a compared side is listed");
    let new_list = new.fields.as_ref().expect("a compared side is listed");
    let mut report = side_line("old", old);
    report.push_str(&side_line("new", new));
    report.push_str(&format!("same variant: {}\n", verdict.0));
    report.push_str(&format!("identical: {}\n", verdict.1));

    for (label, list, indices) in [
        ("added", new_list, &difference.added),
        ("removed", old_list, &difference.removed),
    ] {
        report.push_str(&format!("{label}: {}\n", indices.len()));
        for index in indices {
            let path = quoted(&list.path(*index).text);
            if show_values {
                report.push_str(&format!("  {path} = {}\n", value_text(list, *index)));
            } else {
                report.push_str(&format!("  {path}\n"));
            }
        }
    }

    report.push_str(&format!("changed: {}\n", difference.changed.len()));
    for (from, to) in &difference.changed {
        let path = quoted(&new_list.path(*to).text);
        if show_values {
            report.push_str(&format!(
                "  {path} {} -> {}\n",
                value_text(old_list, *from),
                value_text(new_list, *to)
            ));
        } else {
            report.push_str(&format!("  {path}\n"));
        }
    }
    report.push_str(&format!("unchanged: {}\n", difference.unchanged));
    report.push_str(form::BOUNDARY_LINES);
    report
}
