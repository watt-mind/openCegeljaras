//! Byte-order-mark handling, declaration sniffing, and the two accepted decodes.
//!
//! The policy is deliberately narrow: only the UTF-8 and ISO-8859-2 encodings
//! the official schemas and the dossier profile declare are accepted, and the
//! declared label is read by this module rather than by the XML parser.

use crate::form::{Rejection, UTF_8_LABEL};

/// The longest XML declaration this module is willing to look at.
const DECLARATION_WINDOW: usize = 256;

/// Text decoded under a single accepted encoding, with the sniffed facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decoded {
    /// The decoded document text, without any byte-order mark.
    pub(crate) text: String,
    /// Whether a UTF-8 byte-order mark was stripped.
    pub(crate) bom: bool,
    /// Whether an XML declaration was found by the sniff.
    pub(crate) xml_declaration: bool,
    /// The normalised label of the encoding that was applied.
    pub(crate) encoding: &'static str,
}

/// A decode failure together with the facts established before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodeError {
    /// The reported rejection code.
    pub(crate) code: Rejection,
    /// Whether a UTF-8 byte-order mark was stripped, when that was reached.
    pub(crate) bom: Option<bool>,
    /// Whether an XML declaration was found, when the sniff got that far.
    pub(crate) xml_declaration: Option<bool>,
    /// The selected encoding, when one was selected.
    pub(crate) encoding: Option<&'static str>,
}

impl DecodeError {
    fn new(code: Rejection) -> Self {
        Self {
            code,
            bom: None,
            xml_declaration: None,
            encoding: None,
        }
    }

    fn with_bom(code: Rejection, bom: bool) -> Self {
        Self {
            bom: Some(bom),
            ..Self::new(code)
        }
    }
}

/// The ISO-8859-2 label, normalised as it is reported.
pub(crate) const ISO_8859_2_LABEL: &str = "ISO-8859-2";

const UTF_8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF_16_BE_BOM: &[u8] = &[0xFE, 0xFF];
const UTF_16_LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF_32_BE_BOM: &[u8] = &[0x00, 0x00, 0xFE, 0xFF];
const UTF_32_LE_BOM: &[u8] = &[0xFF, 0xFE, 0x00, 0x00];

fn is_xml_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

/// Decode `bytes` under the accepted encoding policy.
///
/// The steps are applied in the order recorded in the form inspection
/// contract: strip a UTF-8 byte-order mark, reject UTF-16/UTF-32 signatures,
/// sniff the declaration, select an encoding, then decode.
pub(crate) fn decode(bytes: &[u8]) -> Result<Decoded, DecodeError> {
    let bom = bytes.starts_with(UTF_8_BOM);
    let body = if bom {
        &bytes[UTF_8_BOM.len()..]
    } else {
        bytes
    };

    if has_wide_signature(body) {
        return Err(DecodeError::with_bom(Rejection::UnsupportedEncoding, bom));
    }

    let sniff = sniff_declaration(body).map_err(|code| DecodeError::with_bom(code, bom))?;
    let encoding = match sniff.label {
        None => UTF_8_LABEL,
        Some(label) => normalise_label(label).ok_or(DecodeError {
            code: Rejection::UnsupportedEncoding,
            bom: Some(bom),
            xml_declaration: Some(sniff.declaration),
            encoding: None,
        })?,
    };

    if bom && encoding == ISO_8859_2_LABEL {
        return Err(DecodeError {
            code: Rejection::InvalidEncoding,
            bom: Some(bom),
            xml_declaration: Some(sniff.declaration),
            encoding: Some(encoding),
        });
    }

    let invalid = || DecodeError {
        code: Rejection::InvalidEncoding,
        bom: Some(bom),
        xml_declaration: Some(sniff.declaration),
        encoding: Some(encoding),
    };
    let text = decode_body(body, encoding).ok_or_else(invalid)?;

    Ok(Decoded {
        text,
        bom,
        xml_declaration: sniff.declaration,
        encoding,
    })
}

/// A UTF-16 or UTF-32 signature, or a NUL byte in the first four bytes.
fn has_wide_signature(body: &[u8]) -> bool {
    body.starts_with(UTF_32_LE_BOM)
        || body.starts_with(UTF_32_BE_BOM)
        || body.starts_with(UTF_16_LE_BOM)
        || body.starts_with(UTF_16_BE_BOM)
        || body.iter().take(4).any(|&byte| byte == 0)
}

