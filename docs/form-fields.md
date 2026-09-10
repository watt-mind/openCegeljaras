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
element names, which the official schemas fix, plus positional indices for
repeated siblings. A path is therefore stable across runs and across
documents that share structure, but it is positional: removing the first
of three `Névfoglalás` entries renumbers the others, and adding a sibling
to a singleton turns `X` into `X[1]`. One such edit surfaces as several
`changed` entries plus an `added` or `removed` one, and the skill says so.
Values are personal and company data and are treated as such: hidden by
default, shown only with `--show-values`, and never written to stderr.
Value lengths are always reported and are treated as structure, not
content.

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
`[n]`; an element without same-named siblings carries none. Indices apply
to every path component, not only the last (`Adatlap[2]/Tag[1]/@x`), and
a leaf whose same-named sibling is a non-leaf still carries one.
Namespaces do not appear in the path; two siblings with the same local
name in different namespaces are counted as same-named and both carry an
index, and the field records its own namespace separately. Attributes
follow the same rule: when an element carries several attributes with the
same local name in different namespaces, each gets `@name[n]` in document
order. A path therefore identifies exactly one field within a document,
and a unit test asserts that invariant. Paths are deterministic for a
given document and identical across runs.

A **field** is one of:

- a **leaf element**: an element with no element children. Its value is
  the concatenation of all its text children in document order (a comment
  or processing instruction inside the leaf splits the text into several
  nodes, all of which are included; `roxmltree` already merges adjacent
  text and CDATA), with no trimming or whitespace normalisation beyond
  what XML parsing performs: character and entity references resolved and
  CR and CRLF line ends normalised to LF. An empty element has the empty
  value.
- an **attribute**: any attribute on any element from the variant root
  down, except namespace declarations and attributes in the XSI namespace.
  Its path is the owner's path followed by `/@` and the attribute local
  name; the variant root's own attributes have paths beginning with `@`.
  The value is the attribute value after XML attribute-value
  normalisation. Attribute order is the document's.
- a **mixed element**: an element with both element children and text
  that is not only XML whitespace (space, tab, CR, LF). It is listed with
  `kind: "mixed"` and no value, its
  text is not exposed, and its element children are still walked. This
  case does not occur in schema-conforming forms and exists so that no
  text is silently dropped.

Whitespace-only text under an element that has element children is not a
field. Comments and processing instructions are ignored.

Emission order: for each element in document order, its own entry if it
is a leaf or mixed element, then its attribute entries in document order,
then its children. At most 10 000 fields are listed, then
`fields_truncated: true` and the walk stops; `field_count` is the number
of listed entries, and the total is unknown when truncated. A value is cut
at 4096
Unicode scalar values with `…` and `value_truncated: true`. Path components
and attribute names are element and attribute names, already bounded by the
`form inspect` limits, and each is cut at 128 scalars as inspect does; a
whole path is cut at 1024 scalars with `…` and `path_truncated: true`.
Internally a field is keyed by its untruncated component sequence,
borrowed from the parsed tree (name slices plus an index per level), never
by an owned joined string: a 127-deep chain of 128-scalar names over
10 000 leaves would otherwise materialise about 170 MB of paths from a
35 KB input, and a generated test of that shape asserts bounded memory and
time. Two entries may print the same cut `path`; they remain distinct
fields.

## `form fields`

Behaviour: run the complete `form inspect` boundary; return any exit 1 or
exit 4 outcome unchanged except `command: "form fields"`; then walk the
variant root as above.

