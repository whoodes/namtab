# Workspace organization and cleanup

**Later cleanup, 2026-10-20:** [issue61](https://github.com/whoodes/namtab/issues/61) removes retired Electron app source and dedicated plumbing. [Repository practices](14-repository-and-engineering-practices.md) owns the current eight-Rust/three-TS layout; the dated inventory below is preserved as history. Synthetic input assertions survive as test-only support, and Electron reproduction uses the [pinned pre-cleanup revision](https://github.com/whoodes/namtab/tree/eb7bbb2ddcecac646bcf4f8a45eac13d8af57bf0). Historical evidence and required notice records are unchanged.

Date: 2026-10-15. Cleanup precedes EA1; it does not change the assessed engine or locks. [Design index](../index.md) · [Assessment](19-engine-build-adopt-redesign-assessment.md)

## Before and after

Inspected all **143 reviewable paths**, root/ancestor instructions (none present), build commands, module boundaries, docs navigation and evidence layout. Git has no commits, staging or remotes; making Git status empty would require actions outside this cleanup. The objective is an organized, verifiable working tree, preserving all work.

Preserved 139 non-archive files in `evidence/archive/pre-ea1-20261015T173427Z.tar.gz` with an adjacent member/digest manifest. The four earlier archive files are retained and separately hashed in that manifest. This includes prior check logs, the prior inventory and both lockfiles. Cargo lock SHA-256: `15fd6ca90a15ff8ba1f4579fd3b019db1186ba7c5bccd74d764d508b505c3fac`; pnpm lock: `448688b057b8276aa826d0d76aa5b1d73e088f8131ff20128701081bdb149ba4`.

| Concrete issue | Organization change | Preservation |
| --- | --- | --- |
| Assessment-only tools mixed with general developer tooling | Move bootstrap and preservation checker into `scripts/assessment/` | Old paths/bytes/commands remain in checkpoint and historical logs |
| Recorder named for one assessment despite use for general checks | Rename to `scripts/record-command.py`; allow explicit evidence directory and extra input hashes | Default directory and append-only attempt numbering unchanged |
| Root Makefile mixes foundation and experimental native builds | Extract assessment targets into `make/assessment.mk` | Target names, pinned build flags and fixture paths unchanged |
| Reports/history/current source are easy to confuse | Add evidence index and this organization record | No divergent active spec copies or edits to historical JSON logs |

The cleanup leaves 152 reviewable paths, including the checkpoint, command logs and result. Three scripts were renamed; there was no source/package/dependency migration. The machine-readable result records the full before/after inventory. No unrelated formatting, caches removal, Git cleanup/reset or staging is necessary. Existing ignores already cover tools, caches, binaries, generated build output and secret patterns; reviewable generated schemas/bindings and locks remain intentionally visible.

## Working rules

- `docs/` is authoritative design, ADR and result documentation. Numbered phase reports retain their dated verdicts; newer reports reference rather than duplicate them.
- `crates/` contains the foundation and, after cleanup, the single explicitly experimental engine binary. `apps/engine-host`, `packages/terminal-adapter` and `tests/assessment` remain xterm/native assessment references, **not shipping frontends**. The desktop source is synthetic input normalization, not an Electron app.
- `tests/fixtures` contains sanitized shared inputs. The original X4 oracle and fixtures keep their exact paths/bytes. `tests/system` and `tests/fuzz` are foundation checks; assessment/EA1 harnesses remain explicitly labeled experiments.
- `scripts/` provides shared developer checks; `scripts/assessment` is assessment-specific tooling. `make/` holds only cohesive experimental command groups; no framework of empty task modules.
- `evidence/` is reviewable sanitized provenance/results. `evidence/archive` is immutable history, never an alternative active spec. `.cache`, `.tools`, `.artifacts`, `target`, `dist` and `node_modules` are disposable local material; no live user data is a fixture.

Before EA1: strict MkDocs/local links, foundation format/lint/types/tests/partitions/boundaries, archived/original fixture preservation and lock hash equality must pass. See `evidence/cleanup.json` for exact commands/results. Original X4 remains independently FAIL; foundation success does not certify it.
