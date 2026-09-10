# Schema bundle and XSD conformance contract

Reviewed on **2026-09-10**. This contract specifies two packages that
together give `form validate` a truthful XSD-conformance result: a verified
local schema bundle (package 3) and an explicit external validator adapter
(package 4). Neither package redistributes official schemas, and neither
result is acceptance by any authority.

Status: **contract, not implemented.** `capabilities` may list the new
operations only after the behaviour and tests below exist.

## Problem and context

`form inspect` recognises a 2026 form but cannot say whether it conforms to
the official XSD graph. Conformance needs the complete form closure and a
validator that resolves `xs:import` across its 139 target namespaces.

Evidence gathered on 2026-09-10 from the development server:

- The complete closure listed in the [schema manifest](../data/schema-manifest.json)
  was re-fetched into a scratch mirror laid out by URL host and path. All
  149 resources matched their pinned SHA-256 digests and byte counts.
- Every `schemaLocation` in the closure is relative except one absolute
  reference to `http://www.w3.org/2001/03/xml.xsd` inside the dossier
  graph's copy of `xml.xsd`. The form graph therefore resolves entirely
  from local relative paths once the root is a local file.
- Two dossier-graph resources, that `xml.xsd` and `xmldsig-core-schema.xsd`,
  carry a `<!DOCTYPE` declaration; the latter declares an entity and uses
  it in its `targetNamespace`. No form-graph resource has a DTD. The form
  graph uses `xs:pattern` (16 files) and reaches everything by
  `xs:import`; `substitutionGroup` (3 files), `xs:include` (2), and
  `xs:any` (3) appear only in the dossier graph; no resource uses identity
  constraints or `redefine`.
- `xmllint` from libxml2 2.9.14 loads the full form graph offline with
  `--nonet` in about 20 ms and validates a document in the same run. Its
  exit codes are 0 for a conforming document, 3 for schema violations, 1 for
  a document parse error, and 5 for a schema that fails to load. Each
  violation is one stderr line of the form
  `PATH:LINE: element NAME: Schemas validity error : MESSAGE`, and
  `xmllint --version` prints `xmllint: using libxml version 20914` on
  stderr. These formats are established for 2.9.14 only.
- The Python `xmlschema` 4.3.2 package, an independent implementation,
  loads the same form graph and agrees with xmllint on a hand-built
  minimal name-reservation instance (conforming) and on the same instance
  with a malformed company registry number (rejected by the same pattern).
  It also agrees that the existing synthetic inspect fixtures are not
  conforming, which they were never claimed to be. Neither implementation
  was exercised on the dossier graph.
- No all-Rust XSD validator handles this graph. The `xmlschema` crate
  0.0.8 (MIT OR Apache-2.0, MSRV 1.86) implements structural validation
  and facets but documents `xs:import`, `xs:include`, and substitution
  groups as not implemented; every resource in this graph is reached by
  import. The `libxml` crate 0.3.21 (MIT, MSRV 1.88) binds libxml2 through
  bindgen, which needs libclang and libxml2 headers on every build host and
  a separate arrangement on Windows; it failed to build on the development
  server without libclang. The `xsd-parser` crate generates Rust types and
  does not validate.

Decision: XSD conformance is delegated to a caller-installed `xmllint`
through an explicit process adapter in the CLI crate. This keeps the core
free of C dependencies and `unsafe`, keeps libxml2's security updates with
the operating system, and matches the architecture's plan for explicit
process adapters. Linking `libxml` directly remains a recorded alternative
if packaging later favours a self-contained binary. Replacing the adapter
with an all-Rust validator waits for one that passes the same two-fixture
agreement test on this graph.

## Shared parsing entry point

The core's `form` module keeps its `decode`, `prescan`, and `tree` steps
private. Package 3 extracts a shared entry point, "parse bytes under
`Limits` into a bounded tree", used by both the form classifier and the
schema import audit, without changing any `form inspect` behaviour or
test. The schema module reuses that boundary, not the form classifier.

## Package 3: verified schema bundle

