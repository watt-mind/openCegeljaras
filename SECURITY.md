# Security policy

This scaffold performs no document processing and is not suitable for actual
company-court filings. There are no supported releases yet.

## Reporting

Do not put vulnerabilities, real dossiers, personal data, or signing material
in a public issue. A hosted private-reporting channel has not been configured.
Contact the maintainer through an already established private channel; before
public hosting, configure and document a private vulnerability-reporting route.

## Input boundaries

Future parsers must treat every XML, archive, signature, and receipt as hostile
input, enforce bounded resource use, disable external entity/resource loading,
and avoid active HTML or XSLT execution. Rendering a receipt is not permission
to visit its links or acknowledge delivery. Unknown profile versions fail closed.

Symlinks inside a bundle directory are followed when `schemas verify` reads a
mirrored resource, so never run it with elevated privileges over a directory
you do not control. Preserve original signed bytes and verification
provenance. Structural
conformance cannot establish signature validity, filing acceptance, or legal
effect. A receipt's textual claim is untrusted until independently verified
and associated with the relevant submission.

Private inputs belong only in ignored local storage. Public fixtures must be
synthetic. Never log personal data, dossier contents, keys, passphrases, tokens,
or data derived from secret material. No committed private keys, complete PEM
private-key armour lines, `.env` files, or secret-scanner exceptions are allowed.

Submission, signing, payment, and service acknowledgment must have separate
explicit interfaces. Read-only commands must not trigger them.
