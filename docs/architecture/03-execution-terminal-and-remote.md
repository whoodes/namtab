# Execution, process ownership and remote sessions

[Index](../index.md) · [Native contract](../engine/native-contract.md) · [PTY/pipe semantics](../engine/compatibility-and-core.md#execution-boundary) · [Isolation](04-isolation-security-and-approvals.md)

**Accepted-for-design boundaries; lifecycle/platform implementation pending.** This page owns OS launch/process control, optional integrations and remote transport distinctions. Terminal parsing/input is owned by the core specification, not repeated here.

## Frontend and host-terminal boundaries

Both clients use the public service API. Their rendering/focus is distinct from PTY ownership; the host TUI's outer terminal I/O is not managed application I/O. [Frontend integration](../engine/frontend-contracts.md) and [host capabilities](../ux/13-dual-frontend-and-host-tui.md) define those seams. Optional prompt integration is cooperative data, not a frontend or authority.

## Execution modes

| Mode | Reliable observations | Deliberate limitation |
| --- | --- | --- |
| Interactive PTY / managed PTY executable | Managed launch, ordered observed terminal stream, geometry and observed leader status | Merged stdout/stderr; inner shell jobs and widget state remain uncertain |
| Managed pipes | Launch argv/context, separately indexed stdout/stderr and EOF, observed leader exit/signal | No exact cross-stream write order; isatty/buffering differs; cannot convert a running pipe process into a PTY |
| Attached external endpoint | Only facts the SSH/tmux/other adapter exposes | May lack exclusive input, process ownership, completeness or recovery |

Choose explicitly. Existing programs may need `/dev/tty`, inherited environments/venvs, termios, job control, locale or library paths; host mode preserves those possibilities with host-user authority. [Core execution boundary](../engine/compatibility-and-core.md#execution-boundary) owns PTY setup/error/stream semantics and pipe stdin scope.

## Process and PTY ownership

A small runner owns each session's master FD, child lifecycle, input queue, output offsets, and durable session identity. It remains independent of the GUI and broker where practical. The launch path allocates a PTY, establishes a child session and controlling terminal, sets foreground process-group behavior, duplicates the slave onto the intended descriptors, closes unintended inherited FDs, then executes the selected shell/program. Exact APIs depend on the chosen runtime/library; this sequence must pass job-control tests rather than be assumed correct from a wrapper API. [setsid manual](https://man7.org/linux/man-pages/man2/setsid.2.html)

Interactive shells create and switch foreground job groups. The shell leader PID is therefore not always the recipient for Ctrl-C. Delivering an interrupt character through the PTY invokes the active line discipline when enabled; directly signaling a foreground group is a separate operation. Raw-mode programs may interpret control bytes themselves. Expose “send interrupt,” “signal current foreground job,” and “terminate owned session” as distinct operations with clear effects.

**Ctrl-C does not guarantee termination.** Its encoding and effect depend on the input profile, terminal mode, and application handlers; the process may ignore it, catch it, or treat it as data. Even a delivered SIGINT is not an exit result. Observe process/job evidence before claiming cancellation, and use separately authorized lifecycle control when escalation is needed. Never implement human takeover or emergency revocation as an unverified Ctrl-C macro.

Track leader identity using stable process handles where supported, plus boot/session identity; a bare PID is insufficient after reuse. Linux pidfds can address process identity, but do not by themselves manage an entire descendant tree. [pidfd_open manual](https://man7.org/linux/man-pages/man2/pidfd_open.2.html)

Managed execution states: prepared → admitted → starting → running → stopping → exited or unknown. Leader exit, PTY hangup, and all descendant termination are separate observations. A descendant may retain the slave FD after the shell exits. An intentional daemon can detach from the process group/session. Where available, an owned runtime cgroup provides a wider accounting/termination boundary; ordinary host process groups cannot promise complete containment.

Termination first revokes future automation input, then requests graceful completion, then escalates according to an explicit timeout policy. Revalidate process/group identity before signaling. Report remaining descendants and unavailable controls. Never kill by process name, an old PID alone, or an unverified “session is ours” guess. Agent cancellation and process termination have separate receipts; neither rolls back effects.

## Navigation contract references

The following stable anchors route older citations to the single current owner. They are not alternative API definitions.

### Agentic terminal navigation

The [native observe–act–observe contract](../engine/native-contract.md#observeactobserve-races-and-human-override) owns target/lease/freshness checks, partial/unknown receipts and non-idempotent retries. [Core input policy](../engine/compatibility-and-core.md#input-policy) owns the required structured keys, raw fallback and mode-aware encoding. Navigation is core, but opaque application semantics remain inferred.

### Structured keys versus raw bytes

Structured intent is primary; raw is a separately granted bounded fallback, with the same admission checks. [Input policy](../engine/compatibility-and-core.md#input-policy) compares their limits. Neither is an OS shortcut-synthesis API.

### Input modes, layouts, and encoding

[Core input policy](../engine/compatibility-and-core.md#input-policy) owns cursor/keypad/NumLock/keyboard negotiation, layouts, consumed modifiers, composition, phases and Escape ambiguity. Host decoding limits belong to [the TUI profile](../ux/13-dual-frontend-and-host-tui.md#input-modes-and-nesting).

### Observe-act-observe and transition evidence

Use the [native contract](../engine/native-contract.md#observeactobserve-races-and-human-override). A backend revision fence does not atomically compare an opaque widget with key consumption. A timeout or unchanged grid is not permission to repeat Enter.

### Selection, paste, and sensitive prompts

[Core policy](../engine/compatibility-and-core.md#input-policy) owns input encoding; [human UX](../ux/11-human-experience-spec.md#rendering-selection-paste-and-scrollback) owns selection/preview behavior; [security](04-isolation-security-and-approvals.md#navigation-authority-and-sensitive-prompts) owns protected prompts and secret authority.

### Input, ownership, and takeover

[Native authority](../engine/native-contract.md#objects-and-authority) owns the client-bound writer lease. [Single reply authority](../engine/compatibility-and-core.md#single-reply-authority) is distinct from that writer. [Human takeover](../ux/11-human-experience-spec.md#takeover-resume-and-recovery-states) distinguishes future-input fencing from stopping task effects.

### Terminal emulation and uncertain state

Own the compatibility core under [E4](../adr/0004-owned-agent-native-engine.md); do not adopt another emulator as hidden authority. [Complete state](../engine/compatibility-and-core.md#complete-semantic-state) and [recovery](../engine/runtime-persistence-security.md#precise-v1-recovery-contract) distinguish a live presentation, owner loss and process loss. Unknown interpretation disables all application input, including raw fallback.

### Resize and Unicode

[Unicode/reflow](../engine/compatibility-and-core.md#unicode-reflow-and-rendering-boundary) owns canonical occupancy; [view geometry](../ux/11-human-experience-spec.md#tabs-splits-windows-and-geometry) owns one sizing authority. PTY resize notification and application repaint are asynchronous.

### Partial output and backpressure

[Runtime ordering/fairness](../engine/runtime-persistence-security.md#ordering-fairness-and-budgets) owns bounded queues and parser frontiers. Coalescing frames never drops canonical input; capture loss and parser loss have different consequences.

## Legacy shells and optional adapters

Baseline support launches the user's selected shell with explicit login/interactive flags and environment policy, without rewriting startup files. Existing activation scripts, CLI tools, and OS/library environments continue to work in host mode. Shell integration is opt-in, reversible, versioned, and constrained to a negotiated feature set such as cwd hints, activity intervals, or command status reports.

Prompt/OSC markers are spoofable by arbitrary terminal output. Even a random marker can leak or be copied; it is not a security boundary. A dedicated side channel improves attribution but inherits the shell's trust and credential exposure. Hook reports remain cooperative evidence. The externally managed runner is authoritative about its own launch and wait result, not every command entered into an arbitrary shell. Bash, Zsh, Fish, Nushell, and POSIX shells do not share one lifecycle protocol; support is negotiated per adapter.

Agent adapters expose task correlation, structured tool requests, cost reports, progress, and evidence references when supported. An opaque CLI still runs without them. Adapter silence, version drift, duplicate callbacks, and crashes downgrade semantic fidelity; they never stop basic terminal rendering or imply task completion.

## Local, SSH, and remote modes

| Mode | Location of execution truth | Disconnect/recovery contract |
| --- | --- | --- |
| Host TUI through SSH to installed namtab | Backend/keeper local to that remote host; SSH carries the outer TUI | Initial explicit deployment pattern; no remote install implied; disconnect/logout/persistence boundaries remain |
| Local owned runner | Local runner and host process observations | GUI restart can reconnect; host reboot cannot preserve processes |
| SSH to an ordinary shell | Remote shell; local runner observes SSH transport | Connection loss leaves remote outcome uncertain; no claimed durable remote process ownership |
| SSH with existing tmux | Remote tmux plus adapter | Persistence depends on remote tmux; output completeness and other clients must be disclosed |
| Structured remote helper | Remote runner journal, negotiated over authenticated transport | Cursor resume and request reconciliation while remote state is retained; remote restart/reboot still limits recovery |

Reuse system OpenSSH initially to preserve user configuration, host-key checking, ProxyJump, and familiar authentication behavior. Do not disable verification or silently enable agent/X11/port forwarding. PTY and non-PTY SSH sessions are different transports; do not send a binary structured protocol through a cooked interactive stream. [OpenSSH manual](https://man.openbsd.org/ssh)

Nested `ssh` inside a shell remains an opaque application unless a cooperative adapter provides more context. Cwd/title/host markers cannot prove the actual host. Show configured transport identity separately from claimed current execution context. Remote helpers require explicit future installation and negotiated versions. Artifact transfers are staged, hashed, quota checked, and explicitly promoted, never automatically synchronized over dirty local changes.

Ownership and approvals bind to the remote host key/helper identity, session incarnation, and context revision. A reconnect does not silently transfer authority to a new host or resurrect old queued input. Record send/ack/result separately; network loss after a request may leave its outcome unknown. Resume observation first, reconcile the request ledger, and require a new decision where necessary. Moving task metadata or artifacts between hosts is possible; moving a live kernel process is not a promised feature.
