# Execution, terminal, and remote sessions

[Index](../index.md) · [Evidence model](../product/02-work-evidence-and-coordination.md) · [Isolation](04-isolation-security-and-approvals.md) · [Recovery](05-architecture-storage-and-extensions.md)

## Frontend and host-terminal boundaries

Both initial clients attach through the public broker API; neither owns managed PTY FDs or private state. The host TUI reads its own outer terminal and renders canonical cells using Ratatui/Crossterm; it does not parse or pass through inner output escapes. Outer byte decoding and inner canonical encoding are distinct, including lost modifier distinctions, prefix collisions, host paste/query replies and TERM/geometry differences. [13](../ux/13-dual-frontend-and-host-tui.md) specifies those layers. The desktop remains a standalone emulator presentation. Optional prompt integration is neither frontend nor authority.

## Execution modes

| Mode | Intended use | Reliable observations | Deliberate limitation |
| --- | --- | --- | --- |
| Interactive PTY | Shells, editors, debuggers, REPLs, agent TUIs | Ordered observed terminal bytes, geometry, managed session leader status | No recovered stdout/stderr separation or universal command boundaries |
| Managed pipes | Noninteractive jobs and validators | Separate stdout/stderr, explicit launch arguments, leader exit/signal | Programs may buffer differently or change behavior without a TTY |
| Managed PTY launch | A known interactive executable per execution | Explicit launch identity plus PTY observations | Inner shell jobs and interactive application state remain uncertain |
| Attached external session | SSH/tmux/other imported endpoints | What the adapter actually exposes | May lack process, completeness, input-exclusivity, or recovery guarantees |

The mode is selected explicitly, not switched silently to manufacture better logs. An application may require `isatty`, `/dev/tty`, job control, terminal queries, inherited file descriptors, locale, or an existing virtual environment. The terminal must preserve those semantics in the compatible mode and disclose the authority tradeoff of using the host environment.

**PTY merging:** when stdout and stderr are connected to the same PTY slave, the master sees one terminal stream. Coloring, shell annotations, or string analysis cannot recover the original descriptors. PTY evidence is labeled `pty`, including echo and terminal output transformations. Redirecting stderr to a pipe changes semantics and is a distinct launch profile. Pipe stream timestamps reflect read observations; separate pipes do not establish the application's exact cross-stream write order. The Linux PTY and terminal interfaces are the basis for this design. [PTY manual](https://man7.org/linux/man-pages/man7/pty.7.html), [termios manual](https://man7.org/linux/man-pages/man3/termios.3.html)

## Agentic terminal navigation

Navigation is a required execution capability from the terminal foundation onward. It covers shells, interactive CLIs, menus, pagers, editors, REPLs, and agent CLIs without requiring semantic adapters. Agents can use arrows, Tab/completion, Shift-Tab, Enter/Escape, Home/End/PageUp/PageDown/Insert/Delete/Backspace, F1–F12, supported shortcuts/modifiers, explicit control sequences, text, and selection/paste. The required set must pass G7; combinations that a legacy profile cannot distinguish are explicitly reported as aliases or unsupported. Supporting navigation does not mean understanding every application's bindings.

Keep three distinct action planes:

| Plane | Examples and identity | Authority and observable effect |
| --- | --- | --- |
| Terminal UI command | Focus a named Project X view, scroll its history, set a terminal-text selection; target client/window/view and mapping/focus generation | UI capability; no application bytes, no implicit writer acquisition; focus movement needs separate permission |
| Application input | Logical key/chord, text, paste, supported pointer event, raw bytes; target session ID/incarnation and writer generation | Input capability; runner writes the named PTY; does not invoke Project X shortcuts or change desktop focus |
| Process/lifecycle control | Request interrupt, signal a verified foreground group, terminate owned runtime | Separate control capability and receipt; not simulated through a terminal UI shortcut |

Never route automation through “whichever pane is focused.” A direct session-targeted request can operate a background session. If a caller derives its target from a view, it must include that view's mapping/focus generation, and remapping/focus change invalidates the request. UI focus is different from application-internal widget focus, which remains uncertain. Explicit session targeting prevents a desktop focus change from silently redirecting input; it cannot prove which widget or editor mode will consume it.

### Structured keys versus raw bytes

The [engine assessment](../engineering/19-engine-build-adopt-redesign-assessment.md) now recommends a public native encoder behind the same contract. Its tests show that mode 1035/NumLock policy matters alongside application keypad mode. Do not reduce the encoding profile to one cursor/keypad flag or infer a parser bug from a mismatched key oracle. No outer host can recover physical distinctions it never reported; native agent intents remain explicit.


