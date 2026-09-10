# Testing

The scaffold's integration tests exercise the executable's capability claims,
JSON envelope, source output, embedded skill, and rejection of unsupported
operations. No test establishes compatibility with the company courts.

Run `./scripts/check.sh` and `cargo build --release --locked`. CI targets Linux,
macOS, Windows, and the declared minimum Rust version. Local verification only
covers environments actually run; configured CI jobs are not proof of success.

Before implementing parsing, create synthetic fixtures independently from
the published schemas. Cover positive and negative cases, mixed namespaces,
unsupported profiles, old/new effective dates, ISO-8859-2 and UTF-8 handling,
malformed XML, external-resource rejection, depth/size limits, and nested
container limits. Compare XSD results against an independent implementation.

Before authoring, add deterministic output and no-clobber tests. Before
receipt handling, test missing evidence, unverified authority claims, receipt
correlation, and ambiguous state transitions. Never send synthetic cases to
production to test transport.

No private dossier may become a public fixture. Private-corpus testing is not
implemented; `samples/` is ignored and excluded from diagnostics. Third-party
schemas are not covered by this repository's fixture license.
