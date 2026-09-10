# Dossier inspection and preflight contract

Reviewed on **2026-09-10**. This contract specifies two read-only commands
over a 2026 company-court e-dossier (`.es3`): `dossier inspect`, which
reports the dossier's own account of itself and which form it carries, and
`dossier preflight`, which turns that into deterministic findings and,
given a verified bundle, an XSD-conformance result for the outer dossier.
Neither command verifies a signature, sends anything, or decides
acceptance.

Status: implemented and reviewed; this document remains the contract the
implementation is held to.

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
  XSD `pattern` facets that name attachment roles, the first being
  `A kérelem formanyomtatványa` (the application form). They are regular
  expressions, not literals: 34 contain `( … )` groups, two end in
  `( - .*)?` (a fixed prefix plus an arbitrary suffix), and many contain
  an unescaped `.`. Then an optional, untyped `E-category` (the six-value
  enumeration `electronic data`, `electronic document`,
  `electronic record`, `elektronikus adat`, `elektronikus dokumentum`,
  `elektronikus irat` lives on an unreferenced global element and is not
  enforced by the XSD on documents); `CreationDate`; `Format/MIME-Type`
  with required `type`, `subtype`, `extension` and optional `charSet`;
  optional `MimeChecked`; `SourceSize` with `sizeUnit` fixed to `B`;
  `BaseTransform` with optional `Transform/@Algorithm` from `zip`,
  `encrypt`, `base64`; optional `RecipientCertificateList` and
  `Metadata`; then one `ds:Object` and optional signatures or timestamps.
- The XSD permits zero documents and no signatures; the technical
  conditions add signature and timestamp requirements that this contract
  does not encode, because that page could not be re-read with verified
  TLS from the development server.
- `xmllint` 2.9.14 loads the dossier closure offline, including the two
  resources that carry a DOCTYPE with an internal entity, and the Python
  `xmlschema` 4.3.2 package (loaded with `allow='local'` and `base_url`
  set to the mirror directory) agrees with it on a synthetic 2026 dossier
  built by rewriting an openSzigno-created container (namespace,
  `Object0`, procedure title, form-role title): conforming; and on the
  same dossier with a procedure title outside the enumeration: rejected
  on the enumeration facet.
- libxml2 without `--huge` refuses a single text node above 10 000 000
  bytes (`xmlSAX2Characters: huge text node`, exit 1), so a dossier whose
  base64 payload exceeds that cannot be validated through the package 4
  adapter, which forbids `--huge`.
- The sibling openSzigno 0.9.0 inspects that dossier only with
  `--allow-namespace` for the 2026 namespace (its allow-list stops at the
  2014 profile); its `inspect --json` then reports the 2026 namespace and
  `capabilities.structural_validation: true`. Its `create` writes the
  default Microsec namespace and fixed categories, and its signer writes
  `Id="documents"`, confirming the integration gap the research recorded.
  openSzigno keeps ownership of payload extraction beyond plain base64, of
  encrypted and zipped transforms, and of all cryptographic verification.

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
(the 2014 namespace is a test case) the status is `unsupported_version`.
Any other namespace, including the default Microsec namespace
`https://www.microsec.hu/ds/e-szigno30#`, is `unrecognized_root`: a
generic e-dossier is not a company filing. Structural lookups use the
dossier's own namespace and take the first `DossierProfile` and first
`Documents` child, leaving extras to `xsd_conformance`; `ds:Signature`
and `ds:Object` use the XMLDSig namespace. `ds:Object` is associated with
its `Document` by containment only; `OBJREF`, `Id`, `SIGREF`, and
`SIGREFLIST` are never resolved.

