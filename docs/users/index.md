# Use namtab

[Architecture](../architecture/index.md) · [Agent/developer guide](../agents/index.md)

Namtab is an experimental AI-first terminal engine with a native harness for observing attributed work and controlling selected terminals/attempts. This is a **source-built Ubuntu prototype**, not a packaged production terminal. Basic use needs no model account. The real process/keyboard workflows below are implemented on main after PR74/75; broader fleet scale and model-directed Namtab execution remain unvalidated.

## Prerequisites

The exercised environment is Ubuntu22.04 x86_64, Python3.10 with venv, Rust/Cargo1.91.0 with rustfmt/Clippy, Node24.21.0, pnpm10.32.1 and MkDocs1.6.1. Rust/rustup and basic build tools must already be available. Pinned project setup downloads verified tools and locked dependencies into the checkout:

```sh
make bootstrap docs-install install
```

This is developer setup, not a product installer; it does not install system packages or configure credentials. The native window additionally needs an accessible real X11 `DISPLAY`, existing X11/libxkbcommon runtime libraries and `/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf`. Missing display/font/library/surface support is an environment error, not a successful headless GUI launch. macOS/Windows, Wayland and arbitrary remote displays are not qualified by the Ubuntu evidence.

## Launch and run a command

From the repository root:

```sh
make mvp
```

This builds the current native binaries and creates a **fresh** project-local `.artifacts/mvp-<id>/` run with its own broker/workspace. It never searches for existing sessions, replaces a namespace or restarts someone else's run. Keep the printed run path: it identifies diagnostics, explicit attach/status/stop and the workspace in which commands execute.

Select **TERMINAL**, choose **Take input** (Ctrl+Shift+T), type `echo hello`, then Enter. Input requires a focused, known, owned terminal; selection alone does not arm it. Unknown/refused/partial input pauses the route instead of replaying a suffix. Ctrl+C is an application key, not a promise of process termination.

## Common workflows and controls

| Goal | Interaction and result |
| --- | --- |
| Inspect work | **WORK** or Ctrl+Shift+1. Arrows/Home/End select an attributed attempt, terminal or task; only one detail is expanded. Status describes observed process/control facts, not model reasoning |
| Open terminal | Select its Work row and Enter, or Ctrl+Shift+2. Enter opens paused; a pipe-backed attempt does not imply a terminal |
| Switch terminal | Ctrl+Shift+Tab cycles existing terminals and pauses input; explicitly Take input on the chosen target |
| Take/release/resize | Ctrl+Shift+T/L/R use the highlighted terminal's frozen epoch/session/generation. Attempt/task rows do not secretly control a previously opened terminal. Window resize pauses input; R explicitly requests bounded canonical geometry, then take again |
| Cancel one attempt | Select a live attempt, Ctrl+Shift+X, inspect the exact target, then plain Enter once. Escape, selection/view/focus/resize/stale state/disconnect dismisses. Requested cancellation is separate from observed child exit; other effects may continue |
| Inspect provenance | Ctrl+Shift+E expands selected task/file revision detail. A file observation or exit code is not proof of correctness or sealed producer causality |
| Execute literal argv | **RUN ARGV** or Ctrl+Shift+A; enter `["/usr/bin/printf","hello"]`, Enter once; Escape cancels. This is JSON argv, not shell syntax or an agent prompt |
| Try scripted coordination | On a fresh run, **SCRIPTED DEMO** or Ctrl+Shift+D once starts two real protocol clients sharing an immutable task. Output is attributed, with a second terminal available for takeover. No model runs |
| Reconnect live window | Ctrl+Shift+C explicitly resumes that window's same-epoch live endpoint. No input/operation replay; a changed/dead owner is not automatically restarted |

Ctrl+Shift UI chords stay separate from application input. Focus/view/selection changes pause terminal input and discard unsent intent; they cannot undo already-dispatched effects. The [fleet control reference](../ux/fleet-control.md) owns detailed fences and actual evidence. [Native evidence](../engineering/native-mvp.md) records synthetic-key/focus/stall qualification limits.

**Concrete current UI limits:** rendering is the bounded Latin-1 feasibility path, not full Unicode shaping. Native IME and system clipboard/paste integration, terminal scrollback/search and application mouse navigation are not implemented. Shell history keys are application input and are distinct from terminal scrollback. Use synthetic/simple text for feedback; do not assume arbitrary editors or languages are supported.

## Agent configuration

