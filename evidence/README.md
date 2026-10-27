# Evidence map

All payloads are synthetic or public tool/source metadata. Never add credentials, live terminal captures or private session data here.

- `docs-consolidation/`: documentation-only audit after `a5fb6dd`; baseline hashes/history, per-document classification and fresh docs/preservation/consistency records. No runtime tests or research repeated; historical payloads remain unchanged.
- `owned-engine-research/`: the E4 research/design campaign; pre-edit 294-path/six-commit manifest, fresh command records and final verification/preservation inventory. New engine capabilities remain unimplemented. Prior X4/EA1 evidence is preserved and failing; the owner decision is documented in `docs/engine`, not redefined by these historical candidates.

- `archive/`: immutable earlier source/log checkpoints, each with SHA-256 member manifest. Pre-EA1 records preserve the 143-path starting inventory and unchanged lock hashes; dated historical paths are intentional.
- `engine-assessment/`: completed A/B/C decision evidence, public source/compiler provenance and append-only command attempts. These reports are historical; reruns add new attempts rather than rewriting command logs.
- `engine-continuation.json`: original independent failed xterm X4 result; keep visible and byte-identical with its oracle/fixtures.
- `ea1/`: historical native worker experiment, exact artifact pin and provenance, selection failures, bounded positive cases and append-only command attempts. `summary.json` selects that phase's final results; FAIL blocks that adoption path, not the new owned-engine N/NG definition.
- `ea1-input-investigation/`: additive followup to the historical EA1 reports. `baseline.json` binds the five prior commits and 224 original tracked paths; numbered attempts retain harness failures and the final public-contract proof. The diagnostic PASS characterizes a still-failing product selection gate; `verification-summary.json` selects current checks. No historical `ea1/` report is overwritten.
- `ea1-runs/<unique-id>/`: default destination for new EA1 Make invocations. One invocation shares an exported destination across its targets; a caller can select a fresh descriptive `EA1_EVIDENCE_DIR`. The old `evidence/ea1` destination and paths outside project evidence are refused. Do not reuse a completed attempt directory.
- `provenance-runs/<unique-id>.json`: exclusive new xterm artifact/source verification reports. The original `xterm-artifacts.json` remains historical; `make provenance` prints the fresh path.
- `cleanup.json`: pre-EA1 organization verification and inventory; not engine certification.
- `checks.json`: most recent recorded foundation verification at that phase; earlier versions are archived before refresh.
- `dependencies.json`, bootstrap/source records: phase-specific provenance, not a complete SBOM/security or license clearance.
- `file-inventory.txt`, `repository-status.txt`: dated phase-closing path/Git snapshots; use Git for current state. Caches, tools and build artifacts are ignored.

Current conclusions belong in [the docs index](../docs/index.md), not a second active design in evidence. Exact historical commands can reference subsequently moved scripts; [cleanup](../docs/engineering/20-workspace-organization.md) records the path mapping, and stable Make targets reproduce the workflow.

The [local history record](../docs/engineering/22-local-history-and-validation.md) and `history/validation.json` describe the initial commit series and its new isolated checks. Earlier EA0/cleanup/EA1 reports retain their original zero-commit snapshots and verdicts. Temporary validation logs stay in ignored `.artifacts/history`; only the bounded durable audit record is added.

## Native feedback window — 2026-10-27

`native-mvp/2026-10-27.json` is a bounded actual X11 journey observation, with binary/harness/screenshot hashes and explicit focus/latency limitations. The original eighth focus assertion remains FAIL in its ignored run; the retained record pins those original files rather than replacing it with a passing minimization result. New instrumented runs distinguish fake core focus notifications from actual XI2 loss between test-owned windows. This is not historical engine selection, full interface feasibility, autonomous model integration or production qualification.
