# openCegeljaras master orchestrator

Use this prompt from the openCegeljaras checkout on the development server.
Ask Claude: **Read `./orchestrator.md` and begin the bounded run it describes.**
Creating or reading this file alone does not start an unattended service.

## Mission and initial scope

Act as the single coordinator for openCegeljaras. Inspect the actual checkout,
specify small work packages, delegate implementation, verify the results, and
obtain independent review. Start useful local work without requiring a new
remote repository, tracker project, or server registration.

The default run covers the two initial work packages below: a precise form
inspection contract, followed by its bounded read-only implementation when
the evidence permits it. Finish those packages and report the next milestone;
do not automatically implement the entire roadmap. Existing operator scope
and instructions take precedence over these defaults.

This launch authorizes scoped local research, implementation, tests, and
reviewable commits. External publication and merges depend on the current
session's authorization; do not assume them from credentials alone. Do not
ask again for actions already authorized. Complete concrete reviewable work
before requesting a missing decision, and continue independent work meanwhile.

## Read and establish the baseline

1. Read `AGENTS.md`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`, `SECURITY.md`,
   `docs/architecture.md`, `docs/research.md`, `docs/specifications.md`,
   `docs/roadmap.md`, `docs/testing.md`, and both JSON files under `data/`.
2. Inspect the Git branch, staged and unstaged changes, existing worktrees,
   available remotes, and current capabilities. Preserve all existing work.
   Do not print credential-bearing remote URLs or private configuration.
3. Check the server's installed tools against `scripts/check.sh` and the
   current CI/security workflows. Rust stable and the declared MSRV, Git,
   Python, Node/npm, cargo-machete, cargo-deny, and actionlint are needed.
   Coverage requires cargo-llvm-cov and LLVM tools; security scanning uses
   Gitleaks. Report missing tools precisely; never silently skip checks.
4. Run the baseline checks before attributing failures to a new change.
   At scaffold creation only `capabilities`, `sources`, `skill`, help, and
   version existed. Discover current behavior; do not treat this dated
   description or the research snapshot as live state.
5. Reconcile any ignored `tmp/orchestrator-state.md` checkpoint with actual
   branches, claims, PRs, and check results. Confirm no other coordinator is
   working on the same queue or owned paths before dispatch.

## Choose the execution mode

### Standalone bootstrap

Use this mode when the repository has no configured tracker or remote. Missing
Factory setup does not block local profile discovery or implementation.
Keep the bounded plan and progress in ignored `tmp/` after verifying it is
ignored and not a symlink. Never invent Linear IDs, project mappings, labels,
credentials, or public repository URLs.

The original scaffold may have an unborn `codex/scaffold` branch with files
already staged and no initial commit. Inspect the exact staged contents and
baseline results. If they are solely the reviewed project scaffold, create a
Conventional Commit for that baseline before creating worker worktrees.
Never commit unrelated staged files or use `git add .` blindly. If unrelated
work cannot be separated safely, preserve it and identify the specific
ownership decision needed; continue read-only specification meanwhile.

Use the verified local base commit for worker branches. Do not fetch a
nonexistent `origin/develop`, create an empty remote, or fabricate CI results.
Keep completed commits local unless publication is already authorized.

### Configured project workflow

If the server already has this project's authenticated GitHub/tracker routing,
read its installed execution and issue-tracking protocol before claims or
issue writes. Use applicable Factory skills when that workflow is requested;
do not assume slash commands or tool names exist on this server.

Verify the actual project mapping, claim identity, active ownership, and base
branch. Use eligible, unassigned `ai:agent-ready` work within this run's scope,
with dependencies satisfied. Read full issues and confirm the protocol's
actual states/labels. Claim through that protocol, immediately re-read
ownership, and skip a lost race. Do not create competing local work for an
already assigned task. An empty eligible queue is a valid dispatch outcome;
report the specification work required without silently promoting tickets.

Keep an existing manual/report-only configuration unchanged. Do not start
central dispatch, daemons, recurring jobs, or another coordinator. If the
installed protocol forbids the requested action, identify the conflict rather
than bypassing it. A host registry is routing, not permission to send filings.

## Initial work packages

These are local planning names, not tracker identifiers. Before implementation,
each package needs Problem & Context, Acceptance Criteria, Source File
Pointers, Owned Paths, and an executable Verification Command. Identify new
files as proposed. Refine against actual source code and independent review.

### 1. Specify the first read-only form inspection contract

Own `docs/form-inspection.md` and the corresponding documentation-index and
roadmap entries. Reconcile the bundled public schema metadata with the
official sources. Preserve retrieval dates, exact namespaces, dependency
versions, and uncertainty. Do not repeat the entire research exercise when
the existing evidence answers the question; recheck mutable requirements.

Specify one bounded form-inspection surface: accepted encodings, root/form
identification, supported and unknown version behavior, safe field exposure,
resource limits, error codes, JSON/exit semantics, and synthetic fixtures.
Distinguish format recognition from XSD conformance and legal acceptance.
No actual document values or private input paths belong in diagnostics.

Choose a parser only after checking primary documentation, supported Rust
version, licensing, and the security properties needed by this contract.
Record schema redistribution and full XSD-validation decisions separately.
Public schema availability is not a redistribution license. Do not vendor
XSD, XSLT, Java artifacts, or vendor examples without established rights.

Acceptance: a source-backed, independently reviewed contract that another
worker can implement without inventing requirements; explicit unresolved
questions and a clear decision whether package 2 can proceed. If a required
input-format fact is missing, block that behavior, not unrelated discovery.

### 2. Implement bounded `form inspect`

Proceed only after package 1 has a reviewed contract and resolved prerequisites.
Assign explicit ownership for the core parser, CLI adapter, contract tests,
synthetic fixtures, and associated documentation before dispatch. Start with
one implementation worker to avoid shared-contract conflicts.

Implement only the reviewed read-only inspection behavior. Keep file I/O in
the CLI and deterministic parsing in the core. Enforce documented byte,
depth, node, and text limits; reject prohibited XML constructs. Never resolve
document-supplied URLs, entities, schemas, or stylesheets over the network.
Do not execute embedded HTML/XSLT or spawn a container tool implicitly.

Test independently constructed positive/negative inputs, unsupported versions,
malformed XML, resource limits, encoding behavior, private-argument redaction,
and human/JSON output. Do not derive fixtures from private dossiers. Verify
that existing introspection remains accurate and unsupported operations fail.

Only after behavior and tests pass may capabilities and the embedded skill
advertise `form inspect`. Explicitly state that XSD validation, cryptographic
verification, submission, and authority acceptance were not performed.
Do not add working-looking stubs for validate/create/preflight/submit.

Full XSD conformance, dossier preflight, authoring, and receipt interpretation
remain later milestones. openSzigno's generic writer needs a reviewed domain
adapter; do not alter sibling projects as part of this run. Record follow-ups
in the configured tracker when authorized, otherwise in the local checkpoint.

## Delegate with ownership and suitable models

The coordinator owns scope, shared contracts, claims, and integration. Begin
with one implementation worker and one separate reviewer when capacity permits.
Respect server/registry concurrency limits; serialize heavy builds if needed.
Never have two workers edit the same owned paths simultaneously.

Use the strongest available reasoning model for ambiguous specification,
security boundaries, and cold review. A capable coding model is appropriate
for a fully specified implementation; a faster model can handle narrow
read-only searches. With Claude, use the installed Opus/Sonnet equivalents
where available; discover actual supported model names rather than inventing
version IDs. If subagents are unavailable, work serially and report that
independent review remains outstanding instead of simulating reviewer approval.

Give each worker the complete package, exact worktree/base commit, owned paths,
acceptance criteria, non-goals, verification commands, and these instructions:
You are not alone in the codebase. Preserve others' changes; never revert
unrelated edits. Implement only your assigned scope. Report phase changes,
blockers, and final evidence without private inputs. Workers never merge.

Use one isolated worktree and `codex/<public-safe-slug>` branch per package
once a base commit exists. No ports, database, generated environment, or new
worktree framework is needed for this stateless CLI. Do not spawn an agent
only to wait for a process or CI.

## Verify, review, and integrate

Read the current scripts/workflows, then run each package's exact checks and
the applicable repository baseline:

```sh
./scripts/check.sh
cargo build --release --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --locked --fail-under-lines 90
cargo run --locked -p opencegeljaras-cli -- capabilities --json
```

The MSRV and coverage threshold above reflect the scaffold; follow the actual
manifests/workflows if legitimately changed. Never lower a gate to pass a task.
For documentation-only work, run documentation checks and source review;
do not add artificial Rust tests. Distinguish local checks from remote CI and
interoperability tests. Never submit a synthetic filing to production.

Give a cold reviewer the full specification, complete diff, exact head commit,
acceptance evidence, and test results. Require ranked findings and an explicit
approve/fix/blocked verdict. For the new CLI flow, independently exercise help,
human/JSON output, and error recovery with synthetic inputs. Fix in-scope
findings and re-review changed code; green tests alone are not approval.

Integrate reviewed local commits serially, preserving others' work. Where a
remote workflow and publication authorization exist, use public-safe commits
and PRs against the verified integration branch. Keep private tracker links,
IDs, local paths, and internal handoffs out of public descriptions.

Merge a PR only with current-session authorization, independent review of its
actual head, and every applicable CI/security check passing at that head.
Wait for post-merge checks before declaring it landed. If those fail, halt
further integration and diagnose the failure. Never bypass branch protection.
No master/release merges, tags, package publication, or deployment are part of
this default run. Missing remote access should leave a reviewed local result.

## Checkpoint, resume, and completion

Maintain `tmp/orchestrator-state.md` with the run scope, execution mode, base
and head commits, owned paths, active workers/claims, verification results,
review findings, blockers, and next action. Do not store secrets, raw logs,
private dossier contents, or personal data. Reconcile it with live state on
resume; stale notes never authorize taking another worker's claim.

After two unsuccessful fix/review rounds on the same blocker, preserve the
work and report the exact missing evidence or decision. Do not retry unchanged
environment failures indefinitely or mark incomplete work done. Continue any
independent authorized work that remains useful.

Finish when the default two packages are reviewed and integrated locally, the
operator's narrower scope is complete, or no independent progress remains.
Report implemented behavior, commits/PRs, actual verification, remaining
limitations, and the next proposed package. Communicate through the current
operator session and authorized tracker only; no external chat/email messages.

Begin now with baseline discovery and the first runnable package. Do not stop
at proposing a plan when the authorized local work can proceed.
