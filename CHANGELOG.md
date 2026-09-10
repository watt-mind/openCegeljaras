# Changelog

## Unreleased

- Initial unpublished two-crate Rust scaffold for company-filing tooling.
- Capability, source-catalog, and embedded-skill introspection.
- Dated research, metadata-only schema inventory, architecture, and roadmap.
- Local checks and prepared CI/security workflows.
- Reviewed contract and implementation of the read-only `form inspect`
  operation: bounded decoding of UTF-8 and ISO-8859-2, a pre-parse scan for
  prohibited declarations, depth and node limits, DTD-free namespace-aware
  parsing, 2026 wrapper and variant recognition, and a new error envelope with
  exit code 1 for refused inputs and 4 for unrecognised documents.
- Synthetic company-form fixtures under `tests/fixtures/forms/`.
- Pre-parse caps of 64 attributes and 64 namespace declarations per element,
  because the tree parser compares each attribute and each declaration against
  every earlier one on the same element.
- `BSD-3-Clause` added to the licence allowlist for the `encoding_rs`
  ISO-8859-2 decoder.

No XSD validation, signature verification, submission, or receipt handling is
implemented. Recognising a form establishes none of them.
