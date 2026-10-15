# Local history and validation record

Date: 2026-10-15. [Index](../index.md) · [Contributor workflow](contributing.md) · [Manifesto](../product/manifesto.md) · [EA1 result](21-ea1-native-worker-results.md)

The existing configured Git author/committer identity was valid before any mutation. No identity was invented or changed, no existing history was rewritten, and no remote or publication was introduced. This session creates a local linear series from an initially unborn `main`; author dates are actual commit times, not reconstructed research dates.

## Dependency-ordered history

| Order | Commit | Boundary |
| --- | --- | --- |
| 1 | `0a0183b83ee1519063e799a328cb2c824399cd90` | Establish the verified foundation and engine assessment baseline |
| 2 | `b5e80723ecdbaf4a2c3beb471f9a4fcc1f681a2e` | Organize assessment tooling and document contribution boundaries |
| 3 | `a911b0e7316e01cd8d6d20af08785d945c2b0e6e` | Add the bounded native worker and retain the failed EA1 gate |
| 4 | `b8f8d3ff2fb3efd03da4e20702e6f4b158002bbe` | Define the terminal manifesto and responsiveness principles |

The fifth commit is the documentation/evidence audit containing this page. Its hash is discoverable with `git log -1 -- docs/engineering/22-local-history-and-validation.md`; embedding its own hash would be self-referential. The first four hashes and exact source-tree IDs, commands, environment and results are in `evidence/history/validation.json`.

The baseline uses the verified pre-EA1 checkpoint plus its four older archive files, with one README clarification that old Git snapshots are historical. The organization revision reconstructs the documented moves, Make extraction and cleanup evidence, then adds contributor rules applicable to its six-crate stage. It changes no application source or lockfile. The EA1 revision imports the completed worker unchanged, together with generated contracts/tests, provenance, failed gate and relevant docs. The manifesto revision changes only five documentation/navigation files; it introduces no new threshold or benchmark claim.

Original cleanup had 152 reviewable paths; its commit has 153 because the newly requested contributor policy is included. Original EA1 had 220; its commit has 221 with that policy. These are explicit reconstructed logical stages, not a claim that the earlier sessions made these commits. Dated inventories and zero-commit Git snapshots remain historical evidence.

## Per-stage checks and failed selections

Each exact temporary index was checked for whitespace, file modes/content and allowed paths, then materialized under ignored `.artifacts/history/`. Shared resources were existing verified tools/caches and the evaluated library; pnpm workspace links and source/build trees were isolated. Checks never ran an earlier revision against later application source. Generated validation results stayed in the isolated trees rather than overwriting the committed historical records. Raw temporary logs/indexes stay ignored; the durable result record includes exact argv/cwd, tree IDs, exits and output digests, plus failure details and environment.

| Stage | Passing checks | Essential gates retained |
| --- | --- | --- |
| Foundation/assessment | Offline frozen install; 14 foundation checks; 15 Rust tests, 12 JS tests, 192 partition trials; strict docs; A/B assessment/preservation | Original X4 **FAIL**, raw exit 1; no Rust worker yet |
| Organization | Same foundation, strict docs and assessment/preservation; source/locks unchanged | Original X4 **FAIL**, raw exit 1; no Rust worker yet |
| EA1 | Offline install; 14 foundation checks with 20 Rust tests, 12 JS tests, 192 partitions; public-seam ownership/provenance/preservation/shared-input checks; strict docs | X4 **FAIL**, raw exit 1; EA1 **FAIL**, raw Python exit 1, 12 passing methods and three failed alias subcases |
| Manifesto | Same unchanged implementation checks plus strict docs/navigation of the documentation-only tree | X4 and EA1 remain **FAIL** with the same raw exits/subcases |
| Audit record | Strict docs/links, YAML formatting, source boundaries, original evidence preservation and staged-diff checks; implementation bytes identical to validated manifesto stage | No gate resolution or implementation change; reuse the immediately preceding code validation |

Foundation command: `make verify` (includes format/Clippy, Rust properties/tests, generated drift, TS build, boundaries, fixture hashes, JS contracts, seeded partitions and strict docs). Independently ran `node tests/system/engine-probe.mjs --require-pass`, which exits **1** for X4. Stages 1/2 also ran `make assessment-check`. Stages 3/4 ran `make ea1-build`, `python3 -B tests/ea1/ownership_check.py`, `provenance.py`, `preservation.py`, `node tests/ea1/shared-inputs.mjs`, and the raw `python3 -B tests/ea1/worker_test.py`. That last command exits **1** on Ctrl-I/M/[; the Make wrapper would exit 2. Every stage ran `make docs` separately.

Matching an expected failing exit validates faithful reproduction of a counterexample. It does **not** make that selection gate PASS. No ignored exit or `continue-on-error` waiver changed a result. A known-red whole suite alone cannot classify new regressions for bisect; choose invariant-specific checks while continuing to report the unresolved selection failure.

## Scope, preservation and final state

All 62 implementation/schema/lock paths from the completed EA1 snapshot remain byte-identical. The original oracle, fixtures, original X4 result, immutable assessment records and three source/evidence archives are retained. Archive members were verified as UTF-8 text/source records, not executable binaries. No cache, generated build executable, credential file or transient validation log is committed. Historical sanitized command attempts are intentional durable evidence under repository policy, including failures; they are not a second active design.

No global/system configuration, license adoption, engine switch, key workaround, upstream contact, EA2 or broader product work occurred. No commits were amended/rebased/reset; temporary-index commits advanced `main` only with the expected parent. Index-only finalization never restored or overwrote working files. Actual final Git status and the fifth hash are reported after committing; use `git status --short` and `git log --oneline --reverse` to verify. Ignored validation trees/caches are local reproduction artifacts, not product source.

The native core remains the evaluated local build, with embedded original checkout paths. These checks establish neither a portable artifact nor a fresh CI build. Real frontends, accessibility, PTYs, runtime enforcement, durable recovery and end-to-end performance remain unrun/unqualified. The draft manifesto sets priorities; it does not waive those gates or change EA1/X4 verdicts.
