# Project X

Read the [draft manifesto](docs/product/manifesto.md) for the north star and [contributor workflow](docs/engineering/contributing.md) for complete, testable changes.

A local engineering foundation for an agent-first terminal with host-terminal TUI and Electron desktop clients. **No working terminal application exists. Cleanup is complete; the bounded Rust/libghostty-vt EA1 worker is implemented, but its essential legacy control-alias input gate FAILS. Dependent app work is stopped. Original xterm X4 remains independently red.**

Read the [EA1 result and next decision](docs/engineering/21-ea1-native-worker-results.md), [completed BUILD/ADOPT/REDESIGN assessment](docs/engineering/19-engine-build-adopt-redesign-assessment.md) and [ownership ADR](docs/adr/0001-engine-ownership-and-presentation.md) for pass/fail/not-run evidence, limits and the remaining prototype gates. The [design index](docs/index.md) links all 18 original documents, now canonical under `docs/`. No divergent root copies remain.

The [completed legacy-input investigation](docs/engineering/23-legacy-input-investigation.md) confirms intentional fixterms behavior and an insufficient public compatibility-policy seam at the evaluated pin. [ADR E3](docs/adr/0003-legacy-input-public-policy.md) retains the veto: no unsafe remapping, mode inference or production correction was introduced. New append-only probes/refusal/transition tests preserve all prior evidence and five commits; EA1 and original X4 remain FAIL.

Workspace organization, experimental boundaries and preserved checkpoints are recorded in [cleanup](docs/engineering/20-workspace-organization.md) and the [evidence map](evidence/README.md).

The actual foundation contains:

- `crates/{protocol,domain,platform,service,client,console}`: private wire contract, pure receipt/fencing model, allocation validation, offline admission, cursor reconciliation and synthetic TUI/diagnostic code.
- `crates/engine`: experimental binary-only Rust/public-C worker; owned render frames, structured encoding and reply batches, no PTY/grant/dispatch authority or persisted recovery.
- `apps/{desktop,engine-host}` and `packages/{protocol,terminal-adapter}`: synthetic desktop input and headless observation seams, generated bindings/validator and bounded terminal probes. Electron/React and live TUI are not implemented.
- `tests/{fixtures,system,fuzz,assessment}`, `schemas/public`, `scripts`, `evidence`: shared sanitized inputs, executable regressions, generator/provenance checks and exact result records.
- `mkdocs.yml`, pinned docs requirements, Cargo/pnpm lockfiles, local developer commands and an authored CI workflow. No keeper/broker/PTY/runtime implementation or packaging scaffold is presented as complete.

Use existing Python **3.10** with venv, Rust **1.91.0** with rustfmt/Clippy, Git and make on Linux x86_64. Bootstrap installs only the recorded project-local Node **24.21.0** and pnpm **10.32.1** tools and verified source fixtures; registry dependencies/caches stay local. Other OS/Python tuples are not qualified. No globally installed pnpm or particular shell/theme is required.

```sh
make bootstrap
make docs-install install
make assessment-bootstrap assessment-build
make verify
make provenance
make engine-gate
```

`make verify` runs foundation checks and records the known failed gate separately in [checks.json](evidence/checks.json). Its success characterizes the counterexample; **it does not pass X4**. `make engine-gate` exits nonzero for that essential failure and is separately blocking in the authored CI. That workflow has not run on a provider. `make docs` builds strict MkDocs locally into ignored `.artifacts/site`, without a web server or publication.

Useful commands: `make check test stress`, `make format`, `cargo run --locked -p project-x-console -- diagnose` (through Make's project environment), and `node scripts/generate-bindings.mjs` (pinned Node/Cargo environment). The diagnostic uses a TestBackend, prints the blocked state and never enters raw mode or starts a process/PTY.

See the [local history and validation record](docs/engineering/22-local-history-and-validation.md) for commit boundaries and per-stage checks. Git history is local; inspect `git status` and `git log` for current state. Dated evidence reports retain their original pre-commit Git snapshots. Follow the contributor workflow before changing history. Apache-2.0 remains a recommendation; **no project license has been adopted**.

Reproduce the separate assessment with `make assessment-bootstrap assessment-build assessment-check` after the normal bootstrap. It uses official pinned Zig 0.16.0 and Ghostty source locally; no application/PTY/GUI is launched. New evidence is under [evidence/engine-assessment](evidence/engine-assessment/ghostty-assessment.json), with prior logs hash-archived under `evidence/archive`. Both A's canonical state model and B's public C snapshot proof pass their bounded assertions. They do not pass full rendering/accessibility, crash recovery, security, performance or upgrade gates. The original standalone `make engine-gate` remains nonzero.

EA1 commands: `make ea1-build ea1-ownership ea1-provenance`, then `make ea1-test` (**nonzero**: Ctrl-I/M/[ violate the required legacy C0 profile). Source is under `crates/engine` and `tests/ea1`; historical attempts/provenance/results stay under `evidence/ea1`. New invocations default to fresh `evidence/ea1-runs/<unique-id>`; set a fresh `EA1_EVIDENCE_DIR` to group related commands. `make ea1-input-investigation` verifies the diagnostic contract and still reports the selection FAIL. Named keys and representative mode/paste/focus/mouse/query behavior pass bounded synthetic assertions; no full frontend, crash recovery, sanitizer or performance qualification follows. Native builds currently require the recorded Linux x86_64 artifact/compiler/header tuple; another build's binary hash must not be silently adopted. CI has been authored, not run or qualified across runners. No new registry dependency/version was added; Cargo.lock adds the local worker member only, pnpm lock is unchanged.
