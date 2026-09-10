//! Bounded, read-only recognition of a Hungarian company-form XML document.
//!
//! This module decodes one already-read byte slice, applies fixed resource
//! limits, parses the result as namespace-aware XML, and reports structural
//! facts. It performs no filesystem, network, clock, or process work, and it
//! establishes nothing about XSD conformance, signature validity, filing, or
//! authority acceptance.

mod decode;
mod prescan;
mod tree;

use roxmltree::Document;

pub use tree::{PROFILE, WRAPPER_NAMESPACE};

/// The UTF-8 label, normalised as it is reported.
pub const UTF_8_LABEL: &str = "UTF-8";
/// The fixed notice attached to every completed inspection.
pub const NOTICE: &str =
    "format recognition only; not schema validation, verification, or acceptance";
/// The largest number of Unicode scalar values kept from a document string.
const NAME_LIMIT: usize = 128;

/// Fixed resource limits applied to one document. There are no overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// Largest accepted input in bytes.
    pub input_bytes: u64,
    /// Deepest accepted element nesting, with the root counted as one.
    pub depth: usize,
    /// Largest accepted number of elements.
    pub elements: usize,
    /// Largest accepted size of one text node in bytes.
    pub text_bytes: usize,
    /// Largest accepted number of attributes on one element, excluding
    /// namespace declarations.
    pub attributes: usize,
    /// Largest accepted number of `xmlns` and `xmlns:*` declarations on one
    /// element. The parser compares each declaration against every earlier
    /// one on the same element, so an uncapped element is quadratic work. The
    /// cap is 256 rather than 64 because the official variant schemas declare
    /// 82 and 85 prefixes on their root element and generated forms may do the
    /// same; at 256 the quadratic cost is still negligible.
    pub namespace_declarations: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 16 * 1024 * 1024,
            depth: 128,
            elements: 200_000,
            text_bytes: 1024 * 1024,
            attributes: 64,
            namespace_declarations: 256,
        }
    }
}

impl Limits {
    /// The parser's own node backstop, four times the element limit.
    #[must_use]
    pub fn parser_nodes(&self) -> u32 {
        u32::try_from(self.elements.saturating_mul(4)).unwrap_or(u32::MAX)
    }
}

/// A reason for refusing an input outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The named input is not a readable regular file.
    InputUnreadable,
    /// The input is larger than the byte limit.
    InputTooLarge,
    /// The input declares an encoding that is not supported.
    UnsupportedEncoding,
    /// The input does not decode under its declared encoding.
    InvalidEncoding,
    /// The input contains a DTD or another prohibited declaration.
    ProhibitedConstruct,
    /// The input exceeds a depth, node, text, or attribute limit.
    LimitExceeded,
    /// The input is not well-formed XML.
    MalformedXml,
}

impl Rejection {
    /// The stable machine-readable code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::InputUnreadable => "input_unreadable",
            Self::InputTooLarge => "input_too_large",
            Self::UnsupportedEncoding => "unsupported_encoding",
            Self::InvalidEncoding => "invalid_encoding",
            Self::ProhibitedConstruct => "prohibited_construct",
            Self::LimitExceeded => "limit_exceeded",
            Self::MalformedXml => "malformed_xml",
        }
    }

    /// The fixed diagnostic text. It never describes the input itself.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::InputUnreadable => "the input could not be read as a regular file",
            Self::InputTooLarge => "the input exceeds the 16 MiB limit",
            Self::UnsupportedEncoding => "only UTF-8 and ISO-8859-2 XML encodings are supported",
            Self::InvalidEncoding => "the input is not valid in its declared encoding",
            Self::ProhibitedConstruct => "DTD and entity declarations are not allowed",
            Self::LimitExceeded => {
                "the document exceeds a fixed depth, node, text, or attribute limit"
            }
            Self::MalformedXml => "the input is not well-formed XML",
        }
    }
}

