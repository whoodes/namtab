# Engineering foundation and engine gate results

> Historical phase record. [ADR E4](../adr/0004-owned-agent-native-engine.md) chooses an [owned agent-native engine](../engine/index.md). Prior findings, failed gates and evidence remain unchanged; adoption-specific next-step recommendations below are superseded. They are not the new product acceptance definition.

[Design index](../index.md) · [Prototype plan](12-prototype-plan.md) · [Engine vetoes](17-xterm-adapter-risk-and-gates.md) · [Repository target](14-repository-and-engineering-practices.md)

Date: 2026-10-15. **Historical foundation-phase ledger.** The completed follow-up [engine assessment](19-engine-build-adopt-redesign-assessment.md) now owns current recommendations and new evidence. Prior check logs are archived before refresh; original X4 is preserved. Counts and “current” descriptions below refer to this earlier foundation stopping point, not the later assessment file inventory. No project license was adopted.

## Executive decision

Stop dependent xterm replica/product development. The published 6.0.0 browser/headless pair is real and has bounded provenance evidence, but serialize 0.14.0 does not establish safe future interpretation. Saved cursor, scroll regions and custom tabs provide small complete-boundary counterexamples: initial screens match, subsequent application output diverges. An incomplete CSI also diverges and independently illustrates unsafe attachment boundaries.

This rejects the proposed serializer-based replica model, not every possible xterm integration. No complete bounded replacement for that state seam was established. Do not fix this by retaining unlimited output, enabling capture implicitly, resetting application state, adding a second authoritative parser, or turning the desktop into a text-only inspector. Do not claim this proves that all xterm architectures are impossible.

Recommended next decision: run one separately bounded libghostty-vt state/key/presentation feasibility assessment against these same fixtures and requirements, before building either live frontend. Alternatively, explicitly redesign the presentation model with credible bounded state-transfer evidence within the existing patch/private-API budget. Neither alternative is selected or proven here. No alternative engine was downloaded or tested.

## What actually exists

| Path from repository root | Implemented behavior | Explicit limit |
| --- | --- | --- |
| `docs/`, `mkdocs.yml` | Original 18 canonical design documents grouped by topic; this additional evidence page; strict navigation/anchor checking | No deployed site; build output is ignored |
| `evidence/docs-migration.json` | Original path→canonical path map and original/migration SHA-256 hashes | Migration hashes precede explicitly documented current-status edits |
| `crates/protocol` | Typed private request/intent/context, bounded complete-message JSON decoding, duplicate/unknown field rejection, decimal-u64 checks, draft-07 schema export | No streaming frame transport, authentication, capability negotiation, lossless-path contract or complete public API |
| `crates/domain` | Pure bounded in-memory receipt/fencing model: exact context comparison, ID binding conflict, one reservation, partial/full/unknown outcomes | Not a durable store; no expiry, approval/grant system, dependencies, scheduler or real effects |
| `crates/platform` | Checked geometry/allocation planning using domain types | No PTY, process groups, signal handling, filesystem confinement or runtime launch |
| `crates/service` | Offline validated wire→domain admission harness and SHA-256 request binding | No service/keeper executable, IPC, authority or dispatch; digest binds this prototype's Serde serialization, not a public canonical-JSON standard |
| `crates/client` | Shared wire decoding entry, cursor-gap/unknown reconciliation model | No IPC client, subscriptions or private persistence access |
| `crates/console` | Synthetic Crossterm normalization; prefix/paste route tests; Ratatui TestBackend diagnostic and JSON fixture command | No live raw mode, host event decoder loop, managed session or full TUI |
| `apps/desktop` | Synthetic desktop key-event→logical-intent normalization | No Electron/React installation, window, DOM, IME or browser rendering; deliberately withheld after X4 failure |
| `apps/engine-host` | In-memory headless observation utility | No live daemon, private IPC, launch authority or persistent capture |
| `packages/protocol`, `schemas/public` | Rust-generated JSON Schema, TS types, standalone Ajv validator, parsed-value semantic checks | TS does not decode duplicate-key wire JSON; no transport or private engine schema yet |
| `packages/terminal-adapter` | Feed-only headless probe, bounded feed/reply queues, detached cell/mode observations, resize, text/paste encoding | Not a certified engine adapter; structured key encoding fails explicitly instead of guessing; complete hidden state and all attributes are not exported |
| `tests/fixtures`, `tests/system`, `tests/fuzz` | Shared sanitized fixtures, cross-language synthetic checks, exact continuation reproducers, fixed-seed parser partitions | No real shell/editor/REPL/agent CLI, PTY, display or runtime isolation certification |
| `scripts/`, `Makefile`, `.github/workflows/checks.yml` | Local pinned bootstrap/check commands, dependency graph checks, generator drift and fixture checks; authored future CI with blocking X4 | Workflow has not run remotely. No publication/upload step, empty packaging hierarchy, general build orchestrator or empty cargo-fuzz packages |

