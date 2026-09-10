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
the official XSD graph. Conformance needs the complete 139-resource form
closure and a validator that resolves `xs:import` across 142 namespaces.

Evidence gathered on 2026-09-10 from the development server:

- The complete closure listed in the [schema manifest](../data/schema-manifest.json)
  was re-fetched into a scratch mirror laid out by URL host and path. All
  149 resources matched their pinned SHA-256 digests and byte counts.
- Every `schemaLocation` in the closure is relative except one absolute
  reference to `http://www.w3.org/2001/03/xml.xsd` inside the dossier
  graph's copy of `xml.xsd`. The form graph therefore resolves entirely
  from local relative paths once the root is a local file.
- The closure uses `xs:pattern` (16 files), `substitutionGroup` (3),
  `xs:include` (2), `xs:any` (2), and no identity constraints or `redefine`.
- `xmllint` from libxml2 2.9.14 loads the full form graph offline with
  `--nonet` in about 20 ms and validates a document in the same run. Its
  exit codes are 0 for a conforming document, 3 for schema violations, 1 for
  a document parse error, and 5 for a schema that fails to load. Each
  violation is one stderr line of the form
  `PATH:LINE: element NAME: Schemas validity error : MESSAGE`.
- The Python `xmlschema` 4.3.2 package, an independent implementation,
  loads the same graph in 0.3 s and agrees with xmllint on a hand-built
  minimal name-reservation instance (conforming) and on the same instance
  with a malformed company registry number (rejected by the same pattern).
  It also agrees that the existing synthetic inspect fixtures are not
  conforming, which they were never claimed to be.
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

## Package 3: verified schema bundle

### Scope

Two introspection-style subcommands under `schemas`:

```sh
opencegeljaras schemas list [--json] [--urls]
opencegeljaras schemas verify --bundle DIR [--manifest FILE] [--json]
```

`list` prints the pinned manifest compiled into the binary and never
fetches. `verify` checks an operator-supplied local mirror against the
pinned digests and audits its import graph. Neither downloads, writes, or
modifies anything under `DIR`.

Out of scope: fetching schemas, vendoring them, generating catalogs for
other tools, validating documents, and the dossier graph's dependency
audit beyond the digest check (its `xml.xsd` absolute reference is
recorded as an accepted external reference).

### Manifest

`data/schema-manifest.json` is compiled into the core with
`include_str!`, parsed with `deny_unknown_fields`, and exposed as
`SchemaManifest { snapshot_version, retrieved_at, purpose, roots, resources }`
where each resource has `url`, `bytes`, `sha256`, `target_namespace`
(nullable), `dependencies`, and `roots`. A unit test asserts that the
compiled manifest parses, has 149 resources, unique URLs, lowercase
64-hex digests, and that every dependency URL is itself a resource.

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
resources: [{ url, bytes, sha256, target_namespace, dependencies: [...],
roots: [...] }], notice }` with the notice "pinned research metadata;
nothing was fetched or verified". `--urls` prints only the URLs, one per
line, in manifest order, with no header, and is a usage error together
with `--json`.

### `schemas verify`

For each manifest resource, in manifest order:

1. Map the URL to `DIR/<host>/<path>`. The path is the URL path with
   percent-encoding left as is; no `..`, empty, or absolute segments can
   arise from the pinned manifest, and a unit test asserts that.
2. If the file is missing, not a regular file after following symlinks,
   or unreadable: status `missing`.
3. Read at most 4 MiB. Larger: status `oversize`.
4. Compare the byte count and SHA-256 (`sha2` 0.11, MIT OR Apache-2.0,
   MSRV 1.85). Mismatch: status `mismatch`. Match: status `verified`.
5. For a verified resource, decode and parse it with the same bounded XML
   boundary `form inspect` uses (same `Limits`, same prohibited
   constructs, no network). A resource that fails that boundary is status
   `unparseable`. Otherwise collect every `import` and `include` element in
   the XSD namespace and resolve `schemaLocation` against the resource URL
   with plain RFC 3986 relative resolution; then normalise `http://` to
   `https://` for the two hosts `schema.e-cegjegyzek.hu` and
   `www.e-cegjegyzek.hu`, exactly as the manifest records. Each resolved
   location must be a manifest URL, and for `import` its `namespace`
   attribute must equal that resource's `target_namespace`. A location
   outside the manifest, or a namespace mismatch, marks the resource
   `unpinned_import` and lists the offending locations. The single
   `http://www.w3.org/2001/03/xml.xsd` reference is expected and reported as
   `external_reference` on that resource, not as a failure, and a test
   fixes it as the only permitted external reference.

