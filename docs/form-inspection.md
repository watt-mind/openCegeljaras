# Form inspection contract

Reviewed on **2026-09-10**. This is the implementation contract for the first
read-only document operation, `form inspect`. It describes format recognition
of a 2026 company-form XML document. It does not describe XSD conformance,
signature verification, filing, or court acceptance, and its output must never
be read as any of those.

Status: **contract, not implemented.** The `capabilities` command may list
`form inspect` only after the behaviour and tests below exist.

## Problem and context

Attorneys and their agents already hold form XML produced by official or
alternative tools. Nothing in this repository can tell them what such a file
is: which wrapper namespace it carries, which of the three form variants it
contains, how it is encoded, or whether it is even well-formed XML. That is
the smallest useful document operation, and it establishes the hostile-input
boundary every later parser reuses.

The evidence for this contract is the wrapper schema and its three direct
imports. Their bytes were re-read on 2026-09-10 and match the bundled
[schema manifest](../data/schema-manifest.json) exactly:

| Resource | Bytes | SHA-256 prefix | Target namespace |
| :--- | ---: | :--- | :--- |
| `cegnyomtatvany-20260101.xsd` | 1368 | `6f727249d5d3c7d2` | `…/cegnyomtatvany-20260101#` |
| `cegbejegyzesiKerelem-20260101.xsd` | 36094 | `dfb8bc6002de1df0` | `…/cegbejegyzesiKerelem-20260101#` |
| `nevfoglalasiKerelem-20210401.xsd` | 3487 | `27e5c9da4e6767c9` | `…/nevfoglalasiKerelem-20210401#` |
| `valtozasbejegyzesiKerelem-20240101.xsd` | 39118 | `440c8aee609e9fcc` | `…/valtozasbejegyzesiKerelem-20240101#` |

The namespace prefix `…` stands for
`http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany`. All four schemas
declare UTF-8 and `version="1.0"`. The wrapper declares a single global
element `Nyomtatvány` whose content is a `choice` of exactly one of
`ck:CégbejegyzésiKérelem`, `vk:VáltozásbejegyzésiKérelem`, or
`nk:NévfoglalásiKérelem`, each referenced from its own namespace. Each variant
root begins with a sequence whose first child is `Adatlap`; the registration
variant adds `ÁltalánosAdatok`, the name-reservation variant `Névfoglalások`.
These are the only schema facts the inspector relies on, and they are
recorded here as facts about public documents, not as redistributed schema
text.

Mutable requirements were not all rechecked. The developer-guidance page that
states the form acceptance dates (accepted from 2026-02-28, exclusive from
2026-05-01 per the [specification inventory](specifications.md)) failed direct
TLS retrieval from the development server on 2026-09-10. The inspector does
not depend on those dates: it reports format recognition, never acceptance.

## Scope

In scope: one command, `opencegeljaras form inspect <PATH> [--json]`, that
reads one local file, decodes it, checks it against fixed resource limits,
parses it as namespace-aware XML, recognises the 2026 wrapper and variant, and
reports structural facts deterministically.

Out of scope, and not to be stubbed: XSD validation, `schemas list`, field
values, cross-field rules, `form validate`, `dossier preflight`, standard
input, directories, archives, ES3 containers, signatures, network access,
stylesheet rendering, and any writing.

## Parser decision

Use `roxmltree` 0.21.1 for tree parsing and `encoding_rs` 0.8.41 for the
ISO-8859-2 decode. Both were checked on 2026-09-10 against the registry
index and the local source:

- `roxmltree` 0.21.1: MIT OR Apache-2.0; declared MSRV 1.60, within the
  workspace MSRV 1.88. It parses a `&str`, has no network or file access, has
  no external-entity support, rejects DTDs by default (`allow_dtd: false`),
  fails entity-reference loops, and has a `nodes_limit` backstop. It builds
  the tree recursively without a nesting limit, so depth must be enforced
  before it runs.
- `encoding_rs` 0.8.41: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; declared
  MSRV 1.88. Its `ISO-8859-2` decoder is the WHATWG encoding of that name,
  which is byte-identical to IANA ISO-8859-2, unlike the `ISO-8859-1` case.
  Adopting it requires adding `BSD-3-Clause` to the `deny.toml` allowlist.
  This widens the licence policy with an OSI-approved permissive licence and
  is recorded as a deliberate decision; it is not a weakened gate.

Rejected alternatives: `quick-xml` 0.42.0 (MIT, MSRV 1.86) is a streaming
parser that passes DTD events through and leaves namespace resolution and
limits to the caller; `xml-rs` 1.0.0 (MIT, MSRV 1.85) expands entities and
needs the same wrapping. A hand-written ISO-8859-2 table would avoid the
licence change but adds unreviewed decoding code; it remains an option if the
licence decision is refused.

