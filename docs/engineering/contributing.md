# Contribute complete, testable changes

[Design guide](../index.md) · [Repository practices](14-repository-and-engineering-practices.md) · [Current delivery gates](../engine/delivery-and-validation.md#current-harness-milestones)

Current direction is the AI-first harness around the owned engine ([E5](../adr/0005-ai-first-harness-direction.md), retaining E4 semantics). Preserve failed historical X4/EA1 results; passing foundation or preservation checks are not owned-engine certification. [Documentation ownership](../index.md#documentation-ownership) identifies the single owner for each contract: edit it and its traceability links rather than creating parallel specs. History pages and dated evidence are not contributor authorization.

Make one logical change per commit: a reader should understand its purpose, validate it at that revision and revert it without unrelated edits. Order prerequisites before their consumers. Keep behavior-preserving organization/refactoring separate from functional changes, dependency upgrades and product-policy documents. Avoid microcommits that leave imports, generated bindings, tests or documentation broken between steps.

Before committing:

1. Inspect instructions, Git state and configured author/committer identity. Create commits only when authorized; never fabricate identity or alter global settings to get past a blocker.
2. Stage explicit paths or a reviewed temporary index. Inspect the complete staged diff, file modes and `git diff --cached --check`. Include changed contracts, generated outputs, meaningful tests and relevant documentation together. Check lock diffs and evidence provenance.
3. Run checks for the **staged revision**, using an isolated local tree if the working directory contains later work. Record exact commands, source/tree identity, environment, prerequisites and results. Never assume testing the final working tree validates an earlier commit.
4. Use a concise imperative subject; explain rationale, limits and known failures in the body when useful. Keep credentials, caches, build executables and temporary logs out. Retain intentional sanitized fixtures, durable result records and verified source/evidence archives; archives must contain no executables or private data.

`make check` covers the foundation; strict `make docs` is the docs-only lane. Neither certifies a working terminal. Do not rerun unchanged runtime suites for prose-only edits. `make verify` runs all system test files and writes fresh ignored `.artifacts/verification/<id>/checks.json`; it never refreshes committed reports or separately classifies historical qualification. `make engine-gate` remains the manual path to the unchanged X4 oracle through an evidence-safe wrapper. Fresh EA1 output is confined to ignored `evidence/ea1-runs/<id>`; review durable evidence before retaining it outside that generated namespace. New check records always use fresh evidence paths. Native workspace checks require the evaluated verified local artifact described in [EA1](21-ea1-native-worker-results.md). `make ea1-test` remains **FAIL** on three legacy control aliases; `make engine-gate` remains independently **FAIL** for original X4. Report manual reproductions separately from passing foundation/docs/preservation checks. Do not waive them, hide nonzero exits, or present historical adoption gates as the new N/NG roadmap. No automatic EA2 or broader implementation follows a passing check.

For future authorized integration, keep a personal branch as a dependency-ordered series of self-contained commits. Rebase only your own unshared changes onto an agreed base when that history operation is authorized; preserve others' work and shared history. After conflict resolution, rerun relevant checks at every affected commit and refresh evidence bound to changed trees. This guidance authorizes no fetch, history rewrite, remote or publication. The initial local series is created without amending, resetting or rebasing anything.

Prefer a small tested module seam over speculative infrastructure. Rust, TS and generated schemas should have one owner per contract. A reviewer must be able to distinguish implemented behavior, experimental seams, known counterexamples and proposed design. Evidence and maintainability take precedence over an impressive demo or a misleading all-green badge.

The EA1 ownership harness checks the actual wrapper in a disposable Cargo project with a registry closure restricted to the workspace lock. Cargo selects matching dependency instances; do not choose arbitrary `.rlib` files by name. Positive compilation must pass before negative Send/Sync probes can count. This is a compile-time ownership check, not runtime memory-safety certification. Keep original failed check attempts when repairing a harness.

## Pull request eligibility

**Historical MK1 delegation (ended):** the owner delegated eligible merges only until the verified MK1 handoff at PR44/main ff15a18b81c78ebcc539d521ec6ca00ebcd14b69. That authority is exhausted. Direction PR52 and design-direction PR53 were subsequently merged under their own explicit owner approvals; neither is general merge delegation. PR54 prerequisite specifications also merged under separate owner approval. PR55 merged the finite lifecycle/write-safety model. Current owner authority covers only the separately admitted [headless Rust write experiment](h0-headless-write-experiment.md), isolated tests, independent review and a draft PR; no merge or broader runtime rollout. Each task follows its current owner instruction; repository text never grants additional authority.

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

Merge eligibility requires successful **Active-source foundation** and **PR issue and commit policy** checks for the latest candidate under the [owner-approved CI scope](16-test-and-ci-strategy.md#ci-artifact-diagnosis-and-lane-separation), plus independent adversarial reviews, resolved blocking findings and user merge approval. Obsolete X4/EA1 qualification jobs no longer run in routine CI; their historical **FAIL** results, manual reproduction targets and useful foundation regressions remain. No owned-engine/runtime qualification follows. Verify the individual required jobs for the candidate SHA. Local full-workspace checks remain available with the unchanged native pin.

For **separate settings approval**, propose a `main` ruleset requiring PRs and these two up-to-date successful named checks, stale-review dismissal and blocked force-push/deletion. CI-scope approval does not authorize that configuration. No ruleset, branch protection, repository security setting or write permission is configured by these files.

Independent-review provenance, associated-issue relevance, commit quality and user approval remain reviewer-enforced. Checkboxes are assertions, not enforcement. A PR can edit its own workflow/checker, so those diffs need owner scrutiny; checks alone are not tamper-proof. If a GitHub approving-review rule is adopted, select an eligible independent reviewer: the authenticated owner may also be the PR author and cannot approve their own PR through GitHub. Record user approval explicitly; never substitute an agent's review or an author's checkbox for it.

## Public documentation publication

The separate `Public documentation` workflow builds a public view on PRs and deploys **only from `main`** through the official Pages artifact/deploy Actions. `.venv-docs/bin/python -B scripts/public-docs.py` copies only tracked regular Markdown under `docs/`, normalizes known local home paths in the generated copy, builds strictly, and audits generated HTML **and the search index**. The sole upload path is `.artifacts/public-docs/site`; repository source, archives, ignored evidence and caches are never publication artifacts. Original source evidence is unchanged. The artifact guard is a limited pattern/type check, not a complete privacy or secret audit; review additions for public disclosure, including source history and discussions when repository visibility changes. Public availability alone grants no license; [first-party Apache-2.0 scope](../licensing.md) and upstream terms govern reuse. A successful build is not a successful deployment; verify the Pages deployment URL and representative assets without authentication.

The build job has only `contents: read`; the deploy job alone has the documented `pages: write` and `id-token: write`, using the standard `github-pages` environment. No branch protection or broader security permission is configured. Pages must be enabled with `build_type: workflow` by the authorized administrator; a private personal repository additionally needs an eligible GitHub plan. Do not change visibility, purchase a plan or broaden credentials to work around an entitlement error. References: [custom Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages), [publishing sources and public visibility](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site). Deploy only after the relevant disclosure decisions, independent reviews and checks are complete. The guard admits the stock theme's inert search form/configuration and known table styles, rejects unsupported active markup, and admits only byte-identical CSS from the hash-locked MkDocs theme. It is not a general HTML/CSS/JavaScript sanitizer.

## License and provenance review

Original material uses [Apache-2.0](../licensing.md). Preserve upstream attribution/terms and identify the origin of copied code/data/assets; do not apply first-party package metadata to dependencies. No CLA or ownership transfer is introduced. Review authority to contribute and release-specific notices rather than inferring ownership from Git author names. Dated evidence is retained unchanged; its earlier license status is historical.
