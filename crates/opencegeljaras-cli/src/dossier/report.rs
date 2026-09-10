//! The JSON `data` objects and the human reports of both dossier commands.
//!
//! Every document-derived string that reaches an output field passes through
//! the `form inspect` escaping rule, and a declared title or company-identity
//! value appears only when the operator asked for values. A finding's `detail`
//! is a fixed string and never a document value.

use opencegeljaras_core::dossier::{
    self, DocumentFacts, DossierStatus, Finding, FormOutcome, Inspection,
};
use opencegeljaras_core::form::{Failure, FormStatus};
use serde_json::{Map, Value, json};

use super::{Conformed, PREFLIGHT_COMMAND};
use crate::form;
use crate::output::quoted;
use crate::validate::Findings;

/// The `checks` object, with the conformance the command established.
pub(super) fn checks(conformance: &str) -> Value {
    json!({
        "well_formed": "passed",
        "xsd_conformance": conformance,
        "signature": "not_performed",
        "authority_acceptance": "not_performed",
    })
}

/// The `data` object of a refused input, with unestablished facts omitted.
pub(super) fn null_data(rejection: &Failure, command: &str) -> Value {
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
pub(super) fn preflight_nulls(object: &mut Map<String, Value>) {
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
pub(super) fn inspect_data(inspection: &Inspection, show_values: bool) -> Map<String, Value> {
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
pub(super) fn dossier_value(inspection: &Inspection, show_values: bool) -> Value {
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
pub(super) fn document_value(document: &DocumentFacts, show_values: bool) -> Value {
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
pub(super) fn form_value(outcome: &FormOutcome) -> Value {
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
pub(super) fn human_facts(inspection: &Inspection, show_values: bool) -> String {
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
pub(super) fn human_document(document: &DocumentFacts, show_values: bool) -> String {
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
pub(super) fn human_form(document: &DocumentFacts) -> String {
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
pub(super) fn optional(value: Option<&str>) -> String {
    value.map_or_else(|| String::from("none"), quoted)
}

/// The word used for a presence flag.
pub(super) fn presence(present: bool) -> &'static str {
    if present { "present" } else { "absent" }
}

/// The full human `dossier inspect` report.
pub(super) fn human_report(inspection: &Inspection, show_values: bool) -> String {
    let mut report = human_facts(inspection, show_values);
    report.push_str(form::BOUNDARY_LINES);
    report
}

/// The four boundary lines with the conformance the command established.
pub(super) fn boundary_lines(conformance: &str) -> String {
    format!(
        "well-formed: passed\nxsd conformance: {}\nverification: not performed\nsubmission: not performed\n",
        conformance.replace('_', " ")
    )
}

/// The `findings` array, in the contract's order.
pub(super) fn findings_value(findings: &[Finding]) -> Value {
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
pub(super) fn insert_xsd_findings(object: &mut Map<String, Value>, findings: Option<&Findings>) {
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
pub(super) fn preflight_report(
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
