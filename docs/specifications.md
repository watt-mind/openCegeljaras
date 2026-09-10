# Specification inventory

Reviewed on **2026-09-10**. This inventory is a research snapshot, not an
implementation support list or a live legal-status service.

## Evidence quality

Several government HTML pages failed direct retrieval; their indexed official
text was available. The form/dossier XSDs and dependency graph were downloaded
live over validated HTTPS. The Hivatali Kapu PDF, latest linked NJT law, and
Microsec SDK reference were directly opened. Search crawl dates and download
filename dates are not publication or legal effective dates.

The machine-readable [source catalog](../data/source-catalog.json) distinguishes
indexed guidance, downloaded schemas, direct documentation, and references not
retrieved. `opencegeljaras sources` prints that catalog without network access.
No government operation was invoked with case data, and no filing was sent.

## Form, dossier, and articles are separate schemas

| Layer | Observed specification | Version evidence |
| :--- | :--- | :--- |
| Application form | [cegnyomtatvany-20260101.xsd](https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd) | Accepted 2026-02-28; exclusive from 2026-05-01 per developer guidance |
| Dossier | [e-cegeljaras2026.xsd](https://www.e-cegjegyzek.hu/schema/2026/e-cegeljaras2026.xsd) | Technical page also lists [2023](https://www.e-cegjegyzek.hu/schema/2023/e-cegeljaras2023.xsd); no 2023 sunset established |
| Articles template | [szerzodes_minta-20250301.xsd](https://schema.e-cegjegyzek.hu/schema/szerzodes_minta/szerzodes_minta-20250301.xsd) | Separate incorporation-document format; full dependency closure not inventoried |

The form dates come from [developer guidance](https://ceginformaciosszolgalat.kormany.hu/akadalymentes/fejlesztoknek/jogi-hatter),
not the `20260101` filename. The dossier alternatives come from
[technical conditions](https://ceginformaciosszolgalat.kormany.hu/technikai-feltetelek).

The application wrapper has root `Nyomtatvány` and namespace
`http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#`.
Its three direct imports are `cegbejegyzesiKerelem-20260101.xsd`,
`nevfoglalasiKerelem-20210401.xsd`, and
`valtozasbejegyzesiKerelem-20240101.xsd`. The complete retrieved closure has
**139 resources including the wrapper**. Preserve the mixed-version graph.
[Application wrapper](https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd)

The 2026 dossier uses namespace `http://www.e-cegjegyzek.hu/2026/e-cegeljaras#`
and declares ISO-8859-2. It imports XMLDSig, XAdES, and metadata schemas; its
closure has **10 resources including the root**. Its XSD permits absent
signatures and zero documents, illustrating why schema conformance alone is
weaker than the filing conditions.
[Dossier schema](https://www.e-cegjegyzek.hu/schema/2026/e-cegeljaras2026.xsd)

The combined [schema metadata](../data/schema-manifest.json) records 149 unique
resource URLs with hashes, sizes, namespaces, global elements, and dependency
URLs. These are acquisition URLs: literal HTTP imports were normalized to
the same-host HTTPS equivalent during controlled retrieval. Namespace values
are unchanged. This is not a lossless transcript of import declarations, an
XSD bundle, an implemented resolver, or an authority-certified catalog.

Future acquisition must audit the full dependency set, preserve original
bytes/encoding, and resolve only allowlisted local resources. Never allow
arbitrary document-provided schema URLs or stylesheet fetches.

## Other actionable primary references

| Source | Use and limitation |
| :--- | :--- |
| [Authority checks](https://ceginformaciosszolgalat.kormany.hu/elvegzett-ellenorzesek) | Separate deterministic checks from external record comparisons |
| [Filing instructions](https://ceginformaciosszolgalat.kormany.hu/kerelem-megkuldese) | Actual attachment/dispatch route, not an arbitrary KRX upload |
| [Autumn-2025 mail notice](https://ceginformaciosszolgalat.kormany.hu/cegbirosagi-kerelmek-benyujtasa) | Mail infrastructure and whole-message requirements |
| [Service response](https://ceginformaciosszolgalat.kormany.hu/ceginformacios-szolgalat-visszajelzese) | Technical rejection, forwarding, and receipt evidence |
| [Ruling collection](https://ceginformaciosszolgalat.kormany.hu/cegbirosag-visszajelzese) | Signed service acknowledgment; older UI details need confirmation |
| [Official articles portal](https://cegportal.im.gov.hu/frontend/cegeljaras/letesitoOkirat) | XML-to-endorsed-PDF workflow, distinct from filing |
| [Microsec SDK](https://download.e-szigno.hu/eszigno/docs/eszigno3_ref.html) | Existing vendor CLI; no licensing/platform assumptions |
| [NETLOCK MOKKA guide](https://netlock.hu/app/uploads/2024/12/MOKKA-%C3%BAtmutat%C3%B3_2.9.pdf) | Historical workflow only: contains 2012 profile and ActiveX references despite URL date |

The developer guidance documents `POST /api/creationDocument` with
`Content-Type: application/xml`; HTTP 200 returns a PDF. This endpoint was not
called. Authentication, error schema, service limits, and operational support
remain to be established before an adapter is implemented.
[Developer guidance](https://ceginformaciosszolgalat.kormany.hu/akadalymentes/fejlesztoknek/jogi-hatter)

## Hivatali Kapu and adjacent procedures

The [official HKP landing page](https://ceginformaciosszolgalat.kormany.hu/cegbirosagi-elektronikus-kommunikacio-hivatali-kapun-keresztul)
links a [channel protocol PDF](https://ceginformaciosszolgalat.kormany.hu/akadalymentes/download/c/2b/63000/C%C3%A9gb%C3%ADr%C3%B3s%C3%A1gi%20elektronikus%20kommunik%C3%A1ci%C3%B3%20hivatali%20kapun%20kereszt%C3%BCl_20250606.pdf).
Its cover is version 1.3 effective 2025-03-01; its filename says 20250606.
It specifies a raw e-akta rather than KR/KRX wrapping, routing metadata,
test/production destinations, and digitally signed technical-result receipts.
It also mixes old dossier references with a mislabeled form-schema reference.
Treat it as channel evidence requiring reconciliation, not the 2026 schema
authority or a universal interface for attorneys. No test destination was used.

[Legality-supervision developer guidance](https://ceginformaciosszolgalat.kormany.hu/fejlesztoknek2)
publishes `cgturlap20140701.xsd`. This is a distinct procedure, not an older
version of the ordinary registration form. Asset-survey and authority
automatic-entry protocols likewise need separate scopes.

## Redistribution and missing evidence

No explicit open-source redistribution license was established for the
Hungarian XSD/XSLT/Java artifacts. The authority's
[imprint](https://ceginformaciosszolgalat.kormany.hu/impresszum) reserves rights.
A license notice in one imported standard does not license the complete
graph. The repository therefore includes original code and source metadata,
not third-party schemas. This is an unresolved distribution question, not a
conclusion that interoperable implementation is prohibited.

The following remain unestablished: redistribution terms; a reusable official
machine-readable conformance/receipt fixture suite; complete receipt profiles;
authorized live interoperability testing; a general public filing REST API;
and replacement-regime schemas/interfaces. The [research](research.md) cites
current reform legislation and explains the resulting investment boundary.
