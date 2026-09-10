# Dossier inspection and preflight contract

Reviewed on **2026-09-10**. This contract specifies two read-only commands
over a 2026 company-court e-dossier (`.es3`): `dossier inspect`, which
reports the dossier's own account of itself and which form it carries, and
`dossier preflight`, which turns that into deterministic findings and,
given a verified bundle, an XSD-conformance result for the outer dossier.
Neither command verifies a signature, sends anything, or decides
acceptance.

Status: **contract, not implemented.** `capabilities` lists the operations
only after the behaviour and tests below exist.

## Problem and context

A company filing is a dossier: an XML container whose profile names the
procedure, whose documents carry role titles, and whose form is one
base64 payload among attachments. `form inspect` and `form validate` see
only a bare form. Nothing yet answers "is this dossier a 2026 company
filing, which procedure does it claim, is the form inside it the matching
variant, and are the required roles present?" That is the dossier
preflight the research and roadmap put first among integrations.

Evidence gathered on 2026-09-10 from the pinned dossier schema
`e-cegeljaras2026.xsd` (ISO-8859-2, namespace
`http://www.e-cegjegyzek.hu/2026/e-cegeljaras#`, digest-verified in the
package 3 mirror) and its ten-resource closure:

- `Dossier` holds one `DossierProfile`, one `Documents` whose `Id` is
  fixed to `Object0`, then any number of `ds:Signature` or `TimeStamp`
  elements. `Dossier` carries optional `AckEmail` (pattern
  `([^@]+@[^\.]+\..+)?`) and `Location` attributes.
- `DossierProfile` holds `Title`, an enumeration of exactly six procedure
  kinds: `Bejegyzési kérelem`, `Ismételt bejegyzési kérelem`,
  `Változás bejegyzési kérelem`, `Névfoglalási kérelem`, `Egyéb beadvány`,
  `Hiánypótlás`; `E-category`, an enumeration of four values
  (`electronic dossier`, `electronic acknowledgement`, `elektronikus akta`,
  `elektronikus átvételi elismervény`); `CreationDate`; an optional choice
  of `Cégforma` or `Cégjegyzékszám`; optional `Cégnév` and `Cégszékhely`,
  all four company-identity elements typed and carrying a fixed
  `Custom="true"` attribute.
- `Document` holds a `DocumentProfile` with `Title` restricted by 194
  patterns that name attachment roles, the first being
  `A kérelem formanyomtatványa` (the application form); optional
  `E-category`; `CreationDate`; `Format/MIME-Type` with required `type`,
  `subtype`, `extension`; `SourceSize` with `sizeUnit` fixed to `B`;
  `BaseTransform` with `Transform/@Algorithm` from `zip`, `encrypt`,
  `base64`; optional `RecipientCertificateList` and `Metadata`; then one
  `ds:Object` and optional signatures or timestamps.
- The XSD permits zero documents and no signatures; the technical
  conditions add signature and timestamp requirements that this contract
  does not encode, because that page could not be re-read with verified
  TLS from the development server.
- `xmllint` 2.9.14 loads the dossier closure offline, including the two
  resources that carry a DOCTYPE with an internal entity, and the Python
  `xmlschema` 4.3.2 package agrees with it on a synthetic 2026 dossier
  built by rewriting an openSzigno-created container (namespace,
  `Object0`, procedure title, form-role title): conforming; and on the
  same dossier with a procedure title outside the enumeration: rejected.
- The sibling openSzigno 0.9.0 inspects that dossier only with
  `--allow-namespace` for the 2026 namespace (its allow-list stops at the
  2014 profile), reports `valid_structure: true`, and its `create` writes
  the default Microsec namespace, `Id="documents"`, and fixed categories,
  confirming the integration gap the research recorded. openSzigno keeps
  ownership of payload extraction beyond plain base64, of encrypted and
  zipped transforms, and of all cryptographic verification.

## Scope

```sh
opencegeljaras dossier inspect FILE [--show-values] [--json]
opencegeljaras dossier preflight FILE [--bundle DIR] [--manifest FILE]
    [--xmllint EXE] [--show-values] [--json]
```

Out of scope, not to be stubbed: signature or timestamp verification,
decryption, zip transforms, nested dossiers beyond flagging them, writing
or repairing dossiers, the technical conditions' signature-coverage and
timestamp rules, receipt dossiers (`electronic acknowledgement`) beyond
recognising the category, and any claim of acceptance.

## Dossier boundary

A dossier is read through the shared bounded parsing entry point with a
dossier `Limits` profile that differs from the form profile only where the
container shape requires: input 64 MiB, one text node up to 64 MiB
(base64 payloads are text), elements 1 000 000, parser nodes 4 × elements,
depth 128, attributes 64, namespace declarations 256. Prohibited constructs,
encodings (UTF-8, ISO-8859-2), declaration sniffing, and diagnostics rules
are those of `form inspect`. The two profiles are constants in the core
with a unit test asserting each field.