### Scope

Two introspection-style subcommands under `schemas`:

```sh
opencegeljaras schemas list [--json | --urls]
opencegeljaras schemas verify --bundle DIR [--manifest FILE] [--json]
```

`list` prints the pinned manifest compiled into the binary and never
fetches. `verify` checks an operator-supplied local mirror against the
pinned digests and audits the form graph's imports. Neither downloads,
writes, or modifies anything under `DIR`.

Out of scope: fetching schemas, vendoring them, generating catalogs for
other tools, validating documents, and any parse or import audit of the
dossier graph, which is digest-checked only.

### Manifest

`data/schema-manifest.json` is compiled into the core with
`include_str!`, parsed with `deny_unknown_fields`, and exposed as
`SchemaManifest { snapshot_version, retrieved_at, purpose, roots, resources }`
where each resource has `url`, `bytes`, `sha256`, `target_namespace`
(nullable), `dependencies`, and `roots`. `roots[0]` is the form root and
`roots[1]` the dossier root. The **form closure** is the set of resources
reachable from `roots[0]` through `dependencies`, computed from the
manifest alone; the remaining resources form the dossier graph.

Manifest path rules, applied to every resource URL of any manifest, compiled
or from a file: scheme `https`, host exactly `schema.e-cegjegyzek.hu` or
`www.e-cegjegyzek.hu`, no query or fragment, a path of one or more
segments made only of `A-Z a-z 0-9 . - _`, with no segment equal to `.` or
`..`, and total length at most 512 bytes. Percent signs, backslashes,
colons, NUL, and non-ASCII are rejected. A manifest violating any rule, or
with duplicate URLs, a dependency URL that is not a resource, a digest that
is not 64 lowercase hex characters, or `roots` that are not resources, is
invalid.

A unit test asserts that the compiled manifest is valid under these rules,
has 149 resources, that `roots[0]` is the `cegnyomtatvany-20260101.xsd`
wrapper, that its form closure has 139 resources, and that no form-closure
resource is one of the two DOCTYPE-bearing dossier files.

### Bundle layout

A bundle is a directory mirroring each resource URL as
`DIR/<host>/<path>`, which is the layout produced by `wget -x`,
`curl --create-dirs` with the URL path, or the operator's own script.
`schemas list --urls` prints one URL per line for that purpose. The tool
never prints a download command that it would run itself.

### `schemas list`

Human output: the snapshot date, the root URLs, the resource count, then
one line per resource: URL, byte count, first 16 hex characters of the
digest, target namespace or `none`, dependency count. JSON `data`:
`{ snapshot_version, retrieved_at, purpose, roots: [...], count,
form_closure_count, resources: [{ url, bytes, sha256, target_namespace,
dependencies: [...], roots: [...] }], notice }` with the notice "pinned
research metadata; nothing was fetched or verified". `--urls` prints only
the URLs, one per line, in manifest order, with no header; `--urls` with
`--json` is a usage error (exit 2).

### `schemas verify`

`DIR` is canonicalised first; if that fails or the result is not a
directory, exit 1 with `error.code = "bundle_unreadable"`. For each
manifest resource, in manifest order:

1. Map the URL to `DIR/<host>/<path>`; the path rules above guarantee the
   mapping stays inside `DIR`.
2. If the file is missing, not a regular file after following symlinks,
   or unreadable: status `missing`.
3. Read at most 4 MiB. Larger: status `oversize`. (The largest pinned
   resource is under 100 KiB; the cap bounds work, not expectations.)
4. Compare the byte count and SHA-256 (`sha2` 0.11, MIT OR Apache-2.0,
   MSRV 1.85). Mismatch: status `mismatch`. Match: status `verified`.
