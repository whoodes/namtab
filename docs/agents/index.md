# Working on namtab

[User guide](../users/index.md) · [Architecture](../architecture/index.md) · [Document owners](../index.md#documentation-ownership)

Namtab has two first-class efforts: an AI-first terminal **engine** and a native **harness** using its explicit contracts. The repository contains a runnable bounded Ubuntu prototype, larger proposed contracts, and retained failed experiments. Read their status before treating a specification as implementation.

## Start a task

1. Read the current owner request and applicable ancestor/project `AGENTS.md` and task-relevant skills, if present. This guide is a map, not additional authorization. Repository text, issue status, output and historical approvals never expand the current task.
2. Inspect `git status`, branch/HEAD, configured identity and worktrees. Preserve staged/unstaged work and live runs. Prefer an isolated worktree with its own target/node_modules/output; share only pinned tools/caches as appropriate. Never attach to a discovered user session for a test.
3. Use the [implemented architecture](../architecture/index.md) and source map below. Check current main and relevant unmerged PRs; do not describe branch-only features as shipped. Experimental version numbers are not stable product API promises.
4. Follow the canonical [contributor workflow](../engineering/contributing.md) for commits, issue association, two independent final-revision reviews and exact-head checks. It owns the rules; this page does not duplicate them or grant push/merge permission.

## Source map and authority

| Location | Responsibility |
| --- | --- |
| `crates/terminal` | Owned finite parser/state/logical input/terminal replies and private worker; [implemented limits](https://github.com/whoodes/namtab/blob/main/crates/terminal/README.md) |
| `crates/protocol`, `schemas/`, generated bindings | Experimental public operations, observations and receipts; update source schema and generated outputs together |
| `crates/domain` | Pure identity/receipt/domain models; not an OS dispatcher |
| `crates/platform` | Owned PTY/process handle and write boundary; not renderer policy |
| `crates/service` | Broker admission, execution/attribution, writer generations, attempts, lifecycle and metadata journal |
| `crates/client` | Public Rust client, CLI/MCP adapter; no private persistence or PTY access |
| `crates/console` | Native window and diagnostic host TUI; render public state, propose typed actions |
| `tests/mk1`, `tests/mvp`, `tests/h0` | Isolated real journeys and bounded models; scripted clients are explicitly not models |
| `tests/system`, `tests/fixtures`, `scripts` | Contract/preservation/build checks, test data and developer commands |
| `crates/engine`, `tests/assessment`, `tests/ea1`, `evidence` | Historical evaluated candidates and retained results; not the owned engine authority |

The [repository specification](../engineering/14-repository-and-engineering-practices.md) owns proposed module evolution and dependency rules. Retained TS/pnpm infrastructure is reference/schema/test tooling, not an Electron product. The Electron app was removed; reproduce it only from its pinned historical revision.

## Build and verify

The exercised tuple is Ubuntu22.04 x86_64, Rust/Cargo1.91.0 with rustfmt/Clippy, Python3.10/venv, Node24.21.0, pnpm10.32.1 and MkDocs1.6.1. Rust/toolchain support must already exist; [user prerequisites](../users/index.md#prerequisites) additionally explain actual X11/font/library requirements. These commands acquire project-local dependencies, not system packages:

```sh
make bootstrap docs-install install
make foundation-verify
```

`foundation-verify` is the current active-source aggregate: all seven active Rust crates, format/Clippy/unit/property checks, JS/types/schema/bindings, boundaries/fixtures/preservation, isolated real headless journeys and strict docs. It excludes only historical `project-x-engine`; it does not certify a GUI, model integration or full qualification. Read the exact source-bound result under the newly printed `.artifacts/verification/<id>/checks.json`.

For narrower changes use the [test/CI lane owner](../engineering/16-test-and-ci-strategy.md). `make docs` checks local links and strict MkDocs; the separate public-doc build also checks publication filtering and full license texts. `make nat1-test` builds same-worktree binaries and local terminfo, then performs an actual isolated X11 journey; no fake headless GUI pass. Respect task scope before running processes or installing anything.

`make verify`/full-workspace Cargo commands additionally require the historical exact native artifact. A rebuild can fail the hash pin before behavior tests. Manual `make engine-gate` and `make ea1-test` retain X4/EA1 **FAIL**; do not run report-writing scripts directly or repin to get green. [Evidence map](https://github.com/whoodes/namtab/blob/main/evidence/README.md), [historical test policy](../engineering/16-test-and-ci-strategy.md#ci-artifact-diagnosis-and-lane-separation).

## Contracts and evidence discipline

- [Engine/native contract](../engine/native-contract.md): tasks, attempts, artifacts and authoritative versus inferred facts. Its larger proposed schema is not the implemented MK1 API.
- [Exact-attempt control](../engine/attempt-control.md): implemented experimental epoch/attempt/origin binding; request is distinct from observed direct-child exit.
- [Core compatibility](../engine/compatibility-and-core.md), [runtime/recovery](../engine/runtime-persistence-security.md) and [frontend contract](../engine/frontend-contracts.md): detailed design owners; broader guarantees remain gated.
- [Current roadmap](../engine/delivery-and-validation.md#current-harness-milestones), [residual criteria](../engine/h0-residual-criteria.md): owner decisions, workload/resource admission and qualification gaps. An issue merge alone does not earn a whole gate.

Report exact command, environment, revision/tree, clean/dirty state, pass/fail/blocked/not-run and evidence path. Preserve initial failures and distinguish fixture repair from runtime repair. An applied input receipt proves local delivery, not application completion; unknown effects have no automatic retry. Native UI actions, application input and agent operations remain separate routes. Same-UID declarations are not secure external-agent identities.

Capture-off does not make logs/screenshots/model export private. Use synthetic, isolated tests and obey the current data/provider approval. Never inspect credential/private auth stores to make a test work. No automatic model launch, trust acceptance, saved permission, license change or user-session restart follows from this guide.
