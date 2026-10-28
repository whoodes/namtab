# namtab

An AI-first terminal engine and native harness for attributed work, explicit coordination and targeted human control. This is a runnable, bounded Ubuntu prototype—not a production terminal or a fleet-scale benchmark. Basic shell use needs no model account.

**[Use namtab](docs/users/index.md)** · **[Agent/developer guide](docs/agents/index.md)** · **[Architecture](docs/architecture/index.md)** · [Published docs](https://whoodes.github.io/namtab/)

## Try it

From a prepared Ubuntu/X11 source checkout:

```sh
make mvp
```

Choose Terminal and Take input, then run a command. Work shows actual task/attempt origins and selected output; keyboard selection, exact terminal takeover and confirmed single-attempt cancellation are implemented. Closing detaches; use the printed exact run path for status/attach/stop. [Prerequisites, controls, agent configuration and troubleshooting](docs/users/index.md) are maintained in one user guide.

SCRIPTED DEMO uses real protocol clients, not models. Only agent actions routed through Namtab are mediated. The retained live Codex attempt was blocked before task/run execution; real-model mediation remains unvalidated. Two lifetime PTYs/two attempts/four live connections are limits, not dozens-agent qualification.

## Architecture

The Rust engine owns canonical terminal semantics, execution, attribution, ownership and receipts. The native harness consumes versioned public contracts for work navigation, interaction and targeted control; it does not duplicate parser/policy/private persistence. The host TUI remains diagnostic; Electron is removed. [Implemented flow and remaining design](docs/architecture/index.md); [direction and roadmap](docs/engine/delivery-and-validation.md#current-harness-milestones).

## Development

Exercised tuple: Ubuntu22.04 x86_64, Rust/Cargo1.91.0, Python3.10/venv, Node24.21.0, pnpm10.32.1, MkDocs1.6.1. With existing Rust/build prerequisites:

```sh
make bootstrap docs-install install
make foundation-verify
```

These prepare pinned project-local dependencies and run active-source checks, not a product installer or GUI/model certification. [Agent guide](docs/agents/index.md) owns the source map/workflow links; [contributing](docs/engineering/contributing.md) owns issue/commit/review requirements. Optional historical full-workspace artifacts and intentional X4/EA1 FAILs remain [separate](docs/engineering/16-test-and-ci-strategy.md#ci-artifact-diagnosis-and-lane-separation).

## Roadmap and documentation

Read the [three audience guides and document owners](docs/index.md), [manifesto](docs/product/manifesto.md) and [remaining H0 criteria](docs/engine/h0-residual-criteria.md). Wider compatibility, scheduling/capacity, final rendering/accessibility and durable recovery remain gated. Internal project-x package/profile names are retained; documentation is not authorization to implement or publish.

## Security and licensing

This prototype is not a sandbox or a safe place for credentials/private sessions. Same-UID roles are declarations; capture-off is not secret-safe storage. No perfect compatibility, deterministic execution replay or all-side-effect termination is promised. No dedicated security-reporting/support policy is published.

Original namtab material is [Apache-2.0](LICENSE). Upstream material retains its terms; see [license scope and full notices](docs/licensing.md). This is not blanket binary redistribution clearance.
