---
name: opencegeljaras
description: Discover the capabilities and reference sources of the openCegeljaras development scaffold for Hungarian electronic company-court filings, recognise the format of a local company-form XML file, list its fields as stable paths or compare two forms field by field, check a local mirror of the pinned official schemas against its digests, and check one form's XSD conformance against that verified mirror.
---

# openCegeljaras

Run `opencegeljaras capabilities --json` before choosing an operation.
The current stage is `scaffold`. The document operations are `form inspect`,
`form validate`, `form fields`, `form diff`, `schemas list`, and
`schemas verify`; `capabilities`, `sources`, `skill`, help, and version
describe the tool itself.

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

Branch on the exit status first, then on `ok`, then on `error.code`; never on
the wording of a message. Exit 2 with `error.code` `usage.arguments` means the
invocation itself was wrong — an unknown command, a missing or extra argument,
or conflicting options — not that the document or bundle was bad: fix the
command line and run `opencegeljaras --help`. That envelope appears on stdout
only when `--json` is in the arguments before any `--` terminator; otherwise
exit 2 is one fixed stderr line and stdout is empty.

## Listing and comparing fields

Run `opencegeljaras form fields <PATH> --json` on a recognised form to list
its fields as paths, and `opencegeljaras form diff OLD NEW --json` to compare
two of them field by field. Both run the `form inspect` boundary first, so
exit 1 and exit 4 mean there what they mean for `form inspect`; `form diff`
reads OLD first, never opens NEW when OLD fails, and reports `error.code`
`old_not_recognized` or `new_not_recognized` at exit 4 with `data.side` naming
the side. Exit 0 from `form diff` means both documents are recognised, whether
or not they differ: read `data.identical`, never the exit code, to learn
whether anything changed.

A path is the element local names from the variant root down, joined with
`/`, with attributes as `@name`. **Paths are positional**: every element that
has a same-named sibling carries a 1-based index, so inserting or removing one
entry renumbers the others. A single such edit therefore surfaces as several
`changed` entries plus one `added` or `removed` entry; say so rather than
reporting that many fields were edited. `field_count` is the number of listed
fields, and `fields_truncated` marks a listing that stopped at the 10 000-field
cap: entries near the cap may then be artefacts of it, and `identical` is
false whatever else matched.

Field values are personal and company data. They are hidden by default, and
`values_redacted` is `true`; `value_length` still shows whether a field is
empty or filled. Pass `--show-values` **only** when the operator asked to see
or compare values, and say that the output then contains personal and company
data. Values never reach standard error.

Listing or comparing fields is neither validation, verification, nor
acceptance. It says nothing about whether a form is schema-valid, signed,
complete, correct, currently accepted, or filed.

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

A message whose values cannot be removed unambiguously is reported as the
fixed text `message redacted`. Findings otherwise have every document value
replaced by `'…'`, and `values_redacted` is `true`. Never pass
`--show-values` unless the operator asked for the values: it keeps them and
can expose personal data. `unparsed_lines` counts validator
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
