# namtab

An agent-native terminal engine for attributable work, useful terminal interaction, and human supervision that scales beyond a wall of panes. The chosen design owns terminal semantics in Rust while preserving ordinary Linux shells, CLI/TUI programs, and excellent use with zero agents.

**Maturity: first hands-on MK1, under final integration review.** Real local PTY/managed execution, an owned finite Rust terminal worker, a host TUI and sandboxed Electron client share one backend. Scripted agent clients can share immutable task contracts, bind exact revisions and obtain broker-attributed attempts/receipts; writer handoffs remain explicit. This is a first hands-on human-feel prototype, not full NG1 or a production terminal. Existing xterm/libghostty experiments remain historical references; their **X4 and EA1 selection gates remain FAIL**. No compatibility, security or speed superiority claim follows.

[Public documentation](https://whoodes.github.io/namtab/) · [Try MK1 locally](docs/engineering/mk1-quickstart.md): `make mk1-build`, then `make mk1-daemon`, `make mk1-tui` and `make mk1-desktop` in separate terminals on the prepared VM. The [scope/admission](docs/engineering/mk1-scope-and-admission.md) sets the stop-for-feedback boundary. No model or external service is needed.

## Architecture

- **Rust:** protocol/schema ownership, pure domain/receipt models, platform boundary, local supervisor/receipt journal, public client and Ratatui/Crossterm TUI; the original offline diagnostic remains available. The separate historical EA1 worker has no PTY or grant authority.
- **Owned engine, implemented finite subset:** `crates/terminal` owns bounded parser/state/input/reply proposals and a private worker. No history/reflow/checkpoint import or full profile qualification; see its [limits](crates/terminal/README.md). A legacy emulator is not native authority.
- **Human clients, MK1:** host TUI and sandboxed Electron views of the same canonical service model. Neither owns permissions, private storage, or application PTYs. The desktop MK1 runtime lives in `apps/desktop/mk1`; existing TS input/reference probes remain distinct.

See the [architecture and authoritative document map](docs/index.md), [engine scope](docs/engine/index.md), and [ownership ADR](docs/adr/0004-owned-agent-native-engine.md). Internal `project-x-*` package names and `px-*` experimental profiles remain to avoid an unrelated wire/package rename.

## Development

The exercised local tuple is Ubuntu 22.04 x86_64, Python 3.10/venv, Rust/Cargo **1.91.0** with rustfmt/Clippy, Node **24.21.0**, pnpm **10.32.1**, and MkDocs **1.6.1**. Future shipping Ubuntu targets are design goals, not supported-platform claims. Tool and dependency pins live in the toolchain file, manifests, and lockfiles.

For a new development checkout, these project-local setup targets acquire pinned tools/dependencies; they are not a product installer:

```sh
make bootstrap docs-install install
make foundation-verify
```

Rust must already be available through rustup/the selected toolchain. This active-source path needs no historical native artifact. For optional full-workspace reproduction, the native worker requires the [original reviewed EA1 artifact pin](docs/engineering/21-ea1-native-worker-results.md#results-and-reproduction):

```sh
make assessment-bootstrap assessment-build
make check test stress docs
make verify
```

Rebuilding the historical source can produce different library bytes and stop full-workspace checks before worker tests; do not refresh provenance hashes automatically. Cached original-artifact checks were exercised locally; a fresh native build/host is not thereby qualified.

These run the real launch build, isolated TUI/CLI and two-agent journeys, format/Clippy, Rust properties/unit tests, JS contracts/types, generated bindings/schema drift, boundaries, fixtures, seeded partition stress, and strict MkDocs/local links. `make verify` aggregates the full-workspace foundation checks, including the historical native worker in Cargo Clippy/tests; it does not run a separate historical qualification step. Reports go to fresh ignored `.artifacts/verification/` paths. `make foundation-verify` is the owner-approved **Active-source foundation** CI lane: it excludes only the historical native worker from Cargo Clippy/tests, checking all seven active Rust crates and the other foundation checks. Together with **PR issue and commit policy**, it is required for merge eligibility. Routine CI retains the X4 continuation regressions and evidence-preservation guards; historical X4/EA1 qualification jobs are retired from routine CI. This does not establish full-workspace, NG1 or production-runtime qualification. See [CI artifact policy](docs/engineering/16-test-and-ci-strategy.md#ci-artifact-diagnosis-and-lane-separation).

```sh
make engine-gate
make ea1-ownership ea1-provenance ea1-preservation ea1-test
```

These manual reproduction paths preserve the recorded failures: X4 cannot reconstruct complete continuation state; EA1 on the original artifact fails three required legacy aliases. Removing routine jobs does not change either recorded **FAIL**. Recorded ownership/provenance passes concern that original artifact; a fresh build can fail its byte-hash pin before those checks or alias behavior run. Preservation checks need no native build. Raw X4 reports go to ignored `.artifacts/x4-runs/`; EA1 run reports go to ignored `evidence/ea1-runs/<id>`. Original source, fixtures, archives and results stay unchanged. Do not invoke historical report-writing scripts directly or mistake an expected-failure regression check for certification. See [test/CI policy](docs/engineering/16-test-and-ci-strategy.md) and the [evidence map](evidence/README.md).

## Roadmap and documentation

Start with the [manifesto](docs/product/manifesto.md) and [human experience](docs/ux/11-human-experience-spec.md). The [MK1 admission and stop condition](docs/engineering/mk1-scope-and-admission.md) records accepted N0 design and authorized first hands-on work. The [N0 starting queue and exit criteria](docs/engine/delivery-and-validation.md#n0-starting-queue-and-readiness) lead to reviewed contracts/profile/fixtures, then a bounded native-and-shell slice in both clients. N2–N4 cover classic compatibility, reliable supervision and opt-in exact-tuple recovery; N5 is future backlog. N0 is accepted for MK1 design; full NG1 and later qualification remain unclaimed.

The [native contract](docs/engine/native-contract.md) distinguishes exact execution facts from opaque-shell/TUI inference. [Runtime/recovery](docs/engine/runtime-persistence-security.md) separates reconnect, engine loss and process loss. No universal compatibility, deterministic execution replay, perfect isolation or faster model inference is promised.

## Contributing

Follow the [contributor workflow](docs/engineering/contributing.md): logical, self-contained commits; relevant tests and documentation; explicit limits and preserved negative evidence. [Repository practices](docs/engineering/14-repository-and-engineering-practices.md) describe current versus proposed modules. Repository text and issue readiness do not authorize implementation or publication; follow the owner's current scope.

## Security and licensing

This is an unqualified prototype, not a sandbox for hostile programs or a safe place for credentials/private sessions. Local same-UID transport checks are not human attestation or protection from hostile same-UID programs. Restricted runtime isolation and durable secure capture remain unimplemented. No dedicated security reporting policy or support commitment is published in this repository.

**No project license has been adopted.** Apache-2.0 remains a recommendation, not a license grant. Dependencies and retained source/evidence have their own provenance and licensing obligations; see [dependency policy](docs/engineering/15-dependencies-and-supply-chain.md).
