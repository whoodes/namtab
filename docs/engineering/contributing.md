# Contribute complete, testable changes

[Design index](../index.md) · [Repository practices](14-repository-and-engineering-practices.md) · [Current gate](21-ea1-native-worker-results.md)

Make one logical change per commit: a reader should understand its purpose, validate it at that revision and revert it without unrelated edits. Order prerequisites before their consumers. Keep behavior-preserving organization/refactoring separate from functional changes, dependency upgrades and product-policy documents. Avoid microcommits that leave imports, generated bindings, tests or documentation broken between steps.

Before committing:

1. Inspect instructions, Git state and configured author/committer identity. Create commits only when authorized; never fabricate identity or alter global settings to get past a blocker.
2. Stage explicit paths or a reviewed temporary index. Inspect the complete staged diff, file modes and `git diff --cached --check`. Include changed contracts, generated outputs, meaningful tests and relevant documentation together. Check lock diffs and evidence provenance.
3. Run checks for the **staged revision**, using an isolated local tree if the working directory contains later work. Record exact commands, source/tree identity, environment, prerequisites and results. Never assume testing the final working tree validates an earlier commit.
4. Use a concise imperative subject; explain rationale, limits and known failures in the body when useful. Keep credentials, caches, build executables and temporary logs out. Retain intentional sanitized fixtures, durable result records and verified source/evidence archives; archives must contain no executables or private data.

`make check`, `make verify` and strict `make docs` cover foundation contracts, not a working terminal. Native workspace checks require the evaluated verified local artifact described in [EA1](21-ea1-native-worker-results.md). `make ea1-test` remains **FAIL** on three legacy control aliases; `make engine-gate` remains independently **FAIL** for original X4. Report these separately from passing foundation/docs/preservation checks. Do not waive them, hide nonzero exits, or advance EA2 because a foundation command passes.

For future authorized integration, keep a personal branch as a dependency-ordered series of self-contained commits. Rebase only your own unshared changes onto an agreed base when that history operation is authorized; preserve others' work and shared history. After conflict resolution, rerun relevant checks at every affected commit and refresh evidence bound to changed trees. This guidance authorizes no fetch, history rewrite, remote or publication. The initial local series is created without amending, resetting or rebasing anything.

Prefer a small tested module seam over speculative infrastructure. Rust, TS and generated schemas should have one owner per contract. A reviewer must be able to distinguish implemented behavior, experimental seams, known counterexamples and proposed design. Evidence and maintainability take precedence over an impressive demo or a misleading all-green badge.
