# Host-terminal diagnostic profile

[Architecture](../architecture/index.md) · [User guide](../users/index.md) · [Core profiles](../engine/compatibility-and-core.md#profiles-and-negotiation)

The retained Ratatui/Crossterm TUI is diagnostic/attach tooling inside an existing terminal, not a second required product frontend. The Electron parity/layout commitment is retired under [E5](../adr/0005-ai-first-harness-direction.md). Current diagnostic commands are in [MK1 reproduction](../engineering/mk1-quickstart.md); the sections below retain **design constraints for broader host certification**, not implemented key-palette, linear-accessibility, SSH or approval guarantees.

## Shared semantics, no frontend authority

The host client consumes public observations and submits typed actions. The engine owns admission, canonical state/input encoding, managed PTYs and terminal replies. Outer stdin/stdout/raw-mode handling is necessary to run the client but grants no access to application PTYs or private persistence. Do not tunnel untrusted inner output directly to the outer terminal or run a second authoritative parser. A mirror is not a writer; view selection is separate from input/sizing authority.

Shell/prompt integrations can supply cooperative hints, never authority or proof of command completion. They are optional and must not require Zsh, install a theme, source a startup file or download fonts. Oh My Zsh/Powerlevel10k are configuration/theme inspiration, not emulators or dependencies. Keymaps/themes cannot expand permissions.

## Input modes and nesting

Keep control focus, application focus, prefix pending, review/protected prompt and disconnected/stale separate. Arrows/Tab navigate the client in control focus; application keys target an explicit canonical context through the broker. Switching focus/view cancels unsent prefixes/paste/input, never retargets queued work. Automation uses identified operations independently of human keymaps.

The current diagnostic uses **Ctrl+G** as a reserved UI prefix; double Ctrl+G sends one admitted literal Ctrl+G. `Ctrl+G ?` shows implemented commands. This conflicts with some readline/Emacs bindings; outer tmux/screen, OS and SSH processing can consume keys first. Do not silently change those configurations or infer trustworthy nesting from environment/title strings. A future remapping/Send-key palette remains a proposal; do not advertise it as delivered. Invalid/expired prefixes cancel locally rather than emitting unexpected bytes; timing is a UI policy, not proof of key boundaries.

Active capability probes are off by default. Any future optional probe must be bounded, run during explicit setup before application focus, consume its expected replies locally and refuse ambiguous reply/input origin. Never use live application input as a probe channel.

Host decoding can lose Ctrl+I/Tab, Ctrl+M/Enter, keypad/modifier, function-key, release/repeat and lone-Escape/Alt distinctions. Enhanced outer reporting requires actual negotiation and restoration, not assumptions from TERM. The outer decoder is distinct from the engine's inner mode-aware encoder. Unknown input is refused or explicitly unsupported, not silently forwarded raw. A future supported logical-key palette can express intent unavailable physically without inventing physical key/layout facts.

Use one outer event reader. Crossterm forbids conflicting reader/EventStream combinations; [upstream event API](https://docs.rs/crossterm/latest/crossterm/event/index.html). Preserve committed text; a TUI cannot reconstruct the outer IME preedit/candidate interface. Focus events are optional hints, not proof of inner application-widget focus. In application focus Ctrl+C/Ctrl+Z remain application intents, not universal stop/rollback commands. Any future suspend-client operation must release its authority, restore outer modes and discard old queues before resume; client suspension does not suspend managed work.

## Paste, resize and terminal capabilities

Outer bracketed paste becomes one paste intent; the backend independently applies the **inner** mode/wrappers. Never forward outer wrappers, append Enter, infer paste from speed or claim reliable boundaries on an unbracketed host. Paste cannot confirm a broker decision. A future host profile lacking certified paste/input separation may expose read/deny/cancel, but cannot claim a trustworthy Allow action; any approval requires a fresh context-bound challenge and still does not prove physical human presence. OSC52 clipboard export is not automatic; selection/copy/export require explicit capability and consent.

Outer resize changes layout. Only the admitted sizing owner may request inner geometry; mirrors use a viewport/letterbox. Tiny/collapsed views keep a valid inner size and show input limits. Resize invalidates coordinate selections and stale observations; it cannot atomically retarget an in-flight input or partially composed/pasted operation.

Outer TERM describes the host; inner TERM/terminfo must describe the canonical engine's declared profile, never blindly copy the host. COLORTERM is a hint. Crossterm is not a universal terminfo interpreter. Future xterm-256color/tmux/screen certification needs fixtures; a setting alone is not support. A dumb host cannot be advertised as a full-screen/linear accessibility success without an actual fallback implementation. The current native fixture's inner TERM=dumb is a separate explicit test fallback.

Color/monochrome tiers, ASCII fallbacks and theme previews remain presentation options to qualify, not permissions. The outer host owns its fonts/zoom; canonical cell widths remain engine facts while host glyph/Unicode width may disagree. Sanitize control/OSC/DCS and bidi-sensitive chrome. Avoid width-changing truncation; disable unreliable coordinate actions and expose safe text inspection. If essential navigation/editing is unusable, fail that host profile rather than claim pixel/key parity.

## Attach, detach, SSH and failure UX

Attach observes identity/epoch/context before input. Detach and disconnect are not process completion; delivered input cannot be recalled. The implemented live-owner resume differs from a new client, engine crash and process restart. Normal outer cleanup restores raw mode/cursor/alternate-screen/paste modes; SIGKILL/host loss can prevent it. Manual host-terminal repair must be operator-chosen, never an automatic command into an unknown application.

SSH-hosted TUI use would require explicit installation on the remote host, whose local supervisor owns work. It is distinct from running an opaque `ssh` program inside a managed PTY and from a future structured remote helper. No implicit remote install, cooked-stream protocol tunnel or SSH escape disabling. Remote logout/keeper policy determines persistence; connection loss proves neither exit nor continued execution. Measure network latency separately from local control.

Capture-off does not erase outer emulator/multiplexer/SSH/recorder/screenshot history or secure its clipboard. Stale views show last observation and block invalid input. Unknown outcomes require reconciliation, not automatic retry. Dedicated future approval review must hide application output and discard transition keys. Full-screen/linear mode alone does not prove accessibility: test actual terminal/screen-reader pairs and interactive workflows. No inspected Rust TUI framework supplies a native accessibility tree by declaration.

## Provenance and scope

This distills the still-applicable host constraints from the [retired dual-frontend plan](https://github.com/whoodes/namtab/blob/5c99e34bc6534a09f0511557e1f6f1154d46572c/docs/ux/13-dual-frontend-and-host-tui.md). Obsolete Electron parity, dashboard sketches, arbitrary layout thresholds and initial product commitments are removed rather than copied into a new archive. Detailed engine guarantees remain with the [frontend contract](../engine/frontend-contracts.md), [human UX design](11-human-experience-spec.md) and [test profile owner](../engineering/16-test-and-ci-strategy.md).