The preferred API accepts logical keys and modifiers, with explicit text and paste operations. This is a proposed Project X contract, not a claim that terminal programs consume these structured objects directly. The runner encodes them to the negotiated terminal wire profile.

| Interface | Advantages | Limits and fallback rule |
| --- | --- | --- |
| Structured key event | Inspectable key/chord intent; one encoding authority; mode-aware cursor/keypad/function-key handling; explicit unsupported combinations | Still becomes bytes; cannot know a TUI's semantic action or disambiguate a legacy protocol beyond its capabilities |
| Text / paste operation | Preserves declared Unicode text and paste boundaries; separates composition from shortcut intent | No guarantee of literal insertion or nonexecution; newline/control handling and bracketed-paste mode must be explicit |
| Raw-byte fallback | Exact permitted bytes for unusual control sequences, legacy bindings, byte encodings, or diagnostics | Caller must know the applicable profile; same target/freshness/ownership/prompt checks, separately granted raw-input capability, no automatic fallback or retry |
| Semantic application adapter | Can name a known operation or verify a state transition | Optional, versioned, and limited to the actual app/protocol; cannot become a dependency for basic navigation |

A structured key carries a logical key, modifier set, optional keypad location, and an advertised event phase. A normal shortcut is a logical press with modifiers, not a fabricated global OS key-down/key-up sequence. Release reporting and distinct repeat phases require negotiated support; otherwise reject an unrepresentable request before writing bytes. Deliberate bounded repeated presses are separate non-idempotent inputs. Unknown keys/modifiers are errors, not silently omitted fields.

The API does not interpret key names as command strings. `Tab` asks the application to handle Tab; it does not compute shell completion itself. Application `Ctrl+Shift+C` is different from a Project X copy command. Text is encoded according to an explicit profile without invisible Unicode normalization or keyboard-layout remapping; control characters require a declared paste/control/raw operation rather than being hidden in an ordinary text field. Layout, IME, AltGr, and physical-key concerns are covered below.

Raw application input is sent toward the program's input, not fed into the terminal's output parser to manipulate the displayed grid. Display control sequences received from the program remain output data. Their similar byte syntax must not collapse the input/output/UI-command boundaries.

## Input modes, layouts, and encoding

Verified protocol basis: xterm documents normal/application cursor modes, numerical/application keypad modes, function/modifier encodings, and bracketed paste. Cursor Up, for example, has different normal and application encodings. These are distinct from display cursor-movement controls. Project X must track the selected terminal profile and observed input modes rather than hard-code one arrow sequence for every program. [xterm control-sequence reference](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)

The proposed input profile records cursor-key mode; keypad mode and relevant key location/lock policy; supported key/modifier map; Backspace/Enter conventions; keyboard protocol/enhancement flags; text encoding; paste mode/newline policy; and its own revision. Changes to observed modes or selected profile advance the corresponding revision. Main/alternate buffer transitions and their keyboard state are tracked according to the selected engine/protocol, without assuming all engines scope modes identically. A missing or partially parsed mode transition leaves mode-sensitive actions uncertain; the broker must not invent a settled mode.

Application keypad Enter and ordinary Return, or keypad digits and text digits, are distinguishable logical events even when some profiles collapse them to the same bytes. Conventional encodings can also alias Ctrl-I with Tab and Ctrl-M with Enter. The capability response must expose representability/alias limitations. Function keys and modified variants use the actual TERM/terminfo/profile contract; emitting plausible bytes does not prove the receiving program binds them.

Escape is both a key and a common sequence prefix. An Alt/meta chord may use an Escape prefix in legacy profiles; a receiving application's timeout, transport fragmentation, or nested multiplexer can change interpretation. Sending an encoded sequence through one runner queue avoids deliberate interleaving but does not create message boundaries at the application. Arbitrary sleeps do not prove disambiguation. An enhanced protocol can improve this only when supported and requested end to end. Kitty's specification describes negotiated disambiguation and optional event-type reporting; adoption is optional, not assumed. [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/)

Logical agent input is independent of the supervisor machine's physical keyboard layout. Explicit text preserves its declared characters; logical shortcuts use the negotiated key/profile semantics. Human OS keyboard events and IME/AltGr composition normalize through a documented frontend path before encoding, rather than silently converting AltGr text into a Ctrl-Alt shortcut. Physical scancode emulation and arbitrary held-key synthesis are not required for MVP. Unsupported Super/meta variants, release events, or non-UTF-8 encodings return a capability error rather than a guessed substitute. Applications with custom bindings still require observation, documented configuration, or an adapter.

