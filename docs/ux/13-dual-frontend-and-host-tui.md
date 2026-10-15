# Two initial frontends and the host-terminal TUI

> Current ownership decision: [ADR E4](../adr/0004-owned-agent-native-engine.md) and the [owned-engine campaign](../engine/index.md) choose project-owned Rust agent-native semantics with a PTY compatibility layer. Adoption-specific mechanisms below are historical where superseded; product/authority/UX invariants remain. New acceptance is NG0–NG4, not fixing or waiving old X4/EA1 failures. This phase is research/design only.

[Index](../index.md) · [Shared human UX](11-human-experience-spec.md) · [Repository boundaries](../engineering/14-repository-and-engineering-practices.md) · [Engine gate](../engineering/17-xterm-adapter-risk-and-gates.md)

**Accepted-for-design:** two initial human frontends, a host-terminal TUI and Electron desktop. **Recommended:** Rust Ratatui/Crossterm for the TUI; retain Electron/React for desktop. Interoperability, accessibility and performance remain unproven. Neither is a prerequisite for the other, but both require the shared supervisor and terminal-engine service. TUI-only use does not start Electron; it needs the shared canonical native engine worker in the revised ADR E1 shipping design. The current Node/xterm code is assessment/reference infrastructure, not a migrated product.

## Three different products and integrations

| Surface | Role | Explicit boundary |
| --- | --- | --- |
| Host-terminal TUI | Project X control application running inside an existing emulator, tmux, console or SSH PTY | Draws controls and canonical application cells; cannot choose the outer terminal's font, recover lost key distinctions or control its recording |
| Optional shell/prompt integration | Cooperative cwd/activity/status/task hints and optional prompt segments for a particular shell | Not an emulator, not required by either frontend, not authority; never automatically installed/sourced |
| Electron desktop | Standalone emulator presentation plus rich work/review UI | Owns its windows/fonts/clipboard integration; uses the same broker API as the TUI |

