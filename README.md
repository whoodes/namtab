# Project X

**Chosen direction: a project-owned Rust agent-native terminal engine with compatibility for unmodified Linux shells, CLIs and TUIs.** Read the [research campaign and executive scope](docs/engine/index.md), [ownership ADR E4](docs/adr/0004-owned-agent-native-engine.md), and [manifesto](docs/product/manifesto.md).

The host-terminal TUI and Electron desktop are both first-class, including ordinary use with zero agents. **No working owned engine or live application exists.** Existing foundation checks pass at their recorded revisions; historical xterm X4 and libghostty EA1 gates remain **FAIL**. Their evidence is preserved and their adoption roadmap is superseded.

Read the [design guide and authoritative document map](docs/index.md), then [human UX](docs/ux/11-human-experience-spec.md), [native contract](docs/engine/native-contract.md), [core/runtime design](docs/engine/runtime-persistence-security.md) and [N0–N5 delivery plan](docs/engine/delivery-and-validation.md). [Repository practices](docs/engineering/14-repository-and-engineering-practices.md) distinguish the actual seven Rust/four TS foundation from the proposed owned-core package; neither full frontend is implemented. [Contributor workflow](docs/engineering/contributing.md) defines logical, testable local commits.

Existing local prerequisites are Python 3.10/venv, Rust/Cargo 1.91.0 with rustfmt/Clippy, pinned project Node 24.21.0/pnpm 10.32.1, MkDocs 1.6.1 and the verified native artifact needed by the **historical** worker build. This VM is Ubuntu 22.04.5 x86_64, not the shipping-platform qualification. The consolidation changes documentation only. Earlier bootstrap/artifact provenance remains in [foundation](docs/engineering/18-foundation-gate-results.md) and [EA1](docs/engineering/21-ea1-native-worker-results.md).

```sh
make check test stress docs
```

These are foundation/docs checks, not new-engine certification. `make ea1-test` and `make engine-gate` remain raw nonzero historical gates. New EA1 Make invocations write fresh `evidence/ea1-runs/<unique-id>`; preservation checks inspect frozen archives/hashes without refreshing original reports. `make verify` refreshes the older phase's `evidence/checks.json`; preserve it before deliberately running that workflow. No CI-provider run or all-green product claim follows.

Research/check evidence is under `evidence/owned-engine-research`; consolidation checks are under `evidence/docs-consolidation`; historical phases remain in the [evidence map](evidence/README.md). The [initial local history record](docs/engineering/22-local-history-and-validation.md) describes the first series; use Git for current state. Local commits are authorized, publication/remotes are not. **Apache-2.0 remains a recommendation; no project license is adopted.**
