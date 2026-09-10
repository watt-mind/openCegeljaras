---
name: opencegeljaras
description: Discover the capabilities and reference sources of the openCegeljaras development scaffold for Hungarian electronic company-court filings, recognise the format of a local company-form XML file, check a local mirror of the pinned official schemas against its digests, and check one form's XSD conformance against that verified mirror.
---

# openCegeljaras

Run `opencegeljaras capabilities --json` before choosing an operation.
The current stage is `scaffold`. The document operations are `form inspect`,
`form validate`, `schemas list`, and `schemas verify`; `capabilities`,
`sources`, `skill`, help, and version describe the tool itself.

## Inspecting a form

Run `opencegeljaras form inspect <PATH> --json` on a file you are allowed to
read. It reports the encoding, the wrapper and variant it recognises, and
structural counts for one local XML file. It never opens a network connection
and never resolves anything the document names.

Exit 0 means the file is a well-formed 2026 company form of a known variant.
That is format recognition only. Exit 0 never means the form is valid,
verified, complete, correct, accepted, or filed. XSD validation, cryptographic
verification, submission, and authority acceptance are **not performed** by
this tool, and `checks` reports each of them as `not_performed`.

Exit 4 means the file is well-formed XML but is not a supported 2026 form:
an unknown variant, an unsupported wrapper version, or an unrelated root.
Report it as "not a supported 2026 form", never as invalid or rejected by an
authority. Exit 1 means the input was refused before recognition (unreadable,
too large, an unsupported or invalid encoding, a prohibited DTD, a resource
limit, or malformed XML). Exit 2 is a usage error and exit 3 an output
failure. Diagnostics are fixed lines; do not add the path or file content to
them when relaying a failure.

## Checking a schema bundle

Run `opencegeljaras schemas list --json` to see the pinned schema manifest
compiled into the binary. It fetches nothing. `schemas list --urls` prints the
pinned URLs, one per line, so that an operator can mirror them with their own
tool into a directory laid out as `<host>/<path>`. Never present that list as
permission to redistribute the schemas, and do not download them on the
operator's behalf unless they asked.

Run `opencegeljaras schemas verify --bundle DIR --json` on a mirror the
operator already has. It compares each pinned resource's bytes against the
recorded SHA-256 digest and checks that every `xs:import` and `xs:include` in
the form graph resolves to a pinned URL. It reads only the files the manifest
names, opens no network connection, and writes nothing. Symlinks inside the
bundle directory are followed, so never run it with elevated privileges over a
directory the operator does not control.

Exit 0 means every pinned resource matched and the form graph's imports
audited. That is a digest and import audit of the operator's own copy. It is
**not** a redistribution licence, **not** XSD validation, and **not**
acceptance: `checks.xsd_conformance` is `not_performed`, and no form has been
validated against anything. Exit 4 means the mirror does not match the
manifest; read `data.problems` for the pinned URLs and their status
(`missing`, `mismatch`, `oversize`, `unparseable`, `unpinned_import`) and
report it as "the local mirror does not match the pinned digests", never as a
schema being wrong or an authority rejecting anything. Exit 1 means the bundle
directory or the manifest could not be read or is invalid. Read
`data.checks`, not the exit code alone. `--manifest FILE` replaces the pinned
manifest with an unpinned one; say so whenever a result was produced that way,
because the notice then ends with "unpinned manifest".

## Checking XSD conformance

Verify the bundle before you validate anything against it. Then run
`opencegeljaras form validate <PATH> --bundle DIR --json`. It inspects the
file, re-verifies the bundle itself, and runs one `xmllint` from libxml2
2.9.14 up to but excluding 2.13, which the operator installed: `--xmllint EXE`
names it, otherwise the first one on `PATH` is used. The accepted bytes are
copied into a private temporary directory, the child is spawned there with a
cleared environment and a fixed argument list, and nothing the document names
is ever resolved.

Read `data.checks.xsd_conformance`, not the exit code alone. Exit 0 with
`passed` means the form conforms to the pinned 2026 graph as the named libxml2
version interprets it. That is **not** the technical conditions, **not** a
signature check, and **not** acceptance: never call a conforming form valid,
accepted, or filed. Exit 4 means a check was performed and failed: either the
file is not a supported 2026 form, or `error.code` is `nonconformant` and
`data.findings` lists `line`, `element`, and `message` for each violation.
Exit 5 means a required check could not be performed at all — `bundle_unverified`,
`validator_unavailable`, or `validator_failed` — and `xsd_conformance` stays
`not_performed`; report it as "not checked", never as the form being wrong.

Findings have every document value replaced by `'…'`, and `values_redacted` is
`true`. Never pass `--show-values` unless the operator asked for the values:
it keeps them and can expose personal data. `unparsed_lines` counts validator
output that did not match the expected format; that text is never echoed.

Use `opencegeljaras sources --json` for a bundled, dated reference catalog.
This prints metadata without fetching sites or establishing current rules.
Consult the repository research and roadmap for proposed future commands;
do not call them as though they exist.

`ok: true` means the command ran; it is true only on exit 0 and never
describes validity, filing, or legal compliance. `verified: false` and
`submitted: false` mean no signature was verified and no case was submitted.
Never report a filing as valid, accepted, delivered, or legally sufficient
from this tool's output.

Company registration, legality supervision, e-Papír correspondence, and KRX
packaging are different concerns. Do not substitute one channel for another.
Future form/schema checks cannot replace registry checks or attorney review.

Do not read private fixtures or expose client identities, dossier contents,
signing material, or credentials. Read-only receipt inspection must never
acknowledge service. Any future signing, submission, payment, or service
acknowledgment requires an explicit, separately supported operation.
