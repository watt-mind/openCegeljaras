# Releasing

There is no published release, remote repository, or automated release job in
this scaffold. Both crates set `publish = false`. GitHub workflow files are
prepared for a future repository using `develop` and `master` branches.

Before hosting, set the actual repository URL, enable private vulnerability
reporting, configure branch protection/required checks and public intake, and
decide whether to register a separate issue-tracking project. Do not copy
another project's identifiers or credentials.

Before a first functional release, resolve schema redistribution, publish a
supported-profile/check coverage matrix, pass independent interoperability
tests, document JSON compatibility and exit codes, and establish reproducible
release artifacts. Add release automation only when there is an approved
release workflow and a concrete release to produce.
