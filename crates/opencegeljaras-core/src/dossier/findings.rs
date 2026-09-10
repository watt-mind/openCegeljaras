//! The deterministic preflight findings over one inspected dossier.
//!
//! A finding is a statement about the dossier's own declarations, evaluated
//! from the inspection alone. Its `detail` is a fixed string chosen by the
//! code and the status and never carries a document value. Nothing here
//! verifies a signature or a timestamp: presence is all that is established,
//! and a passed finding is never a statement about acceptance.

use super::patterns::FILING_CATEGORIES;
use super::{DocumentFacts, Inspection, PayloadStatus, roles};
use crate::form::FormStatus;

/// The fixed detail for an XSD conformance that was never requested.
pub const XSD_NOT_REQUESTED: &str = "no schema bundle was given";
/// The transform chains the e-dossier specification defines.
const SUPPORTED_CHAINS: [&[&str]; 4] = [
    &["base64"],
    &["zip", "base64"],
    &["encrypt", "base64"],
    &["zip", "encrypt", "base64"],
];

/// Where the rule behind one finding comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The pinned 2026 dossier schema.
    Schema,
    /// The 2026 form contract of this project.
    Form,
    /// A rule this project derives rather than reads off the schema.
    Project,
}

impl Source {
    /// The stable machine-readable source name.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Schema => "xsd-2026",
            Self::Form => "form-2026",
            Self::Project => "openCegeljaras",
        }
    }
}

/// How one finding turned out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingStatus {
    /// The rule holds.
    Passed,
    /// The rule was evaluated and does not hold.
    Failed,
    /// The rule could not be evaluated.
    NotPerformed,
    /// The rule does not apply to this dossier.
    NotApplicable,
}

impl FindingStatus {
    /// The stable machine-readable status.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NotPerformed => "not_performed",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// One evaluated preflight rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The stable machine-readable code.
    pub code: &'static str,
    /// How the rule turned out.
    pub status: FindingStatus,
    /// Where the rule comes from.
    pub source: Source,
    /// The fixed detail for this code and status. Never a document value.
    pub detail: &'static str,
}

/// The overall preflight verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preflight {
    /// Every finding passed or did not apply, and the XSD check was performed.
    Passed,
    /// At least one finding failed.
    Failed,
    /// Nothing failed, but at least one finding was not performed.
    Incomplete,
    /// The dossier is a receipt; the filing rules do not apply to it.
    NotApplicable,
}

