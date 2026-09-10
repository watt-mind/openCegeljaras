---
name: opencegeljaras
description: Discover the capabilities and reference sources of the openCegeljaras development scaffold for Hungarian electronic company-court filings.
---

# openCegeljaras

Run `opencegeljaras capabilities --json` before choosing an operation.
The current stage is `scaffold`, with no document or workflow operations.
Only `capabilities`, `sources`, `skill`, help, and version are available.

Use `opencegeljaras sources --json` for a bundled, dated reference catalog.
This prints metadata without fetching sites or establishing current rules.
Consult the repository research and roadmap for proposed future commands;
do not call them as though they exist.

`ok: true` means introspection succeeded. `verified: false` and
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
