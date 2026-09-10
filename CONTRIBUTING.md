# Contributing

This repository is a scaffold. Consult the [roadmap](docs/roadmap.md) before
implementing a command. A proposed operation must not appear as implemented
in capability output until its supported profiles and tests exist.

Use Rust 1.88 or newer. Run `lefthook install` to enable format, lint, and
Conventional Commit hooks, and `./scripts/check.sh` before review. The check
script also uses Python 3, Node/npm, cargo-machete, cargo-deny, and actionlint.

Use small branches named `codex/<topic>` and Conventional Commit subjects
such as `feat(core): inspect a pinned form profile`. The intended PR target is
`develop`; `master` is reserved for stable releases once hosting is configured.
No Linear team/project mapping or remote repository is established yet.

Changes involving parsing need malformed-input and resource-limit tests.
Validation rules need source references and positive/negative synthetic
examples. Integration tests must demonstrate what a command does not prove,
including the distinction between local checks and official acceptance.

Do not use real client material as a bug report or fixture. See
[SECURITY.md](SECURITY.md) and the [fixture policy](tests/fixtures/README.md).

Before the first release, establish hosted security reporting, branch
protections, required CI checks, independent interoperability evidence,
third-party schema rights, and an explicit release process. Both crates have
`publish = false`; there is no automated release workflow.
