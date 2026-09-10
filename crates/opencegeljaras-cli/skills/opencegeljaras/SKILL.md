---
name: opencegeljaras
description: Discover the capabilities and reference sources of the openCegeljaras development scaffold for Hungarian electronic company-court filings, and recognise the format of a local company-form XML file.
---

# openCegeljaras

Run `opencegeljaras capabilities --json` before choosing an operation.
The current stage is `scaffold`. The only document operation is
`form inspect`; `capabilities`, `sources`, `skill`, help, and version
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