Files under `DIR` that are not manifest resources are counted as
`extra_files` (count only, no names) and do not affect the verdict.

Result: `verified: true` only if every resource is `verified` with no
`unpinned_import`. JSON `data`:

```json
{
  "manifest": { "source": "compiled", "retrieved_at": "2026-09-10", "count": 149 },
  "bundle": { "verified": true, "resources": 149, "verified_count": 149,
              "missing": 0, "mismatch": 0, "oversize": 0, "unparseable": 0,
              "unpinned_import": 0, "extra_files": 0 },
  "problems": [ { "url": "https://…", "status": "mismatch" } ],
  "external_references": [ "http://www.w3.org/2001/03/xml.xsd" ],
  "checks": { "digests": "passed", "import_graph": "passed",
              "xsd_conformance": "not_performed" },
  "notice": "digest and import audit of a local mirror; not a licence, not validation"
}
```

`problems` lists every non-verified resource by URL and status (URLs are
manifest strings, never bundle paths) plus, for `unpinned_import`, the
offending resolved locations. `DIR` itself never appears in any output.

Exit codes: 0 when `verified` is true; 4 with the same envelope shape and
`error.code = "bundle_unverified"` when any resource failed; 1 with
`error.code = "bundle_unreadable"` when `DIR` is not a readable directory;
2 for usage errors. Human output prints the counts, then one line per
problem, then the check lines, then `verification: not performed` and
`submission: not performed`.

`--manifest FILE` reads a manifest from a local file instead of the
compiled one, for tests and future snapshots. It is parsed with the same
strict types and limits (4 MiB). Output then reports `manifest.source:
"file"` and the notice adds "unpinned manifest". `form validate` accepts
the same option with the same reporting.

### Fixtures and tests

`tests/fixtures/schemas/` holds a synthetic mini graph under CC0: a
manifest of five resources on the two real hosts (one wrapper importing
two variant schemas, one shared simple-type schema, and one copy that
carries an `http://` import to exercise normalisation), the mirrored
files, and a duplicate bundle with one tampered file and one missing file.
Tests cover: list human and JSON shape; `--urls` exact lines; verify on the
good bundle (exit 0, all counts); the tampered bundle (exit 4, `mismatch`
and `missing` listed by URL, no path echoed); a bundle whose schema imports
a location outside the manifest (`unpinned_import`); namespace mismatch;
the external-reference allowance; an oversize file; an unreadable `DIR`; a
`--manifest` file with an unknown field (usage-level rejection as
`manifest_invalid`, exit 1); and the compiled-manifest invariants. A test
marked `#[ignore]` verifies the real bundle when the environment variable
`OPENCEGELJARAS_SCHEMA_BUNDLE` names one; CI does not set it.

## Package 4: `form validate` through the xmllint adapter

### Scope

```sh
opencegeljaras form validate PATH --bundle DIR [--manifest FILE]
    [--xmllint EXE] [--show-values] [--json]
```

Behaviour, in order:

1. Run the complete `form inspect` boundary on `PATH`. Any exit 1 outcome
   is returned unchanged. Any exit 4 outcome is returned unchanged with
   `checks.xsd_conformance: "not_performed"`; only a `recognized` form
   proceeds.