Recognition: the root must be `Dossier`. In
`http://www.e-cegjegyzek.hu/2026/e-cegeljaras#` the profile is
`dossier-2026` and status `recognized`. In
`http://www.e-cegjegyzek.hu/<4 digits>/e-cegeljaras#` with another year
the status is `unsupported_version`. Any other namespace, including the
default Microsec namespace `https://www.microsec.hu/ds/e-szigno30#`, is
`unrecognized_root`: a generic e-dossier is not a company filing.
Structural lookups use the dossier's own namespace; `ds:Signature` and
`ds:Object` use the XMLDSig namespace.

## `dossier inspect`

After recognition the inspector reads, never resolves, never decodes
beyond what is stated:

- `dossier`: `title` and `category` are reported verbatim only when they
  are one of the enumerated values above, otherwise `null` with
  `title_known: false` or `category_known: false`; `creation_date` is
  reported verbatim when it matches `xs:dateTime` lexically, otherwise
  `null`; `ack_email_present` and `location_present` booleans; the
  company-identity block as presence booleans `cegforma`,
  `cegjegyzekszam`, `cegnev`, `cegszekhely`; `documents_id_fixed` true
  when `Documents/@Id` is `Object0`.
- `documents[]`, in document order, at most 256 entries then
  `documents_truncated`: `index`, `role` (the document title verbatim
  when it matches one of the 194 role patterns compiled into the core,
  else `null` with `role_known: false`), `category` (as above, with the
  document category enumeration), `mime` (`type`, `subtype`, `extension`
  as declared, each cut at 64 scalars; `charset` presence), `source_size`
  (integer or null), `transforms` (the `Algorithm` values in order, only
  the three known ones, else the entry is `unknown`), `object_present`,
  `nested_dossier` (true when the declared subtype is `nldossier2`
  case-insensitively or the extension is `dosszie`), `signatures` and
  `timestamps` counts placed inside the document, and `form`.
- `form` for a document is the result of running the complete
  `form inspect` boundary over the payload when, and only when, the
  transform chain is exactly `base64`, the declared type is `text/xml` or
  `application/xml` or the extension is `xml`, and the base64 text decodes
  (whitespace ignored, strict alphabet, padding required) to at most
  16 MiB. Then `form` is `{ status, profile, variant, sections }` exactly
  as `form inspect` would report, or a `rejected` object with the inspect
  error code. Otherwise `form` is `null` and `payload` says why:
  `not_xml`, `transform_not_plain_base64`, `base64_invalid`,
  `payload_too_large`, or `object_missing`. Payloads are never written to
  disk and never printed.
- `signatures_present` and `timestamps_present` count the elements at
  dossier level and inside documents; nothing is verified.
- With `--show-values` the dossier title, category, creation date, and the
  company-identity values, and each document's declared title, are
  printed verbatim (cut at 256 scalars); without it only the enumerated
  and pattern-matched values appear, and `values_redacted` is true.

Every string that reaches the output is either an enumerated schema value,
a pattern-matched role, a declared MIME token, or, with `--show-values`,
a document value, all printed under the `form inspect` escaping rule.
Element names inside documents' payloads never appear except through the
`form` object's contract.

JSON `data` on exit 0 (alphabetical when emitted):

```json
{
  "input": { "bytes": 0, "bom": false, "xml_declaration": true,
             "encoding": "UTF-8" },
  "root": { "local_name": "Dossier", "namespace": "http://…#" },
  "dossier": { "status": "recognized", "profile": "dossier-2026",
               "title": "Névfoglalási kérelem", "title_known": true,
               "category": "electronic dossier", "category_known": true,
               "creation_date": "2026-01-01T00:00:00Z",
               "ack_email_present": false, "location_present": false,
               "identity": { "cegforma": false, "cegjegyzekszam": false,
                             "cegnev": false, "cegszekhely": false },
               "documents_id_fixed": true },
  "documents": [ { "index": 0, "role": "A kérelem formanyomtatványa",
                   "role_known": true, "category": "electronic data",
                   "category_known": true,
                   "mime": { "type": "text", "subtype": "xml",
                             "extension": "xml", "charset_present": false },
                   "source_size": 1209, "transforms": ["base64"],
                   "object_present": true, "nested_dossier": false,
                   "signatures": 0, "timestamps": 0,
                   "payload": "inspected",
                   "form": { "status": "recognized", "profile": "form-2026",
                             "variant": { "id": "nevfoglalasi-kerelem-20210401",
                                          "local_name": "…", "namespace": "…" },
                             "sections": [] } } ],
  "documents_truncated": false,
  "signatures_present": 0,
  "timestamps_present": 0,
  "values_redacted": true,
  "checks": { "well_formed": "passed", "xsd_conformance": "not_performed",
              "signature": "not_performed",
              "authority_acceptance": "not_performed" },
  "notice": "structural account of one dossier and of the form it carries; nothing was verified, decrypted, or accepted"
}
```