Oh My Zsh is a Zsh configuration framework; Powerlevel10k is a Zsh theme. Take inspiration from discoverable customization, compact status segments, keyboard habits and guided visual previews, not a dependency on either project. Project X must work with Bash, Fish, POSIX shells, Zsh and direct executable launches. There is no executable theme language, Zsh requirement, startup-file modification or font download in the baseline. [Oh My Zsh upstream](https://github.com/ohmyzsh/ohmyzsh), [Powerlevel10k upstream](https://github.com/romkatv/powerlevel10k)

## Shared semantics, no frontend authority

Both clients authenticate to the supervisor, negotiate capabilities, obtain projections/observations, propose actions, reconcile receipts and display provenance. Only broker/keepers admit actions and own managed PTYs; only the store owns private persistence; only the canonical engine interprets application output/modes and creates permitted terminal replies. Clients never open the database, evidence-store internals, keeper sockets or managed PTY master/slave endpoints. The TUI necessarily uses its own outer stdin/stdout and terminal-mode APIs; those are distinct from managed application PTYs.

The sanctioned public view protocol offers immutable canonical cell/semantic pages and deltas for both TUI and desktop under [ADR E1](../adr/0001-engine-ownership-and-presentation.md). Both require a matching base prefix/generation; a stale/missing frame triggers bounded reobservation. No inner raw output is written to the outer host, and neither frontend runs another authoritative parser. Ratatui uses safe drawing controls and sanitized cells; Electron must supply owned rendering, selection/IME and an accessible semantic representation. A text inspector is not equivalent. Renderer fidelity and host capability gates remain open; see [assessment](../engineering/19-engine-build-adopt-redesign-assessment.md).

Lease ownership names participant **and client endpoint**, session incarnation and generation. Two clients belonging to the same human cannot both become writers just by sharing an account. Selecting a view is observation; taking input is an explicit generation change. Backend approval, protected-prompt, stale-context and partial-delivery checks are identical. Neither keymaps nor a theme can expand permissions. TUI approval controls are trusted application controls within the stated same-account/outer-host threat limits; they are not cryptographic proof of a human or unspoofable pixels.

## Parity and legitimate differences

| Capability | Required shared result | Legitimate presentation difference |
| --- | --- | --- |
| Tasks/groups/dependencies, decisions, artifact provenance | Same identities/revisions, unknown/stale states and evidence quality | Desktop expandable panes/rich diff; TUI paged list, inspector and unified diff |
| Session create/attach/detach, takeover/resume, cancel/status | Same fenced requests and receipts | Desktop tabs/windows; TUI virtual tabs and one primary viewport, optional split after usability gate |
| Interactive navigation | Same canonical encoder and supported logical-key contract | Physical TUI key set depends on outer encoding; Send key palette supplies explicit intent where host chords alias |
| Selection/search/paste | Same evidence range, permissions and paste policy | Desktop native selection/clipboard; TUI keyboard selection and internal copy buffer; system clipboard export explicit and conditional |
| Approvals/secrets | Same context challenge, revocation and fresh protected handoff | Desktop dialog; TUI dedicated review screen with application input suspended; no default Allow/Enter acceptance |
| Accessibility | All critical controls reachable and states comprehensible | Desktop semantic accessibility tree; TUI tested full-screen mode plus linear review/reading mode, with actual host/screen-reader limits |
| Colors/fonts/notifications | Status never color/icon-only; same durable inbox | Desktop controls fonts/OS notifications; TUI uses host font/palette and aggregated status/inbox, no unsolicited bell/OSC notifications |

Do not claim pixel parity, universal key parity or native accessibility semantics from terminal cells. Unsupported rich artifact types still expose safe metadata and an authorized export; no automatic external editor launch. Loss of a required critical workflow blocks certification of that frontend/profile. A linear log is not sufficient evidence that interactive editor operation is accessible.

## Default layout and customization

Use the same Terminal / Work / Review / History lenses as desktop. With no prior sessions/tasks, normal interactive entry offers the zero-agent shell view; creation is a broker request, never a local child hidden behind the client. Reopening existing work uses the last selected live view or a session picker rather than duplicating a shell. An explicit Work entry opens the dashboard. No agent or provider setup is required.

```text
Project X · workspace app · Work 12 · Review 2 · History     prefix ^G ?
Group Build       Task                  State        Age
  compile         debug build           running       1m
  tests           parser cases          review        2m
  docs            API examples          blocked       3m
──────────────────────────────────────────────────────────────────────
Selected: parser cases · attempt 3 · artifact revision 7
check: initial snapshot · output retained · current edit unreviewed
host local · controller: host · tool: offline container · owner: agent A
^G : Palette   ^G t Terminal   ^G r Review   ^G d Detach
```

At 80×24, hide optional columns before truncating authority/unknown indicators; selected details and long digests are paged with explicit expansion. At less than 60×16, enter a compact single-panel mode. Below 40×10, show a resize/linear-mode screen and disable hidden approval actions; the numerical thresholds are proposed UX gates. No flashing dashboard or pane per agent. Stable row identity/order prevents a redraw from changing the selected task.

Versioned TOML selects status segments, order, density, keymap, light/dark/mono palettes, ASCII/Unicode borders and optional icon set. Segments read typed cached projections; no arbitrary shell commands, network calls or filesystem polling per redraw. Expensive status providers belong in the supervisor with bounded refresh. Presets include minimal, information-dense and high-contrast/ASCII. A preview/wizard is optional, reversible and shell-independent. A Nerd Font/Powerline glyph set is opt-in with a plain-text fallback and visual fit preview; no bundled font initially. Config source/precedence and unknown settings follow [the shared configuration policy](11-human-experience-spec.md#onboarding-migration-and-portable-configuration).

## Input modes and nesting

Distinguish **control focus**, **application focus**, **prefix pending**, **review/protected prompt**, and **disconnected/stale**. In control focus, arrows/Tab navigate Project X. In application focus, decoded keys/text/paste become identified broker intents, with no UI shortcut dispatch except the configured escape prefix. Automation bypasses both human keymaps and addresses explicit session identities. Switching focus cancels pending prefixes, paste previews and unsent input; it never moves a queued action onto another session.

Initial TUI prefix proposal: **Ctrl+G**, chosen to avoid tmux's common Ctrl+B and screen's common Ctrl+A defaults. It collides with Bash/Emacs abort bindings, so double Ctrl+G sends one literal Ctrl+G to the application through admission; remapping is visible. Prefix commands: `?` help, `:` palette, `t/w/r/h` lenses, `d` detach. Taking control, approving, or stopping requires a named action/review, not an accidental prefix suffix. Prefix timeout initially 800 ms, configurable and shown; timeout/invalid suffix cancels locally with feedback, never forwards an unexpected control byte. A paste event is not parsed as prefix commands. Delayed input from SSH may still be ambiguous: a timeout is a UI policy, not proof of key boundaries.

Outer emulator, desktop shortcuts, tmux/screen and SSH escape processing can consume input first. The effective-binding help shows layers known from explicit configuration and labels undiscovered layers unknown. Nested Project X TUIs require explicit distinct prefix configuration or quoting at each layer; do not auto-detect trustworthy nesting from environment/title strings. No remote configuration edits to “fix” collisions. Active terminal capability probes are off by default; an optional bounded setup probe runs before application focus, consumes its expected replies locally and fails conservatively if reply/input origin is ambiguous. Never use live application input as a probe channel. The user may choose an SSH escape setting explicitly; Project X does not silently disable the transport's escape facility.

Ratatui/Crossterm does not recover distinctions the host already lost: Ctrl+I/Tab, Ctrl+M/Enter, some modifiers/keypad keys, lone Escape/Alt sequences, release/repeat events and unsupported function keys. Negotiate enhanced outer keyboard reporting only when supported, restore it on exit and retain a conservative legacy profile. The outer decoder and inner canonical encoder are different layers. Unknown input is not automatically tunneled as raw inner bytes. The Send key palette can request a supported inner logical key even when no physical outer chord represents it; native agents always use that API directly.

Use one outer input reader/event system, not competing Crossterm readers or mixed EventStream/read loops. Its docs explicitly constrain those combinations. [Crossterm events](https://docs.rs/crossterm/latest/crossterm/event/index.html) Preserve text committed by the host's IME; the TUI cannot reconstruct pre-edit/candidate behavior. In control focus, Ctrl+C cancels a local pending read/dialog without implying action rollback. In application focus, Ctrl+C/Ctrl+Z are inner application intents. Suspending the control UI is a separate command: release this endpoint’s input/sizing leases, restore outer modes, suspend the TUI, then renegotiate/observe on resume with old queues discarded. Suspending the client does not pause managed work. External signals may still prevent cleanup.

Focus reports are optional hints. Missing outer focus events do not grant authority or promise knowledge of application-widget focus.

## Paste, resize and terminal capabilities

Outer bracketed-paste detection produces a paste intent; the backend independently checks **inner** bracketed-paste mode and adds appropriate wrappers. Do not forward outer wrappers, add Enter, infer paste from typing speed, or claim reliable paste boundaries where the host provides none. Such a profile displays the limitation. Known paste events cannot confirm a broker decision. Unbracketed input may be indistinguishable from typing: do not claim universal paste detection. By default an outer profile without certified paste/input separation cannot issue Allow in the TUI; it offers read/deny/cancel and review in a certified client or an explicit preconfigured policy principal. A fresh broker challenge remains necessary on supported profiles and does not prove physical human presence. Multiline/control-bearing input follows the preview/reject policy. System clipboard writes via OSC 52 are off by default; selection stays in the client buffer unless an explicit supported export is chosen.

Outer resize updates control layout. Only the current sizing-authority view requests inner PTY geometry, subtracting the actual visible chrome. Mirrors use a viewport/letterbox without resizing the process; collapsed/tiny views keep last valid inner geometry and make input restrictions visible. Resizing invalidates coordinate selections and stale observations; it is not an atomic TUI transaction. Prefix, IME/text and partial paste states cannot silently retarget after resize.

`TERM` describes the outer environment when starting the control TUI. The inner managed process receives the canonical engine's declared terminfo/profile, not a blind copy of outer TERM. `COLORTERM` is a hint, not proof. First outer certification targets modern ANSI-compatible xterm-256color environments and nested tmux-256color/screen-compatible profiles; TERM=dumb gets linear control/read mode, not a fake full-screen emulator. Crossterm is not a universal terminfo interpreter; a host setting alone cannot establish support. Terminfo-dependent applications, TERM mismatches and remote entries are fixture cases. Never claim unsupported graphics/enhanced keys in the inner profile.

Offer truecolor, 256-color, basic-color and monochrome presentation tiers with explicit override. Sanitize controls, OSC/DCS and bidi-sensitive chrome; avoid width-changing truncation. Canonical cell widths remain authoritative for agent observations; the outer font/Unicode table can still disagree visually. Disable unreliable coordinate interaction, expose text inspection and fail the claimed profile if essential editing is unusable. TUI fonts/zoom are the host's responsibility. [Terminal compatibility contract](../research/06-technology-distribution-and-compatibility.md#compatibility-contract)

## Attach, detach, SSH and failure UX

Attach negotiates client/view identity, subscription cursor, geometry and canonical prefix before input. It is read-only until an explicit writer lease is acquired or safely reacquired. Detach leaves managed processes running, releases this client's input/sizing leases and cancels its unsent requests. Other clients' leases are unchanged. Inner size holds until a new sizing owner explicitly takes it. Disconnect/crash follows expiry/fencing; it cannot recall delivered input. Normal exit restores outer raw mode, cursor, alternate screen, paste/focus/enhanced-key modes. SIGKILL/host failure can prevent cleanup; document manual host-terminal recovery without executing it automatically.

Running the TUI through SSH is an initial deployment pattern **when Project X is explicitly installed on that remote host**: SSH carries the outer interactive TUI, and its supervisor is local to that host. This differs from a local TUI supervising an opaque managed `ssh` process, and from a later structured-helper transport. No implicit remote binary installation or cooked-stream JSON tunneling. Detach persistence depends on remote keeper/service/logout policy; connection loss alone proves neither exit nor continued execution. Do not promise SSH network latency meets the local 50 ms target.

TUI capture-off means Project X does not persist output. The outer emulator, multiplexer, SSH endpoint, recorder or screenshot tool may retain displayed content independently. Show that limit for secrets/export; the TUI cannot erase host scrollback or guarantee its clipboard is private. Shared secret-handling controls still apply.

Loading and stale views show last observation time and block invalid input; unknown outcomes offer reconciliation, not Retry. Dedicated review hides application output behind a controlled screen, rejects stale decisions and discards buffered transition keys. Linear mode offers numbered task/review pages and explicit commands with the same broker challenges and profile-specific Allow restriction, not generic Enter-to-approve. Accessibility must be assessed in actual terminal/screen-reader pairs; no inspected Rust TUI framework establishes a native accessibility tree. [CI and human gates](../engineering/16-test-and-ci-strategy.md)
