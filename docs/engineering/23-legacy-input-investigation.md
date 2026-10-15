# EA1 legacy-input investigation: completed, correction blocked

> Historical phase record. [ADR E4](../adr/0004-owned-agent-native-engine.md) chooses an [owned agent-native engine](../engine/index.md). Prior findings, failed gates and evidence remain unchanged; adoption-specific next-step recommendations below are superseded. They are not the new product acceptance definition.

Date: 2026-10-15. [Index](../index.md) · [Policy decision](../adr/0003-legacy-input-public-policy.md) · [Historical worker results](21-ea1-native-worker-results.md) · [Engine assessment](19-engine-build-adopt-redesign-assessment.md)

## Conclusion

**EA1 remains FAIL.** The current character-event construction correctly reaches an intentional upstream fixterms policy. The exact pinned public API cannot safely select the required classic-legacy correction while preserving all negotiated input behavior. No production adapter correction was made. The investigation is complete at this real public-API blocker; EA2 and broader application work remain stopped.

The public contracts and reproducible probes distinguish three issues:

1. **Intentional encoder policy:** Ctrl-I/M/[ are deliberately CSI-u in the default encoder. The existing C0 oracle is a retained product requirement, not a claim about what upstream promises.
2. **Missing policy seam:** Kitty flags are readable, but modifyOtherKeys state is not exposed by a typed public getter; there is no classic-legacy/fixterms toggle. Kitty flags alone cannot select a safe remapping policy.
3. **Incomplete event model:** ASCII physical-key guesses and a private logical-character DTO are not full keyboard/layout/IME facts. Consumed Shift matters in Kitty mode. These limits cannot be hidden behind named-key success.

The five previous local commits, historical evidence, original X4 oracle/fixtures/result, pinned native worker sources, protocol/schema and lockfiles are preserved. New evidence is under `evidence/ea1-input-investigation/`. `evidence/ea1-input-investigation/baseline.json` binds all 224 original tracked files to the previous HEAD. The diagnostic verifies 121 protected paths byte for byte and the original five-commit chain. The active evidence README is a documentation index, excluded from immutable payload checks.

## Primary sources actually inspected

All engine statements concern **Ghostty commit `22fa134a07f5749edd9e92c6bc17d8aaf61843be`**, not a moving SDK/release claim. Read the installed 35 public-header family and matching official source locally; verify hashes with the existing artifact pin/provenance check. Opened official pinned header URLs and protocol references on 2026-10-15. No upstream contact, dependency source edit or new dependency occurred.

| Source | Finding used |
| --- | --- |
| [Pinned key event header](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/key/event.h) | Physical keys are layout independent; generated text is separate, before Ctrl/Meta transformation; do not pass transformed C0 text. Text lifetime stays with caller. Actions and consumed modifiers have public setters. |
| [Pinned encoder header](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/key/encoder.h) | Eight options and canonical `setopt_from_terminal`; no classic-legacy toggle or encoder getter. Native press/repeat/release support depends on protocol flags. |
| [Pinned terminal header](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/terminal.h), [modes](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/modes.h) | Kitty flags and ANSI/DEC modes are readable; no typed MOK state getter. MOK is not ANSI insert mode 4 or a DEC mode. |
| [Pinned encoder implementation and tests](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/src/input/key_encode.zig), [C wrapper](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/src/terminal/c/key_encode.zig) | `legacy` includes fixterms and MOK. The three aliases are deliberately excluded from C0 translation and explicitly tested as CSI-u. Terminal-derived options retain MOK/Kitty precedence. Inspection is evidence, not permission to call private functions. |
| [fixterms](https://www.leonerd.org.uk/hacks/fixterms/), [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/), [xterm controls](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) | fixterms deliberately disambiguates these aliases; Kitty documents a classic legacy mapping plus negotiated distinctions; xterm documents MOK controls. Product compatibility must choose a profile rather than conflate these protocols. |

## Reproducible proof and behavior

`tests/ea1/input_contract.c` builds against the exact pinned public headers/library with Zig CC, C11 and warnings as errors. It creates its own terminal/encoder/event, feeds only synthetic sequences, supplies explicit event fields and records bytes, public Kitty flags and parser-ground state. It has no PTY/reply callback or worker linkage. `tests/ea1/input_investigation.py` verifies artifact hashes, invokes the probe and the existing Rust worker, checks hand-authored byte oracles separately from same-core consistency, exercises transitions/refusals and verifies preservation. Native cases never select modes based on emitted bytes.

Final diagnostic attempt: **891 native cases, 839 worker encodings, 913 assertions**; selected contract/consistency checks PASS, product selection FAIL, correction BLOCKED_PUBLIC_SEAM. `evidence/ea1-input-investigation/attempt-6/input-contract.json` and `evidence/ea1-input-investigation/attempt-6/commands/input-contract-1.json` retain arguments, input hashes and outcomes. Mode coverage includes Kitty 1/3/31; MOK 1/2; nested Kitty push/pop and precedence over MOK; MOK disable; RIS; every byte cut of nine negotiation transitions, including incomplete CSI. These are selected cases, not complete Kitty/MOK conformance.

`CSI` below means `ESC [`; values are bytes, not PTY delivery receipts.

| Input / construction | Ordinary pinned default | Kitty disambiguation (1) | MOK 2 |
| --- | --- | --- | --- |
| Logical Ctrl-I | `CSI 105;5u` **required `09`: FAIL** | `CSI 105;5u` | `CSI 27;5;105~` |
| Logical Ctrl-M | `CSI 109;5u` **required `0d`: FAIL** | `CSI 109;5u` | `CSI 27;5;109~` |
| Logical Ctrl-[ | `CSI 91;5u` **required `1b`: FAIL** | `CSI 91;5u` | `CSI 27;5;91~` |
| Actual unmodified Tab / Enter / Escape | `09` / `0d` / `1b` | `09` / `0d` / `CSI 27u` | `09` / `0d` / `1b` |
| Logical Ctrl-C | `03` | `CSI 99;5u` | `CSI 27;5;99~` |
| Logical Alt-I | `ESC i` | `CSI 105;3u` | `CSI 27;3;105~` |
| Ctrl+Shift+I, text `I`, Shift unconsumed | `CSI 105;6u` | `CSI 105;6u` | `CSI 27;6;73~` |
| Ctrl+Alt+I | `CSI 105;7u` | `CSI 105;7u` | `CSI 27;7;105~` |
| Unmodified Unicode `Text` | UTF-8 | UTF-8 | UTF-8 |

Named-key defaults and modifiers also run through the C oracle for 10 shared named keys × eight modifier combinations × six mode setups. Separate Rust/JS tests assert both existing synthetic human normalizers produce those same intents. The worker encodes the identical intents used by agents; this tests a shared contract, **not live host/desktop parity**. Both normalizers currently refuse character chords, so no human Ctrl-I/M/[ character decoding or physical layout claim follows. A legacy host often cannot distinguish Ctrl-I from Tab in the first place.

Input-construction counterexamples:

- Physical ASCII key versus unidentified key plus the same printable text: same failing legacy alias. Physical identity is not the missing switch.
- Physical key with no text: empty legacy output for these aliases; negotiated Kitty can still use the logical codepoint. Dropping text loses the requested action.
- Marking Ctrl as consumed: does not fix legacy, and can turn negotiated Ctrl-I into plain `i`. The OS did not consume Ctrl; inventing that fact is invalid.
- Always normalizing to unmodified Tab: gets `09` in default and Kitty mode, losing negotiated Ctrl-I identity. This probe is a rejected counterexample, not an adapter path.
- In Kitty mode, native unconsumed Shift-I emits `CSI 105;2u`; consumed Shift-I emits `I`. The private DTO cannot describe that distinction. It is an explicit logical profile, not an IME/physical keyboard interface.

Native release is empty without report-events; native repeat remains press-like there. With Kitty flags 3, Ctrl-I release/repeat are `CSI 105;5:3u` / `CSI 105;5:2u`. The worker DTO exposes **press only**: adding action fields yields an uncorrelated `malformed` response with `seq=0`, no input bytes. TUI rejects release/repeat; desktop synthetic normalizer consumes keydown only and refuses repeat/composition/meta. No physical-key repeat or release support has been added.

Control characters in `Character`, modified non-ASCII characters and unknown interpreter state are refused in the tested boundaries. Ordinary Unicode/combining text and supplementary scalars are exercised. Under Kitty report-all, the existing `Text` path produces scalar-key encodings, not a general committed-text/IME operation. Unknown layouts, consumed modifiers, composing keys and unavailable host capabilities require a future explicitly negotiated profile/refusal; the present worker must not be admitted as if that frontend capability existed.

## Verification and provenance

Run from repository root with existing project tools. EA1 Make targets now default to a fresh `evidence/ea1-runs/<unique-id>` shared by that invocation. A fresh descriptive `EA1_EVIDENCE_DIR` can select another attempt. The historical `evidence/ea1` destination and paths outside project evidence are refused. The diagnostic report is exclusive-create; reusing a directory can refresh other phase reports, so do not reuse completed directories. Command attempts retain their numbered append-only logs. Direct Python harness calls require an explicit destination.

```sh
EA1_EVIDENCE_DIR=evidence/ea1-input-investigation/attempt-6 make ea1-input-investigation
EA1_EVIDENCE_DIR=evidence/ea1-input-investigation/final-verification make ea1-ownership ea1-provenance ea1-preservation ea1-test
make check test stress provenance docs
```

The first command above is the **recorded completed attempt**, not a directory to overwrite. Select a new suffix to reproduce. The raw `ea1-test` must remain nonzero; its unchanged C0 assertions are not converted to expected-failure certification. Final check selection and exact commands are collected in `evidence/ea1-input-investigation/verification-summary.json`. Original X4 is rerun through its unchanged exported oracle into new command evidence, leaving its original result untouched.

| Actual command / check | Result |
| --- | --- |
| `make ea1-input-investigation`, fresh attempt 6 | Diagnostic PASS; 891 native cases / 839 worker encodings / 913 assertions; selection FAIL |
| `make ea1-test`, fresh verification destination | **FAIL**, Make exit 2 / unittest exit 1; 13 methods, 12 pass, three alias failures in the remaining method |
| `make ea1-ownership ea1-provenance ea1-preservation` | PASS; owned lifetime, Send/Sync refusal; 35 headers/compiler/library; 660-line adapter; 139 archived members and 37 original paths |
| `make check` | PASS after formatting repair; rustfmt, Clippy warnings denied, Prettier, bindings drift, TS build, acyclic boundary and original fixture integrity checks |
| `make test` | PASS; 22 Rust tests, 14 JS tests, including both new shared named-input/refusal suites |
| `make stress provenance` | PASS bounded; 192 seeded partitions; registry/source-family verification; fresh provenance report retained and original recovered |
| Updated `make provenance` | PASS bounded; exclusive new report, original timestamped report preserved |
| Original X4 exported `run()` with unchanged require-fail exit policy | **FAIL**, exit 1; original three complete-boundary counterexamples retained; partial CSI also fails separately |
| `make docs` | PASS; local links and strict MkDocs; final command and coverage recorded in verification summary |

Not run: live shell/TUI/editor/REPL/agent CLI or physical keyboard/IME tests; real renderer/PTY, accessibility, performance, runtime isolation, allocator/sanitizer campaign, cross-version recovery or production platform qualification. CI was not run on a provider. No legacy behavior fix is claimed.

Artifact: Ghostty SONAME 0.1.0 development C API at the commit above; dynamic library SHA-256 `7233966bb39275b8445a9a8b970170ef6fd109b8b8feb8b164cce00be40d588a`. Existing pin verifies the compiler, library and all 35 headers. Rust/Cargo 1.91.0, Zig 0.16.0 / Clang 21.1.0, Node 24.21.0, pnpm 10.32.1, Python 3.10.12, MkDocs 1.6.1; Ubuntu 22.04.5 x86_64/glibc 2.35. No dependency/lockfile update. This is the previous verified local build, not SDK portability, release ABI or reproducible-build certification.

Early diagnostic attempts remain append-only: attempt 1 incorrectly treated unconsumed Shift-I as ordinary typed text; the public consumed-modifier distinction repaired that oracle and remains an explicit capability gap. Attempt 2 used a valid-request sequence helper on a deliberately malformed request; attempt 3 confused the overflow-triggering native error with the subsequent latched `unknown` refusal. Attempt 4 passed before the evidence-routing safeguards were added. Attempt 5 then treated the active evidence README as an immutable result; only that documentation index was excluded. No production encoding, original C0 oracle or selection requirement changed to make these checks pass.

The existing xterm provenance command initially rewrote a timestamped `evidence/xterm-artifacts.json`. Its fresh output was retained as `verification/xterm-artifacts-rerun.json`, and the historical file was recovered byte for byte from the baseline Git object after checking the baseline hash and preserving the fresh version. No user work was discarded. The script now exclusive-creates `evidence/provenance-runs/<unique-id>.json` and prints that path; a new run verifies the change. Final preservation runs after this recovery. No historical evidence remains refreshed in place. MkDocs initially rejected six links to source/evidence outside its documentation tree; those are now explicit repository paths, preserving strict link validation without copying evidence into the site.

## Adversarial review and stopping point

Rechecked that passing same-core comparisons are not independent compatibility evidence; fixed expectations for actual C0/Kitty/MOK cases remain separate. Checked that the report distinguishes intentional behavior, event-fact gaps and missing public state. Rejected hidden mode tracking, private internals, arbitrary substitutions, advanced-mode suppression and a second parser. No byte stream becomes authority to dispatch or grant input. New evidence routing is confined to project `evidence/`; original logs and the five commits are preserved.

Residual architecture debt is **unimplemented capability**, not a new shim: complete keyboard/layout/consumed-modifier/commit semantics; accurate host capability negotiation; release/repeat API design; complete MOK/Kitty conformance; deployment artifact qualification. Native adapter size stays 660 lines, unsafe stays confined, no upstream patch/private entry/dependency is added. A larger event DTO alone would not expose missing terminal negotiation state.

**Next recommendation:** keep the candidate unadopted. Evaluate a public compatibility-policy seam at an explicitly selected SDK revision, retaining this corpus and the unchanged raw legacy gate. Require classic aliases, preserved extended behavior, explicit event capability/refusal semantics and unchanged single reply authority before considering integration. This is a new bounded EA1 decision, not permission to begin EA2, switch engines or weaken requirements. No renderer, PTY, accessibility, performance, security isolation or cross-version recovery claim follows this investigation.
