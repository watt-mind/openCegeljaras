# Should openCegeljaras exist?

**Recommendation: yes, as a focused company-filing preparation and evidence
tool.** Invest first in forms, reproducible preflight, and receipt
interpretation. Rust fits the existing tool family and a portable local CLI.
A large submission-automation implementation should wait for a supported
interface and clarity on the forthcoming registration regime.

This is an engineering and product recommendation, not a finding of legal
compliance or court approval. Evidence was reviewed on **2026-09-10**. The
[specification inventory](specifications.md) records important access and
version caveats.

## The concrete opportunity

A company filing joins structured form data, legal documents, attachment
roles, signatures, payment evidence, and authority responses. The authority
already permits alternative form-producing software that meets its rules.
That makes an interoperable CLI a plausible product, rather than requiring a
replacement for the entire government system.
[Official developer guidance](https://ceginformaciosszolgalat.kormany.hu/akadalymentes/fejlesztoknek/jogi-hatter)

An attorney's agent could take reviewed matter data, check consistency, identify
missing evidence, prepare a versioned form, and explain a rejection in stable
JSON. The attorney would review substantive choices and authorize consequential
actions. These benefits are a hypothesis based on the workflow and the
operator's reported friction; no attorney interviews or measured time savings
were collected for this scaffold.

| Candidate | Expected value | Recommendation |
| :--- | :--- | :--- |
| Form inspection and field diffs | Makes existing XML usable to agents; reduces repeated reading | First implementation |
| Pinned schema and deterministic field checks | Catches specific defects before submission | First implementation, with explicit coverage |
| Dossier preflight | Explains missing/wrong form and document roles | First integration with container tooling |
| Receipt interpretation/correlation | Separates dispatch, rejection, forwarding, and decisions | Early read-only milestone |
| Selected form creation | Reduces duplicate data entry | After schema/round-trip conformance |
| Endorsed incorporation PDF request | Uses a documented government integration seam | Later explicit online adapter |
| Unattended portal submission and service acceptance | Potential convenience, substantial protocol and state risk | Defer |
| General registry scraping | Different product and access problem | Outside initial scope |

## The specifications are real, and there are several layers

The form root is `cegnyomtatvany-20260101.xsd`. Live retrieval found a
139-resource XSD dependency closure, including imported form variants with
older dates. The dossier has a separate `e-cegeljaras2026.xsd` root and a
10-resource closure. A root URL or namespace match is therefore insufficient
to implement validation. The metadata-only [snapshot](../data/schema-manifest.json)
records the 149 resources, byte sizes, hashes, namespaces, and dependency URLs.
[Form XSD](https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101.xsd),
[dossier XSD](https://www.e-cegjegyzek.hu/schema/2026/e-cegeljaras2026.xsd)

The technical conditions add requirements beyond the XSD: document/signature
coverage, timestamped signatures, form cardinality, nested dossiers, and
attachment formats. The page lists both the 2023 and 2026 dossier schemas.
Do not transfer the form's acceptance dates to the dossier schema.
[Technical conditions](https://ceginformaciosszolgalat.kormany.hu/technikai-feltetelek)

The authority also checks identifiers, addresses, activities, dates, tax
declarations, and official records. Some rules are reproducible locally;
identity/registry comparisons need evidence the CLI may not possess. The
product should report each check as passed, failed, unperformed, or
inapplicable, with its source and version. A schema-valid form is not an
accepted filing.
[Published authority checks](https://ceginformaciosszolgalat.kormany.hu/elvegzett-ellenorzesek)

## How it fits the existing tools

openSzigno owns ES3 bytes and cryptographic operations. openCegeljaras should
own company-procedure semantics and compose through a reviewed interface.
An ES3 writer still needs an interoperability check for the required domain
namespace, metadata, form placement, and attachment roles. Existing support
for a generic container is not sufficient evidence of a valid filing profile.

There is a concrete integration gap in the inspected openSzigno revision:
`create` writes the default e-Szignó namespace and fixed `electronic dossier`
and `electronic data` categories. `DossierSpec` exposes no procedure profile.
It cannot simply be assumed to emit the cégeljárás-specific outer format.
Plan a reviewed authoring extension or adapter, with conformance tests, before
advertising end-to-end preparation.
[Inspected author API](https://github.com/watt-mind/openSzigno/blob/9f8ccccca419360b106de6bae693f9f192a8f6d3/crates/openszigno-author/src/lib.rs),
[renderer](https://github.com/watt-mind/openSzigno/blob/9f8ccccca419360b106de6bae693f9f192a8f6d3/crates/openszigno-author/src/render.rs)

openKRX remains useful wherever a separately documented channel actually
requires KRX. The official filing instructions offer a dedicated portal and
court-specific email route; this research did not establish ordinary
e-Papír/KRX as the registration submission protocol.
[Submission instructions](https://ceginformaciosszolgalat.kormany.hu/kerelem-megkuldese),
[court email routes](https://ceginformaciosszolgalat.kormany.hu/e-mail-cimek)

openPapir can provide an adjacent correspondence/evidence workflow. Keep
receipt association and case events portable rather than building another
archive implicitly into the first two crates. ASiC is a separate container
integration with its own implementation status.

This is not the first way to automate e-aktas: Microsec publishes an SDK CLI
with creation, signing, verification, and receipt operations. The proposed
differentiation is open, inspectable company-procedure rules and agent
contracts, with low-friction local deployment. Vendor components are not
bundled, and their licenses/platform coverage need separate review if used.
[Microsec SDK reference](https://download.e-szigno.hu/eszigno/docs/eszigno3_ref.html)

## Submission and receipts deserve separate state models

The autumn-2025 email notice requires SPF/DMARC, suitable bidirectional TLS,
a CA-issued mail-server certificate, and a complete message smaller than
100 MB. Checking dossier bytes alone cannot establish the mail requirement:
MIME encoding and headers add size, while delivery configuration is external.
An export for manual review could be useful before implementing a sender.
[Submission notice](https://ceginformaciosszolgalat.kormany.hu/cegbirosagi-kerelmek-benyujtasa)

The Service's workflow distinguishes technical rejection from forwarding to
the court, with signed receipt evidence. A technically faulty application can
be treated as unsubmitted; malformed/no-dossier email may receive no response.
Model an uncertain dispatch explicitly instead of treating a successful SMTP
operation as acceptance.
[Service response](https://ceginformaciosszolgalat.kormany.hu/ceginformacios-szolgalat-visszajelzese)

The published ruling-collection workflow includes signed acknowledgment of
service. Receipt inspection must preserve evidence without triggering that
action. The site's older UI instructions need current protocol verification;
this scaffold does not implement legal deadline calculation.
[Court response and collection](https://ceginformaciosszolgalat.kormany.hu/cegbirosag-visszajelzese)

There is a documented HTTP operation worth retaining in the design:
`POST https://cegportal.im.gov.hu/api/creationDocument` accepts XML and returns
a PDF on success. It concerns endorsed incorporation documents, not submission.
No POST was performed, and authentication/error/rate-limit contracts remain
unverified. Offline draft generation cannot manufacture the authority's
endorsement.
[Official developer guidance](https://ceginformaciosszolgalat.kormany.hu/akadalymentes/fejlesztoknek/jogi-hatter)

## The 2027 change limits the sensible investment

The latest opened consolidated **2025 LIX Act, section 105**, states a
**2027-01-01** commencement; section 104 provides for detailed system/form
rules. That is a statutory date, not evidence of an available replacement API.
Recheck amendments, implementation, and transition rules before a release.
No replacement schema/transport contract was established in this research.
[NJT consolidated Act](https://njt.jog.gov.hu/jogszabaly/2025-59-00-00.2)

The older 2021 XCII reform is not a reliable implementation target: the
2025 LXVII Act, section 262, prevents it entering into force.
[NJT 2025 LXVII](https://njt.hu/jogszabaly/2025-67-00-00.0)
The government's separate registry project has a stated planned completion
of 2027-12-31; that project milestone does not resolve operational cutover.
[Government project page](https://2020-2025.kormany.hu/igazsagugyi-miniszterium/dimop-plusz-136-23-2023-00001-szamu-jogi-szemelyek-egyseges-nyilvantartasa-cimu-projekt)

The design implication is to preserve reviewed company data independently
from the current XML and transport. Use replaceable, dated procedure profiles.
Form inspection and receipt evidence can remain valuable across a transition;
an elaborate implementation tied to today's portal UI may not.

## Why Rust, and where to be careful

Rust is a reasonable engineering choice because the sibling toolchain already
uses it, the intended interface is a portable CLI, and bounded local parsing
and deterministic output fit its strengths. Reuse the workspace, security,
and JSON conventions; start with core and CLI crates, without a new runtime
service or mandatory network access.

The difficult dependency choice is a sufficiently complete XML Schema
validator for the actual schema graph. Do not equate XML parsing with XSD
validation or promise an all-Rust validator without a conformance spike.
Evaluate candidate implementations against the downloaded graph and an
independent reference validator. If an adapter is necessary, make its runtime,
licensing, isolation, and packaging costs explicit before adopting it.

## Go/no-go criteria

Proceed with the scaffold and a bounded read-only milestone. Before claiming
functional support, establish schema distribution rights, a reproducible
resolver, independent synthetic conformance fixtures, and a supported-profile
matrix. Before authoring, demonstrate cross-tool interchange and reviewable
output. Before transport, establish a supported interface and recovery from
ambiguous submissions without duplicate sends.

Interview attorneys using synthetic examples to choose the first form and
measure preparation time, diagnosed preflight defects, and receipt-triage
accuracy. Prioritize based on that evidence. The [roadmap](roadmap.md) keeps
these gates separate and marks every proposed command as unimplemented.
