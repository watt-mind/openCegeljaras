//! Strict, bounded base64 decoding of one document payload.
//!
//! The XML whitespace an `xs:base64Binary` value may carry is removed in one
//! pass, the resulting length is compared against the payload limit *before*
//! any decoding, and only then are the characters decoded. The alphabet is the
//! standard one, padding is required, and no other character is accepted: a
//! payload this refuses is reported as refused rather than guessed at.

/// The largest decoded payload accepted, in bytes.
///
/// It is the form profile's own input limit: a payload larger than that could
/// not be inspected as a form anyway.
pub(crate) const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
/// The largest accepted base64 text, in characters, after whitespace removal.
pub(crate) const MAX_CHARACTERS: usize = MAX_PAYLOAD_BYTES * 4 / 3 + 4;
/// The padding character, which is required and only ever trails.
const PAD: u8 = b'=';

/// Why a payload was not decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    /// The base64 text is longer than the payload limit allows.
    TooLarge,
    /// The text is not strict, padded base64.
    Invalid,
}

/// Append one base64 text to a buffer, dropping XML whitespace in one pass.
pub(crate) fn absorb(text: &str, packed: &mut Vec<u8>) {
    packed.extend(
        text.bytes()
            .filter(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n')),
    );
}

/// The whitespace-free form of one base64 text.
#[cfg(test)]
pub(crate) fn packed(text: &str) -> Vec<u8> {
    let mut packed = Vec::new();
    absorb(text, &mut packed);
    packed
}

/// The six-bit value of one standard base64 alphabet character.
fn sextet(byte: u8) -> Option<u32> {
    let value = match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(value))
}

/// Decode one whitespace-free base64 text under the payload limit.
///
/// # Errors
///
/// Returns [`Error::TooLarge`] when the text is longer than the limit, which
/// is decided before anything is decoded, and [`Error::Invalid`] when the text
/// is not a multiple of four characters, carries a character outside the
/// standard alphabet, or pads anywhere but at its very end.
pub(crate) fn decode(packed: &[u8]) -> Result<Vec<u8>, Error> {
    if packed.len() > MAX_CHARACTERS {
        return Err(Error::TooLarge);
    }
    if !packed.len().is_multiple_of(4) {
        return Err(Error::Invalid);
    }
    let mut decoded = Vec::with_capacity(packed.len() / 4 * 3);
    let (blocks, _) = packed.as_chunks::<4>();
    let count = blocks.len();
    for (index, block) in blocks.iter().enumerate() {
        let last = index + 1 == count;
        let padding = block.iter().filter(|byte| **byte == PAD).count();
        if padding > 2 || (padding > 0 && !last) {
            return Err(Error::Invalid);
        }
        if padding > 0 && block[4 - padding..].iter().any(|byte| *byte != PAD) {
            return Err(Error::Invalid);
        }
        let mut packed_bits = 0u32;
        for byte in &block[..4 - padding] {
            packed_bits = (packed_bits << 6) | sextet(*byte).ok_or(Error::Invalid)?;
        }
        packed_bits <<= 6 * padding;
        let bytes = packed_bits.to_be_bytes();
        decoded.extend_from_slice(&bytes[1..4 - padding]);
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) -> Vec<u8> {
        decode(&packed(text)).expect("the fixture should decode")
    }

    #[test]
    fn whitespace_is_removed_before_the_length_is_judged() {
        assert_eq!(packed(" a\tb\r\nc "), b"abc");
        assert_eq!(round_trip("aGVs\n bG8="), b"hello");
    }

    #[test]
    fn the_standard_alphabet_decodes_with_both_paddings() {
        assert_eq!(round_trip(""), b"");
        assert_eq!(round_trip("aGVsbG8="), b"hello");
        assert_eq!(round_trip("aGVsbG9v"), b"helloo");
        assert_eq!(round_trip("aQ=="), b"i");
        assert_eq!(round_trip("+/8="), [0xfb, 0xff]);
    }

    #[test]
    fn unpadded_misplaced_and_foreign_characters_are_refused() {
        for text in ["aGVsbG8", "a===", "=aaa", "aa=a", "aa=", "aa$a", "aaaa="] {
            assert_eq!(decode(&packed(text)), Err(Error::Invalid), "{text}");
        }
    }

    #[test]
    fn a_text_past_the_character_limit_is_refused_before_decoding() {
        let oversized = vec![b'A'; MAX_CHARACTERS + 1];
        assert_eq!(decode(&oversized), Err(Error::TooLarge));
        let at_limit = vec![b'A'; MAX_CHARACTERS - MAX_CHARACTERS % 4];
        assert!(decode(&at_limit).is_ok());
    }
}