5. For a verified resource **in the form closure only**, parse it through
   the shared bounded entry point (the same default `Limits` as
   `form inspect`, whose namespace-declaration cap of 256 accommodates the
   82 and 85 prefixes the two large variant schemas declare; same
   prohibited constructs; no network). Failure is status `unparseable`. Otherwise
   collect every `import` and `include` element in the XSD namespace and
   resolve its `schemaLocation` against the resource URL with plain
   RFC 3986 relative resolution, then normalise `http://` to `https://`
   for the two allowed hosts, exactly as the manifest records. Each
   resolved location must be a manifest URL, and for `import` its
   `namespace` attribute must equal that resource's `target_namespace`.
   Otherwise the resource is status `unpinned_import` and lists the
   offending resolved locations. Dossier-graph resources stop after
   step 4 and are reported with `import_audit: "not_performed"`.

No directory walk is performed; files that are not manifest resources are
never opened, listed, or counted.

Results: `form_closure_verified` is true when every form-closure resource
is `verified` after step 5; `verified` is true when additionally every
dossier-graph resource is `verified` after step 4. JSON `data`:

```json
{
  "manifest": { "source": "compiled", "retrieved_at": "2026-09-10",
                "count": 149, "form_closure_count": 139 },
  "bundle": { "verified": true, "form_closure_verified": true,
              "verified_count": 149, "missing": 0, "mismatch": 0,
              "oversize": 0, "unparseable": 0, "unpinned_import": 0 },
  "problems": [ { "url": "https://…", "status": "mismatch",
                  "locations": [] } ],
  "checks": { "digests": "passed", "import_audit": "passed",
              "xsd_conformance": "not_performed" },
  "notice": "digest and import audit of a local mirror; not a licence, not validation"
}
```

`checks.import_audit` describes the form closure only. `problems` lists
every non-verified resource by manifest URL, status, and (for
`unpinned_import`) the offending resolved locations, which are
bundle-derived strings and therefore follow the `form inspect` escaping
rule and 128-scalar cut. `DIR` never appears in any output.

Exit codes: 0 when `verified` is true; 4 with the same envelope shape and
`error.code = "bundle_unverified"` otherwise; 1 with `bundle_unreadable`,
`manifest_unreadable`, or `manifest_invalid`; 2 for usage errors. Human
output prints the manifest line, the counts, one line per problem, the
check lines, then `verification: not performed` and `submission: not
performed`. The stderr line is `opencegeljaras: schemas verify: <code>
(exit N)`.

`--manifest FILE` reads a manifest from a local regular file (at most
4 MiB) instead of the compiled one. It is parsed with the same strict
types and validated with the same path rules; an unreadable file is
`manifest_unreadable` and any rule violation is `manifest_invalid`, both
exit 1 with the error envelope. Output then reports `manifest.source:
"file"` and the notice gains "; unpinned manifest". `form validate`
accepts the same option with the same reporting; with a manifest file the
form root is still `roots[0]`.

### Fixtures and tests

`tests/fixtures/schemas/` holds a synthetic mini graph under CC0: a
manifest of five resources on the two real hosts (a wrapper importing two
variant schemas, one shared simple-type schema reached through an
`http://` import to exercise normalisation, and one dossier-style resource
outside the form closure that contains a `<!DOCTYPE` so the digest-only
rule is exercised), the mirrored files, and a second bundle with one
tampered file and one missing file. Tests cover: list human and JSON
shape; `--urls` exact lines and its conflict with `--json`; verify on the
good bundle (exit 0, all counts, `import_audit` passed, the DOCTYPE file
verified digest-only); the tampered bundle (exit 4, `mismatch` and
`missing` listed by URL, no path echoed, `form_closure_verified` reported
truthfully); a form-closure resource importing a location outside the
manifest (`unpinned_import` with locations); an `import` namespace
mismatch; a symlinked resource file (followed) and a symlinked `DIR`
(canonicalised); an oversize file; an unreadable `DIR`; a `--manifest`
file that is unreadable, has an unknown field, a `..` segment, a
non-allowed host, or an `http` scheme; and the compiled-manifest
invariants. A test marked `#[ignore]` verifies the real bundle named by
`OPENCEGELJARAS_SCHEMA_BUNDLE`; run with `--ignored` and no variable it
panics with a fixed message rather than returning. CI does not run it.

