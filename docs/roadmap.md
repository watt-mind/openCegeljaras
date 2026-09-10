# Roadmap

Only milestone 0 is implemented. All command names below are proposals.

## 0. Research and scaffold

- Two unpublished Rust crates; human output and versioned JSON introspection.
- Official-source inventory, dated evidence, architecture, fixture policy.
- CI, security scans, local checks, and explicit non-capabilities.

## 1. Pinned schema discovery and read-only form inspection

Start with company-registration/change forms and a limited, named set of
supported variants. Acquire the complete 2026 form import graph, document
redistribution rights, pin bytes/digests, and verify it in an independent XSD
implementation. Do not implement only the outer wrapper and call it complete.

Proposed commands:

```sh
opencegeljaras schemas list --json
opencegeljaras form inspect synthetic-form.xml --json
```

Gate: independent synthetic valid/invalid fixtures, namespace/encoding tests,
resource-limit tests, deterministic output, unknown-version handling, and
reviewed redistribution terms. Current `sources` is not this schema registry.

## 2. Form validation and dossier preflight

Proposed commands:

```sh
opencegeljaras form validate synthetic-form.xml --profile PROFILE --json
opencegeljaras dossier preflight synthetic.es3 --profile PROFILE --json
```

Implement pinned XSD conformance before claiming it. Layer deterministic
cross-field checks over the schema with source-linked rule IDs. Check dossier
structure, form cardinality, document formats, nested-dossier restrictions,
and relevant attachment roles through reviewed container adapters. A local
check can state whether evidence is present; cryptographic conclusions require
a real verifier and explicit trust material.

Gate: both schema validity and semantic counterexamples; unsupported remote
checks listed explicitly; no blanket "ready to file" verdict. Publish a
coverage matrix for rules implemented versus required/unknown.

## 3. Deterministic preparation

Proposed commands:

```sh
opencegeljaras form create --spec synthetic-case.json --profile PROFILE
opencegeljaras dossier plan --spec synthetic-case.json --json
```

Begin with one procedure/form variant and reviewed human-readable output.
Record attachment roles and missing evidence; defer legal document drafting
to the attorney. Compose with a container writer only once it supports the
exact cégeljárás metadata/form profile. Signing and verification are explicit
separate operations. No signing key handling is needed in this project's MVP.
The inspected openSzigno author currently writes a fixed default namespace
and categories; this integration requires a reviewed extension, not merely
calling its existing `create` command with a different filename.

Gate: schema-valid deterministic output, no-clobber writes, round trips,
reviewable field diffs, independent vendor interoperability, and synthetic
test-service acceptance where an authorized service exists. Do not test against
a production filing channel by sending a fabricated case.

## 4. Receipt interpretation and evidence association

Proposed commands:

```sh
opencegeljaras receipt inspect synthetic-receipt.es3 --json
opencegeljaras receipt correlate --manifest synthetic-submission.json --json
```

Prioritize this alongside authoring if attorney interviews show greater value
in rejection triage and receipt tracking. Distinguish authority claims from
verified evidence and technical forwarding from court acceptance. Preserve
unmatched, duplicate, and contradictory receipts. Read-only interpretation
must never acknowledge service or open active receipt links.

Gate: documented receipt profile, independent synthetic fixtures, verification
provenance, robust correlation, and unknown-receipt handling. No inferred legal
deadline automation in the initial implementation.

## 5. Explicit transport adapters, only after evidence

Consider an export for human submission before any sender. A future sender
needs a documented supported interface, authentication/access terms, testing
arrangements, duplicate prevention, ambiguous-send recovery, reviewed channel
selection, and an explicit distinction between dispatch and official receipt.
Portal login, payment, and service acknowledgment remain separate actions.

Neither browser automation nor an ordinary mail-send success establishes
filing success. Do not treat the special Hivatali Kapu protocol for authorities
as a universal attorney interface. Reassess against the newer registration
regime before investing in transport-specific code.