2. Run the package 3 bundle verification on `DIR`. If it does not verify,
   exit 5 with `error.code = "bundle_unverified"` and the verification
   `problems`; nothing is spawned.
3. Locate the validator: `--xmllint EXE` if given, else the first `xmllint`
   on `PATH`. Run it with `--version`, read the `using libxml version N`
   line from stderr, and require `N >= 20914`. Missing executable, failed
   spawn, unparseable version, or an older version: exit 5,
   `error.code = "validator_unavailable"`, with `validator.version` when it
   was read.
4. Create a private temporary directory (`tempfile` 3.27, MIT OR
   Apache-2.0, MSRV 1.63) containing an OASIS XML catalog that rewrites the
   `http://` and `https://` prefixes of the two schema hosts and
   `http://www.w3.org/2001/03/xml.xsd` to `file://` locations inside the
   bundle. The catalog is defence in depth: the form graph resolves
   relatively without it.
5. Spawn the validator once with a cleared environment plus `PATH`, the
   temporary directory as working directory, `XML_CATALOG_FILES` set to the
   catalog, and the arguments `--nonet --noout --schema ROOT PATH`, where
   `ROOT` is the bundle path of the wrapper schema. `--noent`, `--xinclude`,
   `--load-trace`, and `--huge` are never passed. Stdout is discarded,
   stderr is captured up to 1 MiB, and the process is killed after 60
   seconds. Timeout or kill: exit 5, `error.code = "validator_failed"`.
6. Map the exit code: 0 is conforming; 3 is nonconforming; anything else
   (including 1 and 5) is `validator_failed` with the exit code reported.
   Parse stderr lines of the form `PATH:LINE: element NAME: Schemas
   validity error : MESSAGE`. `PATH` is dropped; `LINE` becomes an
   integer; `NAME` becomes `element`; `MESSAGE` is trimmed. Lines that do
   not match are counted in `unparsed_lines` and never echoed. The
   trailing `PATH fails to validate` line is expected and dropped.
7. Redact values. By default every single-quoted span in `MESSAGE` whose
   content does not start with `{` is replaced by `'…'`; spans starting
   with `{` are schema-qualified names and are kept. `--show-values` keeps
   every span verbatim and sets `values_redacted: false`. Messages are cut
   at 512 Unicode scalar values with `…`. At most 256 findings are kept,
   then `findings_truncated: true`.

Every document-derived string in human output follows the `form inspect`
escaping rule, and stderr diagnostics stay fixed lines that never contain
the path, the bundle path, or validator output.

### Output

Exit 0: conforming. `data` is the `form inspect` structure plus
`validator: { name: "xmllint", version: 20914, exit_code: 0 }`,
`bundle: { verified: true, manifest: { source, retrieved_at } }`,
`findings: []`, `values_redacted`, and `checks` with `xsd_conformance:
"passed"` alongside the unchanged `well_formed`, `signature`, and
`authority_acceptance` entries. The `notice` reads "XSD conformance of one
form against the pinned 2026 graph; not the technical conditions, not a
signature check, not acceptance".

Exit 4: the document was inspected but is not a recognised form (as
`form inspect`), or it is recognised and nonconforming; in the latter case
`error.code = "nonconformant"`, `checks.xsd_conformance = "failed"`, and
`findings` holds `{ line, element, message }` entries in validator order.

Exit 5: the check could not be performed. `error.code` is one of
`bundle_unverified`, `validator_unavailable`, `validator_failed`;
`checks.xsd_conformance = "not_performed"`; `data` carries whatever was
established (the inspect structure, the bundle report, the validator
version and exit code). Exit 5 is new: it means "unperformed", which the
project boundaries require to stay distinct from "failed".

Exit 1, 2, and 3 keep their meanings. The `ok` field is true only on exit
0. Human output prints the `form inspect` lines, then `bundle:`,
`validator:`, `xsd conformance:`, one indented line per finding as
`line N element "NAME": MESSAGE`, then the unchanged boundary lines.

### Determinism and safety