## Observe-act-observe and transition evidence

1. **Negotiate and identify.** Obtain session/host incarnation, transport limitations, allowed input kinds, supported key/modifier/event sets, input-profile revision, active modes, observation support, and size/batch limits. Acquire or renew the authorized writer lease. Basic key navigation must work without an agent-specific adapter.
2. **Observe.** Capture an observation ID with the runner's current observed output epoch/offset, grid/cursor, active buffer and buffer generation, geometry revision, mode revision, input-profile/capability revision, writer generation, and any relevant source-view mapping/focus generation. Include completeness, parser-lag, and capture-policy labels. This is the latest parsed prefix, not knowledge of bytes still in a kernel/network buffer or the program's internal state.
3. **Choose a bounded action.** Name the intended transition and evidence that could support it, then submit a targeted key/chord, text, paste, or explicit raw request with expected context and a unique request ID. Prefer one step followed by observation; a bounded batch is appropriate only under an explicit policy and known interaction contract. Do not use a long blind key script as proof of navigation correctness.
4. **Validate and deliver.** At admission and dispatch compare authority, session incarnation, applicable view/focus mapping, screen-prefix precondition, geometry/buffer generation, modes/profile, and approval/prompt constraints. Reject stale or unsupported requests before delivery. Revalidate each not-yet-started event in a batch. Encoding is fixed before its first byte; a partially written event is never re-encoded under a new mode. Fencing may stop the remaining suffix and leave the receiver uncertain.
5. **Observe the receipt and transition.** Distinguish queued, rejected-with-zero-bytes, partial, fully written, and unknown delivery. “Written” means accepted by the transport/kernel boundary, not consumed or understood by the application. Wait for bounded output/adapter/process evidence or a timeout, then take a new observation and assess the intended transition. Cursor motion, changed row highlighting, completion text, or a known adapter result can support a limited claim; none is a universal semantic acknowledgment.
6. **Continue, reconcile, or hand off.** Proceed only with the newly observed context. If the transition is ambiguous, expose uncertainty, inspect a more reliable artifact/process result, or request human takeover. A timeout or unchanged screen is not permission to press Enter again. Budget steps, time, and observation work so an agent cannot spin indefinitely or defeat human override.

A strict screen-prefix precondition rejects a known intervening output change. Some streaming interfaces need an explicit policy that permits output advancement while still fencing identity, geometry, modes, buffer, and ownership; the receipt records the actual dispatch prefix and this weaker condition. A gap or lag that prevents required checks blocks such automation. Under output floods, bounded waiting or escalation is preferable to silently dropping the freshness checks. Neither policy can freeze an opaque application: it can change after validation, have unread output, or interpret an action differently without a visible change. There is no universal compare-and-swap on TUI semantics.

Record prior observation → action intent → actual encoding/profile and permitted delivery metadata → subsequent observations → transition claim. Raw payload retention follows capture authority; secrets are never made recoverable just to complete this chain. File-save or job-completion claims require artifact/process evidence where possible. Two identical grids may conceal different state, and a changed grid may be unrelated asynchronous output. Evidence references preserve those limitations.

Input is non-idempotent. Retrying ArrowDown can skip a row; repeating Tab can accept a different candidate; repeating Enter can submit twice. Repeating the same request ID/body queries or returns the retained receipt without writing again; changing a body's contents under that ID is an error. If the request ledger cannot establish whether delivery happened, return unknown rather than treating the request as new. A fresh request ID after a lost acknowledgment is not a safe retry strategy. Raw fallback, reconnect, and restored queues obey the same rule.

## Selection, paste, and sensitive prompts

Terminal UI selection identifies a view, buffer/geometry generation, and cell/text range. Reading it is an authorized observation/export, not an application selection command or implicit OS clipboard read. Application selection uses that program's keys or negotiated pointer reports and must be observed separately. Scrolling Project X history does not send PageUp to a pager. A selection invalidated by resize/reflow/buffer replacement must be refreshed rather than pasted from stale coordinates.

Paste is a distinct request with an authorized payload source, encoding/newline policy, size limit, expected context, and observed bracketed-paste state. When enabled, the runner adds the negotiated wrappers. When absent, an unbracketed fallback requires explicit policy; it is not silently equivalent. Embedded control codes or paste terminators require rejection under the ordinary paste policy or a newly explicit control/raw grant. Any transformation is shown in the action identity. Partial paste leaves uncertain program state; wrappers cannot be assumed to have completed. Bracketed paste does not prevent a shell/REPL/editor from interpreting content or executing a submission.

