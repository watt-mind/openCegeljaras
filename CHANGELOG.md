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
- Reviewed contract and implementation of the read-only `schemas list` and
  `schemas verify` operations: the pinned 149-resource schema manifest compiled
  into the binary, strict manifest path and digest rules, the 139-resource form
  closure, SHA-256 comparison of an operator-supplied local mirror, and an
  `xs:import`/`xs:include` audit of the form closure with RFC 3986 relative
  resolution. Exit 4 reports a bundle that does not match its manifest; exit 1
  reports an unreadable bundle or an unreadable or invalid manifest.
- A shared "parse bytes under `Limits` into a bounded tree" entry point in the
  core, used by both the form classifier and the schema import audit, with a
  raised namespace-declaration cap for schema resources only.
- Synthetic CC0 schema fixtures under `tests/fixtures/schemas/`.
- `sha2` 0.11 added for digesting already-read bytes.
- `BSD-3-Clause` added to the licence allowlist for the `encoding_rs`
  ISO-8859-2 decoder.

No XSD validation, signature verification, submission, or receipt handling is
implemented. Recognising a form establishes none of them, and verifying a
schema bundle is a digest and import audit of the operator's own copy: not a
redistribution licence, not validation, and not acceptance.
