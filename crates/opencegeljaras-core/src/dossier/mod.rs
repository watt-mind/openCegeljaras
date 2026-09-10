//! Bounded, read-only recognition of a 2026 company-court e-dossier.
//!
//! This module reads one already-read byte slice through the shared bounded
//! parsing entry point under the dossier `Limits` profile and reports the
//! dossier's own account of itself and of the form it carries. It resolves no
//! reference, contacts no address, applies no zip or encrypt transform, and
//! verifies no signature: `OBJREF`, `Id`, `SIGREF`, `SIGREFLIST`, `Location`,
//! and `AckEmail` are never followed. Recognising a dossier establishes
//! nothing about XSD conformance, signature validity, filing, or acceptance.

mod base64;
mod findings;
mod patterns;
mod read;
mod roles;

use crate::form::{self, DOSSIER_LIMITS, Failure, FormFacts, Limits, Rejection, RootFacts};

pub use findings::{
    Conformance, Finding, FindingStatus, Preflight, Source, XSD_NOT_REQUESTED, findings, verdict,
};

/// The fixed notice attached to every completed dossier inspection.
pub const NOTICE: &str = "structural account of one dossier and of the form it carries; nothing was verified, decrypted, or accepted";
/// The fixed notice attached to every completed preflight.
pub const PREFLIGHT_NOTICE: &str = "deterministic preflight findings over one dossier; not the technical conditions, not signature validity, not acceptance";
/// Appended to the preflight notice when the dossier is a receipt.
pub const RECEIPT_NOTICE: &str = "; receipt recognised, not acknowledged";
/// The fixed diagnostic for an input refused under the dossier profile.
pub const INPUT_TOO_LARGE: &str = "the input exceeds the 64 MiB limit";
/// Profile identifier reported for the supported 2026 dossier.
pub const PROFILE: &str = "dossier-2026";
/// Namespace of the supported 2026 dossier.
pub const NAMESPACE: &str = "http://www.e-cegjegyzek.hu/2026/e-cegeljaras#";
/// The largest number of documents reported.
pub const DOCUMENT_LIMIT: usize = 256;
/// The largest number of Unicode scalar values kept from a document value.
pub const VALUE_LIMIT: usize = 256;
/// The largest number of Unicode scalar values kept from a declared token.
pub const TOKEN_LIMIT: usize = 64;
/// The largest text node the package 4 validator accepts, in bytes.
///
/// libxml2 without `--huge` refuses a single text node above this, and the
/// adapter forbids `--huge`, so a larger payload is never handed to it.
pub const VALIDATOR_TEXT_LIMIT: usize = 10_000_000;

/// The `form inspect` outcome over one decoded payload.
///
/// `Ok` carries the recognition result, `Err` the code the inspect boundary
/// refused the payload with. Neither establishes conformance or validity.
pub type FormOutcome = Result<FormFacts, Rejection>;

/// How a well-formed document relates to the supported 2026 dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DossierStatus {
    /// A `Dossier` root in the supported 2026 namespace.
    Recognized,
    /// A `Dossier` root in a differently dated dossier namespace.
    UnsupportedVersion,
    /// Any other root element or namespace.
    UnrecognizedRoot,
}

impl DossierStatus {
    /// The stable machine-readable status.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Recognized => "recognized",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnrecognizedRoot => "unrecognized_root",
        }
    }

    /// The fixed diagnostic for a status that is reported as an error.
    #[must_use]
    pub fn message(self) -> Option<&'static str> {
        match self {
            Self::Recognized => None,
            Self::UnsupportedVersion => Some("the dossier namespace version is not supported"),
            Self::UnrecognizedRoot => Some("the root element is not a company dossier"),
        }
    }
}

/// Why one document's payload was or was not inspected.
///
/// The variants are decided in declaration order and the first applicable one
/// wins, so a payload that is not plain base64 is never decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadStatus {
    /// The document carries no `ds:Object`.
    ObjectMissing,
    /// The `ds:Object` has element children rather than text.
    ObjectNotText,
    /// The transform chain is not exactly `base64`.
    TransformNotPlainBase64,
    /// Neither an XML media type nor an `xml` extension is declared.
    NotXml,
    /// The base64 text is longer than the decoded payload limit allows.
    PayloadTooLarge,
    /// The base64 text is not strict, padded base64.
    Base64Invalid,
    /// The payload was decoded and inspected as a form.
    Inspected,
}

impl PayloadStatus {
    /// The stable machine-readable reason.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::ObjectMissing => "object_missing",
            Self::ObjectNotText => "object_not_text",
            Self::TransformNotPlainBase64 => "transform_not_plain_base64",
            Self::NotXml => "not_xml",
            Self::PayloadTooLarge => "payload_too_large",
            Self::Base64Invalid => "base64_invalid",
            Self::Inspected => "inspected",
        }
    }
}

/// Facts about the input bytes and their encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFacts {
    /// Size of the input in bytes, including any byte-order mark.
    pub bytes: u64,
    /// Whether a UTF-8 byte-order mark was present.
    pub bom: bool,
    /// Whether an XML declaration was present.
    pub xml_declaration: bool,
    /// The normalised label of the encoding that was applied.
    pub encoding: &'static str,
}

