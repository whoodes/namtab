# Human experience and dual-frontend contract

[Campaign](index.md) · [Native contract](native-contract.md) · [Existing detailed human UX](../ux/11-human-experience-spec.md) · [Host TUI specification](../ux/13-dual-frontend-and-host-tui.md)

The owned-engine decision preserves the existing Terminal / Work / Review / History information architecture, configurability and both initial frontends. This page defines their new canonical-model integration and concrete acceptance journeys. It is a design, not an implemented interface or accessibility result. Sources S10–S16 and S23–S25 constrain host/input/text behavior.

## One model, two useful clients

Both clients consume the public service API: revisioned terminal frames/history pages, task/attempt/process facts, receipts, decisions and authorized artifacts. No inner VT parser, independent input-mode tracker, private store/worker endpoint or managed PTY access. Each frontend owns layout, glyph/row caches, scroll position, selection anchors and transient focus. Those caches can be discarded/rebuilt; they are not canonical session state.

With zero agents, open a normal shell terminal with conventional editing, search, selection, copy/paste, tabs and later optional splits. No provider account, shell hook, Zsh or task creation is required. An ordinary foreground command is not forcibly wrapped into a native task. Native managed work can be promoted into the Work view without converting all terminal output into command blocks.

Open terminal is also the human fallback from a structured task/artifact view. That ordinary or “raw terminal” view still renders canonical state and uses admitted input; it is not an unmanaged PTY tunnel or second emulator. The explicitly granted raw-byte input operation is a different capability.

```text
Terminal     Work 8     Review 2     History          local / workspace A
Task: parser tests · attempt 3   [observed process: running] [owner: agent A]
─────────────────────────────────────────────────────────────────────────
Selected session or native stdout/stderr/artifact view
One primary viewport; optional details drawer; no pane per agent
─────────────────────────────────────────────────────────────────────────
Evidence at E418 · current · memory-only      Palette / Take control / Stop…
```

Progressive disclosure: ordinary terminal first; Work groups tasks/agents/dependencies with stable selection; Review shows exact artifact/context changes and decisions; History exposes provenance/retention/unknown outcomes. Notifications enter a deduplicated attention inbox by task/severity, with quiet summaries and bounded rate. Output bells/titles cannot fabricate trusted product notices. Agent count or pane count is not the success metric.

N1 may implement only a narrow session/task selector, one viewport, receipt/status view and takeover in each client. Rich diff, multiwindow, full search and advanced splits arrive later. Both must be usable at each claimed release; sequencing is not permission to silently ship only the backend or desktop.

## Input, focus and command routing

The visible modes are **Project X controls**, **application input**, **prefix pending**, **protected review/secret handoff**, and **disconnected/stale**. Keyboard focus is per endpoint/view/generation. Writer and sizing leases are independent. Clicking a view does not steal a writer or resize another person's session; an agent request never changes UI focus. Switching session/focus cancels unsent composition/prefix/paste and increments the local generation, preventing old queued events from retargeting.

UI commands operate layout/search/navigation and emit no managed application-input bytes. Application named keys/text/paste/mouse use the identified broker path. Automation bypasses human shortcut maps and supplies explicit typed targets. Keep the command palette keyboard accessible, with visible effective bindings and a Send key action for Tab/arrows/modifiers/F-keys/control chords that a host reserves or aliases.

Host TUI keeps the configurable Ctrl+G prefix proposal with visible pending/timeout state, quoting/double-prefix and nested tmux/screen/SSH collision help. The quoted Ctrl+G must become a control-key intent, not C0 embedded in `CommitText`; current foundation routing is not a complete implementation of the new contract. Paste is never parsed as UI commands. No hidden global keyboard capture or automatic host keymap edits.

