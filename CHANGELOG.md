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
- Pre-parse caps of 64 attributes and 256 namespace declarations per element,
  because the tree parser compares each attribute and each declaration against
  every earlier one on the same element. The declaration cap is 256 rather than
  64 because the official variant schemas declare 82 and 85 prefixes on their
  root element.
- Reviewed contract and implementation of the read-only `schemas list` and
  `schemas verify` operations: the pinned 149-resource schema manifest compiled
  into the binary, strict manifest path and digest rules, the 139-resource form
  closure, SHA-256 comparison of an operator-supplied local mirror, and an
  `xs:import`/`xs:include` audit of the form closure with RFC 3986 relative
  resolution. Exit 4 reports a bundle that does not match its manifest; exit 1
  reports an unreadable bundle or an unreadable or invalid manifest.
- A shared "parse bytes under `Limits` into a bounded tree" entry point in the
  core, used by both the form classifier and the schema import audit under one
  default `Limits`.
- Offending import locations are de-duplicated in first-appearance order and
  capped at 64 per resource, with `locations_truncated` on every `problems`
  entry.
- Synthetic CC0 schema fixtures under `tests/fixtures/schemas/`.
- Reviewed contract and implementation of the read-only `form validate`
  operation: the `form inspect` boundary, the package 3 bundle verification,
  and one external `xmllint` from libxml2 2.9.14 up to but excluding 2.13, run
  once over a private temporary copy of the accepted bytes with a cleared
  environment and a fixed argument list. Exit 5 is new and means a required
  check could not be performed (`bundle_unverified`, `validator_unavailable`,
  `validator_failed`); exit 4 reports a recognised form that is
  `nonconformant`. Findings carry every document value replaced by `'…'`
  unless `--show-values` is passed, and a quoted span survives only where
  libxml2 never prints a value. Any warning that the schema graph did not load
  completely is `validator_failed` whatever the exit code, and the
  standard-error reader is bounded by the same deadline as the child.
- Synthetic conforming and non-conforming name-reservation fixtures under
  `tests/fixtures/forms/`, recorded as agreeing with two independent XSD
  implementations on 2026-09-10.
- `sha2` 0.11 added for digesting already-read bytes.
- `tempfile` 3.27 added for the private temporary directory the validator
  reads.
- `BSD-3-Clause` added to the licence allowlist for the `encoding_rs`
  ISO-8859-2 decoder.

No signature verification, submission, or receipt handling is implemented.
Recognising a form establishes none of them, verifying a schema bundle is a
digest and import audit of the operator's own copy, and XSD conformance is not
the technical conditions, not a signature check, and not acceptance by any
authority.