/// The four company-identity elements, as presence and as values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    /// `Cégforma`, when present.
    pub cegforma: Option<String>,
    /// `Cégjegyzékszám`, when present.
    pub cegjegyzekszam: Option<String>,
    /// `Cégnév`, when present.
    pub cegnev: Option<String>,
    /// `Cégszékhely`, when present.
    pub cegszekhely: Option<String>,
}

/// The dossier profile's own account of the procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DossierFacts {
    /// The recognition status.
    pub status: DossierStatus,
    /// The dossier profile, when the namespace is supported.
    pub profile: Option<&'static str>,
    /// The procedure title, when it is one of the enumerated six.
    pub title: Option<&'static str>,
    /// The dossier category, when it is one of the enumerated four.
    pub category: Option<&'static str>,
    /// Whether the category marks a receipt rather than a filing.
    pub receipt: bool,
    /// The creation date, when it is lexically an `xs:dateTime`.
    pub creation_date: Option<String>,
    /// Whether the `AckEmail` attribute is present. It is never contacted.
    pub ack_email_present: bool,
    /// Whether the `Location` attribute is present. It is never resolved.
    pub location_present: bool,
    /// The four company-identity elements.
    pub identity: Identity,
    /// Whether `Documents/@Id` carries the fixed `Object0`.
    pub documents_id_fixed: bool,
}

/// The declared media type of one document. Nothing is sniffed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mime {
    /// The declared `type`, cut to a fixed length.
    pub type_name: Option<String>,
    /// The declared `subtype`, cut to a fixed length.
    pub subtype: Option<String>,
    /// The declared `extension`, cut to a fixed length.
    pub extension: Option<String>,
    /// Whether a `charSet` attribute is present. Its value is never read.
    pub charset_present: bool,
}

/// One listed document of the dossier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentFacts {
    /// The 0-based position in document order.
    pub index: usize,
    /// The fixed text of the matched role pattern, when one matched.
    pub role: Option<&'static str>,
    /// The declared title, cut to a fixed length. Reported only on request.
    pub title: Option<String>,
    /// The document category, when it is one of the enumerated six.
    pub category: Option<&'static str>,
    /// The declared media type.
    pub mime: Mime,
    /// The declared source size, when it is an integer.
    pub source_size: Option<i64>,
    /// The declared transform chain, with unknown algorithms as `unknown`.
    pub transforms: Vec<&'static str>,
    /// Whether a `ds:Object` is present inside the document.
    pub object_present: bool,
    /// Whether the document declares a nested dossier. It is never opened.
    pub nested_dossier: bool,
    /// Signature elements inside the document. Nothing is verified.
    pub signatures: usize,
    /// Timestamp elements inside the document. Nothing is verified.
    pub timestamps: usize,
    /// Why the payload was or was not inspected.
    pub payload: PayloadStatus,
    /// The `form inspect` outcome over the decoded payload, when inspected.
    pub form: Option<FormOutcome>,
}

/// Everything one dossier inspection established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// Facts about the input bytes and their encoding.
    pub input: InputFacts,
    /// Facts about the root element.
    pub root: RootFacts,
    /// The dossier profile's account of itself.
    pub dossier: DossierFacts,
    /// The listed documents, absent when the dossier is not recognised.
    pub documents: Option<Vec<DocumentFacts>>,
    /// Whether the document list was cut at the document limit.
    pub documents_truncated: bool,
    /// Signature elements at dossier level and inside documents.
    pub signatures_present: usize,
    /// Timestamp elements at dossier level and inside documents.
    pub timestamps_present: usize,
    /// The largest element's text content in bytes, for the validator gate.
    pub max_text_bytes: usize,
}

impl Inspection {
    /// The listed documents, or an empty slice when there are none.
    #[must_use]
    pub fn documents(&self) -> &[DocumentFacts] {
        self.documents.as_deref().unwrap_or_default()
    }

    /// Whether a text node would exceed what the package 4 validator accepts.
    #[must_use]
    pub fn exceeds_validator_limit(&self) -> bool {
        self.max_text_bytes > VALIDATOR_TEXT_LIMIT
    }
}

/// The fixed limits applied to one dossier container.
#[must_use]
pub fn limits() -> Limits {
    DOSSIER_LIMITS
}

/// Inspect one already-read dossier under the dossier limits profile.
///
/// # Errors
///
/// Returns a [`Failure`] when the input is too large, is not decodable under
/// an accepted encoding, contains a prohibited construct, exceeds a limit, or
/// is not well-formed XML.
pub fn inspect(bytes: &[u8]) -> Result<Inspection, Failure> {
    let limits = limits();
    form::parse_bounded(bytes, &limits, |document, input| {
        read::read(document, input, &limits)
    })
}

/// The fixed text of every role pattern the schema lists, in schema order.
#[must_use]
pub fn role_texts() -> Vec<&'static str> {
    roles::texts()
}

#[cfg(test)]
mod tests;
