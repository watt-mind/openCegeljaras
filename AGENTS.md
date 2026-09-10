# openCegeljaras

openCegeljaras is a development scaffold for an agent-first Rust CLI for
Hungarian electronic company-court filings. Read README.md,
docs/architecture.md, docs/research.md, docs/specifications.md, and
docs/roadmap.md before changing scope.

## Boundaries

- `crates/opencegeljaras-core`: domain capabilities and bundled source metadata;
  future bounded parsing and deterministic rules. No filesystem, network,
  credentials, process spawning, or ambient clock in the core.
- `crates/opencegeljaras-cli`: arguments, human/JSON output, embedded skill;
  future explicit file and process adapters.
- Only introspection exists. Never claim form validation, signature validity,
  filing, delivery, or court acceptance from successful introspection.
- Keep structural checks, cryptographic evidence, official receipts, and legal
  assessment separate. Unknown or unperformed checks are not successes.
- Reuse reviewed sibling interfaces for container work; never copy a crypto
  stack or silently assume an ES3 container meets the cégeljárás profile.
- Do not infer a KRX/e-Papír channel from the procedure name. Bind each future
  procedure profile to evidence for its authorized route.
- Official XSDs are not vendored. Availability is not a redistribution license.
  Pin and audit the complete import graph before implementing schema validation.
- Never resolve user-provided schema URLs, XML external entities, or XSLT
  resources over the network. Establish size, depth, and expansion limits first.
- Transport, signing, fee payment, and acknowledgment of service are separate
  explicit actions. Read-only receipt inspection must not acknowledge service.

## Private inputs and secrets

`samples/` is private and git-ignored. Never enumerate, print, hash, or include
its filenames, metadata, contents, identities, or cryptographic values in
diagnostics, patches, tests, or chat. Only synthetic, independently constructed
fixtures are allowed under `tests/fixtures/`.

Never commit private keys, complete PEM private-key armour lines, credentials,
or `.env` files. Never allowlist a secret-scanner rule. Future key access must
use files or injected interfaces, never secret bytes in argv or log output.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
./scripts/check.sh
```

Use Conventional Commits and `codex/` branches. The intended hosted workflow
uses `develop` for integration and `master` for stable releases. No remote or
Linear project has been configured. Do not copy another project's IDs or
register external resources implicitly. Before future issue filing, consult
the operator's applicable issue-tracking protocol; keep private routing out
of public artifacts.
