# Changelog

## Unreleased

- Reviewed contract and implementation of the read-only `dossier inspect` and
  `dossier preflight` operations over one 2026 company-court e-dossier: a
  second bounded `Limits` profile, dossier recognition, the schema's 194
  document-role patterns as compiled interoperability data, strict base64
  decoding of one plain-base64 XML payload with a pre-decode length check, the
  carried form's `form inspect` result, eleven deterministic findings with
  fixed details, and XSD conformance of the outer dossier against a
  digest-verified dossier closure through the existing `xmllint` adapter.
- New exit-5 code `payload_exceeds_validator_limit` for a text node past the
  validator's own 10 000 000-byte ceiling, reported without spawning it.
- `schemas verify` reports `dossier_closure_verified` alongside
  `form_closure_verified`, additively.
- Refuse a path that is not a regular file before opening it, so that a FIFO
  named as a form, a dossier, a manifest, or a bundle resource can no longer
  block the command.
- Synthetic company-dossier fixtures under `tests/fixtures/dossiers/`.
- Parse on a dedicated 16 MiB stack so the 128-level depth limit holds on
  Windows and in debug builds.
- Install the XSD validator in the coverage job, run CodeQL only where code
  scanning uploads are available, and keep fixture bytes exact on checkout.
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
- A JSON usage-error contract, matching the reviewed sibling CLI: a rejected
  command line whose raw arguments carry the exact option `--json` before any
  `--` terminator writes one `usage.arguments` error envelope on stdout, with
  `command: "usage"` and `data: null`, nothing on stderr, and exit 2. Without
  `--json`, and with `--json` only after `--`, the existing fixed stderr line
  and empty stdout are unchanged, as are plain-text `--help` and `--version` on
  exit 0. No rejected argument is echoed in either form.
- The incomplete-graph markers that make an `xmllint` run `validator_failed`
  are now looked for only on standard-error lines that are not themselves
  `document.xml:LINE: element NAME: Schemas validity error :` findings, so a
  document value quoting a marker's text stays an ordinary finding.
- Reviewed contract and implementation of the read-only `form fields` and
  `form diff` operations: the `form inspect` boundary followed by a bounded
  walk of the variant root that lists every leaf element, attribute, and mixed
  element as a positional path, and a comparison of two such listings by their
  untruncated paths. Paths carry a 1-based index on every component with a
  same-named sibling, are cut at 128 scalars per component and 1024 per path,
  and at most 10 000 fields are listed. Values are cut at 4096 scalars, are
  emitted only with `--show-values`, and never reach standard error; value
  lengths are always reported. `form diff` reads OLD first, never opens NEW
  when OLD fails, exits 4 with `old_not_recognized` or `new_not_recognized`,
  and exits 0 whenever both sides are recognised, with the difference in
  `data.identical` rather than in the exit code.
- A field is keyed internally by one index into a component arena rather than
  by an owned joined path, so a deep chain of long names over many leaves
  costs the arena rather than the product of depth and field count, and
  siblings are counted once per parent so that a very wide parent stays
  linear.
- Synthetic field-walk fixtures under `tests/fixtures/forms/`.

No signature verification, submission, or receipt handling is implemented.
Recognising a form establishes none of them, verifying a schema bundle is a
digest and import audit of the operator's own copy, and XSD conformance is not
the technical conditions, not a signature check, and not acceptance by any
authority.