JSON `data`: the `form inspect` structure (`input`, `root`, `form`,
`structure`, `checks`, `notice`) plus the keys below, which are present as
`null` on exit 1 and 4 (`values_redacted` is always a boolean) and filled
on exit 0. Keys are emitted alphabetically as in every other command; the
examples here are grouped for reading.

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
  "values_redacted": true
}
```

`value` is present only with `--show-values`, which sets
`values_redacted: false` (the same flag and polarity as `form validate`);
without it the key is absent and `value_length` (in Unicode scalar values,
before any cut) still allows an agent to see that a field is empty or
filled. `kind` is `element`,
`attribute`, or `mixed`; a `mixed` entry has no `value`, `value_length`,
or `value_truncated`. `namespace` is the field's own namespace URI or
null. The `notice` reads "field listing of one recognised form; values
are shown only on request; not validation, verification, or acceptance".

Human output: the `form inspect` lines up to and including the
`namespaces:` list,
then exactly `fields: N` or `fields: N (truncated)`, then one indented
line per field in one of three shapes: `PATH = VALUE` with
`--show-values`, `PATH (N chars)` without, and `PATH (mixed content)` for
mixed elements, where `PATH` and `VALUE` are each one escaped JSON string
literal under the `form inspect` rule. Then inspect's four boundary lines.

Exit codes: 0 listed; 1, 2, 3, 4 as `form inspect`; stderr line
`opencegeljaras: form fields: <code> (exit N)` whenever the exit code is
not 0.

## `form diff`

Behaviour: run `form fields` logic on `OLD`; if it succeeds, on `NEW`.
`NEW` is never opened when `OLD` fails. If a side is rejected at the input
boundary, exit 1 with that inspect code, `data.side` naming `"old"` or
`"new"`, that side's inspect rejection data under `data.old` or
`data.new`, and the other side null. If a side is not a recognised form,
exit 4 with `error.code` `old_not_recognized` or `new_not_recognized`,
that side's full inspect data under its key, and the other side null,
except that `new_not_recognized` keeps `OLD`'s exit-0 summary (`input`,
`root`, `form`, `structure`, `field_count`, `fields_truncated`), since it
was already computed. Fixed messages:

| Code | Message |
| :--- | :--- |
| `old_not_recognized` | `the OLD document is not a recognised form` |
| `new_not_recognized` | `the NEW document is not a recognised form` |

The architecture sentence "on exit 4 `error.code` equals
`data.form.status`" is amended to add "or, for `form diff`, names the
unrecognised side".

Both documents' fields are keyed by their untruncated paths. Keys present
only in `OLD` are `removed`; only in `NEW` are `added`; present in both
with a different kind, namespace, or value are `changed`; present in both
and equal are counted in `unchanged`. Comparison is exact on the full
values; the 4096-scalar cut applies only to what is emitted, so a
difference beyond the cut is still reported as `changed` with both emitted
values equal and `value_truncated: true` on both. `added` and `changed`
entries are emitted in `NEW` document order, `removed` in `OLD` document
order. When either side's field list was truncated, the diff is computed
over the listed fields, `identical` is false, and added or removed entries
near the cap may be artefacts of the cap; the per-side `fields_truncated`
flag makes that visible.

JSON `data`, grouped for reading (emitted alphabetically):

```json
{
  "side": null,
  "old": { "input": {}, "root": {}, "form": {}, "structure": {},
           "field_count": 0, "fields_truncated": false },
  "new": { "…": "…" },
  "same_variant": true,
  "identical": false,
  "added": [ { "path": "…", "path_truncated": false, "kind": "element",
               "namespace": "…", "value_length": 3,
               "value_truncated": false } ],
  "removed": [ { "path": "…", "path_truncated": false, "kind": "…",
                 "namespace": "…", "value_length": 3,
                 "value_truncated": false } ],
  "changed": [ { "path": "…", "path_truncated": false,
                 "old_kind": "element", "new_kind": "element",
                 "old_namespace": "…", "new_namespace": "…",
                 "old_value_length": 3, "new_value_length": 5,
                 "old_value_truncated": false,
                 "new_value_truncated": false } ],
  "unchanged": 40,
  "values_redacted": true,
  "checks": { "well_formed": "passed", "xsd_conformance": "not_performed",
              "signature": "not_performed",
              "authority_acceptance": "not_performed" },
  "notice": "field-level comparison of two recognised forms; not validation, verification, or acceptance"
}
```

With `--show-values`, `added` and `removed` entries carry `value`, and
`changed` entries carry `old_value` and `new_value`; `values_redacted`
becomes false. Value-length and value-truncation keys are null for
`mixed` entries. `same_variant` is false when the two variant ids differ;
the diff is still produced by path. `identical` is true only when
`added`, `removed`, and `changed` are all empty, `same_variant` is true,
and neither side is truncated.

Nullability: on exit 0 every key is filled and `checks.well_formed` is
`passed`. On exit 4, `side` names the failing side, its inspect data is
under its key, `same_variant`, `identical`, `added`, `removed`,
`changed`, and `unchanged` are null, `checks` reflects that side, and
`notice` is present. On exit 1 the same holds with the failing side's
rejection data and `checks` null; nothing is printed on stdout without
`--json`. `values_redacted` is always a boolean.

Exit 0 whenever both sides are recognised, identical or not; an agent
reads `identical`. The exit code does not encode difference, because
exit 1 and exit 4 already mean rejection and failed recognition in this
tool.

Human output on exit 0, in this order: `old: VARIANT, N fields` and
`new: VARIANT, N fields` (the count followed by the word `truncated` in
parentheses when cut),
`same variant: true|false`, `identical: true|false`, `added: N`,
`removed: N`, `changed: N`, each count line followed by one indented line
per entry, `unchanged: N`, then the four boundary lines. Entry lines are
`PATH` alone without `--show-values`; with it, `PATH = VALUE` for added
and removed entries and `PATH OLD -> NEW` for changed entries, every
`PATH`, `VALUE`, `OLD`, and `NEW` being one escaped JSON string literal.
On exit 4 the human report is `old: STATUS` or `old: VARIANT, N fields`
followed by `new: STATUS` or `new: none`, then the boundary lines. The
stderr line is `opencegeljaras: form diff: <code> (exit N)`.

## Determinism and safety

Both commands are pure functions of their inputs: no clock, no network,
no environment. Values reach stdout only with `--show-values`, never
stderr, never a diagnostic; value lengths are always reported as
structure. The skill tells agents to pass `--show-values`
only when the operator asked to see or compare values, and that the
output then contains personal and company data. A field listing or diff
says nothing about validity, verification, or acceptance, and the notice
and boundary lines say so on every run.

Field walks are bounded by the `form inspect` limits (elements, depth,
text, attributes) plus the 10 000-field cap, so cost is linear in the
document. Path construction for an element with many same-named siblings
must count siblings once per parent, not once per child, so a parent
with 200 000 children stays linear: one sibling-count map per parent,
asserted by a unit test. The CLI-level timing tests use a generous
60-second bound so they never flake on a slow, instrumented CI runner while
still catching a quadratic regression, which would take minutes.

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
| `fields-repeated-siblings.xml` | Name reservation with three `Névfoglalás` entries, an attribute on the variant root, an attribute on an indexed element, two same-local-name attributes in different namespaces, same-named siblings in different namespaces, an intermediate indexed component, an empty leaf, a whitespace-only leaf, a CDATA leaf, a leaf split by a comment, a mixed element, an XSI attribute, a comment, and the sentinel value `SENTINEL-VALUE-7731` in one leaf |
| `fields-repeated-siblings-edited.xml` | The same with the first `Névfoglalás` removed (so the others renumber and surface as changed), one value changed, one attribute added |
| `fields-other-variant.xml` | A registration variant whose first path, index form included, equals the name-reservation fixture's, so `same_variant: false` with a non-empty `unchanged` |

Tests cover: exact field lists (paths, kinds, namespaces, lengths) with
and without `--show-values`; index assignment for repeated siblings and
none for singletons; attributes incl. root attributes and the XSI and
namespace-declaration exclusions; CDATA and empty values; the mixed rule;
value, path, and field-count truncation with generated documents; the
path-uniqueness invariant; the deep-chain memory document; the
100 000-sibling timing; determinism across two runs; human shapes; every
exit path of `form diff` incl. old and new rejection and non-recognition,
`same_variant: false`, `identical: true` on a file diffed with itself, and
each of added/removed/changed with and without values; that the sentinel
value never appears on stderr and appears on stdout
only with `--show-values`; that
`capabilities` lists both operations; and that existing commands are
unchanged.

## Capability advertisement

After the tests pass, `capabilities.operations` gains `"form fields"` and
`"form diff"`. The skill documents both, the `--show-values` rule, and
that paths are positional: an insertion or removal renumbers same-named
siblings.

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
- Key-based identity for repeated entries (matching a `Névfoglalás` by
  its name rather than its position) needs schema knowledge of which
  child identifies an entry; positional indices are the honest default
  until then.