impl Preflight {
    /// The stable machine-readable verdict.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Incomplete => "incomplete",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// How the XSD conformance check turned out, as the caller established it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conformance {
    /// The validator was not run; the fixed detail says why.
    NotPerformed(&'static str),
    /// The validator reported the dossier as conforming.
    Passed,
    /// The validator reported the dossier as not conforming.
    Failed,
}

/// One finding built from a code, a source, and a status.
fn finding(code: &'static str, source: Source, status: FindingStatus) -> Finding {
    Finding {
        code,
        status,
        source,
        detail: detail(code, status),
    }
}

/// The one status a rule takes when a boolean decides it.
fn holds(rule: bool) -> FindingStatus {
    if rule {
        FindingStatus::Passed
    } else {
        FindingStatus::Failed
    }
}

/// Evaluate every preflight rule over one recognised dossier.
///
/// The XSD conformance the caller established is reported unchanged; nothing
/// in this crate spawns a validator or reads a bundle.
#[must_use]
pub fn findings(inspection: &Inspection, conformance: Conformance) -> Vec<Finding> {
    let xsd = finding(
        "xsd_conformance",
        Source::Schema,
        match conformance {
            Conformance::NotPerformed(_) => FindingStatus::NotPerformed,
            Conformance::Passed => FindingStatus::Passed,
            Conformance::Failed => FindingStatus::Failed,
        },
    );
    let xsd = Finding {
        detail: match conformance {
            Conformance::NotPerformed(detail) => detail,
            _ => xsd.detail,
        },
        ..xsd
    };
    if inspection.dossier.receipt {
        return receipt_findings(xsd);
    }
    let mut findings = filing_findings(inspection);
    findings.push(xsd);
    findings
}

/// The findings of a receipt: everything but the XSD check does not apply.
fn receipt_findings(xsd: Finding) -> Vec<Finding> {
    let mut findings: Vec<Finding> = FILING_RULES
        .iter()
        .map(|(code, source)| Finding {
            code,
            status: FindingStatus::NotApplicable,
            source: *source,
            detail: RECEIPT_DETAIL,
        })
        .collect();
    findings.push(xsd);
    findings
}

/// The fixed detail of every finding a receipt makes inapplicable.
const RECEIPT_DETAIL: &str = "the dossier is a receipt";

/// The filing rules, in report order, with the source of each.
const FILING_RULES: [(&str, Source); 10] = [
    ("procedure_title", Source::Schema),
    ("dossier_category", Source::Project),
    ("documents_id", Source::Schema),
    ("form_document_present", Source::Project),
    ("form_recognized", Source::Form),
    ("form_matches_procedure", Source::Project),
    ("document_roles_known", Source::Schema),
    ("transforms_supported", Source::Project),
    ("signature_present", Source::Project),
    ("timestamp_present", Source::Project),
];

/// Evaluate the ten filing rules over one recognised, non-receipt dossier.
fn filing_findings(inspection: &Inspection) -> Vec<Finding> {
    let dossier = &inspection.dossier;
    let documents = inspection.documents();
    let forms: Vec<&DocumentFacts> = documents
        .iter()
        .filter(|document| document.role.is_some_and(roles::is_form_role))
        .collect();
    let form = (forms.len() == 1).then(|| forms[0]);
    let recognized = form.and_then(form_variant);

    let roles_known = if inspection.documents_truncated {
        FindingStatus::NotPerformed
    } else {
        holds(documents.iter().all(|document| document.role.is_some()))
    };
    vec![
        finding(
            "procedure_title",
            Source::Schema,
            holds(dossier.title.is_some()),
        ),
        finding(
            "dossier_category",
            Source::Project,
            holds(
                dossier
                    .category
                    .is_some_and(|value| FILING_CATEGORIES.contains(&value)),
            ),
        ),
        finding(
            "documents_id",
            Source::Schema,
            holds(dossier.documents_id_fixed),
        ),
        finding(
            "form_document_present",
            Source::Project,
            holds(form.is_some()),
        ),
        finding("form_recognized", Source::Form, form_recognized(form)),
        finding(
            "form_matches_procedure",
            Source::Project,
            matches_procedure(dossier.title, recognized),
        ),
        finding("document_roles_known", Source::Schema, roles_known),
        finding(
            "transforms_supported",
            Source::Project,
            holds(documents.iter().all(supported_chain)),
        ),
        finding(
            "signature_present",
            Source::Project,
            holds(inspection.signatures_present >= 1),
        ),
        finding(
            "timestamp_present",
            Source::Project,
            holds(inspection.timestamps_present >= 1),
        ),
    ]
}

/// The recognised variant identifier of the form document, if it has one.
fn form_variant(document: &DocumentFacts) -> Option<&'static str> {
    let Some(Ok(facts)) = &document.form else {
        return None;
    };
    if facts.status != FormStatus::Recognized {
        return None;
    }
    facts.variant.as_ref().map(|variant| variant.id)
}

/// Whether the form document's payload was inspected and recognised.
fn form_recognized(document: Option<&DocumentFacts>) -> FindingStatus {
    let Some(document) = document else {
        return FindingStatus::NotPerformed;
    };
    if document.payload != PayloadStatus::Inspected {
        return FindingStatus::NotPerformed;
    }
    holds(form_variant(document).is_some())
}

/// The variant each procedure title requires, when it requires one.
fn required_variant(title: &str) -> Option<&'static str> {
    match title {
        "Névfoglalási kérelem" => Some("nevfoglalasi-kerelem-20210401"),
        "Bejegyzési kérelem" | "Ismételt bejegyzési kérelem" => {
            Some("cegbejegyzesi-kerelem-20260101")
        }
        "Változás bejegyzési kérelem" => Some("valtozasbejegyzesi-kerelem-20240101"),
        _ => None,
    }
}

