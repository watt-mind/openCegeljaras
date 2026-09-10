# Testing

The scaffold's integration tests exercise the executable's capability claims,
JSON envelope, source output, embedded skill, and rejection of unsupported
operations. No test establishes compatibility with the company courts.

Run `./scripts/check.sh` and `cargo build --release --locked`. CI targets Linux,
macOS, Windows, and the declared minimum Rust version. Local verification only
covers environments actually run; configured CI jobs are not proof of success.

The `form inspect` contract tests run the built binary against the synthetic
fixtures in `tests/fixtures/forms/` and against generated oversized, over-deep,
over-wide, and long-text inputs, a missing file, and a directory. They assert
each fixture's exit code, `error.code`, and `error.message`, the JSON and human
shapes for exit 0 and exit 4, that stderr is empty on exit 0 and exactly one
line otherwise, that no path ever reaches stdout or stderr, that limits hold at
the limit and one past it, and that output is byte-identical across runs. Core
unit tests cover decoding, the pre-parse scan, and classification without the
binary. None of this establishes XSD conformance or acceptance.

Before implementing further parsing, create synthetic fixtures independently
from the published schemas. Cover positive and negative cases, mixed namespaces,
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