The sibling openSzigno core uses the same pair with `allow_dtd: false`, a
linear pre-scan for depth and node counts, and a decode policy limited to
UTF-8 and ISO-8859-2. Reuse the approach, not the code: openSzigno is not a
build dependency, and its dossier-specific limits do not apply to forms.

## Accepted input

The CLI reads exactly one regular file named by `PATH`. Before reading, it
checks the metadata size against the byte limit; while reading it enforces
the limit again so a growing file cannot bypass it. The core receives bytes
plus the fixed `Limits` and never touches the filesystem.

Encoding policy, applied in order:

1. A UTF-8 byte-order mark is stripped and reported as `bom: true`.
2. A UTF-16 or UTF-32 byte-order mark, or a NUL byte in the first four
   bytes, is `unsupported_encoding`.
3. If the document starts with an XML declaration, its `encoding` label is
   read. Labels compare ASCII case-insensitively after replacing `_` with
   `-`. `utf-8`/`utf8` select UTF-8; `iso-8859-2`/`iso8859-2` select
   ISO-8859-2; any other label is `unsupported_encoding`. The XML
   declaration must be at byte 0 (after the BOM) and end within the first
   256 bytes, otherwise it is treated as absent.
4. No declaration selects UTF-8. A BOM with a declared ISO-8859-2 label is
   `invalid_encoding`.
5. Decoding failure under the selected encoding is `invalid_encoding`.

Only the two encodings the official schemas and dossier declare are accepted.
The decoded text is what gets parsed; original bytes are not preserved,
because a form has no signature that depends on them.

## Resource limits

Fixed constants in `opencegeljaras-core`, exposed as a `Limits` struct with
`Default`; the CLI does not offer overrides in this milestone.

| Limit | Default | Error |
| :--- | ---: | :--- |
| Input bytes | 16 MiB | `input_too_large` |
| Element nesting depth (root = 1) | 128 | `limit_exceeded` |
| Elements | 200 000 | `limit_exceeded` |
| Parser nodes (`roxmltree` backstop) | 4 × elements | `limit_exceeded` |
| Bytes in one text or CDATA node | 1 MiB | `limit_exceeded` |
| Attributes on one element | 64 | `limit_exceeded` |

Depth and element count are enforced by a linear pre-scan of the decoded
text that skips comments, CDATA sections, and processing instructions, so a
deeply nested document is rejected before the recursive parser sees it. A
document nested 100 000 levels deep must be rejected without a stack
overflow. Text and attribute limits are checked while walking the finished
tree. The pre-scan only needs to be conservative for well-formed input;
anything it miscounts on malformed input is rejected by the parser.

## Prohibited constructs

- Any `<!DOCTYPE` or other `<!` declaration (outside comments and CDATA) is
  `prohibited_construct`. There is no internal-subset allowance.
- An entity reference other than the five predefined ones and numeric
  character references is `malformed_xml`, because no DTD can declare it.
- Nothing supplied by the document is ever resolved: not `xsi:schemaLocation`,
  not `xml-stylesheet`, not namespace URIs, not any attribute value.
- An `xml-stylesheet` processing instruction is permitted, because official
  forms carry one for rendering, and is reported only as `stylesheet_pi:
  true`. Its target is neither read nor printed. HTML or XSLT content is
  never executed.
- A root element in the XSD namespace or the XMLDSig namespace is reported
  as `unrecognized_root`; the inspector does not interpret schemas or
  signatures.

## Root, form, and version identification

After a successful parse the inspector classifies the document:

1. If the root element is `Nyomtatvány` in the wrapper namespace
   `http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#`,
   the profile is `form-2026`.
2. Within that profile, the root must have exactly one child element, and it
   must be one of the three known variant roots in its own namespace:

   | Variant id | Local name | Namespace suffix |
   | :--- | :--- | :--- |
   | `cegbejegyzesi-kerelem-20260101` | `CégbejegyzésiKérelem` | `cegbejegyzesiKerelem-20260101#` |
   | `valtozasbejegyzesi-kerelem-20240101` | `VáltozásbejegyzésiKérelem` | `valtozasbejegyzesiKerelem-20240101#` |
   | `nevfoglalasi-kerelem-20210401` | `NévfoglalásiKérelem` | `nevfoglalasiKerelem-20210401#` |

   A match yields `status: "recognized"` and exit 0.
3. A `Nyomtatvány` root in a namespace matching
   `http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-<8 digits>#`
   other than the 2026 one, or a `form-2026` wrapper whose single child is
   not a known variant, or a wrapper with zero or several element children,
   is `unsupported_version`. Older or newer official forms fail closed here.
4. Anything else is `unrecognized_root`.

Recognition is a namespace and element-name match. It does not establish
XSD validity, that the variant is currently accepted, or that the document
came from an official tool. Mixed-version imports inside a variant (for
example `termeszetesSzemely-20230820`) are expected and are not a version
error; they are reported in `namespaces`.