The outer host decoder may lose Ctrl-I/Tab, Ctrl-M/Enter, Escape/Alt, keypad identity, release/repeat and consumed-modifier/layout facts. Negotiate enhanced outer reporting only on a certified profile and restore it on normal exit. One Crossterm reader model; no competing raw readers. Escape timeouts are explicit UI heuristics, not proof. Missing host facts produce a disclosed limited event or refusal, not fabricated physical input. An explicit Send key action still lets a human request a supported inner logical chord. Electron needs real keydown/keyup/IME/preedit and shortcut-collision tests; a synthetic normalizer is not that implementation.

Ctrl-C in application focus requests the inner key; in controls it cancels a local pending UI action. Stop/signal/runtime termination is a named separately reviewed action with observed outcomes. Neither Ctrl-C nor takeover guarantees termination or rollback. Text/IME commits, key events and paste remain distinct through both clients and the engine's negotiated input policy.

## Text, rendering, selection and accessibility

Recommended N1 Electron starting point: a virtualized DOM terminal viewport using owned canonical cells/runs and explicit grid geometry, with accessible chrome/reading projection. Browser shaping/font fallback is reused; no browser terminal emulator is embedded. Test grid occupancy, cursor and copying before optimizing. A canvas/WebGL renderer can replace painting only if measured DOM limits justify it and the same semantic accessibility/selection contract survives. Do not build two renderers in advance or claim a DOM node per cell will satisfy G6.

Default fonts are ordinary monospace with fallback; ligatures/icons are optional and cannot change canonical occupancy. Keep default/indexed/RGB colors and cursor visibility/contrast distinct. Themes offer dark/light/high-contrast/monochrome and text labels rather than color-only status. Test zoom/DPI, fallback glyphs, combining/wide/emoji at cell edges, active/alternate screens, resize and selection across reflow. Unknown glyph rendering is visible degradation, not a new cell-width calculation in the frontend.

Ratatui paints canonical cells into the outer terminal, respecting color/Unicode/ASCII fallback tiers. The outer emulator controls fonts, widths, zoom and screen-reader exposure. Host TERM/terminfo is separate from the engine's **inner** advertised profile. TERM=dumb gets linear control/read mode, not fake full-screen support. Width mismatch disables unreliable coordinate interaction and marks that host/profile degraded; text inspection does not certify an unusable editor. SSH latency/host recording remain outside local latency/capture guarantees.

Selection anchors cite session/buffer/logical-line/column and revision, with explicit stale handling after erase/reflow/eviction. UI text selection and application selection via keyboard/mouse are separate actions. Copy uses an authorized immutable range; search is paged over available authorized history and reports gaps/expiry rather than silently searching only a tail. Clipboard import/export is explicit; OSC 52 writes/reads, links, external opens and file requests never execute from raw output. TUI internal copy buffer works without system clipboard; supported export is optional.

Outer bracketed-paste detection becomes a typed paste event; the core adds only the **inner** wrappers appropriate to canonical mode. Preview multiline/dangerous content, disclose newline transformation, reject delimiter/control injection for ordinary paste and never append Enter implicitly. Lack of trustworthy outer paste separation blocks approval confirmation in that profile, as specified in the existing TUI UX. Protected approvals require a fresh context-bound product challenge, no default Allow, and terminal application input suspended; output resembling the dialog is still data.

Desktop accessibility needs semantic names/roles, focus order, keyboard-only task/review/takeover, controlled announcement of output and a logical reading projection linked to canonical evidence. Test with real Linux accessibility tooling/screen readers, not only ARIA checks. IME candidate placement must track cursor/viewport without sending preedit as committed input. TUI offers linear reading/review and tested host-specific interactive operation; it cannot promise a universal screen-reader tree through ANSI cells. v1 cannot certify either frontend if critical terminal/review/override workflows are inaccessible in its published profile.

## Native-to-legacy journeys

**J1: zero-agent shell.** Start an explicit shell argv in a new PTY session → ordinary terminal viewport → type/edit/Tab completion → run a command → select/search authorized text → detach → reattach read-only to the surviving session → acquire writer. Prompt markers improve navigation if enabled, but apparent prompt return is not a managed exit receipt. N1 proves this without shell hooks.