## Package 4: `form validate` through the xmllint adapter

### Scope

```sh
opencegeljaras form validate PATH --bundle DIR [--manifest FILE]
    [--xmllint EXE] [--show-values] [--json]
```

Behaviour, in order:

1. Run the complete `form inspect` boundary on `PATH`, keeping the bytes
   read. Any exit 1 outcome is returned unchanged except that `command` is
   `"form validate"`. Any exit 4 outcome is returned unchanged in the same
   way with `checks.xsd_conformance: "not_performed"`; only a `recognized`
   form proceeds.
2. Run the package 3 bundle verification on `DIR`. If
   `form_closure_verified` is false, exit 5 with
   `error.code = "bundle_unverified"`; nothing is spawned. Dossier-graph
   problems do not block validation but stay visible in `data.bundle`.
3. Locate the validator: `--xmllint EXE` if given, else the first `xmllint`
   on `PATH`. Canonicalise it. Run it with `--version`, read the
   `using libxml version N` line from stderr, and require
   `20914 <= N < 21300`; the upper bound exists because later libxml2
   releases changed xmllint's message formats and must be re-checked before
   the ceiling is raised. Missing executable, failed spawn, unparseable
   version, or a version outside the range: exit 5,
   `error.code = "validator_unavailable"`, with `validator.version` when it
   was read.
4. Create a private temporary directory (`tempfile` 3.27, MIT OR
   Apache-2.0, MSRV 1.63) and write the bytes from step 1 into it as
   `document.xml`. The validator therefore reads exactly the bytes the
   inspect boundary accepted: no second read of the user's path, no
   `-`-as-stdin ambiguity, no user path in the child's argv or stderr. The
   cost is one temporary write of at most 16 MiB. No catalog is created:
   the form graph resolves relatively from the canonical bundle path, and
   `--nonet` makes any absolute location fail closed.
5. Spawn the validator once with the temporary directory as working
   directory, a cleared environment plus the pass-through allowlist below,
   and the arguments `--nonet --noout --schema ROOT document.xml`, where
   `ROOT` is the canonical bundle path of `roots[0]`. `--noent`,
   `--xinclude`, `--load-trace`, `--huge`, `--path`, and `--catalogs` are
   never passed. Stdout is discarded. Stderr is read continuously into a
   buffer capped at 1 MiB; bytes beyond the cap are discarded so the child
   cannot block on a full pipe. The adapter takes the timeout as a
   `Duration`; the CLI passes 60 seconds and tests pass milliseconds. On
   timeout the child is killed and reaped: exit 5,
   `error.code = "validator_failed"`.
6. Map the exit code: 0 is conforming; 3 is nonconforming; anything else
   (including 1 and 5) is `validator_failed` with the exit code reported.
   Parse stderr lines matching `document.xml:LINE: element NAME: Schemas
   validity error : MESSAGE`. `LINE` becomes an integer, `NAME` becomes
   `element` (cut at 128 scalars), `MESSAGE` is trimmed. Lines that do not
   match are counted in `unparsed_lines` and never echoed; the trailing
   `document.xml fails to validate` line is expected and not counted.
7. Redact values. A single-quoted span in `MESSAGE` is kept only if it is
   a qualified name matching `'\{[^{}']*\}[^'\s]*'` or it directly follows
   the text `[facet` plus one space and is one of the XSD facet names. Every other
   single-quoted span is replaced by `'…'`. If the message contains an odd
   number of apostrophes, or any quoted span cannot be classified
   unambiguously, the entire message becomes the fixed text
   `message redacted`. `--show-values` skips redaction, sets
   `values_redacted: false`, and is documented as able to expose personal
   data; the skill never passes it by default. Messages are cut at 512
   Unicode scalar values with `…`. At most 256 findings are kept, then
   `findings_truncated: true`.

