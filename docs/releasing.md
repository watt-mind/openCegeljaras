# Releasing

There is no published release or automated release job. Both crates set
`publish = false`. The repository is hosted privately at
`github.com/watt-mind/openCegeljaras` since 2026-09-10, with `develop` as the
default branch and `master` reserved for stable releases; the CI and security
workflows run there on every push and pull request.

Before hosting, set the actual repository URL, enable private vulnerability
reporting, configure branch protection/required checks and public intake, and
decide whether to register a separate issue-tracking project. Do not copy
another project's identifiers or credentials.

Before a first functional release, resolve schema redistribution, publish a
supported-profile/check coverage matrix, pass independent interoperability
tests, document JSON compatibility and exit codes, and establish reproducible
release artifacts. Add release automation only when there is an approved
release workflow and a concrete release to produce.