For a known or suspected password, MFA, host-verification, or approval prompt, stop the ordinary navigation loop. Use human takeover or a narrowly authorized credential/action path bound to the fresh prompt/session context. Do not let structured keys, raw bytes, Tab completion, or clipboard paste bypass a protected prompt. ECHO-off and visual prompt detection are hints, not authentication or complete prompt detection. A spoofed prompt cannot create authority, and an agent CLI's own permission dialog cannot substitute for a Project X broker decision. [Security and prompt policy](04-isolation-security-and-approvals.md#navigation-authority-and-sensitive-prompts)

## Process and PTY ownership

A small runner owns each session's master FD, child lifecycle, input queue, output offsets, and durable session identity. It remains independent of the GUI and broker where practical. The launch path allocates a PTY, establishes a child session and controlling terminal, sets foreground process-group behavior, duplicates the slave onto the intended descriptors, closes unintended inherited FDs, then executes the selected shell/program. Exact APIs depend on the chosen runtime/library; this sequence must pass job-control tests rather than be assumed correct from a wrapper API. [setsid manual](https://man7.org/linux/man-pages/man2/setsid.2.html)

Interactive shells create and switch foreground job groups. The shell leader PID is therefore not always the recipient for Ctrl-C. Delivering an interrupt character through the PTY invokes the active line discipline when enabled; directly signaling a foreground group is a separate operation. Raw-mode programs may interpret control bytes themselves. Expose “send interrupt,” “signal current foreground job,” and “terminate owned session” as distinct operations with clear effects.

**Ctrl-C does not guarantee termination.** Its encoding and effect depend on the input profile, terminal mode, and application handlers; the process may ignore it, catch it, or treat it as data. Even a delivered SIGINT is not an exit result. Observe process/job evidence before claiming cancellation, and use separately authorized lifecycle control when escalation is needed. Never implement human takeover or emergency revocation as an unverified Ctrl-C macro.