/// What the declaration sniff established.
struct Sniff {
    declaration: bool,
    label: Option<Vec<u8>>,
}

/// Find the XML declaration and its `encoding` pseudo-attribute, if any.
fn sniff_declaration(body: &[u8]) -> Result<Sniff, Rejection> {
    if !(body.starts_with(b"<?xml") && body.get(5).copied().is_some_and(is_xml_space)) {
        return Ok(Sniff {
            declaration: false,
            label: None,
        });
    }

    let window = &body[..body.len().min(DECLARATION_WINDOW)];
    let end = window
        .windows(2)
        .position(|pair| pair == b"?>")
        .ok_or(Rejection::MalformedXml)?;

    Ok(Sniff {
        declaration: true,
        label: encoding_label(&body[5..end])?,
    })
}

/// Read the `encoding` pseudo-attribute value out of a declaration body.
///
/// The name must appear whole, preceded by XML whitespace, followed by `=`
/// with optional whitespace and a quoted value.
///
/// # Errors
///
/// A value whose opening quote is never closed inside the declaration is
/// `malformed_xml`: the declaration ran past the `?>` this sniff found.
fn encoding_label(declaration: &[u8]) -> Result<Option<Vec<u8>>, Rejection> {
    let name = b"encoding";
    for start in 1..declaration.len() {
        if !declaration[start..].starts_with(name) || !is_xml_space(declaration[start - 1]) {
            continue;
        }
        let mut cursor = start + name.len();
        while declaration.get(cursor).copied().is_some_and(is_xml_space) {
            cursor += 1;
        }
        if declaration.get(cursor) != Some(&b'=') {
            continue;
        }
        cursor += 1;
        while declaration.get(cursor).copied().is_some_and(is_xml_space) {
            cursor += 1;
        }
        let quote = match declaration.get(cursor).copied() {
            Some(quote) if quote == b'"' || quote == b'\'' => quote,
            _ => continue,
        };
        cursor += 1;
        let value = &declaration[cursor..];
        let end = value
            .iter()
            .position(|&byte| byte == quote)
            .ok_or(Rejection::MalformedXml)?;
        return Ok(Some(value[..end].to_vec()));
    }
    Ok(None)
}

