# Form fields and diff contract

Reviewed on **2026-09-10**. This contract specifies two read-only commands
that make a recognised 2026 form usable to an agent without XML tooling:
`form fields`, which lists a form's leaf fields as stable paths, and
`form diff`, which compares two forms field by field. Both build on the
`form inspect` boundary and expose values only on explicit request.

Status: **contract, not implemented.** `capabilities` lists the operations
only after the behaviour and tests below exist.

## Problem and context

The research names "form inspection and field diffs" as the first thing
worth building: an agent holding a form should be able to read which
fields are present, compare a regenerated form with the previous one, and
explain a change in stable terms. `form inspect` stops at the variant and
its sections; `form validate` reports schema violations by line number.
Neither gives an addressable field.

No external evidence is needed. Paths are derived from the document's own
element names, which the official schemas fix; a path is therefore a
stable identifier for as long as the schema version is, and it changes
exactly when the schema does. Values are personal and company data and are
treated as such: hidden by default, shown only with `--show-values`, and
never written to stderr.

## Scope

```sh
opencegeljaras form fields PATH [--show-values] [--json]
opencegeljaras form diff OLD NEW [--show-values] [--json]
```

Out of scope: schema-driven field typing, required-field checks, value
normalisation, editing or writing forms, merging, comparing more than two
documents, and any output of `samples/`-style private material into tests.

## Field model

Applied to a document that passed the `form inspect` boundary with
`status: "recognized"`. Paths are relative to the variant root
(`CégbejegyzésiKérelem`, `VáltozásbejegyzésiKérelem`, or
`NévfoglalásiKérelem`), which is itself not a field.

A **path** is the sequence of element local names from the variant root's
child down to the field, joined with `/`. Whenever an element has one or
more same-named siblings under the same parent (counting element children
only), every one of them carries a 1-based document-order index
`[n]`; an element without same-named siblings carries none. Namespaces do
not appear in the path; two siblings with the same local name in different
namespaces are counted as same-named and both carry an index, and the
field records its own namespace separately. Paths are therefore
deterministic for a given document and identical across runs.

A **field** is one of:

- a **leaf element**: an element with no element children. Its value is
  the concatenation of its text and CDATA nodes with no trimming or
  normalisation; an empty element has the empty value.
- an **attribute**: any attribute on any element from the variant root
  down, except namespace declarations and attributes in the XSI namespace.
  Its path is the owner's path followed by `/@` and the attribute local
  name; the variant root's own attributes have paths beginning with `@`.
  Attribute order is the document's.
- a **mixed element**: an element with both element children and
  non-whitespace text. It is listed with `kind: "mixed"` and no value, its
  text is not exposed, and its element children are still walked. This
  case does not occur in schema-conforming forms and exists so that no
  text is silently dropped.

Whitespace-only text under an element that has element children is not a
field. Comments and processing instructions are ignored.

Fields are emitted in document order. At most 10 000 fields are listed,
then `fields_truncated: true`; the walk stops there. A value is cut at 4096
Unicode scalar values with `…` and `value_truncated: true`. Path components
and attribute names are element and attribute names, already bounded by the
`form inspect` limits, and each is cut at 128 scalars as inspect does; a
whole path is cut at 1024 scalars with `…` and `path_truncated: true`.

## `form fields`

Behaviour: run the complete `form inspect` boundary; return any exit 1 or
exit 4 outcome unchanged except `command: "form fields"`; then walk the
variant root as above.

JSON `data` on exit 0: the `form inspect` structure (`input`, `root`,
`form`, `structure`, `checks`, `notice`) plus:

```json
{
  "fields": [
    { "path": "Adatlap/TagoltCím/Hely", "kind": "element",
      "namespace": "http://…/tagoltCim-20210101#",
      "value": "Példaváros", "value_length": 10,
      "value_truncated": false, "path_truncated": false },
    { "path": "@Cégforma", "kind": "attribute", "namespace": null,
      "value_length": 3, "value_truncated": false, "path_truncated": false }
  ],
  "field_count": 2,
  "fields_truncated": false,
  "values_shown": false
}
```

