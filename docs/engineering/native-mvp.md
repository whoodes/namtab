# Native hands-on MVP increment

The subsequent [external-agent/lifecycle increment](external-agent-mvp.md) adds MCP tools, explicit headless status/stop and one reserved lifecycle identity within the same16total. Its15ordinary+1restricted namespace supersedes ordinary identity admission in new brokers only. The original PR66 results below remain dated evidence, not live-model proof.

## Experimental admission

The 2026-10-27 owner authorization admits one runnable feedback increment, its isolated tests and reviewed draft PR, **not merge**, a final renderer or full H0/H1/H2 acceptance. Based on main `dcd0c69246b8cddccd1daddfc239dd29625beb81` and the [native-window study](../ux/h0-interface-decision.md#bounded-native-stack-study-2026-10-20), this deliberately reduces graphical scope to a useful native Work/Terminal window. The diagnostic host TUI remains available; Electron stays removed.

- **Native tuple:** Rust1.91.0/Linux x86_64, winit=0.30.13 (`default-features=false`, `x11,rwh_06`), softbuffer=0.4.8 (`default-features=false`, `x11,x11-dlopen`), fontdue=0.9.3 (`default-features=false`). Public window/input/software-surface/rasterization APIs only, no private patch. CPU drawing avoids GPU/toolchain closure in this small increment; it is not a selection against the studied wgpu/COSMIC Text/AccessKit composition for a later product. No system packages, graphics/security settings or credentials are changed.
- **Text/profile:** system `/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf` read at runtime, not copied/distributed. Fixed Latin-1 monochrome canonical lines, cursor and trusted chrome; unsupported glyphs shown explicitly. No shaping/reflow, terminal SGR colors, clipboard/selection, IME, screen-reader/accessibility tree, Wayland/macOS/Windows or full TUI fidelity claim. IME disabled rather than duplicate ambiguous committed input. Existing owned engine encodes application input and generates replies; the window is never a parser/reply authority.
- **API/runtime:** existing `namtab.mk1/1`, advertised shared-task/attempt-origin capabilities and same-UID developer-host trust. No wire extension in original PR66 or bypass/private PTY access. The subsequent lifecycle operations are explicitly admitted separately above. Fresh private run directory/socket/workspace per explicit launch. Existing two-total-PTY/two-total-managed-attempt, four-live-connection and sixteen-identity limits remain. Captureoff; metadata journal/live bounded output are not durable terminal recovery. No sandbox/hostile-same-UID claim.
- **UI resources:** framebuffer at most1600×1200pixels, bounded Latin-1 glyph cache, one selected canonical activity, coalesced redraw requests spaced at least33ms apart and75ms observation polling (an approximate rendering target, not a bound on OS redraw events). Eight queued application intents and two reserved control commands; one RPC at a time off the GUI thread. Queue overflow, focus/session/view change or uncertain/refused input pauses application input and discards queued tails. Already submitted input can still have effects. Fresh explicit takeover re-arms only after current ownership is confirmed; no automatic input retry/reconnect/replay.
- **Human interaction:** Work is a factual task/attempt/result projection, with explicit literal argv entry; it is not an AI prompt. Terminal focus is distinct from writer ownership. Window closing detaches, leaving owned broker/shells/work running; launcher reports exact attach paths. It does not kill a user's live sessions. A separate scripted demo uses real public protocol clients and fixed synthetic commands, never labels them autonomous models. No paid model or new credential is required.

## Sources and dependency bounds

Official sources checked 2026-10-27: [winit0.30.13 manifest/API](https://github.com/rust-windowing/winit/tree/v0.30.13), [softbuffer0.4.8 manifest/examples](https://github.com/rust-windowing/softbuffer/tree/v0.4.8), [Fontdue upstream API/licenses](https://github.com/mooman219/fontdue). Winit uses Apache-2.0 with its DPI subpackage's own license, softbuffer MIT OR Apache-2.0, Fontdue Apache-2.0 OR MIT OR Zlib; registry metadata/full-text hashes for the three direct crates are recorded in the [dependency tuple](https://github.com/whoodes/namtab/blob/main/third_party/native-mvp-dependencies.json). The Linux build dependency graph is separately inventoried; that is not a release-specific license/notices audit. Fontdue rasterizes but does not shape text. Existing published-doc licenses remain separate. No binary release clearance follows; future distribution needs the complete selected notices and platform/font review.

## Launch and results

The bounded native window now has real VM journey evidence, described below. Broader H0 decisions remain open independently of this admitted narrow slice.

```sh
make mvp
```

Run from the repository root with the [development prerequisites](https://github.com/whoodes/namtab#development) and an accessible X11 `DISPLAY`, the existing X11/libxkbcommon runtime libraries, and the named system font. `make mvp-build` builds the same-lock native binaries without installing system packages. Missing display/font/library or surface initialization is a real unsupported-environment error, not a headless substitute. This VM exercised an actual native window on its existing display. Headless hosted CI compiles/tests this code but does not validate a GUI.

The launcher prints a fresh project-local `.artifacts/mvp-<id>/` workspace, broker PID, diagnostics and exact attach command. It never attaches to a discovered socket, replaces an existing directory, restarts an old broker or kills user work. A collision fails without changing the prior namespace. Closing the window **detaches**: broker, shell and managed work may continue. To open the same live namespace again, use its printed path:

```sh
python3 -B scripts/mvp.py attach --run .artifacts/mvp-PRINTED_ID
```

This creates a new human endpoint; existing ownership must be explicitly taken again. **Ctrl+Shift+C** inside a still-running window instead resumes that window's live endpoint/token/request counter, which exist only in memory. An expired/broken binding never falls back to a new identity or replays input. The sole retry is a HELLO that received the exact known-zero `endpoint_already_connected` refusal while the previous connection closes, within a 250ms retry window (each socket operation retains its separate timeout). This does not promise process/crash/durable recovery. Captureoff still retains bounded live observations and receipt metadata, not a secret-safe environment.

### Try the interaction

| Action | Interaction / honest result |
| --- | --- |
| Inspect work / terminal | Click **WORK** / **TERMINAL**, or Ctrl+Shift+1 /2. Work shows actual attempt exits, origins, stream previews and observed files; it does not invent agent reasoning. |
| Use a real shell | Select Terminal, **Take input** (Ctrl+Shift+T), type `echo hello`, Enter. Arrows/history, Tab, common navigation/function keys and Ctrl+C use the broker's mode-aware encoder. Ctrl+C need not terminate a program. |
| Pause / hand off | Focus/view/session/resize changes pause input. Ctrl+Shift+L releases ownership; Ctrl+Shift+Tab cycles sessions. Take input confirms current ownership before arming. Already submitted work/side effects continue. |
| Scripted agent proof | In a **fresh** launch, click **SCRIPTED DEMO** or Ctrl+Shift+D once. Two actual protocol clients bind one immutable task contract and start separate attributed managed attempts. A scripted endpoint owns the second shell; select it and Take input. The helper verifies its stale write is refused. No model runs. |
| Inspect exact evidence | Ctrl+Shift+E expands task/file revision detail. File observation is not correctness or sealed producer causality. The second managed attempt is independent, not a dependency consumer; the separate [producer→consumer experiment](producer-consumer-experiment.md) proves that narrower contract. |
| Run one literal command | Ctrl+Shift+A or **RUN ARGV**, enter `["/usr/bin/printf","hello"]`, Enter once; Escape cancels. This directly executes argv, not an agent prompt and not implicit shell syntax. Enter auto-repeat does not resubmit. Two lifetime attempt slots are shared with the demo, so consuming either first can prevent the demo. Refusal is explicit, not an automatic fresh namespace. |
| Resize / reconnect | Window resizing pauses input but does not mutate canonical PTY dimensions automatically. Ctrl+Shift+R explicitly requests bounded geometry, then Take input. Ctrl+Shift+C explicitly attempts live endpoint resume; no queued suffix replay. |

Application keys are only routed in the focused, owned, known Terminal view. Ctrl+Shift shortcuts are reserved UI commands. Unknown initial focus, unsupported keys, stale context, mailbox exhaustion, refused/partial/unknown delivery and disconnect fail closed. Winit's synthesized held-key presses on focus restoration are rejected **before UI shortcuts, the argv form and application input**. This is not physical-key attestation: X11 input is spoofable by same-UID peers. The finite profile has no clipboard/paste, IME or alternate-screen application qualification; use only nonsecret ASCII/Latin-1 developer commands for feedback.

### Validation and preserved focus failure

The isolated journey launches only its owned broker/window/PTY resources on the actual display; screenshots cover only its PID-verified window. It sends synthetic X11 keys, not a physical keyboard. It exercises repeated commands and exact file side effects, history navigation, interruption, two scripted origins/task revisions, human takeover/stale-writer refusal, resize, explicit live resume, a stopped **test-owned** broker, and exhausted-capacity rejection without hidden process execution. Cleanup uses the existing owned-process/pidfd harness and records unreaped-descendant limits.

```sh
make mvp-build
python3 -B tests/mvp/native.py --bin-dir target/debug --terminfo . --artifacts .artifacts/native-NEW-UNUSED-ID
```

The inherited `--terminfo` argument only identifies an existing directory for helper bookkeeping in this native test; it is not passed to the broker, which uses the explicit TERM=dumb fallback. The artifact path must not exist. Adjust `--bin-dir` for `CARGO_TARGET_DIR`. This display-dependent test is **manual/VM evidence**, not silently skipped into a CI pass. `make foundation-verify` covers the native code with all-target Clippy, Rust tests (including actual public connector fake-server tests), launcher positive/negative tests, and the retained real headless/TUI/agent journeys. No historical X4/EA1 gate expectation changed.

**Original failure retained:** the eighth native journey failed `unfocused native input reached the app`, and the screenshot shows `WRONG_FOCUS` reached Bash. It sent a synthetic **core** FocusOut, but did not record actual X focus, event order or input byte receipts. Its assertion was invalid evidence of actual loss; the observed execution remains a failure, not a deleted/flipped result. Minimization-only later runs did not resolve this evidence gap.

The pinned winit X11 backend handles focus via **XInput2** and routes core keys through its active-window state. A fresh reproduction establishes XGetInputFocus before/after the fake core notification, the library/model focus state, causal queued/dispatch/result IDs and exact receipt counts: the harmless comment canary delivered **19 bytes** while the window remained focused. A separate regression transfers actual server focus only between two owned windows, observes XI2 focus loss, sends the canary to the inactive native window, and observes **zero input queued or dispatched**, with unchanged canonical screen. Zero application bytes are inferred from this path absence; there is no rejected-input receipt carrying zero bytes. Focus return remains paused until explicit Take input. Winit suppresses inactive core key delivery in this journey; it alone does not certify the frontend guard. Independent model/connector tests exercise unknown focus, stale grants, stale queued input, overflow, partial results and no suffix retries. Synthetic held-key rejection is source-reviewed plus a gate unit test; physical held-key restoration, IME and every window manager remain untested.

The opt-in `NAMTAB_MVP_TRACE=/absolute/NEW/file` is test metadata only: no key names/text, argv, output or resume credentials. It is disabled normally, created exclusively, bounded to128 queued records/4096 events/1MiB, and written off interaction paths. Gaps/truncation invalidate the journey's evidence. Allocation sequence is not cross-thread file/timestamp order; correlate focus generations, local intent IDs and receipt request IDs. The harness awaits required records; process-exit flush is not guaranteed.

[Retained observation record](https://github.com/whoodes/namtab/blob/main/evidence/native-mvp/2026-10-27.json) identifies original failure hashes, final tested binary/harness/screenshots and fresh ignored logs. A [fresh follow-up](https://github.com/whoodes/namtab/blob/main/evidence/native-mvp/2026-10-27-trace-cap.json) re-exercised the native journey after correcting total trace marker accounting; the earlier observation record remains unchanged. The earlier successful instrumented run recorded109 dispatch-to-response samples: p50=2153µs, p95=7085µs, p99=7869µs using sorted lower-rank quantiles. This is **one synthetic local sequential-RPC workload**, not key-to-paint latency, a capacity/fairness benchmark, model inference timing or an achieved product SLO. Reproduce rather than generalize these values.

### Residual gates and next feedback

This increment asks whether the real Work/Terminal/handoff loop is useful. It does not choose the final toolkit/renderer or fulfill the study's shaping/accessibility/device matrix. Two declared scripted endpoints are not integrated autonomous models, secure principals or dozens-agent admission. Real crash recovery, terminal history/full VT fidelity, durable capture, process cleanup UX, arbitrary file seals, native wire/resource admission and external tool mediation remain separate H0/H1/H2 gaps. The existing headless producer→consumer and write experiments remain distinct; this window does not silently replace their stronger experimental tuples with MK1's API.

For feedback, report the printed run path, window diagnostics, selected task/session, exact interaction and visible uncertainty. Do not put credentials in the prototype. No production release, automatic restart, security-policy change or merge is authorized by these instructions.
