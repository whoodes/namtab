# Contribute complete, testable changes

[Design guide](../index.md) · [Repository practices](14-repository-and-engineering-practices.md) · [Current delivery gates](../engine/delivery-and-validation.md#ordered-milestones)

Current direction is the owned agent-native engine ([E4](../adr/0004-owned-agent-native-engine.md)). Preserve failed historical X4/EA1 results; passing foundation or preservation checks are not owned-engine certification. [Documentation ownership](../index.md#documentation-ownership) identifies the single owner for each contract: edit it and its traceability links rather than creating parallel specs. History pages and dated evidence are not contributor authorization.

Make one logical change per commit: a reader should understand its purpose, validate it at that revision and revert it without unrelated edits. Order prerequisites before their consumers. Keep behavior-preserving organization/refactoring separate from functional changes, dependency upgrades and product-policy documents. Avoid microcommits that leave imports, generated bindings, tests or documentation broken between steps.

Before committing:

1. Inspect instructions, Git state and configured author/committer identity. Create commits only when authorized; never fabricate identity or alter global settings to get past a blocker.
2. Stage explicit paths or a reviewed temporary index. Inspect the complete staged diff, file modes and `git diff --cached --check`. Include changed contracts, generated outputs, meaningful tests and relevant documentation together. Check lock diffs and evidence provenance.
3. Run checks for the **staged revision**, using an isolated local tree if the working directory contains later work. Record exact commands, source/tree identity, environment, prerequisites and results. Never assume testing the final working tree validates an earlier commit.
4. Use a concise imperative subject; explain rationale, limits and known failures in the body when useful. Keep credentials, caches, build executables and temporary logs out. Retain intentional sanitized fixtures, durable result records and verified source/evidence archives; archives must contain no executables or private data.

`make check` covers the foundation; strict `make docs` is the docs-only lane. Neither certifies a working terminal. Do not rerun unchanged runtime suites for prose-only edits. `make verify` runs all system test files and writes fresh ignored `.artifacts/verification/<id>/checks.json`; it never refreshes committed reports. `make engine-gate` invokes the unchanged X4 oracle through an evidence-safe wrapper. Fresh EA1 output is confined to ignored `evidence/ea1-runs/<id>`; review durable evidence before retaining it outside that generated namespace. New check records always use fresh evidence paths. Native workspace checks require the evaluated verified local artifact described in [EA1](21-ea1-native-worker-results.md). `make ea1-test` remains **FAIL** on three legacy control aliases; `make engine-gate` remains independently **FAIL** for original X4. Report these separately from passing foundation/docs/preservation checks. Do not waive them, hide nonzero exits, or present historical adoption gates as the new N/NG roadmap. No automatic EA2 or broader implementation follows a passing check.

For future authorized integration, keep a personal branch as a dependency-ordered series of self-contained commits. Rebase only your own unshared changes onto an agreed base when that history operation is authorized; preserve others' work and shared history. After conflict resolution, rerun relevant checks at every affected commit and refresh evidence bound to changed trees. This guidance authorizes no fetch, history rewrite, remote or publication. The initial local series is created without amending, resetting or rebasing anything.

Prefer a small tested module seam over speculative infrastructure. Rust, TS and generated schemas should have one owner per contract. A reviewer must be able to distinguish implemented behavior, experimental seams, known counterexamples and proposed design. Evidence and maintainability take precedence over an impressive demo or a misleading all-green badge.

The EA1 ownership harness checks the actual wrapper in a disposable Cargo project with a registry closure restricted to the workspace lock. Cargo selects matching dependency instances; do not choose arbitrary `.rlib` files by name. Positive compilation must pass before negative Send/Sync probes can count. This is a compile-time ownership check, not runtime memory-safety certification. Keep original failed check attempts when repairing a harness.

## Pull request eligibility

Every PR needs one associated **issue in this repository**, required checks passing for its candidate SHA, two independent adversarial agent reviews, and explicit user merge approval. Start the body with this exact non-closing field, followed by the ordinary description:

```text
Associated issue: https://github.com/whoodes/namtab/issues/NUMBER
```

Use the real issue number, not an example, a PR URL, arbitrary prose or a checklist assertion. The read-only governance check verifies existence/type and the complete PR-introduced commit range. Related issues can be linked normally; association does not require auto-closing anything and must not prematurely close an N0 umbrella. An audit issue/PR does not implement N0. The versioned `.github/pull_request_template.md` records scope, evidence and review obligations.

Run separate fresh review contexts for (1) correctness/contracts/tests/CI and (2) docs/README/readiness/scope. Record the exact reviewed SHA, counterexamples sought, severity/disposition and test evidence. Fix substantiated findings in focused commits, rerun affected/full applicable checks and re-review the final HEAD; old reviews are not evidence about later code. Agent review never grants merge approval. The user must approve the current revision; do not merge or enable auto-merge automatically.

## Commit messages and local hook

Use concise imperative titles and a body when rationale, non-obvious choices, validation or limits help a reviewer. No Conventional Commits prefix, issue-per-commit rule, body word count or filler is required. Wrap prose where practical; keep exact commands/URLs readable. Logical completeness, imperative grammar and rationale quality require human/agent review, not natural-language scoring.

One shared validator, `scripts/commit-message.mjs`, enforces objective formatting in the hook and CI: a nonempty single-line title without control characters, at most **72 Unicode code points**, and a blank separator before a nonempty body. Git-generated Merge/Revert titles get a **120-code-point** limit. Unicode, multiline bodies, LF/CRLF and body tabs are supported. The hook validates the literal UTF-8 message file, including comment-like text, and does not edit it or guess invocation-specific `--cleanup` flags. Git may remove editor comments afterward; keep comment blocks separated from the title by a blank line. Noninteractive `-m`/`-F` messages and `#`-prefixed titles follow the same formatting rules. Body content/line length is not mechanically scored. Local `fixup!`/`squash!`/`amend!` messages with a target are allowed for ordinary workflows; unfinished autosquash commits fail candidate CI before merge. This does not authorize rebasing or rewriting shared history.

Cloning **does not activate hooks**. Prerequisites: POSIX `sh`, Git and Node ≥20; the hook prefers the pinned local Node binary on Linux and otherwise uses `node` on PATH. Project checks still use the exact pinned version. The hook is noninteractive and installs/downloads nothing. Inspect existing configuration and active hooks before opting in:

```sh
git config --show-origin --get-all core.hooksPath
git rev-parse --git-path hooks
# Inspect existing active hooks; then, if there is no conflict:
git config --local core.hooksPath .githooks
```

Do not replace unrelated hooks or change global configuration. Preserve the executable bit on `.githooks/commit-msg`. `--no-verify` can bypass a local hook; the CI check independently validates only commits introduced by this PR, never the original main history. Errors state the formatting correction without echoing the whole message. No hook proves absence of secrets or the quality of a change.

## Mechanical enforcement and proposed protection settings

The `PR issue and commit policy` job reads the live PR/issue/commit APIs using a short-lived read-only Actions token; it runs for opened/reopened/synchronize/**edited**/ready-for-review events, with per-PR concurrency cancellation. Missing/inaccessible associations, stale HEADs, incomplete ranges or API errors fail closed. GitHub's PR-commits API limit is 250; larger ranges need a separately reviewed mechanism, not silent truncation. This is a check-time observation, not an atomic merge-time guarantee.

For separate owner approval, propose a `main` ruleset requiring PRs, up-to-date successful **Foundation contracts** and **PR issue and commit policy** checks, stale-review dismissal, and blocked force-push/deletion. Keep raw historical X4/EA1 jobs visibly failing; do not relabel them green or treat them as owned-engine NG certification. Their non-required status would be a protection-policy decision, not a changed experiment verdict. No ruleset, branch protection, repository security setting or write permission is configured by these files.

Independent-review provenance, associated-issue relevance, commit quality and user approval remain reviewer-enforced. Checkboxes are assertions, not enforcement. A PR can edit its own workflow/checker, so those diffs need owner scrutiny; checks alone are not tamper-proof. If a GitHub approving-review rule is adopted, select an eligible independent reviewer: the authenticated owner may also be the PR author and cannot approve their own PR through GitHub. Record user approval explicitly; never substitute an agent's review or an author's checkbox for it.
