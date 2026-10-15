# Project X

Read the [contributor workflow](docs/engineering/contributing.md) for complete, testable changes.


A local engineering foundation for an agent-first terminal with two initial clients: host-terminal TUI and Electron desktop. **This is not a working terminal application. The completed engine assessment recommends official libghostty-vt plus owned semantic/cell rendering for the next prototype. The original serializer X4 remains failed; no dependent product implementation has begun.**

Read the [completed BUILD/ADOPT/REDESIGN assessment](docs/engineering/19-engine-build-adopt-redesign-assessment.md) and [ownership ADR](docs/adr/0001-engine-ownership-and-presentation.md) for pass/fail/not-run evidence, limits and the remaining prototype gates. The [design index](docs/index.md) links all 18 original documents, now canonical under `docs/`. No divergent root copies remain.

Workspace organization, experimental boundaries and preserved checkpoints are recorded in [cleanup](docs/engineering/20-workspace-organization.md) and the [evidence map](evidence/README.md).

The actual foundation contains:

- `crates/{protocol,domain,platform,service,client,console}`: private wire contract, pure receipt/fencing model, allocation validation, offline admission, cursor reconciliation and synthetic TUI/diagnostic code.
- `apps/{desktop,engine-host}` and `packages/{protocol,terminal-adapter}`: synthetic desktop input and headless observation seams, generated bindings/validator and bounded terminal probes. Electron/React and live TUI are not implemented.
- `tests/{fixtures,system,fuzz,assessment}`, `schemas/public`, `scripts`, `evidence`: shared sanitized inputs, executable regressions, generator/provenance checks and exact result records.
- `mkdocs.yml`, pinned docs requirements, Cargo/pnpm lockfiles, local developer commands and an authored CI workflow. No keeper/broker/PTY/runtime implementation or packaging scaffold is presented as complete.

Use existing Python **3.10** with venv, Rust **1.91.0** with rustfmt/Clippy, Git and make on Linux x86_64. Bootstrap installs only the recorded project-local Node **24.21.0** and pnpm **10.32.1** tools and verified source fixtures; registry dependencies/caches stay local. Other OS/Python tuples are not qualified. No globally installed pnpm or particular shell/theme is required.

```sh
make bootstrap
make docs-install install
make verify
make provenance
make engine-gate
```

`make verify` runs foundation checks and records the known failed gate separately in [checks.json](evidence/checks.json). Its success characterizes the counterexample; **it does not pass X4**. `make engine-gate` exits nonzero for that essential failure and is separately blocking in the authored CI. That workflow has not run on a provider. `make docs` builds strict MkDocs locally into ignored `.artifacts/site`, without a web server or publication.

Useful commands: `make check test stress`, `make format`, `cargo run --locked -p project-x-console -- diagnose` (through Make's project environment), and `node scripts/generate-bindings.mjs` (pinned Node/Cargo environment). The diagnostic uses a TestBackend, prints the blocked state and never enters raw mode or starts a process/PTY.

Git history is local; inspect `git status` and `git log` for current state. Dated evidence reports retain their original pre-commit Git snapshots. Apache-2.0 remains a recommendation; **no project license has been adopted**.

Reproduce the separate assessment with `make assessment-bootstrap assessment-build assessment-check` after the normal bootstrap. It uses official pinned Zig 0.16.0 and Ghostty source locally; no application/PTY/GUI is launched. New evidence is under [evidence/engine-assessment](evidence/engine-assessment/ghostty-assessment.json), with prior logs hash-archived under `evidence/archive`. Both A's canonical state model and B's public C snapshot proof pass their bounded assertions. They do not pass full rendering/accessibility, crash recovery, security, performance or upgrade gates. The original standalone `make engine-gate` remains nonzero.