Track leader identity using stable process handles where supported, plus boot/session identity; a bare PID is insufficient after reuse. Linux pidfds can address process identity, but do not by themselves manage an entire descendant tree. [pidfd_open manual](https://man7.org/linux/man-pages/man2/pidfd_open.2.html)

Managed execution states: prepared → admitted → starting → running → stopping → exited or unknown. Leader exit, PTY hangup, and all descendant termination are separate observations. A descendant may retain the slave FD after the shell exits. An intentional daemon can detach from the process group/session. Where available, an owned runtime cgroup provides a wider accounting/termination boundary; ordinary host process groups cannot promise complete containment.

Termination first revokes future automation input, then requests graceful completion, then escalates according to an explicit timeout policy. Revalidate process/group identity before signaling. Report remaining descendants and unavailable controls. Never kill by process name, an old PID alone, or an unverified “session is ours” guess. Agent cancellation and process termination have separate receipts; neither rolls back effects.

## Input, ownership, and takeover

Each session has a broker-controlled writer lease `(participant, client endpoint, generation, expiry)`. Every text, key, paste, mouse report, resize-authority change, and automation input request names the current generation. Observers cannot inject input. View focus does not acquire control. Atomic takeover advances the generation, revokes old input admissions, and rejects queued unsent writes. Already written bytes and partially delivered input remain recorded as delivered or uncertain; they cannot be recalled.

One session writer queue handles partial writes and preserves byte order for accepted requests. A request identifies its exact bytes or key encoding profile, expected screen/context revision where relevant, and action/approval reference. A timeout after some bytes were written is not safely retryable. A new controller sees the delivered prefix and must reconcile the application state. Inputs containing secrets have delivery metadata without routine plaintext retention.

Paste and key APIs from both frontends converge on this path. Bracketed paste is encoded only if supported by current terminal mode; it is not an execution safety feature. Embedded newlines or a program's paste handling can still cause actions. IME composition is committed once. A key named “Enter” is encoded according to the active input profile, not replayed against an unrelated later screen.

Terminal-generated device/status replies also write input. The canonical emulator produces only bounded protocol replies through a restricted reply lane; output cannot request arbitrary broker actions. Replies cannot include clipboard contents or authority tokens. This lane remains distinct from user/agent input and cannot be used by an adapter to bypass the writer policy.

In a normal same-user host session, another process or external multiplexer client may bypass Project X input control. Do not advertise exclusivity against that threat. Strict runtimes restrict endpoint access and alternate control channels; attached sessions without those properties are explicitly labeled. See [uncontrolled bypass](04-isolation-security-and-approvals.md#uncontrolled-input-and-action-bypass).

Session takeover changes terminal input ownership; it does not stop an agent's other tool calls, background edits, or already admitted processes. Offer a separate task takeover that revokes future mediated actions and coordinates execution/workspace ownership. Show any continuing effects and unenforced host writers before claiming the task is quiescent.

## Terminal emulation and uncertain state

Reuse a mature emulator core. The canonical parser consumes byte chunks, not decoded log lines. It handles incomplete escape sequences, split UTF-8, invalid bytes, carriage-return updates, tabs, combining sequences, cursor motion, scroll regions, colors, modes, and primary/alternate buffers. No agent hook or regular-expression output filter sits in the compatibility path.

One canonical interpretation authority per session controls modes and query replies. [ADR E1](../adr/0001-engine-ownership-and-presentation.md) now recommends the public native Ghostty core and semantic/cell rendering for both views; the xterm single-owner redesign is a fallback. Neither view parses inner output or independently replies. Every subscription and input request passes through broker authority, with no frontend private endpoint/PTY access. Human/agent keys, committed text/IME, paste, mouse, raw fallback and generated replies keep distinct routes. [19](../engineering/19-engine-build-adopt-redesign-assessment.md) records bounded proof and remaining gates; no full navigation certification is claimed.

Clients attach at explicit prefixes using a validated presentation snapshot plus subsequent bytes. Framebuffer serialization is not a complete canonical checkpoint: canonical recovery requires proven complete state/prefix or becomes interpretation-unknown. A terminal engine migration creates a new interpretation version and never silently reinterprets historical citations. A fallback text inspector is useful degraded inspection, not universal recovery or an accessible-terminal substitute. Lost canonical state disables application input, including raw fallback, until valid state is restored; separately authorized process controls and creation of a new session remain available. There is no initial guessed-mode recovery bypass.

An agent's screen read returns the observed output offset, geometry revision, active buffer, cell content, cursor/modes, confidence, and a timestamp. It cannot certify whether a TUI is busy, which widget has focus, whether an Enter key will approve something, or whether another process will emit output next. Image/OCR and prompt heuristics remain uncertain. No “idle for 500 ms” rule establishes a safe action boundary. Risky input into an uncertain TUI requires human takeover or a bounded session-level grant explicitly accepting that uncertainty; a narrow semantic action requires a cooperative adapter or managed API.

Alternate-screen content is not ordinary scrollback. Keep its current state and optional retained snapshots; returning to the primary buffer restores that buffer according to the engine's semantics. Raw history, if retained, can explain transitions, but cannot be flattened into a reliable command transcript. Copy/export states which buffer and time it represents.

## Resize and Unicode

One explicit sizing authority chooses rows/columns; observers with different window sizes scale/letterbox or request control rather than continually resizing each other. The runner records requested and applied geometry revisions and updates the PTY window size. Linux window-size changes can notify the foreground group through SIGWINCH. Output repaint and resize are not atomic from the application's perspective. [TIOCSWINSZ manual](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html)

Debounce layout churn without discarding the final size. Include resize events in interpretation history. After a resize or active-buffer transition, stale screen-relative clicks/coordinates are rejected; a new observation is needed. Hidden sessions retain their canonical geometry. Reflow behavior and scrollback anchoring must be specified and tested per engine; evidence byte offsets remain stable even when display rows move.

UTF-8 is the initial display encoding. Invalid input is preserved where capture policy permits and displayed through a documented replacement/escape policy. Track code units, grapheme boundaries, terminal cells, and byte offsets separately. Pin the Unicode and width profile used for terminal interpretation; support configurable ambiguous width, font fallback, emoji sequences, combining marks, and safe visualization of bidi/control characters in review chrome. The terminal's width policy will not match every application or font. Unicode explicitly requires tailoring for terminal width, while grapheme segmentation is a separate concern. [Unicode UAX #11](https://www.unicode.org/reports/tr11/), [Unicode UAX #29](https://www.unicode.org/reports/tr29/)

## Partial output and backpressure

Output is arbitrary bytes with arbitrary chunk boundaries. A stream epoch and monotonically increasing observed byte offsets identify it. Parsing, durable capture, UI delivery, search indexing, and agent context selection are separate consumers with separate cursors. A UI may coalesce rendered frames while the parser still consumes every received byte, whether or not those bytes are retained for history. It cannot drop arbitrary parser input and continue pretending its terminal state is current.

Use bounded memory, bounded disk spools, fair read budgets, and subscriber credit/acknowledgment. If durable storage cannot keep up, the configured policy either backpressures the producer or explicitly switches capture state and records a gap. PTY backpressure can block the program and change timing; make that consequence visible. If parser ingestion is skipped after a gap, the terminal state becomes unknown until an authoritative checkpoint or new session; repaint/capture recovery is adapter-specific and may be approximate. Do not assert that an escape-sequence reset reconstructs a lost TUI screen. Loss of persisted capture alone does not invalidate a canonical parser that independently received the complete stream.

Reserve capacity for revocation, cancellation, and storage-failure events. Slow observers can be disconnected with a resumable cursor rather than starving execution. xterm.js and tmux both document flow-control considerations; their mechanisms are options to evaluate, not evidence that our buffering is solved. [xterm.js flow control](https://xtermjs.org/docs/guides/flowcontrol/), [tmux control mode](https://github.com/tmux/tmux/wiki/Control-Mode)

The outer host-terminal screen/scrollback/recorder is outside Project X capture control. TUI memory-only mode prevents Project X persistence, not independent outer recording. Outer terminal probes occur only in bounded setup/control state, never as application input.

## Legacy shells and optional adapters

Baseline support launches the user's selected shell with explicit login/interactive flags and environment policy, without rewriting startup files. Existing activation scripts, CLI tools, and OS/library environments continue to work in host mode. Shell integration is opt-in, reversible, versioned, and constrained to a negotiated feature set such as cwd hints, activity intervals, or command status reports.

Prompt/OSC markers are spoofable by arbitrary terminal output. Even a random marker can leak or be copied; it is not a security boundary. A dedicated side channel improves attribution but inherits the shell's trust and credential exposure. Hook reports remain cooperative evidence. The externally managed runner is authoritative about its own launch and wait result, not every command entered into an arbitrary shell. Bash, Zsh, Fish, Nushell, and POSIX shells do not share one lifecycle protocol; support is negotiated per adapter.

Agent adapters expose task correlation, structured tool requests, cost reports, progress, and evidence references when supported. An opaque CLI still runs without them. Adapter silence, version drift, duplicate callbacks, and crashes downgrade semantic fidelity; they never stop basic terminal rendering or imply task completion.

## Local, SSH, and remote modes

| Mode | Location of execution truth | Disconnect/recovery contract |
| --- | --- | --- |
| Host TUI through SSH to installed Project X | Backend/keeper local to that remote host; SSH carries the outer TUI | Initial explicit deployment pattern; no remote install implied; disconnect/logout/persistence boundaries remain |
| Local owned runner | Local runner and host process observations | GUI restart can reconnect; host reboot cannot preserve processes |
| SSH to an ordinary shell | Remote shell; local runner observes SSH transport | Connection loss leaves remote outcome uncertain; no claimed durable remote process ownership |
| SSH with existing tmux | Remote tmux plus adapter | Persistence depends on remote tmux; output completeness and other clients must be disclosed |
| Structured remote helper | Remote runner journal, negotiated over authenticated transport | Cursor resume and request reconciliation while remote state is retained; remote restart/reboot still limits recovery |

Reuse system OpenSSH initially to preserve user configuration, host-key checking, ProxyJump, and familiar authentication behavior. Do not disable verification or silently enable agent/X11/port forwarding. PTY and non-PTY SSH sessions are different transports; do not send a binary structured protocol through a cooked interactive stream. [OpenSSH manual](https://man.openbsd.org/ssh)

Nested `ssh` inside a shell remains an opaque application unless a cooperative adapter provides more context. Cwd/title/host markers cannot prove the actual host. Show configured transport identity separately from claimed current execution context. Remote helpers require explicit future installation and negotiated versions; this session installs nothing. Artifact transfers are staged, hashed, quota checked, and explicitly promoted, never automatically synchronized over dirty local changes.

Ownership and approvals bind to the remote host key/helper identity, session incarnation, and context revision. A reconnect does not silently transfer authority to a new host or resurrect old queued input. Record send/ack/result separately; network loss after a request may leave its outcome unknown. Resume observation first, reconcile the request ledger, and require a new decision where necessary. Moving task metadata or artifacts between hosts is possible; moving a live kernel process is not a promised feature.