/// Whether the recognised variant is the one the procedure title requires.
///
/// `Egyéb beadvány` and `Hiánypótlás` fix no variant, so the rule does not
/// apply to them. A title outside the enumeration fixes none either, but
/// `procedure_title` has already failed, so the match cannot be established.
fn matches_procedure(title: Option<&'static str>, variant: Option<&'static str>) -> FindingStatus {
    let Some(variant) = variant else {
        return FindingStatus::NotPerformed;
    };
    let Some(title) = title else {
        return FindingStatus::Failed;
    };
    match required_variant(title) {
        None => FindingStatus::NotApplicable,
        Some(required) => holds(required == variant),
    }
}

/// Whether one document's transform chain is a supported chain.
fn supported_chain(document: &DocumentFacts) -> bool {
    SUPPORTED_CHAINS
        .iter()
        .any(|chain| document.transforms == *chain)
}

/// The overall verdict over one dossier's findings.
///
/// A `passed` verdict means these findings passed and the outer XML conforms.
/// It is never a statement that a filing is complete, signed, or acceptable.
#[must_use]
pub fn verdict(findings: &[Finding], receipt: bool) -> Preflight {
    let has = |status| findings.iter().any(|finding| finding.status == status);
    if has(FindingStatus::Failed) {
        Preflight::Failed
    } else if receipt {
        Preflight::NotApplicable
    } else if has(FindingStatus::NotPerformed) {
        Preflight::Incomplete
    } else {
        Preflight::Passed
    }
}

/// The fixed detail string for one code and status.
///
/// Every string here is chosen by the code and the status alone: no declared
/// title, value, or validator message ever reaches a finding's `detail`.
fn detail(code: &str, status: FindingStatus) -> &'static str {
    match (code, status) {
        ("procedure_title", FindingStatus::Passed) => {
            "the procedure title is one of the six enumerated titles"
        }
        ("procedure_title", _) => "the procedure title is not one of the six enumerated titles",
        ("dossier_category", FindingStatus::Passed) => "the dossier category is a filing category",
        ("dossier_category", _) => "the dossier category is not a filing category",
        ("documents_id", FindingStatus::Passed) => {
            "the documents container carries the fixed identifier"
        }
        ("documents_id", _) => "the documents container does not carry the fixed identifier",
        ("form_document_present", FindingStatus::Passed) => {
            "exactly one document carries the application-form role"
        }
        ("form_document_present", _) => {
            "the application-form role is carried by no document or by several"
        }
        ("form_recognized", FindingStatus::Passed) => {
            "the form document is a recognised 2026 company form"
        }
        ("form_recognized", FindingStatus::Failed) => {
            "the form document is not a recognised 2026 company form"
        }
        ("form_recognized", _) => "no inspected form payload was available",
        ("form_matches_procedure", FindingStatus::Passed) => {
            "the form variant is the one the procedure title requires"
        }
        ("form_matches_procedure", FindingStatus::Failed) => {
            "the form variant is not the one the procedure title requires"
        }
        ("form_matches_procedure", FindingStatus::NotApplicable) => {
            "the procedure title requires no particular form variant"
        }
        ("form_matches_procedure", _) => "no recognised form was available to compare",
        ("document_roles_known", FindingStatus::Passed) => {
            "every listed document title matches a schema role pattern"
        }
        ("document_roles_known", FindingStatus::Failed) => {
            "a listed document title matches no schema role pattern"
        }
        ("document_roles_known", _) => "the document list was cut at the document limit",
        ("transforms_supported", FindingStatus::Passed) => {
            "every transform chain is one the specification defines"
        }
        ("transforms_supported", _) => "a transform chain is not one the specification defines",
        ("signature_present", FindingStatus::Passed) => {
            "at least one signature element is present; validity was not assessed"
        }
        ("signature_present", _) => "no signature element is present",
        ("timestamp_present", FindingStatus::Passed) => {
            "at least one timestamp element is present; validity was not assessed"
        }
        ("timestamp_present", _) => "no timestamp element is present",
        ("xsd_conformance", FindingStatus::Passed) => {
            "the dossier conforms to the pinned 2026 dossier graph"
        }
        ("xsd_conformance", FindingStatus::Failed) => {
            "the dossier does not conform to the pinned 2026 dossier graph"
        }
        _ => XSD_NOT_REQUESTED,
    }
}