Environment pass-through allowlist: `PATH` on every platform; additionally
`SystemRoot`, `SYSTEMROOT`, `TEMP`, `TMP`, and `PATHEXT` on Windows.
Nothing else, explicitly not `LD_LIBRARY_PATH`, `DYLD_*`,
`XML_CATALOG_FILES`, `SGML_CATALOG_FILES`, or `XML_DEBUG_CATALOG`; an
xmllint that needs those fails closed as `validator_unavailable`.

### Output

`data` for `form validate` on every exit, with nullability:

| Field | Content |
| :--- | :--- |
| `input`, `root`, `form`, `structure` | As `form inspect`; null where inspect would leave them null |
| `bundle` | The full `schemas verify` `data` object; null before step 2 |
| `validator` | `{ name: "xmllint", version, exit_code }`; null before step 3; `version` or `exit_code` null when not established |
| `findings` | Array of `{ line, element, message }` in validator order; empty on exit 0; null before step 6 |
| `findings_truncated`, `unparsed_lines`, `values_redacted` | Booleans and a count; null before step 6 |
| `checks` | `well_formed`, `xsd_conformance`, `signature`, `authority_acceptance` |
| `notice` | Fixed text below |

Exit 0: conforming; `checks.xsd_conformance = "passed"`, `ok: true`. The
`notice` reads "XSD conformance of one form against the pinned 2026 graph
as interpreted by the named libxml2 version; not the technical conditions,
not a signature check, not acceptance".

Exit 4: a performed check failed. Either the document is not a recognised
form (as `form inspect`, `xsd_conformance: "not_performed"`), or it is
recognised and nonconforming: `error.code = "nonconformant"`,
`checks.xsd_conformance = "failed"`, `findings` populated.

Exit 5: a required check could not be performed. `error.code` is one of
`bundle_unverified`, `validator_unavailable`, `validator_failed`;
`checks.xsd_conformance = "not_performed"`; `data` follows the table.
Exit 5 is new; it keeps "unperformed" distinct from "failed" as the
project boundaries require.

Exit 1, 2, and 3 keep their meanings. `ok` is true only on exit 0.
Whenever the exit code is not 0, stderr carries exactly one fixed line
`opencegeljaras: form validate: <code> (exit N)`. The architecture
document gains: "exit 4 denotes a performed check that failed; exit 5
denotes a required check that could not be performed".

Human output prints the `form inspect` lines, then `bundle:` (verified /
form closure verified / unverified), `validator:` (name and version or
`none`), `xsd conformance:`, one indented line per finding as
`line N element "NAME": "MESSAGE"` with both strings as escaped JSON
literals, then the unchanged boundary lines.

### Determinism and safety

The output is byte-identical for identical input, bundle, and validator
version. Nothing supplied by the document is ever resolved: the bytes
passed the `form inspect` boundary, which forbids DTDs and entities, so
xmllint sees no declarations to expand; `--nonet` blocks network fetches;
the environment carries no catalog variables; the bundle is digest-verified
and its paths are constrained by the manifest rules; and the validator
reads a private temporary copy, which removes the double-read race.

A conforming result means the XML matches the pinned form graph as
interpreted by the named libxml2 version. It does not mean the technical
conditions (signature coverage, timestamps, form cardinality, attachment
formats), the authority's published checks, registry consistency, or legal
sufficiency are met. The skill and README must say so wherever the
command is mentioned.

### Fixtures and tests

Synthetic form fixtures under `tests/fixtures/forms/`:

| Fixture | Purpose |
| :--- | :--- |
| `nevfoglalasi-conforming.xml` | Minimal name-reservation instance that both xmllint and the Python reference accepted on 2026-09-10 |
| `nevfoglalasi-bad-registry-number.xml` | Same with a registry number violating the `cegjegyzekszam` pattern |

Both are hand-built from the public schema text and contain placeholder
data only. The conformance claim in the first row is evidence recorded
here, not something CI re-establishes on every run.

