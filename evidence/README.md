# Evidence map

All payloads are synthetic or public tool/source metadata. Never add credentials, live terminal captures or private session data here.

- `archive/`: immutable earlier source/log checkpoints, each with SHA-256 member manifest. Pre-EA1 records preserve the 143-path starting inventory and unchanged lock hashes; dated historical paths are intentional.
- `engine-assessment/`: completed A/B/C decision evidence, public source/compiler provenance and append-only command attempts. These reports are historical; reruns add new attempts rather than rewriting command logs.
- `engine-continuation.json`: original independent failed xterm X4 result; keep visible and byte-identical with its oracle/fixtures.
- `ea1/`: current native worker experiment, exact artifact pin and provenance, selection failures, bounded positive cases and append-only command attempts. `summary.json` selects final results; FAIL remains a blocking result.
- `cleanup.json`: pre-EA1 organization verification and inventory; not engine certification.
- `checks.json`: most recent foundation verification. Earlier versions are archived before refresh.
- `dependencies.json`, bootstrap/source records: phase-specific provenance, not a complete SBOM/security or license clearance.
- `file-inventory.txt`, `repository-status.txt`: current reviewable path/Git snapshot; prior snapshots are archived. Caches, tools and build artifacts are ignored.

Current conclusions belong in [the docs index](../docs/index.md), not a second active design in evidence. Exact historical commands can reference subsequently moved scripts; [cleanup](../docs/engineering/20-workspace-organization.md) records the path mapping, and stable Make targets reproduce the workflow.
