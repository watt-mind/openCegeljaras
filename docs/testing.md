# Testing

The scaffold's integration tests exercise the executable's capability claims,
JSON envelope, source output, embedded skill, and rejection of unsupported
operations. No test establishes compatibility with the company courts.

Run `./scripts/check.sh` and `cargo build --release --locked`. CI targets Linux,
macOS, Windows, and the declared minimum Rust version. Local verification only
covers environments actually run; configured CI jobs are not proof of success.

The `cli` contract tests cover the usage-error contract in both forms: a
rejected command line carrying `--json` writes exactly the one-line
`usage.arguments` envelope on stdout with empty stderr and exit 2, for an
unknown command, an unknown option, `schemas list --urls --json`, and
`form validate` without its `--bundle`; the same lines without `--json`, and
`--json` after a `--` terminator, keep the fixed stderr diagnostic with empty
stdout; a sentinel argument reaches neither stream in either form; `--help`,
`-h`, `--version`, and `-V` stay plain text on exit 0 even alongside `--json`;
and an unwritable stdout turns the envelope into exit 3.

The `form inspect` contract tests run the built binary against the synthetic
fixtures in `tests/fixtures/forms/` and against generated oversized, over-deep,
over-wide, and long-text inputs, a missing file, and a directory. They assert
each fixture's exit code, `error.code`, and `error.message`, the JSON and human
shapes for exit 0 and exit 4, that stderr is empty on exit 0 and exactly one
line otherwise, that no path ever reaches stdout or stderr, that every limit
holds at the limit and one past it, including the per-element caps of 64
attributes and 256 namespace declarations, and that output is byte-identical
across runs.
Core unit tests cover decoding, the pre-parse scan, and classification without
the binary. None of this establishes XSD conformance or acceptance.

The `form fields` contract tests run the built binary over three synthetic
fixtures and over generated documents. They assert the exact field list of the
repeated-sibling fixture (paths, kinds, namespaces, and lengths) with and
without `--show-values`, index assignment for repeated siblings and its
absence for singletons, root and namespaced attributes together with the XSI
and namespace-declaration exclusions, empty, whitespace-only, CDATA and
comment-split values, the mixed rule, the 4096-scalar value cut, the
1024-scalar path cut, the 10 000-field cap, that every listed path is unique,
that a 110-deep chain of 120-scalar names over 10 000 leaves keeps its output
bounded and finishes well inside ten seconds, that a 100 000-sibling document
is listed and compared inside the same bound, and that both commands are
byte-identical across runs. For `form diff` they cover every exit path: a
refused or unrecognised OLD or NEW, the rule that NEW is never opened when OLD
fails, `same_variant: false`, `identical: true` for a file compared with
itself, and added, removed, and changed entries with and without values. One
test asserts that the fixture's sentinel value never reaches standard error
and reaches standard output only with `--show-values`. Core unit tests cover
the walk, the path model, the arena that keeps one entry per node rather than
one per path component, and the comparison, without the binary.

The `schemas` contract tests run the built binary over the synthetic CC0 mini
graph in `tests/fixtures/schemas/`: the stored good and tampered bundles, and
copies generated into a scratch directory for an unpinned import, more
offending locations than the 64-location cap, an import namespace mismatch,
an unparseable resource, a missing form root, an oversize file,
symlinked resources and directories, an unreadable bundle, and manifest
files that are unreadable, not JSON, carry an unknown field, a `..` segment, a
non-allowed host, or an `http` scheme. They assert the list, `--urls`, and
verify shapes, every exit code and fixed message, that stderr is empty on
exit 0 and exactly one line otherwise, that no local path reaches either
stream, and that output is byte-identical across runs. Core unit tests cover
the manifest rules, the compiled manifest's 149 resources, its 139-resource
form closure and its 10-resource dossier closure disjoint from it, RFC 3986
resolution, scheme normalisation, and digesting.

The `form validate` tests never need a real bundle or a real xmllint for the
mechanics. The fake validator is the running test executable itself,
re-executed through `std::env::current_exe()` with a marker test name by a
launcher that `--xmllint` points at, and driven by a control file with a fixed
name in the case directory. It exercises the version gate at both bounds, the
fixed argument list, the working directory, the environment allowlist, the
exit-code mapping, the timeout with a short `Duration`, a grandchild that keeps
the standard-error pipe open past that timeout, the 1 MiB standard-error cap,
the rule that a warning about the schema graph is a failure whatever the exit
code and only on a line that is not itself a validity error, and that the
`document.xml` copy is byte-identical to the input. Line
parsing, value redaction, message and finding truncation are unit-tested
without any child process. Real-validator tests run the installed `xmllint`
over the synthetic mini graph for a conforming and a nonconforming instance and
over a bundle directory whose path contains `#`, `?`, and `%`. Those tests and
the launcher-based ones compile only on Linux, with ignored stubs of the same
names elsewhere, so the workspace still builds warning-free on macOS and
Windows; on Linux they fail rather than skip when `xmllint` is absent, and CI
installs `libxml2-utils` on the Linux test job. Every test scratch directory is
removed when its test ends.

The `dossier` contract tests run the built binary over the fifteen synthetic
CC0 containers in `tests/fixtures/dossiers/` and over dossiers generated into a
scratch directory. They assert every recognition status including the 2014 and
2023 namespaces and the default Microsec namespace, the documented JSON and
human shapes, that the carried `form` object is exactly what `form inspect`
reports over the same payload, every `payload` reason in precedence order
including the pre-decode length check, the `rejected` and exit-4 payload
shapes, the nested-dossier and unknown-algorithm rules, the ISO-8859-2 dossier,
the 256-document cap with `documents_truncated`, that a declared title, the
suffix-role sentinel, and the company-identity values reach standard output
only with `--show-values` and standard error never, each finding code in each
reachable status, the receipt rule and its notice, every exit path, and that
output is byte-identical across runs. With the fake validator they cover the
preflight XSD path: a passing and a failing conformance with redacted messages,
the human `xsd findings` lines, and a validator that produces no result. A
tampered copy of the mini graph's dossier root exercises `bundle_unverified`, a
manifest stripped of its second root `manifest_invalid`, and a generated
10 000 004-byte text node `payload_exceeds_validator_limit` with the validator
never spawned. The mini graph's dossier-style resource carries a DOCTYPE and is
not a usable schema, so there is no Linux real-xmllint test for preflight. Core
unit tests cover the two `Limits` profiles field by field, recognition, the
payload precedence, the strict base64 decoder, the `xs:dateTime` lexical check,
the findings and the verdict, a text node the dossier profile accepts and the
form profile refuses, and — for all 194 role patterns — that only the reviewed
construct subset occurs, that every pattern compiles, and that the literal
title derived from each one matches it. A `#[cfg(unix)]` test creates a FIFO
with `mkfifo` and asserts under a timeout that `form inspect` refuses it as
`input_unreadable` instead of blocking on the open.

Three tests marked `#[ignore]` use a real mirror named by
`OPENCEGELJARAS_SCHEMA_BUNDLE`: one verifies the mirror, one validates the two
real form fixtures against it, and one runs `dossier preflight` over the two
evidence dossiers, expecting the conforming one to pass the XSD finding and the
bad-procedure-title one to fail it on the enumeration facet. Run them with
`cargo test --workspace -- --ignored`; they panic with a fixed message when the
variable is unset. CI does not run them, and no official schema is stored in
this repository. Verifying a bundle is
a digest comparison and an import audit: not a licence, not validation, and not
acceptance.

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