Mechanics are tested without the real bundle and without a real xmllint:
a fake validator that is the integration-test executable itself,
re-executed through `std::env::current_exe()` with a marker argument so no
extra binary ships, controlled by a script file whose name is fixed in its
working directory, that prints a chosen version line, writes
chosen stderr lines, exits with a chosen code, sleeps, or floods stderr.
That exercises the version gate at both bounds, argument construction
(the fake records its argv and cwd), the environment allowlist (the fake
records its environment), exit-code mapping, line parsing, redaction with
and without `--show-values` including an apostrophe inside a value and a
value starting with `{`, message and finding truncation, the timeout with
a short `Duration`, the stderr cap, a missing validator, and the
`document.xml` copy being byte-identical to the input.

Real-validator tests are `#[cfg_attr(not(target_os = "linux"), ignore)]`;
on Linux they fail, not skip, when `xmllint` is absent, and CI installs
`libxml2-utils` on the Linux test job. They cover the synthetic mini graph
with the real xmllint for a conforming and a nonconforming instance. A
further `#[ignore]` test validates the two real fixtures against the
bundle named by `OPENCEGELJARAS_SCHEMA_BUNDLE` and panics with a fixed
message when the variable is unset.

### Ownership

| Path | Package |
| :--- | :--- |
| `crates/opencegeljaras-core/src/form/` (shared entry point extraction), `schema/` (new), `lib.rs` exports | 3 |
| `crates/opencegeljaras-cli/src/schemas.rs` (new) | 3 |
| `tests/fixtures/schemas/` (new) | 3 |
| `crates/opencegeljaras-cli/src/validate.rs`, `xmllint.rs`, the fake validator bin (new) | 4 |
| `tests/fixtures/forms/` additions | 4 |
| `.github/workflows/ci.yml` (install `libxml2-utils` on the Linux test job only) | 4 |
| `Cargo.toml`, `Cargo.lock`, `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `SKILL.md` | both |

The core stays free of filesystem, network, clock, and process use:
package 3's core takes bytes and returns digests and import edges; the CLI
walks nothing and opens only manifest-mapped files. Package 4's process
adapter lives entirely in the CLI.

## Capability advertisement

After package 3: `operations: ["form inspect", "schemas list", "schemas
verify"]`. After package 4: add `"form validate"`. `stage` stays
`scaffold`. The skill tells agents to verify a bundle before validating,
to read `checks.xsd_conformance` rather than the exit code alone, to treat
exit 5 as "not checked", never to pass `--show-values` unless the operator
asked for values, and never to call a conforming form valid, accepted, or
filed.

## Verification

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- schemas list --json
cargo run --locked -p opencegeljaras-cli -- schemas verify \
  --bundle tests/fixtures/schemas/good \
  --manifest tests/fixtures/schemas/manifest.json --json
cargo run --locked -p opencegeljaras-cli -- form validate \
  tests/fixtures/forms/nevfoglalasi-conforming.xml \
  --bundle "$OPENCEGELJARAS_SCHEMA_BUNDLE" --json
```

The last command needs a real bundle and xmllint and is evidence for the
review, not a CI gate.

## Decisions recorded separately

- **Redistribution.** Unchanged: no official schema is vendored. The
  bundle is the operator's own copy; the tool verifies it against pinned
  public digests and reads it locally.
- **Validator choice.** External libxml2 through `xmllint`, versions
  2.9.14 up to but excluding 2.13, with the alternatives and their costs
  recorded above.

## Unresolved questions

- Whether the authority publishes schema updates without changing file
  names; the pinned digests would then fail verification, which is the
  intended fail-closed behaviour, and re-pinning is a maintainer action.
- libxml2 2.13 and later changed xmllint's diagnostic formatting; the
  version line and violation line formats must be re-established before
  the version ceiling is raised.
- macOS ships an older system xmllint; real-validator tests are ignored
  off Linux, and the adapter's out-of-range path is what runs there.
- Bundle directories whose canonical path contains `#`, `?`, or `%` may
  confuse libxml2's URI-based relative import resolution; package 4 must
  test one such directory and either handle or reject it with
  `bundle_unreadable`.
- The dossier graph is digest-verified but never parsed or used; dossier
  preflight is a later milestone.