Exit codes: 0 recognised 2026 dossier; 1 input rejected with the
`form inspect` codes; 2 usage; 3 output; 4 `unsupported_version` or
`unrecognized_root` with the null rules of `form inspect` (`dossier`
carries only `status`; `documents` null). Stderr line
`opencegeljaras: dossier inspect: <code> (exit N)`. Human output: one
fact per line in the order of the JSON keys, `documents:` followed by one
indented block per document (`[N] role: …`, `mime: …`, `transforms: …`,
`form: STATUS VARIANT`), then the four boundary lines, document-derived
strings as escaped JSON literals.

## `dossier preflight`

Preflight runs `dossier inspect` and then evaluates findings. Each finding
is `{ code, status, source, detail }` where `status` is `passed`,
`failed`, `not_performed`, or `not_applicable`, `source` names the
evidence (`xsd-2026` for the dossier schema, `form-2026` for the form
contract, `openCegeljaras` for a rule this project derives), and `detail`
is a fixed string per code and status, never a document value.

| Code | Rule | Source |
| :--- | :--- | :--- |
| `procedure_title` | `dossier.title_known` | `xsd-2026` |
| `dossier_category` | `dossier.category_known` and the value is `electronic dossier` or `elektronikus akta`; `electronic acknowledgement` and its Hungarian form make every later finding `not_applicable` and set `receipt: true` | `xsd-2026` |
| `documents_id` | `documents_id_fixed` | `xsd-2026` |
| `form_document_present` | exactly one document whose role is `A kérelem formanyomtatványa`; zero or several fail | `xsd-2026` |
| `form_recognized` | that document's `form.status` is `recognized`; `not_performed` when `payload` is not `inspected` | `form-2026` |
| `form_matches_procedure` | the variant matches the title: `Névfoglalási kérelem` needs `nevfoglalasi-kerelem-20210401`; `Bejegyzési kérelem` and `Ismételt bejegyzési kérelem` need `cegbejegyzesi-kerelem-20260101`; `Változás bejegyzési kérelem` needs `valtozasbejegyzesi-kerelem-20240101`; `Egyéb beadvány` and `Hiánypótlás` are `not_applicable`; `not_performed` when the form is not recognised | `openCegeljaras` |
| `document_roles_known` | every document has `role_known` | `xsd-2026` |
| `transforms_supported` | every document's transform chain is one of `base64`, `zip,base64`, `encrypt,base64`, `zip,encrypt,base64` | `xsd-2026` |
| `signature_present` | `signatures_present >= 1`; presence only, validity `not_performed` | `openCegeljaras` |
| `timestamp_present` | `timestamps_present >= 1`; presence only | `openCegeljaras` |
| `xsd_conformance` | the outer dossier XML conforms to the pinned dossier graph; `not_performed` without `--bundle` | `xsd-2026` |

With `--bundle`, preflight verifies the bundle with package 3 rules,
requires `dossier_closure_verified` (every resource reachable from
`roots[1]` digest-verified; the dossier closure's DOCTYPE resources are
digest-only by contract), and runs the package 4 xmllint adapter with
`ROOT` = the bundle path of `roots[1]` and the dossier bytes as
`document.xml`, under the dossier `Limits`. Findings from the validator
are reported as in `form validate` under `xsd_findings`, redacted by the
same rules. `bundle_unverified`, `validator_unavailable`, and
`validator_failed` keep their exit-5 meaning and leave `xsd_conformance`
`not_performed`. Package 3 gains `dossier_closure_verified` in its verify
output, computed like `form_closure_verified` over `roots[1]`, and the
manifest unit test asserts the dossier closure has 10 resources.

Verdict: `preflight.status` is `passed` when every finding is `passed` or
`not_applicable` and at least `xsd_conformance` was performed; `failed`
when any finding failed; `incomplete` when nothing failed but a finding
is `not_performed`. There is no "ready to file" wording anywhere: the
notice reads "deterministic preflight findings over one dossier; not the
technical conditions, not signature validity, not acceptance".

Exit codes: 0 when `preflight.status` is `passed` or `incomplete` with no
failure; 4 when a finding failed (`error.code = "preflight_failed"`) or
the dossier is not recognised (inspect codes); 5 for the unperformed
XSD cases as in `form validate`; 1, 2, 3 as usual. `ok` is true only on
exit 0. Human output: the inspect report, then `findings:` with one
indented `CODE: STATUS (SOURCE)` line each, `xsd findings:` when present,
`preflight: STATUS`, then the boundary lines.

## Determinism and safety