Fixed messages: `unsupported_version` → `the dossier namespace version is
not supported`; `unrecognized_root` → `the root element is not a company
dossier`; `input_too_large` → `the input exceeds the 64 MiB limit` (the
message carries the active profile's limit); `preflight_failed` → `at
least one preflight finding failed`;
`payload_exceeds_validator_limit` → `a text node exceeds the validator's
10 000 000-byte limit`.

### Role and title matching

The six procedure titles and the four dossier categories are compared by
exact string equality. The 194 document-role patterns are compiled once,
at build or first use, as anchored regular expressions with XSD
semantics: the only constructs present are literal text, `( … )` groups,
`.`, `.*`, `?`, and `\)`, and a unit test asserts that no other
metacharacter occurs, that every pattern compiles, and that a literal
title derived from each pattern (groups kept, `.*` removed) matches. The
`regex` crate (MIT OR Apache-2.0) is used with a size limit and full
anchoring; a role is matched against the whole title. `role` reports the
pattern's fixed text: for the 192 patterns without `.*` that is the
pattern with its group parentheses removed and escaped parentheses kept
as literal `)`, and for the two `…( - .*)?` patterns it is the prefix
before ` - `. The `regex` size limit is 1 MiB of compiled program per
pattern. The title itself is
never `role`. Reporting a matched enumeration or pattern value reveals
only that the document contains that public value, which is accepted.

The pattern texts are reproduced in the core as data: an excerpt of the
role names the authority requires documents to carry literally, needed
for interoperability. The schema itself is still not vendored, and the
specification inventory's redistribution note is extended to say so.

## `dossier inspect`

After recognition the inspector reads, never resolves, never decodes
beyond what is stated:

- `dossier`: `title` and `category` are reported only when they are one
  of the enumerated values above, otherwise `null` with
  `title_known: false` or `category_known: false`; `creation_date` is
  reported when it matches `xs:dateTime` lexically, otherwise `null`;
  `ack_email_present` and `location_present` booleans; the
  company-identity block as presence booleans `cegforma`,
  `cegjegyzekszam`, `cegnev`, `cegszekhely`; `documents_id_fixed` true
  when `Documents/@Id` is `Object0`; `receipt` true when the category is
  `electronic acknowledgement` or `elektronikus átvételi elismervény`.
- `documents[]`, in document order, at most 256 entries then
  `documents_truncated`: `index`, `role` (the matched pattern's fixed
  text, else `null` with `role_known: false`), `category` (one of the six
  document values or `null` with `category_known: false`; informative
  only, since the XSD does not enforce it), `mime` (`type`, `subtype`,
  `extension` as declared, each cut at 64 scalars, and `charset_present`
  for the `charSet` attribute; these declared tokens are printed without
  `--show-values` as a deliberate exception), `source_size` (integer or
  null), `transforms` (the `Algorithm` values in order; a missing or
  unknown value is the entry `unknown`), `object_present`,
  `nested_dossier` (true when the declared subtype is `nldossier2`
  case-insensitively or the extension is `dosszie`), `signatures` and
  `timestamps` counts placed inside the document, `payload`, and `form`.
- `payload` and `form`. `payload` is decided in this order and the first
  applicable reason wins: `object_missing` (no `ds:Object` inside the
  `Document`); `object_not_text` (the object has element children);
  `transform_not_plain_base64` (the chain is not exactly `base64`);
  `not_xml` (neither `text/xml` nor `application/xml` is declared and the
  extension is not `xml`, case-insensitively); `payload_too_large` (after
  removing XML whitespace in one pass, the base64 text is longer than
  16 MiB × 4 / 3 + 4 characters, checked before any decoding);
  `base64_invalid` (strict alphabet, padding required, decode failure);
  otherwise `inspected`. When `inspected`, `form` is the `form inspect`
  result over the decoded bytes: on a recognised form
  `{ status: "recognized", profile, variant, sections }`; on exit-4
  outcomes `{ status, profile, variant: null, sections: null }` with the
  inspect status; on exit-1 outcomes `{ status: "rejected", code }` with
  the inspect error code. Otherwise `form` is `null`. Payloads are never
  written to disk and never printed.
- `signatures_present` and `timestamps_present` count the elements at
  dossier level and inside documents; nothing is verified.
- Always shown: enumerated titles and categories, a lexically valid
  `creation_date`, matched roles, declared MIME tokens, counts, and
  presence flags. With `--show-values` each document's declared title and
  the four company-identity values are added verbatim (cut at 256
  scalars) as `title` on the document and `identity_values` on the
  dossier (an object with the keys `cegforma`, `cegjegyzekszam`, `cegnev`,
  `cegszekhely`, each a string or null), and `values_redacted` becomes
  false.

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
               "receipt": false,
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
`opencegeljaras: dossier inspect: <code> (exit N)`.

Human output lines, in order: the `input` and `root` lines as
`form inspect` prints them, `status:`, `profile:`, `title:`,
`category:`, `receipt:`, `creation date:`, `ack email:`, `location:`,
`identity:` (the four names with `present` or `absent`),
`documents id fixed:`, `documents: N` (the count followed by the word
`truncated` in parentheses when cut),
then per document the indented block `[N] role: ROLE|none`,
`category: …|none`, `mime: TYPE/SUBTYPE EXT`, `transforms: a,b|none`,
`payload: REASON`, `form: STATUS VARIANT|none`,
`signatures: N timestamps: N`, and with `--show-values` `title: …`;
then `signatures present:`, `timestamps present:`, and the four
boundary lines. Document-derived strings are escaped JSON literals.

### Cost

The 64 MiB input bounds everything: the raw input is one buffer (64
MiB), the decoded text is one copy (64 MiB), the whitespace-stripped
base64 of one document is at most one more
copy, the decoded payload is at most 16 MiB, and `form inspect` over it
adds its own 16 MiB decode copy, so the worst case stays near 225 MiB and
the total decoded payload across documents is under 48 MiB. Documents
are processed one at a time and their buffers dropped before the next.

## `dossier preflight`

Preflight runs `dossier inspect` and then evaluates findings. Each finding
is `{ code, status, source, detail }` where `status` is `passed`,
`failed`, `not_performed`, or `not_applicable`, `source` names the
evidence (`xsd-2026` for the dossier schema, `form-2026` for the form
contract, `openCegeljaras` for a rule this project derives), and `detail`
is a fixed string per code and status, never a document value.

| Code | Rule | Source |
| :--- | :--- | :--- |
| `procedure_title` | `passed` when `title_known`, else `failed` | `xsd-2026` |
| `dossier_category` | `passed` when the category is `electronic dossier` or `elektronikus akta`; `not_applicable` for a receipt; `failed` otherwise (the XSD allows all four; the filing rule is this project's) | `openCegeljaras` |
| `documents_id` | `passed` when `documents_id_fixed`, else `failed` | `xsd-2026` |
| `form_document_present` | `passed` when exactly one document has the role `A kérelem formanyomtatványa`; `failed` for zero or several (the XSD allows zero documents) | `openCegeljaras` |
| `form_recognized` | `passed` when that document's `form.status` is `recognized`; `failed` when it is any other inspect status or `rejected`; `not_performed` when `payload` is not `inspected` or `form_document_present` failed | `form-2026` |
| `form_matches_procedure` | `passed` when the variant matches the title: `Névfoglalási kérelem` needs `nevfoglalasi-kerelem-20210401`; `Bejegyzési kérelem` and `Ismételt bejegyzési kérelem` need `cegbejegyzesi-kerelem-20260101`; `Változás bejegyzési kérelem` needs `valtozasbejegyzesi-kerelem-20240101`; `failed` on a mismatch; `not_applicable` for `Egyéb beadvány` and `Hiánypótlás`; `not_performed` when `form_recognized` did not pass | `openCegeljaras` |
| `document_roles_known` | `passed` when every listed document has `role_known`, else `failed`; `not_performed` when `documents_truncated` | `xsd-2026` |
| `transforms_supported` | `passed` when every chain is one of `base64`, `zip,base64`, `encrypt,base64`, `zip,encrypt,base64`, else `failed` (the XSD permits any sequence; the chain rule is the e-dossier specification as openSzigno implements it) | `openCegeljaras` |
| `signature_present` | `passed` when `signatures_present >= 1`, else `failed`; presence only, validity is never assessed | `openCegeljaras` |
| `timestamp_present` | `passed` when `timestamps_present >= 1`, else `failed`; presence only | `openCegeljaras` |
| `xsd_conformance` | `passed`/`failed` from the validator; `not_performed` without `--bundle` or on any exit-5 condition, including the new exit-5 code `payload_exceeds_validator_limit` when any text node exceeds 10 000 000 bytes, in which case the validator is not spawned and the code is both `error.code` and the finding's `detail` | `xsd-2026` |

For a receipt (`dossier.receipt` true) every finding except
`xsd_conformance` is `not_applicable`; `preflight.status` is `failed`
with exit 4 when `xsd_conformance` failed, otherwise `not_applicable`
with exit 0 (or 5 for the unperformed XSD cases when `--bundle` was
given), and the notice gains "; receipt recognised, not acknowledged".
`detail` is a fixed string per code and status, listed in
the architecture document by the implementation, never a document value.

With `--bundle`, preflight verifies the bundle with package 3 rules,
requires `dossier_closure_verified` (every resource reachable from
`roots[1]` digest-verified; the dossier closure's DOCTYPE resources are
digest-only by contract; a manifest without a second root is
`manifest_invalid`), and, unless a text node exceeds the validator limit,
runs the package 4 xmllint adapter with `ROOT` = the bundle path of
`roots[1]` and the dossier bytes as `document.xml`; that temporary write
is therefore up to 64 MiB for dossiers, and the package 4 sentence is
amended accordingly. The adapter's collection and redaction functions are
exposed within the crate and reused unchanged. Validator findings are
reported under `xsd_findings` with `xsd_findings_truncated`,
`unparsed_lines`, `validator`, and `bundle` exactly as `form validate`
defines them, null when the validator did not run. `bundle_unverified`,
`validator_unavailable`, and `validator_failed` keep their exit-5 meaning
and leave `xsd_conformance` `not_performed`. Package 3 gains
`dossier_closure_verified` in its verify output, computed like
`form_closure_verified` over `roots[1]`, and the manifest unit test
asserts the dossier closure has 10 resources, disjoint from the form
closure, including both DOCTYPE files.

Verdict: `preflight.status` is `passed` when every finding is `passed` or
`not_applicable` and `xsd_conformance` was performed; `failed` when any
finding failed; `incomplete` when nothing failed but some finding is
`not_performed`; `not_applicable` for a receipt. There is no "ready to
file" wording anywhere: the notice reads "deterministic preflight
findings over one dossier; not the technical conditions, not signature
validity, not acceptance". A `passed` status means those findings passed
and the outer XML conforms, and nothing more.

Exit codes: 0 for `passed`, `incomplete`, and `not_applicable`; 4 when a
finding failed (`error.code = "preflight_failed"`) or the dossier is not
recognised (inspect codes); 5 for the unperformed XSD cases as in
`form validate`; 1, 2, 3 as usual. `ok` is true only on exit 0. `data`
carries the full inspect data plus `findings`, `preflight`, and the
validator keys; on exit 4 and 5 all of these are present with the
nullability of `form validate`. Human output: the inspect report, then
`findings:` with one indented `CODE: STATUS (SOURCE)` line each,
`xsd findings:` when present in the `form validate` line shape,
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
| `crates/opencegeljaras-cli/src/dossier/` (new), `main.rs` wiring, `schemas.rs` for `dossier_closure_verified`, `validate.rs`/`xmllint.rs` reuse | CLI |
| `crates/opencegeljaras-cli/tests/dossier.rs` (new) | Contract tests |
| `tests/fixtures/dossiers/` (new) | Synthetic fixtures |
| `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `SKILL.md` | Advertisement, last |