Merged ENG3/PR81 migrates repeated MCP reads to scoped requests; use same-build broker/bridge binaries. Older brokers are explicitly unsupported by `mcp-local/1`, with no automatic legacy fallback. This changes no provider/workspace/tool approval and does not authorize a model run. [Contract and isolated proof](../engine/scoped-observations.md#eng3-mcp-consumer-admission).

Namtab does not automatically choose a provider, read auth stores, accept folder trust or start a model. For an explicitly chosen nonsecret run:

```sh
python3 -B scripts/mvp.py agent-config --run /absolute/printed/run
```

This prints one-process Codex MCP configuration and starts nothing. It does not save configuration or authorization. The human must separately choose the data/workspace/provider/tool permissions, using the client's supported approval flow. Do not paste secrets into task/terminal content or grant persistent trust merely to complete a demo.

The implemented stdio bridge exposes task contracts, literal managed execution, targeted observation, terminal operations, artifact observations and known receipts. Only calls actually routed through Namtab are mediated/attributed. An agent running in a PTY or using its own shell/file/web/model tools is not automatically controlled. Current native cancellation does not imply a new MCP cancellation tool.

The retained live Codex attempt discovered tools but did **not** execute a Namtab task/run: a tool approval refusal and then a saved folder-trust prompt blocked validation. The temporary session disappeared with no completed result; it has not been restarted. Scripted/fixture PASS results are not real-model validation. [Exact adapter profile, gate limits and evidence](../engineering/external-agent-mvp.md).

## Close, attach, status and stop

Closing the window **detaches**; shell, broker and managed work can continue. Use only the exact printed path from the run you intend to control:

```sh
python3 -B scripts/mvp.py status --run /absolute/printed/run
python3 -B scripts/mvp.py attach --run /absolute/printed/run
python3 -B scripts/mvp.py stop --run /absolute/printed/run
```

Status/stop need no display. Attach creates a new human endpoint and needs explicit takeover. In-window reconnect instead retains its live in-memory identity. Stop checks the recorded epoch/capability and asks for protective shutdown of the broker's still-owned direct processes. It is not a global session/PID killer or descendant-containment guarantee. Lost/unknown response, wrong epoch, replacement socket or timeout is uncertainty; do not automatically retry, delete a socket, kill a guessed PID or restart. Stop preserves files/evidence.

Older launch records/daemons can lack this handshake. Do not point the new launcher at discovered older sessions; leave them intact for explicit operator review. The [lifecycle contract](../engineering/external-agent-mvp.md#detach-status-and-explicit-stop) explains cleanup and uncertainty.

## Troubleshooting and limits

| Observation | Meaning / next step |
| --- | --- |
| No display, font or surface | Native prerequisite is missing. Use diagnostics to identify it; a headless build is not a GUI pass. Do not silently replace the display or install system packages |
| Key did nothing / INPUT PAUSED | Check selected view, focus, ownership and known state. Inspect the last receipt. Take the intended terminal explicitly; never resend unknown partial work blindly |
| LAST OBSERVED / disconnected | State may be stale. Reconnect only to the live same-epoch owner; engine crash/process restart is a different operation, not screen replay |
| Session/attempt capacity refused | Two **lifetime** PTYs/two managed attempts; four simultaneous connections and separately bounded retained identities. Demo consumes both attempt slots. Use a consciously fresh run for another demo; no automatic slot reuse or reset of another run |
| Agent cannot force takeover | Agent claims do not override human ownership. Use the targeted human control, not an external PTY/input bypass |
| Stop/status refused | Verify your exact run path/epoch and connection budget. Do not convert refusal into broad process discovery or a force-kill fallback |
| Full-workspace build hash failure | Optional historical EA1 requires its original exact artifact. Use the [active-source workflow](../agents/index.md#build-and-verify) for current development; do not repin historical evidence to make it pass |

Capture-off means ordinary output is bounded live state, not durable retained capture or a secret-safe environment. Test screenshots/logs and the outer OS/terminal can retain content. Same-UID roles are declarations, not authenticated human/model principals. Full VT/editor/Unicode compatibility, IME/a11y, secure runtime isolation, portable crash recovery, dozens-agent capacity and measured product latency targets remain unqualified. Cancellation/takeover cannot promise every side effect stopped.

## Feedback and validation

Report the exact run path, revision, selected task/attempt/session, action, expected result and observed receipt/status. Include only nonsecret diagnostics. Current canonical limits and remaining gates are in the [architecture](../architecture/index.md) and [roadmap](../engine/delivery-and-validation.md).

Developers can run `make foundation-verify` for headless active checks. `make nat1-test` prepares same-worktree binaries and local terminfo through `mk1-build`, then runs a fresh isolated **actual X11** fleet journey. It additionally needs existing `tic`, ImageMagick `import`, libX11 and the font above. No manual terminfo directory setup is required. Native fixtures use the declared `TERM=dumb` fallback; they do not certify the generated `namtab-mk1` profile. No model is invoked by either check.
