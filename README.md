# openCegeljaras

A proposed local-first, agent-first Rust library and CLI for preparing and
checking Hungarian electronic company-court filings (**e-cégeljárás**) and
interpreting their receipts.

**Status: scaffold.** Alongside `capabilities`, `sources`, `skill`, help, and
version there are six read-only operations: `form inspect`, `form validate`,
`form fields`, `form diff`, `schemas list`, and `schemas verify`.
`form inspect` decodes a local form file under fixed limits, checks that it is
well-formed XML, and reports which company-form wrapper and variant it
carries. `form fields` lists a recognised form's fields as stable paths, and
`form diff` compares two of them field by field. `schemas list` prints the pinned
schema manifest compiled into the binary, and `schemas verify` checks a local
mirror you supplied yourself against those pinned digests and audits the form
graph's imports. `form validate` adds XSD conformance against that verified
mirror, through an `xmllint` you installed yourself. None of this proves that
a file is signed, correctly filled in, currently accepted, or filed: there is
still no dossier builder, signature verifier, submission transport, or receipt
parser.
No release has been published. This independent project is not a government
service.

## Why this project

The missing layer is the company procedure: versioned forms, required
documents, consistent data, preflight findings, and evidence of what happened
after dispatch. Container handling belongs in the existing tools:

| Project | Responsibility |
| :--- | :--- |
| openCegeljaras | Company-filing forms, procedure profiles, preflight, receipt meaning |
| [openSzigno](https://github.com/watt-mind/openSzigno) | ES3 dossier inspection, creation, signing, verification |
| [openKRX](https://github.com/watt-mind/openKRX) | KRX transport packages where the selected channel requires them |
| [openPapir](https://github.com/watt-mind/openPapir) | Correspondence and evidence workflows |
| [openASiC](https://github.com/watt-mind/openASiC) | Proposed ASiC container tooling; separate implementation status |

These are architectural boundaries, not dependencies or an assertion that
every proposed integration is available. Ordinary company registration is a
distinct procedure; packaging something as KRX or sending it through e-Papír
does not make it an acceptable company-registration filing.

The [research and recommendation](docs/research.md) explains the official
specifications, current changes, investment rationale, and remaining gates.
The [source inventory](docs/specifications.md) distinguishes downloaded schema
evidence from indexed guidance and does not distribute third-party XSD files.

## Try the scaffold

Build with Rust 1.88 or newer:

```sh
cargo run --locked -p opencegeljaras-cli -- --help
cargo run --locked -p opencegeljaras-cli -- capabilities --json
cargo run --locked -p opencegeljaras-cli -- sources --json
cargo run --locked -p opencegeljaras-cli -- skill
cargo run --locked -p opencegeljaras-cli -- form inspect \
  tests/fixtures/forms/cegbejegyzesi-minimal-utf8.xml --json
cargo run --locked -p opencegeljaras-cli -- schemas list --json
cargo run --locked -p opencegeljaras-cli -- schemas verify \
  --bundle tests/fixtures/schemas/good \
  --manifest tests/fixtures/schemas/manifest.json --json
cargo run --locked -p opencegeljaras-cli -- form validate \
  tests/fixtures/forms/nevfoglalasi-conforming.xml --bundle YOUR_MIRROR --json
cargo run --locked -p opencegeljaras-cli -- form fields \
  tests/fixtures/forms/fields-repeated-siblings.xml --json
cargo run --locked -p opencegeljaras-cli -- form diff \
  tests/fixtures/forms/fields-repeated-siblings.xml \
  tests/fixtures/forms/fields-repeated-siblings-edited.xml --json
```

`capabilities` reports `stage: "scaffold"` and
`operations: ["form inspect", "form validate", "form fields", "form diff",
"schemas list", "schemas verify"]`.
Introspection commands are listed separately from document operations.
`form inspect` exits 0 for a recognised 2026 form, 1 for an input it refuses,
and 4 for a well-formed document that is not a supported form; recognition is
format recognition only. A successful command
reports `ok: true`, `verified: false`, and `submitted: false`: it has neither
verified a signature nor filed a case. `sources` prints a bundled research
catalog; it does not contact the listed sites or check whether guidance changed.
A rejected command line exits 2: with `--json` before any `--` terminator it
writes one `usage.arguments` error object on stdout and nothing on stderr,
otherwise one fixed stderr line. Neither form repeats a rejected argument, and
`--help` and `--version` stay plain text on exit 0.

No official schema is vendored here. `schemas list --urls` prints the pinned
URLs so that you can mirror them yourself with your own tool; `schemas verify`
then compares your mirror's bytes against the pinned SHA-256 digests and checks
that every `xs:import` and `xs:include` in the form graph resolves to a pinned
URL. That is a digest and import audit of your own copy: it is not a
redistribution licence, not XSD validation, and not acceptance by any
authority. It exits 4 when the mirror does not match, and never downloads,
writes, or modifies anything.

## Checking XSD conformance

`form validate PATH --bundle DIR` inspects the file, verifies the bundle, and
then runs one `xmllint` from libxml2 2.9.14 up to but excluding 2.13, which you
install yourself: `--xmllint EXE` names it, otherwise the first one on `PATH`
is used. The bytes the inspector accepted are copied into a private temporary
directory as `document.xml`, and the validator is spawned once there with a
cleared environment and exactly `--nonet --noout --schema ROOT document.xml`.
Nothing the document names is ever resolved and no catalog is used.

Exit 0 means the form conforms to the pinned 2026 graph as that libxml2 version
interprets it. It is **not** the technical conditions, **not** a signature
check, and **not** acceptance by any authority. Exit 4 means a check was
performed and failed: the document is not a recognised form, or it is
nonconforming and `data.findings` lists the violations. Exit 5 is new and means
a required check could not be performed at all — an unverified bundle, no
supported validator, or a validator that produced no result — so conformance is
reported as `not_performed` rather than as a failure. Findings have every
document value replaced by `'…'`; `--show-values` keeps them and can expose
personal data, so pass it only when you want the values.

## Reading and comparing fields

`form fields PATH` lists every field of a recognised form: each leaf element
and attribute as a path of element names from the variant root down, such as
`Adatlap[1]/TagoltCím/Hely` or `@Cégforma`. Paths are **positional**: an
element with a same-named sibling carries a 1-based index, so removing the
first of three entries renumbers the rest and one such edit shows up as
several changes plus a removal. `form diff OLD NEW` compares two forms by
those paths and reports `added`, `removed`, `changed`, and `unchanged`; it
exits 0 whenever both files are recognised forms, so read `identical` rather
than the exit code, and exits 4 naming `old_not_recognized` or
`new_not_recognized` when a side is not a supported 2026 form. NEW is never
opened when OLD fails.

Values are personal and company data: they are hidden by default and shown
only with `--show-values`, which can expose them, and they never reach
standard error. Field lengths are always reported, so an agent can see that a
field is empty or filled without reading it. Neither command validates,
verifies, or accepts anything.

## Intended workflow

The [roadmap](docs/roadmap.md) proposes a read-only form inspector and pinned
schema support first, then field validation and dossier preflight. Subsequent
work can add deterministic form creation, dossier integration, and receipt
interpretation. Transport and acknowledgment of service require a separately
reviewed integration. Proposed commands are documented, not installed stubs.

Local checks cannot establish court acceptance. Unknown schema versions and
unperformed checks must remain explicit. Cryptographic verification belongs
to a verifier with caller-supplied trust evidence; legal review belongs to the
responsible attorney.

## Development

Two unpublished crates, `opencegeljaras-core` and `opencegeljaras-cli`, use
Rust edition 2024 and the [MIT license](LICENSE).

```sh
./scripts/check.sh
cargo build --release --locked
```

See [contributing](CONTRIBUTING.md), [security](SECURITY.md), and the
[documentation index](docs/index.md). Public fixtures must be synthetic.
`samples/` is reserved for ignored private inputs and is never part of tests
or diagnostic output. GitHub publishing, releases, and Factory dispatch are
not configured by this scaffold.

## Claude orchestration

On the development server, start Claude in this checkout and ask it:

```text
Read ./orchestrator.md and begin the bounded run it describes.
```

The [orchestrator prompt](orchestrator.md) handles the initial local scaffold
without requiring a remote or ticket queue. It starts with a reviewed form
inspection contract, then bounded implementation and independent review.
It also supports an already configured project workflow. Checkpoints remain
in ignored `tmp/`; launching this prompt does not configure a background service.