Both commands are pure functions of their inputs and the bundle. Nothing
in a dossier is resolved: no `OBJREF`, `SIGREF`, `Location`, or `AckEmail`
is followed or used; base64 is decoded only under the stated conditions
and only into memory; zip and encrypt transforms are never applied;
signatures are counted, never parsed beyond their element and placement.
The acknowledgement e-mail is never contacted, and reading a receipt
dossier never acknowledges service. Timeouts, environment, and argv rules
of the xmllint adapter apply unchanged.

## Ownership

| Path | Role |
| :--- | :--- |
| `crates/opencegeljaras-core/src/dossier/` (new: model, recognition, roles table, base64, findings), `form/mod.rs` limits profile, `schema` dossier closure | Core |
| `crates/opencegeljaras-cli/src/dossier.rs` (new), `main.rs` wiring, `schemas.rs` for `dossier_closure_verified`, `validate.rs`/`xmllint.rs` reuse | CLI |
| `crates/opencegeljaras-cli/tests/dossier.rs` (new) | Contract tests |
| `tests/fixtures/dossiers/` (new) | Synthetic fixtures |
| `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `SKILL.md` | Advertisement, last |

One implementation worker. The 194 role patterns and the six procedure
titles are copied into the core as string constants: they are enumerated
values a document must literally contain, not schema text, and the
redistribution note in the specification inventory records that reading.

## Fixtures and tests

Synthetic dossiers under `tests/fixtures/dossiers/`, CC0, built by hand
from the schema facts above (the first two are the evidence dossiers from
2026-09-10, whose payload is the synthetic conforming name-reservation
form):

| Fixture | Purpose |
| :--- | :--- |
| `nevfoglalas-2026.es3` | Conforming 2026 dossier, one form document, no signature; both validators accepted it |
| `nevfoglalas-2026-bad-procedure-title.es3` | Same with a title outside the enumeration; both rejected it |
| `microsec-default.es3` | Default-namespace container → `unrecognized_root` |
| `dossier-2023.es3` | 2023 namespace → `unsupported_version` |
| `mismatched-variant.es3` | `Bejegyzési kérelem` carrying the name-reservation form |
| `two-forms.es3` | Two documents with the form role |
| `zip-form.es3` | Form document with `zip,base64` → `transform_not_plain_base64` |
| `unknown-role.es3` | A document whose title matches no role pattern |
| `receipt.es3` | `electronic acknowledgement` category, findings `not_applicable` |
| `bad-base64.es3` | Invalid base64 in the form object |

Tests cover: recognition statuses; every `payload` reason; the `form`
object equality with `form inspect` on the extracted synthetic form;
`--show-values` gating with a sentinel title; each finding code in each
reachable status; the receipt rule; `documents_truncated` and payload
caps with generated dossiers; the 64 MiB text node accepted and 64 MiB
plus one rejected under the dossier profile while the form profile still
rejects 1 MiB plus one; determinism; human shapes; exit paths; with the
fake validator, the preflight XSD path; `#[cfg_attr(not(target_os =
"linux"), ignore)]` real-xmllint tests on the two evidence dossiers
against the mini graph's dossier-style resource is not possible, so the
real-validator test is `#[ignore]` on `OPENCEGELJARAS_SCHEMA_BUNDLE` and
validates both evidence dossiers against the real dossier closure; that
`capabilities` lists both operations; that nothing under `samples/` is
referenced.

## Capability advertisement

After the tests pass, `operations` gains `"dossier inspect"` and
`"dossier preflight"`. The skill states that `preflight.status: passed`
means every deterministic finding passed and the outer XML conforms, and
nothing more; that signature validity needs openSzigno's `verify` with
trust material; that receipts are recognised, never acknowledged.

## Verification

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- dossier inspect \
  tests/fixtures/dossiers/nevfoglalas-2026.es3 --json
cargo run --locked -p opencegeljaras-cli -- dossier preflight \
  tests/fixtures/dossiers/nevfoglalas-2026.es3 \
  --bundle "$OPENCEGELJARAS_SCHEMA_BUNDLE" --json
```

## Decisions recorded separately

- **Own structural read versus delegating to openSzigno.** The outer
  dossier is XML this project already parses under its boundary; reading
  profile facts and one plain-base64 payload needs no container code
  beyond that. Everything cryptographic, every other transform, and
  extraction stay with openSzigno, to be composed through an explicit
  adapter in a later package (`dossier verify`).
- **Role and title constants.** Literal enumerated values are reproduced
  as constants; the schema text itself is still not vendored.

## Unresolved questions

- The technical conditions (signature coverage, timestamps, nested
  dossier rules, attachment formats) were not re-read; presence findings
  are the honest maximum until then.
- Whether the authority's own tools set `E-category` to the English or
  the Hungarian value; both are accepted.
- Receipt dossiers deserve their own contract (milestone 4); this one only
  recognises them.