## Output

Exit codes extend the existing contract without changing existing meanings:

| Exit | Meaning | Where |
| ---: | :--- | :--- |
| 0 | Recognised, supported form; inspection data complete | stdout |
| 1 | Input rejected: unreadable, too large, encoding, malformed, limits, prohibited | JSON on stdout with `--json`; otherwise stderr only |
| 2 | Usage error, unchanged fixed diagnostic | stderr |
| 3 | Output or internal failure, unchanged | stderr |
| 4 | Well-formed within limits but `unsupported_version` or `unrecognized_root`; partial inspection data | as exit 0 |

Whenever the exit code is not 0, stderr carries exactly one fixed line of the
form `opencegeljaras: form inspect: <code> (exit N)`. No diagnostic ever
contains the path, any byte of the file, an element name, a namespace, a
declared encoding label, or a parser position. Usage errors keep their
existing behaviour, including when `PATH` is missing or extra arguments are
given.

### JSON envelope

The success envelope is the existing one (`schema_version: 1`, `ok: true`,
`command: "form inspect"`, `data`, `verified: false`, `submitted: false`).
Exit 1 and exit 4 add an error envelope, which is new in schema version 1 as
an additive shape for a new command: `ok: false`, an `error` object with a
stable `code` and a fixed human `message`, and `data` holding whatever safe
facts were established before the failure (`input.bytes` at minimum for exit
1; the full structure for exit 4). Usage errors still produce no JSON.

`data` for a completed parse:

```json
{
  "input": {
    "bytes": 2048,
    "bom": false,
    "xml_declaration": true,
    "encoding": "UTF-8",
    "stylesheet_pi": true,
    "schema_location_declared": false
  },
  "root": { "local_name": "Nyomtatvány", "namespace": "http://…#" },
  "form": {
    "status": "recognized",
    "profile": "form-2026",
    "variant": {
      "id": "cegbejegyzesi-kerelem-20260101",
      "local_name": "CégbejegyzésiKérelem",
      "namespace": "http://…#"
    },
    "sections": [
      { "local_name": "Adatlap", "count": 1 },
      { "local_name": "ÁltalánosAdatok", "count": 1 }
    ]
  },
  "structure": {
    "elements": 57,
    "max_depth": 6,
    "attributes": 3,
    "text_bytes": 812,
    "namespaces": ["http://…#", "http://…#"],
    "namespaces_truncated": false
  },
  "checks": {
    "well_formed": "passed",
    "xsd_conformance": "not_performed",
    "signature": "not_performed",
    "authority_acceptance": "not_performed"
  },
  "notice": "format recognition only; not schema validation, verification, or acceptance"
}
```

Rules for the fields:

- `form.status` is `recognized`, `unsupported_version`, or
  `unrecognized_root`. `profile` and `variant` are `null` when not
  established; `sections` is `null` unless the variant is recognised.
- `sections` lists the direct element children of the variant root in first
  appearance order with their counts, at most 64 distinct names, then
  `sections_truncated: true`. Names outside the schema are still listed;
  they are structure, not judgement.
- `structure.namespaces` is the sorted set of element namespace URIs, at
  most 64 entries.
- Element local names and namespace URIs are the only document-derived
  strings in the output. They are never text content or attribute values,
  and each is cut at 128 characters with a `…` marker. Unnamespaced elements
  report `namespace: null`.
- `encoding` is one of the two accepted labels, normalised; `bytes` counts
  the file including any BOM.
- Every `checks` entry uses the finding vocabulary from the
  [architecture](architecture.md): `passed`, `failed`, `not_performed`,
  `not_applicable`. Only `well_formed` can be `passed` here.
- Output is byte-identical for identical input: no timestamps, paths,
  hostnames, or versions inside `data`.

### Human output

One fact per line in the style of `capabilities`, ending with the same
boundary lines: `xsd conformance: not performed`, `verification: not
performed`, `submission: not performed`. Exit 4 prints the report and the
stderr line; exit 1 prints nothing on stdout.

## Command-line shape

`form` is a subcommand group with the single `inspect` subcommand.
`opencegeljaras form` alone, `form validate`, `form inspect` without a path,
or a second positional are usage errors (exit 2) with the existing fixed
diagnostic. `--json` keeps its meaning. No `--limit`, `--schema`, or
`--profile` options exist in this milestone.

## Ownership for package 2