**J2: native agent work, then human debugging.** Agent starts task T/attempt A, runs an argv test command with pipes, watches separate streams and an exact execution ID, and registers authorized artifact revision R. Human receives one Review item, inspects exit/drain facts and artifact diff, then opens a **new** PTY editor session at that context. A pipe-backed process cannot simply be converted into a terminal-attached process. Validation of R becomes stale if the editor changes it; task acceptance requires a new bound result.

**J3: opaque legacy navigation and takeover.** Agent observes session/buffer/frontier/geometry, sends a menu arrow, then observes a changed selection that remains an inference. An unexpected password/approval prompt or ambiguous transition sends the task to attention, not automatic Enter. Human selects Take control; new writer epoch fences queued agent input, while already delivered bytes remain visible/uncertain. Human uses a protected handoff or interactive terminal, then Resume creates a fresh agent lease/observation. Takeover is not “stop all effects”.

**J4: flood and reconnect.** Several attempts emit output while a human edits one terminal. Parse every admitted byte with fair bounded work; coalesce hidden frames, keep human controls responsive, and mark slow views resync-required. After desktop/TUI restart, a fresh canonical frame restores the view without a second parser or disk transcript. If the engine died in memory-only mode instead, show interpretation unknown and disable all application input; offer inspect/process controls/new session. Do not show a plausible blank screen as “recovered”.

**J5: remote use.** A local managed `ssh` program is an opaque legacy PTY with unknown remote process outcomes on disconnect. A TUI over SSH can use an explicitly installed remote Project X service with remote host/context identity. Later structured transport is separate, authenticated and version negotiated; no helper auto-install or assumption that disconnect killed remote jobs.

## States and transitions

| State | Human presentation / permitted action |
| --- | --- |
| Empty | Start shell or create task; no provider onboarding required; no fabricated last session |
| Loading/attaching | Target identity shown, read-only, progress/cancel attach; no queued input into a hidden target |
| Live/current | Owner/sizing/capture/profile visible; controls governed by grants, not view focus |
| Behind/gap/degraded | Explain lag, retained range and capability limit; resync/inspect; restrict affected actions |
| Review/protected prompt | Trusted product chrome and exact action/context; deny/cancel available; no automatic acceptance from paste or app text |
| Disconnected | Last frame labeled stale with time/frontier; pending operations reconciled by ID; no local mutation authority |
| Engine unknown | Preserve evidence, show process facts separately, refuse application input; no guessed key modes |
| Process exited/output draining | Exit status separate from remaining streams/descendants; bounded wait/close with explicit loss policy |
| Storage/capability/version failure | Explain which guarantee is unavailable; refuse new dependent action, preserve review and separately authorized controls |

Tabs/splits/multiwindow change view layout, not terminal/task identity. One sizing owner; mirrors letterbox/viewport. Resizing invalidates coordinate/composition anchors and is an ordered request, not an atomic application repaint. Restore outer modes on normal TUI exit; SIGKILL can prevent cleanup, so document manual host recovery. Portable declarative config migrates without shell execution or silent import of credentials/session state.

## Measurement contract

Use existing G6/reference hardware and [all-process methodology](../engineering/16-test-and-ci-strategy.md#all-process-performance): eight cores, 16 GiB, SSD, 1080p, exact CPU/GPU/driver/font/scaling recorded. Measure p50/p95/p99/max end to end for native admission/receipts, observed state freshness, human key→actual paint, navigation, resize, takeover fencing and decision updates. Include queueing, worker parsing, IPC, store stalls, rendering, hidden sessions and all product processes. Report outer emulator/tmux and child-tool resources separately as well as the total deployment cost.

TUI-only must run without Electron/Node engine runtime; desktop-only and both attached share the same canonical owner. Output flush is not actual host paint evidence. Model inference/provider latency and application execution latency are separately reported, not credited to or blamed on Rust terminal control. G6 numerical budgets remain provisional targets; N1's lower workload is labeled, not presented as the 32-session result. Test floods, slow clients/storage, cancellation and fairness, not just an idle key loop. No performance result is claimed in this campaign.
