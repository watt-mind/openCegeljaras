# Synthetic schema bundle fixtures

Everything in this directory is **synthetic**. It was written by hand for the
`schemas verify` contract tests and is not derived from, copied from, or a
sample of any official Hungarian company-court schema. No official XSD is
vendored in this repository. These files are released under
[CC0](../LICENSE), like every other fixture here; that licence says nothing
about third-party schemas.

`manifest.json` pins a five-resource mini graph on the two real hosts, so that
the manifest path rules are exercised as they are in production. Nothing is
ever fetched from those hosts: the mirrored bytes are the ones stored here.

| Path | Role |
| :--- | :--- |
| `manifest.json` | The fixture manifest, passed with `--manifest` |
| `good/` | A complete mirror laid out as `<host>/<path>` |
| `tampered/` | The same mirror with one changed and one removed file |
| `instance-conforming.xml` | A document the mini graph accepts, for the real-validator tests |
| `instance-nonconforming.xml` | The same with a number violating the shared pattern |

The graph is a wrapper importing two variant schemas, both of which import one
shared simple-type schema. One of those imports is written with an `http://`
scheme so that scheme normalisation is exercised. The fifth resource is the
dossier root: it sits outside the form closure, and it carries a `<!DOCTYPE`
declaration and an entity on purpose, so that the digest-only rule for the
dossier graph is exercised by a resource the bounded parser would refuse.

`tampered/` changes one byte-range of `variant-a-20260101.xsd` (reported as
`mismatch`) and omits `variant-b-20260101.xsd` entirely (reported as
`missing`). Tests that need an unpinned import, a namespace mismatch, an
oversize file, a symlink, or an invalid manifest build those in a scratch
directory at run time; they are not stored here.

Verification of a bundle is a digest comparison and an import audit. It is not
a redistribution licence, not XSD validation, and not acceptance by any
authority.
