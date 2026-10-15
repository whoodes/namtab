# Project X

**Chosen direction: a project-owned Rust agent-native terminal engine with compatibility for unmodified Linux shells, CLIs and TUIs.** Read the [research campaign and executive scope](docs/engine/index.md), [ownership ADR E4](docs/adr/0004-owned-agent-native-engine.md), and [manifesto](docs/product/manifesto.md).

The native contract centers on identified tasks/attempts, typed operations and receipts, exact observations, artifact provenance and controllable human handoff. PTYs provide an explicit compatibility boundary; a legacy emulator or prompt guess will not become native authority. The host-terminal TUI and Electron desktop remain first-class, including excellent ordinary use with zero agents. [Sources and limits](docs/engine/research.md) distinguish current primary evidence from proposals; [milestones and validation](docs/engine/delivery-and-validation.md) define a useful initial slice before broad qualification.

**Current reality:** no working owned engine or full terminal application exists. This continuation is research/design only. Earlier xterm X4 and libghostty EA1 gates remain **FAIL**, with source, fixtures and historical evidence preserved. Their adoption-specific next steps are superseded; they are not the new product definition. No SDK patch, engine switch implementation, dependency installation or license adoption occurs here.

Start with:

- [Native agent/work/control contract](docs/engine/native-contract.md).
- [PTY compatibility, owned parser/state/input and feature profiles](docs/engine/compatibility-and-core.md).
- [Rust modules, scheduling, persistence and security](docs/engine/runtime-persistence-security.md).
- [Human/frontend contracts and concrete journeys](docs/engine/frontend-contracts.md).
- [Traceability, tests, staffing, risks and staged plan](docs/engine/delivery-and-validation.md).
- [Full design index](docs/index.md) and [contributor workflow](docs/engineering/contributing.md).

The actual foundation remains:

- `crates/{protocol,domain,platform,service,client,console}`: wire/domain/offline admission/client and synthetic TUI foundations.
- `crates/engine`: historical binary-only Rust/public-C libghostty experiment; no PTY/grant/dispatch authority, no persisted recovery; its alias gate still fails.
- `apps/{desktop,engine-host}`, `packages/{protocol,terminal-adapter}`: synthetic input, generated bindings and xterm reference probes. Live Electron/TUI are not implemented.
- `tests`, `schemas`, `scripts`, `evidence`: sanitized fixtures, checks, source/provenance and append-only phase records. `docs/` and strict `mkdocs.yml` own the design.

A future single `crates/terminal` pure-library/worker package is proposed, **not created**. The current workspace still has seven Rust/four TS packages. No private ABI/plugin ecosystem, new shell/OS or deterministic execution replay is promised. See the [module boundary](docs/engine/runtime-persistence-security.md#module-and-process-design).

Existing local prerequisites are Python 3.10/venv, Rust/Cargo 1.91.0 with rustfmt/Clippy, pinned project Node 24.21.0/pnpm 10.32.1, MkDocs 1.6.1 and the verified native artifact needed by the **historical** worker build. This VM is Ubuntu 22.04.5 x86_64, not the shipping-platform qualification. No tool installation was authorized or needed for this campaign. Earlier bootstrap/artifact provenance remains in [foundation](docs/engineering/18-foundation-gate-results.md) and [EA1](docs/engineering/21-ea1-native-worker-results.md).

```sh
make check test stress docs
```

These are foundation/docs checks, not new-engine certification. `make ea1-test` and `make engine-gate` remain raw nonzero historical gates. New EA1 Make invocations write fresh `evidence/ea1-runs/<unique-id>`; preservation verification calls the unchanged exported X4 oracle into fresh command records to avoid refreshing its original report. `make verify` refreshes the older phase's `evidence/checks.json`; preserve it before deliberately running that workflow. No CI-provider run or all-green product claim follows.

Research/check evidence is under `evidence/owned-engine-research`; historical phases remain in the [evidence map](evidence/README.md). The [initial local history record](docs/engineering/22-local-history-and-validation.md) describes the first series; use Git for current state. Local commits are authorized, publication/remotes are not. **Apache-2.0 remains a recommendation; no project license is adopted.**