`value` is present only with `--show-values`; without it the key is absent
and `value_length` (in Unicode scalar values, before any cut) still allows
an agent to see that a field is empty or filled. `kind` is `element`,
`attribute`, or `mixed`; a `mixed` entry has no `value`, `value_length`,
or `value_truncated`. `namespace` is the field's own namespace URI or
null. The `notice` reads "field listing of one recognised form; values
are shown only on request; not validation, verification, or acceptance".

Human output: the `form inspect` lines, then `fields: N` (followed by
the word `truncated` in parentheses when cut), then one indented line per
field:
`PATH = "VALUE"` with `--show-values`, otherwise `PATH (N chars)`;
attribute paths as given; mixed elements as `PATH (mixed content)`. Paths
and values are printed as escaped JSON string literals under the
`form inspect` rule; a path is one literal. Then the unchanged boundary
lines.

Exit codes: 0 listed; 1, 2, 3, 4 as `form inspect`; stderr line
`opencegeljaras: form fields: <code> (exit N)` whenever the exit code is
not 0.

## `form diff`

Behaviour: run `form fields` logic on `OLD`, then on `NEW`. If either is
not a recognised form, exit 4 with `error.code = "old_not_recognized"` or
`"new_not_recognized"` (the first failing side, `OLD` checked first), and
`data.old` / `data.new` holding that side's inspect data (the other side
null). If either is rejected at the input boundary, exit 1 with the
inspect code and `data.side: "old" | "new"` naming the side and the
inspect `data` of that side, and nothing about the other side. If either
side's field list was truncated, the diff is still computed over the listed
fields and `fields_truncated` is reported per side.

Both documents' fields are keyed by path. Keys present only in `OLD` are
`removed`; only in `NEW` are `added`; present in both with different
`kind`, `namespace`, or value are `changed`; present in both and equal are
counted in `unchanged`. Comparison is exact on the untruncated value
strings (both sides cut identically, so a change beyond the cut can be
missed only when both values share the first 4096 scalars; the
`value_truncated` flags on both sides make that visible). Each diff entry
is emitted in `NEW` document order for `added` and `changed`, and `OLD`
document order for `removed`.

JSON `data` on exit 0:

```json
{
  "old": { "input": {}, "root": {}, "form": {}, "structure": {},
           "field_count": 0, "fields_truncated": false },
  "new": { "…": "…" },
  "same_variant": true,
  "identical": false,
  "added": [ { "path": "…", "kind": "element", "namespace": "…",
               "value_length": 3 } ],
  "removed": [ { "path": "…", "kind": "…", "namespace": "…",
                 "value_length": 3 } ],
  "changed": [ { "path": "…", "kind": "…", "namespace": "…",
                 "old_value_length": 3, "new_value_length": 5 } ],
  "unchanged": 40,
  "values_shown": false,
  "checks": { "well_formed": "passed", "xsd_conformance": "not_performed",
              "signature": "not_performed",
              "authority_acceptance": "not_performed" },
  "notice": "field-level comparison of two recognised forms; not validation, verification, or acceptance"
}
```

With `--show-values`, `added` and `removed` entries carry `value`, and
`changed` entries carry `old_value` and `new_value`. `same_variant` is
false when the two variant ids differ; the diff is still produced by path.
`identical` is true when `added`, `removed`, and `changed` are all empty
and `same_variant` is true. Entries carry `path_truncated` and the
relevant `value_truncated` flags as in `form fields`. `checks.well_formed`
is `passed` because both sides parsed.

