# Synthetic dossier fixtures

These containers are hand-constructed from the public element, attribute, and
namespace names of the 2026 dossier schema, as recorded in the
[dossier preflight contract](../../../docs/dossier-preflight.md). They contain
placeholder text only: no real company, person, case number, filing, or
signature appears in them, none is derived from a client dossier or from any
ignored private material, and no cryptographic value in them is genuine.

The base64 payload of every form document is
[`nevfoglalasi-conforming.xml`](../forms/nevfoglalasi-conforming.xml), the
synthetic name-reservation form of the form fixtures, byte for byte.

`nevfoglalas-2026.es3` and `nevfoglalas-2026-bad-procedure-title.es3` are the
evidence dossiers of **2026-09-10**: against a digest-verified mirror of the
pinned dossier closure, `xmllint` from libxml2 2.9.14 and the independent
Python `xmlschema` 4.3.2 agreed that the first conforms to the 2026 dossier
graph and that the second violates the procedure-title enumeration facet. That
is recorded evidence, not something the test suite re-establishes on every run,
and it is not the technical conditions, not a signature check, and not
acceptance by any authority. The other fixtures are **not** asserted to be
XSD-valid: they exercise recognition, payload reasons, findings, and rejection
paths. `nevfoglalas-2026-signed-shape.es3` in particular carries an empty
`ds:Signature` and `TimeStamp` for the presence findings alone; it does not
conform to the dossier schema and nothing in it is verifiable.

`suffix-role.es3` carries the placeholder suffix `SENTINEL-ROLE-4419` and
`unknown-role.es3` the placeholder title `SENTINEL-TITLE-8802`, which the tests
use to prove that a declared title reaches standard output only with
`--show-values` and standard error never.

Oversized payloads, over-long text nodes, and document lists past the 256-entry
cap are generated inside the tests rather than stored here.

Like the rest of `tests/fixtures`, they are released under [CC0](../LICENSE).

| Fixture | Purpose |
| :--- | :--- |
| `nevfoglalas-2026.es3` | Conforming 2026 dossier, one form document, no signature; preflight exits 4 on `signature_present` and `timestamp_present` by construction |
| `nevfoglalas-2026-bad-procedure-title.es3` | The same with a procedure title outside the enumeration |
| `nevfoglalas-2026-signed-shape.es3` | The same with a synthetic, unverifiable `ds:Signature` and `TimeStamp`, for the exit-0 path |
| `nested-dossier.es3` | A document declaring `application/nldossier2`, flagged and never opened |
| `encrypted-form.es3` | Form document with `encrypt,base64` and a document with an unknown algorithm |
| `iso-8859-2.es3` | The signed-shape dossier encoded in ISO-8859-2 |
| `suffix-role.es3` | A document titled with one of the two suffix patterns plus a sentinel suffix |
| `microsec-default.es3` | Default Microsec-namespace container, exit 4 `unrecognized_root` |
| `dossier-2023.es3` | 2023 dossier namespace, exit 4 `unsupported_version` |
| `mismatched-variant.es3` | `Bejegyzési kérelem` carrying the name-reservation form |
| `two-forms.es3` | Two documents with the application-form role |
| `zip-form.es3` | Form document with `zip,base64`, so `transform_not_plain_base64` |
| `unknown-role.es3` | A document whose title matches no role pattern |
| `receipt.es3` | `electronic acknowledgement` category; every filing finding is `not_applicable` |
| `bad-base64.es3` | Invalid base64 in the form object |