Six Rust crates and four internal TS packages are real compile/ownership boundaries, not ten running services. Protocol/domain are independent leaves; service maps between them; console depends on client; only the terminal adapter declares xterm dependencies. No browser adapter entry is exported until its authority/presentation seam is proven. Static dependency checks verify actual manifest edges and selected source restrictions; they do not prove runtime containment or catch every possible dynamic import.

## Locked tools, dependencies and provenance

Use `make bootstrap` to place the exact recorded Node/pnpm tools and immutable xterm research sources locally. Existing tool directories are left unchanged; version checks catch the wrong selected runtime. `make install` uses the single pnpm lock, frozen resolution and disabled dependency scripts. Cargo uses the single application lock. Project caches/build output stay under `.tools`, `.cache`, `.venv-docs`, `target` and `.artifacts`. These are ignored; manifests, lockfiles, synthetic fixtures and curated evidence are reviewable.

| Tool/dependency | Actual selection | Evidence / boundary |
| --- | --- | --- |
| Rust/Cargo | 1.91.0, edition 2024, resolver 3, rust-version 1.91 | Existing installed compiler; pinned `rust-toolchain.toml`, `Cargo.lock` |
| Node | 24.21.0 Linux x86_64 | Official nodejs.org archive checked against SHA-256; URL/hash/index entry in `evidence/node-bootstrap.json`. Selected release predates the seven-day age boundary. Same-origin checksum is not independent signature/build verification |
| Bootstrap runtime | Existing Node 20.20.2; npm 10.8.2 version read only | Used for initial bootstrap/first synthetic runs; final checks use project-local Node 24.21.0; npm is not the workspace manager |
| pnpm | 10.32.1 | Registry tarball SHA-512 verified before extraction; `evidence/pnpm-bootstrap.json`; no lifecycle scripts used |
| Python/MkDocs | Python 3.10.12, MkDocs 1.6.1 | `requirements-docs.in` and full exact wheel/hash closure in `requirements-docs.txt`; closure currently targets Python 3.10/Linux x86_64. No external theme/plugins |
| Rust direct subset | serde 1.0.229, serde_json 1.0.154, schemars 1.2.3, sha2 0.10.9, ratatui 0.30.2, crossterm 0.29.0, proptest 1.12.0 | Complete closure/checksums in Cargo.lock; only needed features; no tokio/SQLite/Podman/PTY stack installed |
| TS/build subset | TypeScript 5.9.3, @types/node 20.19.17, Ajv 8.17.1, json-schema-to-typescript 15.0.4, Prettier 3.6.2 | Exact direct versions plus pnpm lock integrity. Conservative Node typing surface is intentionally smaller than runtime 24; no newer Node APIs used |
| xterm family | @xterm/xterm 6.0.0, @xterm/headless 6.0.0, @xterm/addon-serialize 0.14.0 | Same commit `f447274f430fd22513f6adbf9862d19524471c04`; actual registry tarballs re-fetched and SHA-512 verified; all 176 packaged installed files compared with the tarballs, 43 shared common source-map files compared across browser/headless, and selected InputHandler/Keyboard sources compared with the pinned upstream source. `evidence/xterm-artifacts.json` |

The old source-tree headless package manifest said 5.5.0; publication used 6.0.0 and embeds the release commit. No registry headless 5.5.0 substitution or private repackaging was needed. These checks establish a credible tested family, not a reproducible-build proof. The authored workflow pins actions/checkout v7.0.1 at commit 3d3c42e5aac5ba805825da76410c181273ba90b1, verified from its official release/ref and node24 action manifest; runner execution is untested. The browser CommonJS entry and headless `main` are used only as appropriate; a known-looking `module` field is not assumed valid. Optional WebGL/search/grapheme addons, Electron, React, Vite, Forge and runtime images are not admitted yet.

