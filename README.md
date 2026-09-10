# openCegeljaras

A proposed local-first, agent-first Rust library and CLI for preparing and
checking Hungarian electronic company-court filings (**e-cégeljárás**) and
interpreting their receipts.

**Status: scaffold.** Only `capabilities`, `sources`, `skill`, help, and version
are implemented. There is no form parser, XSD validator, dossier builder,
signature verifier, submission transport, or receipt parser yet. No release
has been published. This independent project is not a government service.

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
```

`capabilities` reports `stage: "scaffold"` and `operations: []`. Introspection
commands are listed separately from document operations. A successful command
reports `ok: true`, `verified: false`, and `submitted: false`: it has neither
verified a signature nor filed a case. `sources` prints a bundled research
catalog; it does not contact the listed sites or check whether guidance changed.

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