/// How a well-formed document relates to the supported 2026 company form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormStatus {
    /// A supported wrapper containing exactly one known variant.
    Recognized,
    /// A supported wrapper that does not contain one known variant.
    UnrecognizedVariant,
    /// A wrapper element in a differently dated wrapper namespace.
    UnsupportedVersion,
    /// Any other root element.
    UnrecognizedRoot,
}

impl FormStatus {
    /// The stable machine-readable status.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Recognized => "recognized",
            Self::UnrecognizedVariant => "unrecognized_variant",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnrecognizedRoot => "unrecognized_root",
        }
    }

    /// The fixed diagnostic for a status that is reported as an error.
    #[must_use]
    pub fn message(self) -> Option<&'static str> {
        match self {
            Self::Recognized => None,
            Self::UnrecognizedVariant => {
                Some("the 2026 wrapper does not contain exactly one known form variant")
            }
            Self::UnsupportedVersion => Some("the form wrapper version is not supported"),
            Self::UnrecognizedRoot => Some("the root element is not a known company form"),
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
    /// Whether an `xml-stylesheet` instruction precedes the root element.
    pub stylesheet_pi: bool,
    /// Whether any element carries an XSI schema-location hint.
    pub schema_location_declared: bool,
}

/// The input facts established before a rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialInput {
    /// Size of the input in bytes.
    pub bytes: u64,
    /// Whether a UTF-8 byte-order mark was present, when that was reached.
    pub bom: Option<bool>,
    /// Whether an XML declaration was present, when that was reached.
    pub xml_declaration: Option<bool>,
    /// The selected encoding, when one was selected.
    pub encoding: Option<&'static str>,
}

impl PartialInput {
    /// The facts known when only the input size was established.
    #[must_use]
    pub fn sized(bytes: u64) -> Self {
        Self {
            bytes,
            bom: None,
            xml_declaration: None,
            encoding: None,
        }
    }
}

/// A rejection together with whatever was safely established beforehand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The reason the input was refused.
    pub code: Rejection,
    /// The input facts established before the failure, if any.
    pub input: Option<PartialInput>,
}

/// The root element's name, as far as it is reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootFacts {
    /// The root element's local name.
    pub local_name: String,
    /// The root element's namespace, or `None` when it is unnamespaced.
    pub namespace: Option<String>,
}

/// One known form variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The stable variant identifier.
    pub id: &'static str,
    /// The variant root's local name.
    pub local_name: String,
    /// The variant root's namespace.
    pub namespace: Option<String>,
}

/// One distinct direct child name of the variant root, with its count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The child element's local name.
    pub local_name: String,
    /// How many times it appears directly under the variant root.
    pub count: usize,
}

/// The recognition result for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFacts {
    /// The recognition status.
    pub status: FormStatus,
    /// The wrapper profile, when the wrapper is supported.
    pub profile: Option<&'static str>,
    /// The identified variant, when one was identified.
    pub variant: Option<Variant>,
    /// The variant root's sections, when a variant was identified.
    pub sections: Option<Vec<Section>>,
    /// Whether the section list was cut short.
    pub sections_truncated: bool,
}

impl FormFacts {
    /// The facts for a document whose root is not a supported wrapper.
    fn unrecognized(status: FormStatus) -> Self {
        Self {
            status,
            profile: None,
            variant: None,
            sections: None,
            sections_truncated: false,
        }
    }

    /// The facts for a supported wrapper without one known variant child.
    fn wrapper_only(status: FormStatus, profile: &'static str) -> Self {
        Self {
            profile: Some(profile),
            ..Self::unrecognized(status)
        }
    }
}

/// Structural counts taken from the parsed tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureFacts {
    /// Number of elements.
    pub elements: usize,
    /// Deepest element nesting, with the root counted as one.
    pub max_depth: usize,
    /// Number of attributes, excluding namespace declarations.
    pub attributes: usize,
    /// Total size of all text nodes in UTF-8 bytes.
    pub text_bytes: usize,
    /// The sorted set of element namespaces, capped.
    pub namespaces: Vec<String>,
    /// Whether the namespace list was cut short.
    pub namespaces_truncated: bool,
}