Primary sources: [release commit](https://github.com/xtermjs/xterm.js/commit/f447274f430fd22513f6adbf9862d19524471c04), [headless registry record](https://registry.npmjs.org/@xterm/headless/6.0.0), [browser registry record](https://registry.npmjs.org/@xterm/xterm/6.0.0), [serializer registry record](https://registry.npmjs.org/@xterm/addon-serialize/0.14.0), [Node checksums](https://nodejs.org/dist/v24.21.0/SHASUMS256.txt). Direct HTTPS downloads succeeded even where the browsing renderer could not open a checksum page.

`evidence/dependencies.json` records resolved license metadata and lock hashes, not legal clearance or a release SBOM. Rust build scripts/proc macros execute trusted registry dependency code during compilation; JS dependency lifecycle scripts were disabled. No full vulnerability advisory, transitive legal, bundled-runtime SBOM or reproducibility audit was performed. A release remains blocked on those reviews. Project Apache-2.0 is still only a recommendation; dependency licenses do not grant a project license.

## Reproduce and distinguish check results

Prerequisites used: existing Python 3.10 with venv, Rust 1.91 with rustfmt/Clippy, Git, make and network access to the recorded official registries/tool sources. No sudo, container setup or desktop/display dependency was used. This VM is **Ubuntu 22.04.5, Linux 6.8.0-138-generic, x86_64, glibc 2.35**. It is a development experiment host, not certification of the proposed Ubuntu 24.04/26.04 product baseline. Exact machine details, versions, commands, durations, stdout/stderr and exit codes are in `evidence/checks.json`; durations are not product latency measurements.

```sh
make bootstrap
make docs-install install
make verify          # independent foundation checks; records the red X4 separately
make provenance      # networked re-verification of official artifacts/source mapping
make engine-gate     # intentionally FAILS/nonzero for the current essential gate
```

`make verify` succeeds only when foundation checks pass and the known counterexample gate exits exactly 1; this is characterization of a failure, never a green X4. The authored CI runs `make engine-gate` as a separate blocking step and will remain red until an explicit reviewed engine decision changes the evidence. It is not hidden with continue-on-error, a skip or a renamed gate. Nothing was submitted to a CI provider.

| Check / exact component command | Result | What it establishes |
| --- | --- | --- |
| Pinned `prettier --check` over authored JS/TS/config | PASS | Source formatting; generated artifacts checked by generator; design prose preserved |
| `cargo fmt --all -- --check` | PASS | Rust formatting |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | Selected compiler/lint closure, not authority correctness |
| `PROPTEST_RNG_SEED=424242 PROPTEST_CASES=128 cargo test --workspace --locked` | PASS | Rust examples/unit/property checks; exact test counts in checks.json; bounded receipt/fence/shape models only |
| `node scripts/generate-bindings.mjs --check` | PASS | Committed schema/types/validator match locked generators |
| `pnpm build` through pinned local pnpm | PASS | Strict TS build for all four packages; generated validator copied to output |
| `node scripts/check-boundaries.mjs` | PASS | Six/four package manifests obey allowed acyclic edges and selected source bans |
| `node scripts/check-fixtures.mjs` | PASS | Two synthetic JSON payload hashes match provenance manifest |
| `node --test tests/system/foundation.test.mjs` | PASS | Twelve synthetic tests, including explicit known-failure assertions; not twelve product acceptance gates |
| `node tests/fuzz/chunk-corpus.mjs` | PASS | 192 deterministic partitions, seed 0x51a7e; no coverage-guided fuzzing claim |
| `cargo run --quiet --locked -p project-x-console -- diagnose` | PASS | Offline TestBackend diagnostic, no raw mode, subprocess or PTY |
| `.venv-docs/bin/python -m mkdocs build --strict` | PASS | Full documentation nav, local document links and heading anchors |
| `node scripts/verify-provenance.mjs` | PASS, bounded | Selected official registry/source artifacts; not full supply-chain clearance |
| `node tests/system/engine-probe.mjs --require-pass` | **FAIL, exit 1** | Unsafe serializer continuation for complete valid terminal states |

`evidence/checks.json` is the authoritative final run; the table describes its intended checks and must agree before stopping. Generated schema uses draft-07 and explicit bounded numeric constraints. Initial strict Ajv compilation caught Schemars' nonstandard uint8 annotation; the generator now verifies 0..255 bounds before removing that annotation, while rejecting other unexpected format drift. Adding the pinned formatter changed only generated union layout through the resolved generator formatter; that diff was inspected and the generated types regenerated. Version-1 and decimal-u64 semantics remain separately checked in Rust and TS. TS parsed-value validation is not a duplicate-key wire decoder.

## Essential gate ledger and counterexamples

| Gate | Verdict | Evidence and remaining obligation |
| --- | --- | --- |
| X1 provenance | **PASS, bounded first family** | Registry integrity/embedded commit/source-map checks, `make provenance`; optional addons and independent build verification absent |
| X2 keys/origins | **BLOCKED / partial tests pass** | Same normalized Up/Shift-Tab/Ctrl-F12 intents from synthetic Crossterm and desktop; upstream private helper handles normal/application arrows, Ctrl modifiers, Shift-Tab and F12. Helper has no application-keypad-mode parameter; keypad Enter returns CR. Full keypad/IME/mouse/legacy alias/origin certification unproven |
| X3 observation/replies | **Partial PASS; full gate NOT RUN** | Feed-only headless observations copy cells/cursor/modes at callbacks without UI. Two identical DSR queries generate two canonical replies; two headless stand-ins also generate replies that the harness does not dispatch. Real browser/client IPC authority and asynchronous origin cases remain untested |
| X4 attach/recovery | **FAIL — essential veto** | Same-family headless→serialize→headless necessary-condition tests fail saved cursor, scroll regions and custom tabs after equal screens. Plain text and simple alternate-buffer continuation pass; incomplete CSI fails. No complete checkpoint or safe generic attach established |
| X5 security/resources | **NOT RUN at product scale** | Local 16KiB write and 64-reply probe caps reject/mark unknown. No 32-session/all-process load, fairness, parser preemption, GPU, OS sandbox or independent-revocation certification |
| X6 maintenance | **NOT RUN / blocked** | Zero upstream patches or production private entry points adopted; adapter is far below 800 lines. No two-family upgrade rehearsal or cost measurement; no permission to normalize a semantics fork |
| PP1 overall | **FAIL / dependent product work stopped** | Minimal useful seams do not outweigh X4 failure. PP2–PP8 and G0–G8 product certification remain unpassed |

The continuation corpus lives once at `tests/fixtures/terminal/continuations.json`; `evidence/engine-continuation.json` records complete observations. Minimal saved-cursor reproducer: feed `ESC [ 2 ; 3 H`, `ESC 7`, `ESC [ 4 ; 7 H`; serialize/attach; feed `ESC 8`, `X`. The canonical X appears at row 2/column 3; the replica X appears at row 1/column 1 (human coordinates). This is a complete parser boundary, so waiting for a completed CSI alone cannot fix it.

The test uses a headless replica to falsify the serializer's state-transfer promise within the paired common parser family. It does **not** claim to have executed Electron/browser DOM. A desktop-specific workaround would need fresh evidence, including future-state equivalence, not just a screenshot. The [serializer source](https://raw.githubusercontent.com/xtermjs/xterm.js/f447274f430fd22513f6adbf9862d19524471c04/addons/addon-serialize/src/SerializeAddon.ts) serializes selected cells/modes and itself accesses `_core._inputHandler._curAttrData`; this is an upstream transitive private dependency, not a Project X production private graph traversal. HTML serialization additionally touches theme internals; that path was not used.

The key experiment extracts one pinned private evaluateKeyboardEvent function plus supporting enum/C0 constants into a disposable VM context using TypeScript transpilation. Sources are verified/cached; no upstream edits are carried. This is a research-only dependency on source shape, explicitly excluded from the production adapter. Shipping it would require private-seam/license/maintenance review, not an exemption from the two-entry/two-file budget. The production encoder returns UnsupportedCapability for structured keys and only demonstrates bounded committed text/paste; raw bytes are not a hidden replacement.

## Capture, recovery and untested surfaces

Synthetic test artifacts are deliberate test evidence. No ordinary user terminal output, passwords or provider secrets were captured. No disk spool, terminal checkpoint, capture-policy enforcement or crash-restoration implementation exists. Loss of canonical interpretation still means unknown state and disabled application input until valid authorized reconstruction; no implementation here can claim otherwise. No execution replay, exactly-once effects, process-group termination or cancellation rollback was promised or tested.

Real host TUI decoding, prefix timeouts, Escape/Alt ambiguity, enhanced keyboard negotiation, TERM/terminfo, fonts/IME/Orca, clipboard/password/approval prompts, SSH, browser human-origin separation, output floods, live PTYs, Bash/editors/REPLs, runtime isolation, disk-full faults and reconnect/crash transitions are **NOT RUN**, not inferred from a TestBackend or a synthetic event object. Browser and host accessibility remain independent gates. No application/package is ready to launch.

## Self-review and corrected issues

Second engineering review challenged authority, recovery and maintenance rather than only checking compilation:

- Caught equal-screen versus future-state confusion with complete-boundary fixtures; suspended the recommendation instead of blessing the serializer or weakening X4.
- Kept both initial frontends in scope. Desktop input-normalization tests and TUI TestBackend checks are useful shared-contract evidence, not substitutes for real frontend usability or accessibility.
- Bound receipt IDs to request digests and reject conflicting payload reuse. Copied accepted parser input before asynchronous interpretation to avoid caller mutation; snapshots also remain detached. Existing/unknown receipts never authorize another reservation; context/focus fencing remains exact. The in-memory model cannot survive a crash and is not advertised as a durable broker.
- Removed an unused no-op replica listener: an ignored callback does not enforce reply suppression. The real browser authority boundary remains unimplemented. Added cross-language Unicode rejection of unpaired surrogates to prevent lossy input replacement.
- Kept domain/protocol independent, moved the browser xterm dependency into the sole adapter package, and checked actual workspace DAGs. No unnecessary async/store/container stack, empty crate forest or second authoritative parser was added.
- Preserved strict Ajv behavior while explicitly normalizing one verified generator annotation; distinguished TS parsed-value validation from wire decoding. Fixed cursor overflow handling rather than allowing saturating cursor reuse.
- Checked archive paths/links before extraction. Initial overly strict checks safely rejected official archive root/relative-link entries; corrected confined path validation and verified checksums before execution. No system configuration or arbitrary install script was used.
- Distinguished fixture files and test evidence from product capture. No auto-log/transcript/persistent recovery feature was slipped into an output-memory-only design.
- Made the known failing gate independently nonzero and blocking in authored CI. Foundation-green and product-gate-red are reported separately; CI provider execution remains unverified.

Residual risks include xterm's incomplete state/key seam, experimental cell APIs, frontend and screen-reader feasibility, Node process fairness, missing actual runtime/PTY/receipt durability, supply-chain closure and maintenance cost. A changing upstream package version is not a reason to skip a locked-family rehearsal.

## Ordered next work, after the engine decision

1. Review these counterexamples and choose one bounded replacement/seam experiment. Success requires complete-state continuation, required keys/origins, immutable observations, one reply authority and budgeted maintenance for both frontends. Failure stops the candidate; do not build application UI around it.
2. Reuse these fixtures with the selected engine; add attribute/cursor/wrap/resize and hostile-input cases plus a real presentation test. Prove a bounded reattach strategy under capture off and honest canonical-loss behavior. Upgrade rehearsal remains mandatory.
3. Only after essential seams pass, add owned Linux PTY/keeper and real IPC with durable receipts, lifecycle faults and process groups. Then build both zero-agent terminal slices and certify actual outer-host/desktop accessibility/input profiles.
4. Verify restricted runtime admission, revision-bound approvals and artifact evidence before supervision claims; then measure full-process performance, package/provenance/license closure and human journeys.

Do not treat these later steps as completed or as authority to publish. The current stopping point is a reviewable local foundation and a reproducible essential failure.

## Final verification and inspection scope

Verified root: `/home/w/codex-dir/project-x`. Local Git is on main with zero commits, zero staged files and no remotes; all 96 reviewable source/config/design/evidence files are untracked and preserved. `evidence/file-inventory.txt` is the full inventory; `evidence/repository-status.txt` records the verified status. Ignored toolchains/caches/build output are intentionally outside that source inventory. No project LICENSE file exists.

The final code run recorded 14 passing foundation checks: **15 Rust tests, 12 synthetic JS tests, 192 seeded partition runs**, strict compilation/formatting, schema drift, boundaries, fixture integrity and documentation checks. Documentation-only final edits were rechecked separately in checks.json. There are **19 canonical docs pages** (18 relocated originals plus this result page), 22 Markdown files overall and 350 local links/fragments with zero errors. Strict MkDocs completed without warnings. The production terminal-adapter source is 135 lines; there are zero carried upstream patches and no adopted production private entry point. Research-only private helper extraction and upstream serializer internals remain explicitly accounted for above; these counts do not pass X6.

Inspection covered the 18 original Markdown sources during relocation/link/status review, with targeted architecture reads of 10, 12 and 14–17; current manifests/source/fixtures/generator outputs and actual check results; selected official registry tarballs, source maps and pinned upstream files. Applicable instruction checks covered `/AGENTS.md`, `/home/AGENTS.md`, `/home/w/AGENTS.md`, `/home/w/codex-dir/AGENTS.md` and the project; none were present. No credentials/private session storage, unrelated project content, live target/service scanning or bounty research was inspected or continued. No commit, remote, publication, upload, project-license adoption, system configuration or broader application implementation occurred.
