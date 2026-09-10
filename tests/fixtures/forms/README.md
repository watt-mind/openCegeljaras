# Synthetic form fixtures

These files are hand-constructed from the public element and namespace names
of the company-form schemas, as recorded in the
[form inspection contract](../../../docs/form-inspection.md). They contain
placeholder text only: no real company, person, case number, or filing appears
in them, and none of them is derived from a client dossier or from any ignored
private material.

They are **not** asserted to be XSD-valid. They exercise format recognition,
encoding handling, resource limits, and rejection paths, and nothing here
establishes schema conformance, signature validity, filing, or acceptance.

Like the rest of `tests/fixtures`, they are released under [CC0](../LICENSE).

Oversized, over-deep, over-wide, and long-text inputs, a missing file, a
directory, and an oversized declaration are generated inside the tests rather
than stored here.

| Fixture | Purpose |
| :--- | :--- |
| `cegbejegyzesi-minimal-utf8.xml` | Recognised registration variant, declaration, no BOM |
| `valtozasbejegyzesi-stylesheet-bom.xml` | Change variant with BOM and `xml-stylesheet` PI |
| `nevfoglalasi-iso-8859-2.xml` | Name-reservation variant in ISO-8859-2 bytes |
| `nevfoglalasi-no-declaration.xml` | UTF-8 without an XML declaration |
| `cegbejegyzesi-no-encoding-label.xml` | Declaration without `encoding`; XSI schema location |
| `wrapper-no-namespace.xml` | Unnamespaced wrapper, exit 4 `unrecognized_root` |
| `bom-declared-iso-8859-2.xml` | BOM with an ISO-8859-2 label, exit 1 `invalid_encoding` |
| `wrapper-20240101-unsupported.xml` | Other wrapper date, exit 4 `unsupported_version` |
| `wrapper-unknown-child.xml` | Unknown single child, exit 4 `unrecognized_variant` |
| `wrapper-two-children.xml` | Two variant children, exit 4 `unrecognized_variant` |
| `unrecognized-root.xml` | Unrelated document, exit 4 `unrecognized_root` |
| `doctype.xml` | Internal subset, exit 1 `prohibited_construct` |
| `undeclared-entity.xml` | Undeclared entity reference, exit 1 `malformed_xml` |
| `malformed.xml` | Unclosed element, exit 1 `malformed_xml` |
| `utf16-bom.xml` | UTF-16 BOM, exit 1 `unsupported_encoding` |
| `declared-utf8-invalid-bytes.xml` | ISO-8859-2 bytes under a UTF-8 label, exit 1 `invalid_encoding` |
| `declared-windows-1250.xml` | Unsupported label, exit 1 `unsupported_encoding` |