/// Everything one inspection established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// Facts about the input bytes and their encoding.
    pub input: InputFacts,
    /// Facts about the root element.
    pub root: RootFacts,
    /// The recognition result.
    pub form: FormFacts,
    /// Structural counts.
    pub structure: StructureFacts,
}

/// Cut a document-derived string to a fixed number of Unicode scalar values.
///
/// Element names and namespace URIs are the only document strings this crate
/// ever reports, and an untrusted document can make either arbitrarily long.
fn truncate_name(name: &str) -> String {
    crate::truncate_scalars(name, NAME_LIMIT)
}

/// The input facts established by the shared bounded parsing entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundedInput {
    /// Size of the input in bytes, including any byte-order mark.
    pub(crate) bytes: u64,
    /// Whether a UTF-8 byte-order mark was stripped.
    pub(crate) bom: bool,
    /// Whether an XML declaration was found by the sniff.
    pub(crate) xml_declaration: bool,
    /// The normalised label of the encoding that was applied.
    pub(crate) encoding: &'static str,
}

impl BoundedInput {
    /// A failure carrying everything established before it.
    pub(crate) fn failure(self, code: Rejection) -> Failure {
        Failure {
            code,
            input: Some(PartialInput {
                bytes: self.bytes,
                bom: Some(self.bom),
                xml_declaration: Some(self.xml_declaration),
                encoding: Some(self.encoding),
            }),
        }
    }
}

/// Parse already-read bytes under `limits` into a bounded tree.
///
/// This is the single entry point into decoding, the pre-parse scan, and
/// DTD-free tree parsing. The form classifier and the schema import audit both
/// go through it, so both refuse the same encodings, the same prohibited
/// declarations, and the same depth and node limits. The parsed tree borrows
/// the decoded text, which lives only for the call, so callers receive it
/// through `visit` rather than as a return value.
///
/// # Errors
///
/// Returns a [`Failure`] when the input is too large, is not decodable under
/// an accepted encoding, contains a prohibited construct, exceeds a limit, or
/// is not well-formed XML, and whatever `visit` itself returns.
pub(crate) fn parse_bounded<T>(
    bytes: &[u8],
    limits: &Limits,
    visit: impl FnOnce(&Document<'_>, BoundedInput) -> Result<T, Failure>,
) -> Result<T, Failure> {
    let size = bytes.len() as u64;
    if size > limits.input_bytes {
        return Err(Failure {
            code: Rejection::InputTooLarge,
            input: Some(PartialInput::sized(size)),
        });
    }

    let decoded = decode::decode(bytes).map_err(|error| Failure {
        code: error.code,
        input: Some(PartialInput {
            bytes: size,
            bom: error.bom,
            xml_declaration: error.xml_declaration,
            encoding: error.encoding,
        }),
    })?;
    let input = BoundedInput {
        bytes: size,
        bom: decoded.bom,
        xml_declaration: decoded.xml_declaration,
        encoding: decoded.encoding,
    };

    prescan::prescan(&decoded.text, limits).map_err(|code| input.failure(code))?;
    let document = tree::parse(&decoded.text, limits).map_err(|code| input.failure(code))?;
    visit(&document, input)
}

/// Inspect one already-read document under the given limits.
///
/// # Errors
///
/// Returns a [`Failure`] when the input is too large, is not decodable under
/// an accepted encoding, contains a prohibited construct, exceeds a limit, or
/// is not well-formed XML.
pub fn inspect(bytes: &[u8], limits: &Limits) -> Result<Inspection, Failure> {
    parse_bounded(bytes, limits, |document, input| {
        let (structure, schema_location_declared) =
            tree::walk(document, limits).map_err(|code| input.failure(code))?;
        Ok(Inspection {
            input: InputFacts {
                bytes: input.bytes,
                bom: input.bom,
                xml_declaration: input.xml_declaration,
                encoding: input.encoding,
                stylesheet_pi: tree::stylesheet_pi(document),
                schema_location_declared,
            },
            root: tree::root_facts(document),
            form: tree::classify(document),
            structure,
        })
    })
}

#[cfg(test)]
mod tests;