The output is byte-identical for identical input, bundle, and validator
version. Nothing supplied by the document is ever resolved: the input has
already passed the `form inspect` boundary, which forbids DTDs and
entities, so xmllint sees no declarations to expand; `--nonet` blocks
network fetches; the catalog points only inside the bundle; and the bundle
content is digest-verified before use. The adapter reads the input file
twice (inspect, then xmllint), which a concurrent writer could exploit to
show the validator a different file; this is accepted for a local
read-only tool and recorded here.

A conforming result means the XML matches the pinned schema graph as
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

Mechanics are tested without the real bundle: a synthetic schema graph in
`tests/fixtures/schemas/` with a root that accepts one tiny document
shape, a conforming and a nonconforming instance, and a fake `xmllint`
script (or a tiny Rust test binary) that prints a chosen version line,
writes chosen stderr lines, and exits with a chosen code. That exercises
the version gate, argument construction (asserted from a recorded argv
file), catalog contents, exit-code mapping, line parsing, redaction with
and without `--show-values`, truncation, the timeout (script sleeps), the
stderr cap, a missing validator, and an older version.

Real-validator tests run only where `xmllint` is present and are skipped
with a visible message otherwise; CI installs `libxml2-utils` on the
Linux test job so they run there. They cover: the synthetic graph with the
real xmllint for both instances; and, gated on
`OPENCEGELJARAS_SCHEMA_BUNDLE`, the two real fixtures against a verified
real bundle. A skipped real-validator test is reported as skipped, never
as passed.

### Ownership

| Path | Package |
| :--- | :--- |
| `crates/opencegeljaras-core/src/schema/` (new), `lib.rs` exports | 3 |
| `crates/opencegeljaras-cli/src/schemas.rs` (new) | 3 |
| `tests/fixtures/schemas/` (new) | 3 |
| `crates/opencegeljaras-cli/src/validate.rs`, `xmllint.rs` (new) | 4 |
| `tests/fixtures/forms/` additions | 4 |
| `.github/workflows/ci.yml` (install `libxml2-utils` on the Linux test job only) | 4 |
| `Cargo.toml`, `Cargo.lock`, `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `SKILL.md` | both |

The core stays free of filesystem, network, clock, and process use:
package 3's core takes bytes and returns digests and import edges; the CLI
walks the directory. Package 4's process adapter lives entirely in the CLI.

## Capability advertisement

After package 3: `operations: ["form inspect", "schemas list", "schemas
verify"]`. After package 4: add `"form validate"`. `stage` stays
`scaffold`. The skill tells agents to verify a bundle before validating,
to read `checks.xsd_conformance` rather than the exit code alone, to treat
exit 5 as "not checked", and never to call a conforming form valid,
accepted, or filed.

## Verification

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- schemas list --json
cargo run --locked -p opencegeljaras-cli -- schemas verify --bundle tests/fixtures/schemas/good --manifest tests/fixtures/schemas/manifest.json --json
cargo run --locked -p opencegeljaras-cli -- form validate tests/fixtures/forms/nevfoglalasi-conforming.xml --bundle "$OPENCEGELJARAS_SCHEMA_BUNDLE" --json
```

The last command needs a real bundle and xmllint and is evidence for the
review, not a CI gate.

## Decisions recorded separately

- **Redistribution.** Unchanged: no official schema is vendored. The
  bundle is the operator's own copy; the tool verifies it against pinned
  public digests and reads it locally.
- **Validator choice.** External libxml2 through `xmllint`, minimum
  2.9.14, with the alternatives and their costs recorded above.

## Unresolved questions

- Whether the authority publishes schema updates without changing file
  names; the pinned digests would then fail verification, which is the
  intended fail-closed behaviour, and re-pinning is a maintainer action.
- Windows CI has no `xmllint`; real-validator tests are skipped there and
  the adapter's unavailable path is what runs.
- The dossier graph (`e-cegeljaras2026.xsd`) is digest-verified but not
  used by `form validate`; dossier preflight is a later milestone.