/// Normalise a declared label to one of the two accepted encodings.
fn normalise_label(label: Vec<u8>) -> Option<&'static str> {
    let normalised: Vec<u8> = label
        .iter()
        .map(|byte| match byte {
            b'_' => b'-',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    match normalised.as_slice() {
        b"utf-8" | b"utf8" => Some(UTF_8_LABEL),
        b"iso-8859-2" | b"iso8859-2" => Some(ISO_8859_2_LABEL),
        _ => None,
    }
}

/// Decode under the selected encoding, refusing any replacement.
fn decode_body(body: &[u8], encoding: &'static str) -> Option<String> {
    if encoding == ISO_8859_2_LABEL {
        let (text, had_errors) = encoding_rs::ISO_8859_2.decode_without_bom_handling(body);
        if had_errors {
            return None;
        }
        return Some(text.into_owned());
    }
    std::str::from_utf8(body).ok().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded(bytes: &[u8]) -> Decoded {
        decode(bytes).expect("input should decode")
    }

    #[test]
    fn plain_utf8_without_a_declaration_is_accepted() {
        let result = decoded(b"<a/>");
        assert_eq!(result.encoding, UTF_8_LABEL);
        assert!(!result.bom);
        assert!(!result.xml_declaration);
        assert_eq!(result.text, "<a/>");
    }

    #[test]
    fn a_utf8_byte_order_mark_is_stripped_and_reported() {
        let mut bytes = UTF_8_BOM.to_vec();
        bytes.extend_from_slice(b"<a/>");
        let result = decoded(&bytes);
        assert!(result.bom);
        assert_eq!(result.text, "<a/>");
    }

    #[test]
    fn wide_signatures_and_embedded_nul_are_unsupported() {
        for bytes in [
            [0xFF, 0xFE, b'<', b'a'].as_slice(),
            [0xFE, 0xFF, b'<', b'a'].as_slice(),
            [0x00, 0x00, 0xFE, 0xFF].as_slice(),
            [0xFF, 0xFE, 0x00, 0x00].as_slice(),
            [b'<', 0x00, b'a', b'/'].as_slice(),
        ] {
            let error = decode(bytes).expect_err("wide encodings are refused");
            assert_eq!(error.code, Rejection::UnsupportedEncoding);
            assert_eq!(error.encoding, None);
        }
    }

    #[test]
    fn a_stylesheet_instruction_is_not_a_declaration() {
        let result = decoded(b"<?xml-stylesheet href='x'?><a/>");
        assert!(!result.xml_declaration);
        assert_eq!(result.encoding, UTF_8_LABEL);
    }

    #[test]
    fn an_unclosed_declaration_is_malformed() {
        let mut bytes = b"<?xml version='1.0' encoding='utf-8' ".to_vec();
        bytes.resize(4096, b' ');
        let error = decode(&bytes).expect_err("an unclosed declaration is refused");
        assert_eq!(error.code, Rejection::MalformedXml);
    }

    #[test]
    fn labels_are_matched_whole_and_case_insensitively() {
        for (declaration, expected) in [
            ("<?xml version='1.0' encoding='UTF_8'?><a/>", UTF_8_LABEL),
            ("<?xml version='1.0' encoding=\"utf8\"?><a/>", UTF_8_LABEL),
            (
                "<?xml version='1.0' encoding = 'Iso-8859-2'?><a/>",
                ISO_8859_2_LABEL,
            ),
            (
                "<?xml version='1.0' encoding='iso8859_2'?><a/>",
                ISO_8859_2_LABEL,
            ),
            ("<?xml version='1.0'?><a/>", UTF_8_LABEL),
            ("<?xml version='1.0' xencoding='cp1250'?><a/>", UTF_8_LABEL),
            (
                "<?xml version='1.0' encodingx='cp1250' encoding='utf-8'?><a/>",
                UTF_8_LABEL,
            ),
        ] {
            let result = decoded(declaration.as_bytes());
            assert_eq!(result.encoding, expected, "declaration: {declaration}");
            assert!(result.xml_declaration);
        }
    }

    #[test]
    fn other_labels_are_unsupported() {
        let error = decode(b"<?xml version='1.0' encoding='windows-1250'?><a/>")
            .expect_err("other labels are refused");
        assert_eq!(error.code, Rejection::UnsupportedEncoding);
        assert_eq!(error.xml_declaration, Some(true));
    }

    #[test]
    fn a_byte_order_mark_with_an_iso_label_is_invalid() {
        let mut bytes = UTF_8_BOM.to_vec();
        bytes.extend_from_slice(b"<?xml version='1.0' encoding='iso-8859-2'?><a/>");
        let error = decode(&bytes).expect_err("a mismatched mark is refused");
        assert_eq!(error.code, Rejection::InvalidEncoding);
        assert_eq!(error.encoding, Some(ISO_8859_2_LABEL));
    }

    #[test]
    fn iso_8859_2_bytes_decode_to_hungarian_names() {
        let mut bytes = b"<?xml version='1.0' encoding='iso-8859-2'?><N".to_vec();
        bytes.extend_from_slice(&[0xE9, 0x76]);
        bytes.extend_from_slice(b"/>");
        let result = decoded(&bytes);
        assert_eq!(result.encoding, ISO_8859_2_LABEL);
        assert!(result.text.ends_with("<Név/>"));
    }

    #[test]
    fn iso_8859_2_bytes_under_a_utf8_label_are_invalid() {
        let mut bytes = b"<?xml version='1.0' encoding='utf-8'?><N".to_vec();
        bytes.extend_from_slice(&[0xE9, 0x76]);
        bytes.extend_from_slice(b"/>");
        let error = decode(&bytes).expect_err("undecodable bytes are refused");
        assert_eq!(error.code, Rejection::InvalidEncoding);
        assert_eq!(error.encoding, Some(UTF_8_LABEL));
    }

    #[test]
    fn an_unterminated_quoted_label_is_malformed() {
        for bytes in [
            b"<?xml version='1.0' encoding='utf-8 ?><a/>".as_slice(),
            b"<?xml version=\"1.0\" encoding=\"?>windows-1250\"?><a/>".as_slice(),
        ] {
            let error = decode(bytes).expect_err("an unterminated value is refused");
            assert_eq!(error.code, Rejection::MalformedXml);
            assert_eq!(error.encoding, None);
        }
    }
}
