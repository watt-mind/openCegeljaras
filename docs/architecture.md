# Architecture

## Purpose and crate boundaries

The procedure layer turns a reviewed company-filing case into explicit
documents, schema-bound fields, check results, and receipt evidence. It does
not own general electronic-signature implementation, mailbox infrastructure,
or business-registry scraping.

Start with two crates. `opencegeljaras-core` contains immutable capability and
source metadata today; future work adds profile selection, bounded form
models, and deterministic checks. `opencegeljaras-cli` owns command parsing,
output, and later file/process adapters. Split a forms or authoring crate only
when implementation complexity warrants it. Sibling tools are not build
dependencies at this stage.

## Current CLI contract

| Command | Behavior |
| :--- | :--- |
| `capabilities [--json]` | Reports scaffold status, the implemented document operations, introspection commands |
| `sources [--json]` | Prints bundled reference metadata; no fetching or freshness check |
| `skill [--json]` | Prints the embedded agent instructions |
| `form inspect <PATH> [--json]` | Recognises one local form document's format under fixed limits |
| `schemas list [--json \| --urls]` | Prints the pinned schema manifest compiled into the binary; fetches nothing |
| `schemas verify --bundle DIR [--manifest FILE] [--json]` | Digest-checks a local schema mirror and audits the form graph's imports |
| `--help`, `--version` | Prints CLI usage or package version |

Successful JSON output is one object with `schema_version: 1`, `ok: true`,
`command`, command-specific `data`, `verified: false`, and `submitted: false`.
The capability schema's `operations` contains implemented document/workflow
operations only; its `introspection_commands` lists bootstrap commands.
`ok` describes command execution, not a dossier, a filing, or legal compliance.
`ok` is true only on exit 0; it never describes validity, filing, or legal
compliance.

Exit 0 denotes success (including help/version); exit 1 denotes a refused
document or bundle input; exit 2 denotes usage errors; exit 3 denotes
output/internal failure; exit 4 denotes a performed check that failed: a
well-formed document that is not a recognised form, or a bundle that does not
match its manifest.
Exit 1 and exit 4 use an error envelope, additive within schema version 1:
`schema_version`, `ok: false`, `command`, `error` with a fixed `code` and
`message`, `data` holding the facts established before the failure, `verified`,
and `submitted`. On exit 4 `error.code` equals `data.form.status`. Whenever the
exit code is not 0, exactly one fixed stderr line names the code and the exit
code, and it never contains the path, document bytes, a name, a namespace, a
declared encoding label, or a parser position. The
[form inspection contract](form-inspection.md) fixes the codes and messages.
`schemas verify` uses the same envelope with these fixed codes and messages:

| Code | Exit | Message |
| :--- | :--- | :--- |
| `bundle_unreadable` | 1 | the bundle directory could not be read |
| `manifest_unreadable` | 1 | the manifest could not be read as a regular file |
| `manifest_invalid` | 1 | the manifest does not satisfy the pinned schema rules |
| `bundle_unverified` | 4 | the bundle does not match the pinned manifest |

Verifying a bundle is a digest comparison plus an import audit of the form
closure. It is not a redistribution licence, not XSD validation, and not
acceptance: `checks.xsd_conformance` stays `not_performed`. The bundle
directory never appears in any output, and bundle-derived strings follow the
`form inspect` escaping rule and 128-scalar cut.
The [schema conformance contract](schema-conformance.md) fixes the manifest
rules, the resource statuses, and both output shapes.

Usage errors emit fixed plain-text stderr diagnostics even when `--json` is
present, without echoing rejected arguments. There is no JSON usage-error
contract in this scaffold. Proposed document
commands are rejected as unknown commands. Do not add permissive stubs.

Additive JSON fields can retain schema version 1; removing, renaming, retyping,
or changing a field's meaning requires an explicit compatibility decision.
Consumers should ignore unknown fields and reject unknown schema versions.

## Proposed domain model

The following is a design, not an implemented API. A `ProcedureProfile` binds
procedure kind, effective period, outer dossier schema, form schema, dependency
bundle digest, attachment rules, and separately evidenced delivery channels.
The form's namespace is evidence of format, not proof that that format is
currently accepted. Schema acquisition must be an explicit maintainer action.

A `Finding` has a stable code, check identifier, location, severity,
provenance, and `passed`, `failed`, `not_performed`, or `not_applicable` status.
No complete/acceptable verdict is permitted while a required check is
unperformed. Remote identity, registry, and authority decisions must stay
outside purely local check claims. Human review findings remain distinct from
mechanical schema failures.

A `ReceiptEvent` preserves original evidence and separately represents the
authority's asserted outcome, authentication status, correlation confidence,
and timestamp provenance. Outbound preparation, transport handoff, receipt,
technical rejection, forwarding, court decision, and acknowledgment of service
are different events. Their order can be incomplete or ambiguous. Do not
compute legal deadlines from an email Date header.

## Proposed composition

```text
Reviewed case data + existing documents
                 |
       openCegeljaras forms / preflight
                 |
       openSzigno ES3 writer / signer / verifier
       or a separately supported container adapter
                 |
       Explicit channel handoff, separately authorized
                 |
       Original receipts -> verifier -> receipt interpretation
                 |
       Correspondence archive / attorney review
```

Integration must pin tool versions, validate JSON contract versions, keep
argv free of secrets, and preserve original signed bytes. The authoring API
must demonstrate support for domain metadata and form placement; the ability
to create a generic ES3 does not establish that support. Unsupported containers
remain unsupported rather than being renamed or silently converted.

## Resource and trust boundaries

Before accepting input, implement limits for bytes, XML depth, node/text counts,
per-element attribute and namespace-declaration counts, archive entry count,
expanded size, and compression ratio. A cap that only the parser would enforce
is not enough when the parser's own handling of that construct is quadratic:
`form inspect` caps both attributes and namespace declarations per element in
its pre-parse scan for that reason, and re-checks them on the parsed tree.
One shared entry point, "parse bytes under `Limits` into a bounded tree",
serves both the form classifier and the schema import audit, so both refuse the
same encodings, declarations, and limits. The audit runs under the same limits
with one documented exception: two form-closure resources of the pinned 2026
graph declare 82 and 85 namespace prefixes on their root element, so the
64-declaration form cap is raised to 256 for schema resources only, and never
for a form document.
Reject DTD/entity expansion and resolve XSD imports only from an audited local
bundle. Preserve
original encoding and bytes where signatures depend on them. External XSLT
and HTML content must not execute during inspection.

No ambient network or clock belongs in the core. Future checks needing a date
or external evidence receive those explicitly. Do not write cryptography or
assume that a signature timestamp alone proves qualified status. No local
report is an authority receipt or a determination of legal effect.