One implementation worker. The `regex` crate is the only new
dependency; `deny.toml` needs no change if its licences are already
allowed, which the worker verifies.

## Fixtures and tests

Synthetic dossiers under `tests/fixtures/dossiers/`, CC0, built by hand
from the schema facts above (the first two are the evidence dossiers from
2026-09-10, whose payload is the synthetic conforming name-reservation
form):

| Fixture | Purpose |
| :--- | :--- |
| `nevfoglalas-2026.es3` | Conforming 2026 dossier, one form document, no signature; both validators accepted it; preflight exits 4 on `signature_present` and `timestamp_present` by construction |
| `nevfoglalas-2026-bad-procedure-title.es3` | Same with a title outside the enumeration; both rejected it |
| `nevfoglalas-2026-signed-shape.es3` | Same with a synthetic, unverifiable `ds:Signature` and `TimeStamp` element, for the exit-0 path |
| `nested-dossier.es3` | A document declaring `application/nldossier2` |
| `encrypted-form.es3` | Form document with `encrypt,base64` and a document with an unknown algorithm |
| `iso-8859-2.es3` | The conforming dossier encoded in ISO-8859-2 |
| `suffix-role.es3` | A document titled with one of the two suffix patterns plus a sentinel suffix, which must never be printed |
| `microsec-default.es3` | Default-namespace container → `unrecognized_root` |
| `dossier-2023.es3` | 2023 namespace → `unsupported_version` |
| `mismatched-variant.es3` | `Bejegyzési kérelem` carrying the name-reservation form |
| `two-forms.es3` | Two documents with the form role |
| `zip-form.es3` | Form document with `zip,base64` → `transform_not_plain_base64` |
| `unknown-role.es3` | A document whose title matches no role pattern |
| `receipt.es3` | `electronic acknowledgement` category, findings `not_applicable` |
| `bad-base64.es3` | Invalid base64 in the form object |