| Path | Role |
| :--- | :--- |
| `crates/opencegeljaras-core/src/` (new modules; `lib.rs` exports) | Core parser: decode, pre-scan, parse, classify, `Limits`, error codes |
| `crates/opencegeljaras-cli/src/main.rs` (may split into modules) | File adapter, `form inspect` command, envelopes, exit codes |
| `crates/opencegeljaras-cli/tests/` | Contract tests over the built binary |
| `tests/fixtures/forms/` (proposed) | Synthetic fixtures, CC0 |
| `crates/opencegeljaras-cli/skills/opencegeljaras/SKILL.md` | Advertise the command last |
| `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `deny.toml`, `Cargo.toml`, `Cargo.lock` | Contract and policy updates |

One implementation worker owns all of these for package 2. The core must
stay free of filesystem, network, clock, and process use; `cargo doc` and
clippy gates stay as they are.

## Capability advertisement

After the tests pass, `capabilities` reports `operations:
["form inspect"]` while `stage` remains `scaffold` and
`introspection_commands` is unchanged. The README sentence stating that
`operations` is empty is updated in the same change. The skill tells agents
to run `form inspect --json` on a file they are allowed to read, to treat
exit 4 as "not a supported 2026 form", and never to describe exit 0 as valid,
verified, accepted, or filed.

## Synthetic fixtures

All fixtures under `tests/fixtures/forms/` are constructed by hand from the
public element and namespace names above, contain placeholder text only, and
are not asserted to be XSD-valid. Their README must say so. None may be
derived from, resemble, or be checked against private material.

| Fixture | Purpose |
| :--- | :--- |
| `cegbejegyzesi-minimal-utf8.xml` | Recognised registration variant, declaration, no BOM |
| `valtozasbejegyzesi-stylesheet-bom.xml` | Change variant with BOM and `xml-stylesheet` PI |
| `nevfoglalasi-iso-8859-2.xml` | Name-reservation variant encoded in ISO-8859-2 with accented names |
| `nevfoglalasi-no-declaration.xml` | UTF-8 without an XML declaration |
| `wrapper-20240101-unsupported.xml` | `Nyomtatvány` in a differently dated namespace → exit 4 |
| `wrapper-unknown-child.xml` | 2026 wrapper with an unknown single child → exit 4 |
| `wrapper-two-children.xml` | 2026 wrapper with two variant children → exit 4 |
| `unrecognized-root.xml` | Unrelated well-formed document → exit 4 |
| `doctype.xml` | Internal subset with an entity → exit 1 `prohibited_construct` |
| `undeclared-entity.xml` | `&foo;` without DTD → exit 1 `malformed_xml` |
| `malformed.xml` | Unclosed element → exit 1 `malformed_xml` |
| `utf16-bom.xml` | UTF-16 BOM → exit 1 `unsupported_encoding` |
| `declared-utf8-invalid-bytes.xml` | ISO-8859-2 bytes under a UTF-8 label → exit 1 `invalid_encoding` |
| `declared-windows-1250.xml` | Unsupported label → exit 1 `unsupported_encoding` |

Oversized, over-deep, over-wide, and long-text inputs are generated inside
the tests rather than stored.

## Verification

Package 2 is verified by the repository baseline plus the contract tests:

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- capabilities --json
cargo run --locked -p opencegeljaras-cli -- form inspect \
  tests/fixtures/forms/cegbejegyzesi-minimal-utf8.xml --json
```

Contract tests must cover: every fixture's exit code and `error.code`; the
JSON and human shapes for exit 0 and 4; that stderr is empty on exit 0 and
exactly one line otherwise; that a private-looking path never appears on
stderr or stdout for a missing file; generated depth, element, text, and
attribute limits at the limit and one past it; a 100 000-deep document; an
oversize file; determinism across two runs; that `capabilities` lists the
operation; and that `form validate`, `form`, and `preflight` still exit 2.
Core unit tests cover decode, pre-scan, and classification without the
binary.

## Decisions recorded separately

- **Schema redistribution.** No XSD, XSLT, or vendor example is vendored.
  The inspector embeds namespace URIs, element names, and schema file names,
  which are facts, not the copyrighted schema text. The
  [specification inventory](specifications.md) keeps the open redistribution
  question.
- **Full XSD validation.** Deferred to roadmap milestone 2. It needs a pinned
  and audited 139-resource import graph, resolved distribution rights, an
  all-Rust or isolated validator proven against an independent
  implementation, and fixtures that are asserted valid and invalid. This
  contract deliberately avoids any partial validation that could be mistaken
  for it.

## Unresolved questions

- The acceptance-date guidance was not re-read; it does not affect this
  milestone but must be rechecked before any "accepted form" claim.
- Whether official tools emit the wrapper with a BOM or with ISO-8859-2 is
  unknown; both are accepted and reported, so neither assumption is baked in.
- The `BSD-3-Clause` licence allowance for `encoding_rs` needs operator
  acknowledgement at review; the hand-written table is the fallback.

## Decision on package 2

Package 2 may proceed against this contract once it has been independently
reviewed. Every input-format fact it depends on is established from the
re-read public schemas; the unrechecked guidance affects only claims this
milestone does not make.