Exit 0 whenever both sides are recognised, identical or not; an agent
reads `identical`. The exit code does not encode difference, because
exit 1 and exit 4 already mean rejection and failed recognition in this
tool. Human output: `old:` and `new:` summary lines (variant id, field
count), `same variant:`, `identical:`, then `added N`, `removed N`,
`changed N` headers each followed by one indented line per entry (the
path literal alone, or with
`--show-values` the path, `=`, and the value literal for added and removed
entries, and the path, the old literal, `->`, and the new literal for
changed entries), `unchanged N`, then the boundary lines. The stderr
line is `opencegeljaras: form diff: <code> (exit N)`.

## Determinism and safety

Both commands are pure functions of their inputs: no clock, no network,
no environment. Values reach stdout only with `--show-values`, never
stderr, never a diagnostic. The skill tells agents to pass `--show-values`
only when the operator asked to see or compare values, and that the
output then contains personal and company data. A field listing or diff
says nothing about validity, verification, or acceptance, and the notice
and boundary lines say so on every run.

Field walks are bounded by the `form inspect` limits (elements, depth,
text, attributes) plus the 10 000-field cap, so cost is linear in the
document. Path construction for an element with many same-named siblings
must count siblings once per parent, not once per child, so a parent
with 200 000 children stays linear; a test asserts that a 100 000-sibling
document lists in well under a second.

## Ownership

| Path | Role |
| :--- | :--- |
| `crates/opencegeljaras-core/src/form/fields.rs` (new), `mod.rs` exports | Field walk and diff over the bounded tree |
| `crates/opencegeljaras-cli/src/fields.rs` (new), `main.rs` wiring | Two commands, envelopes, human output |
| `crates/opencegeljaras-cli/tests/form_fields.rs` (new) | Contract tests |
| `tests/fixtures/forms/` additions | Synthetic fixtures below |
| `README.md`, `docs/architecture.md`, `docs/testing.md`, `CHANGELOG.md`, `SKILL.md` | Advertisement, last |

The core walk takes the parsed tree and returns fields; the CLI owns
files, the second document, and output. One implementation worker.

## Fixtures and tests

Synthetic fixtures under `tests/fixtures/forms/`, hand-built, placeholder
data only, CC0, not asserted XSD-valid:

| Fixture | Purpose |
| :--- | :--- |
| `fields-repeated-siblings.xml` | Name reservation with three `Névfoglalás` entries, an attribute on the variant root, an empty leaf, a CDATA leaf, a mixed element, an XSI attribute, and a comment |
| `fields-repeated-siblings-edited.xml` | The same with one value changed, one entry removed, one attribute added |
| `fields-other-variant.xml` | A registration variant with the same first path |

Tests cover: exact field lists (paths, kinds, namespaces, lengths) with
and without `--show-values`; index assignment for repeated siblings and
none for singletons; attributes incl. root attributes and the XSI and
namespace-declaration exclusions; CDATA and empty values; the mixed rule;
value, path, and field-count truncation with generated documents; the
100 000-sibling timing; determinism across two runs; human shapes; every
exit path of `form diff` incl. old and new rejection and non-recognition,
`same_variant: false`, `identical: true` on a file diffed with itself, and
each of added/removed/changed with and without values; that no value
appears on stderr or in any non-`--show-values` output (a sentinel value
in the fixture must be absent from stdout without the flag); that
`capabilities` lists both operations; and that existing commands are
unchanged.

## Capability advertisement

After the tests pass, `capabilities.operations` gains `"form fields"` and
`"form diff"`. The skill documents both, the `--show-values` rule, and
that paths are stable per schema version only.

## Verification

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- form fields \
  tests/fixtures/forms/fields-repeated-siblings.xml --json
cargo run --locked -p opencegeljaras-cli -- form diff \
  tests/fixtures/forms/fields-repeated-siblings.xml \
  tests/fixtures/forms/fields-repeated-siblings-edited.xml --json
```

## Unresolved questions

- Whether agents need a schema-derived "expected fields" listing (which
  fields the variant allows and which are required); that needs the
  bundle and a schema walker and is a later package.
- Whether `form diff` should also compare against a schema-canonical
  ordering; it does not, and reports document order only.