Tests cover: recognition statuses including the 2014 namespace; every
`payload` reason in precedence order; the `form` object equality with
`form inspect` on the extracted synthetic form; `--show-values` gating
with a sentinel title and the suffix-role sentinel; the pattern
constraints and the literal-title match for all 194 roles; each finding
code in each reachable status; the receipt rule; `documents_truncated`
and payload caps with generated dossiers; the 10 000 000-byte validator
ceiling reason; a text node accepted under the dossier profile that the
form profile rejects; determinism; human shapes; exit paths; with the
fake validator, the preflight XSD path. The mini graph's dossier-style
resource carries a DOCTYPE and is not a usable schema, so no Linux
real-xmllint test exists for preflight; one `#[ignore]` test validates
the evidence dossiers against `OPENCEGELJARAS_SCHEMA_BUNDLE`. Also: that
`capabilities` lists both operations and that nothing under `samples/`
is referenced.

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
- **Role patterns as data.** The 194 pattern texts and the enumerated
  titles and categories are reproduced in the core as interoperability
  data with a comment naming their source; the schema itself is still not
  vendored, and the specification inventory's redistribution note is
  extended in this package to record that excerpt.

## Unresolved questions

- The technical conditions (signature coverage, timestamps, nested
  dossier rules, attachment formats) were not re-read; presence findings
  are the honest maximum until then.
- Whether the authority's own tools set `E-category` to the English or
  the Hungarian value; both are accepted.
- Receipt dossiers deserve their own contract (milestone 4); this one only
  recognises them.
