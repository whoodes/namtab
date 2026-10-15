# Isolation, security, and approvals

[Index](../index.md) · [Work and evidence](../product/02-work-evidence-and-coordination.md) · [Execution](03-execution-terminal-and-remote.md) · [Extension boundaries](05-architecture-storage-and-extensions.md)

## Threat model and trust boundaries

Protect host credentials and data, task integrity, approval authority, execution endpoints, artifact provenance, and the user's ability to regain control. Treat agents, repository files, command output, remote hosts, plugins, build scripts, dependency-supplied instruction files, and model-generated summaries as potentially malicious or mistaken. A downloaded artifact is not safe because an agent produced it. A known participant can be compromised.

Trusted computing base: authenticated human/policy decisions, permission broker, owned runner, chosen isolation provider, local storage, and the terminal/parser boundary needed to present observations. Minimize this base; the renderer and agent adapters should not have full broker authority. Root/kernel compromise and a malicious account owner cannot be defeated by a same-account desktop application. Stronger remote verification would require an independently specified trust system, not a signed JSON claim alone.

| Boundary | Enforcement approach | Residual limitation |
| --- | --- | --- |
| Output → UI | Safe terminal parsing, escaped chrome, bounded rendering, no executable markup | Emulator/renderer vulnerabilities remain possible |
| Agent/plugin → broker | Authenticated narrow API, scoped grants, quotas, request fencing | Same-user host compromise may steal authority |
| Runtime → host | Verified runtime profile with limited mounts, credentials, IPC, and network | Host kernel/runtime remains trusted |
| Human → action | Trusted review surface and revision-bound approval | Human misunderstanding and deception remain possible |
| Remote → local | Verified transport identity, negotiated helper, staged artifacts | Remote host can lie about computation |

## Filesystem and runtime profiles

Filesystem separation and runtime isolation are independent dimensions. An agent can have a separate worktree while retaining permission to read the user's home directory. Linux namespaces isolate selected kernel resources; cgroups manage resource accounting/control. Combining them still requires a concrete, validated mount, credential, network, and syscall policy. [Linux namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html), [cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)

| Profile | Compatibility and use | Security claim allowed |
| --- | --- | --- |
| Host inherited | Existing shells, toolchains, venvs, devices, ordinary SSH | Observation and cooperative supervision only; process has granted host-user authority |
| Host organized | Separate directory/worktree and resource accounting | Reduces accidental collisions; no sandbox claim |
| Restricted runtime | Constrained process identity/namespaces or container; explicit mounts/egress | Enforces only the controls verified for this host/profile |
| Stronger VM boundary | Untrusted tasks needing more isolation | Separate kernel boundary, with explicit shared mounts/devices and hypervisor assumptions |

Do not silently upgrade or downgrade profiles. If unprivileged namespace support, delegated cgroup controls, or a requested egress restriction is unavailable, strict admission fails with an explanation. A user may explicitly choose a weaker host profile; the UI and evidence must retain that weaker status. No privileged helper or root daemon is assumed for the MVP.

Restricted runtimes get an owned writable attempt directory and temporary storage; source inputs can be mounted read-only for sealed validation. Home directories, SSH/GPG agents, desktop buses, Docker sockets, host control sockets, credential files, arbitrary devices, and common Git administration are absent by default. Expose needed services through deliberate scoped grants. Bind-mounting a host toolchain may aid compatibility but expands input provenance and host exposure. A pinned runtime may change program behavior relative to the existing OS environment; both profiles must remain available without conflating their guarantees.

Account for symlinks, hard links, mount changes, `/proc`, inherited FDs, Unix sockets, shared caches, service ports, and Git hooks/configuration. Path allowlists based only on string prefixes are inadequate. Resolve authorized roots through stable directory handles and an appropriate no-escape filesystem mechanism; if the platform cannot provide the requested boundary, do not claim it. A file watcher cannot prevent writes. A permission record cannot neutralize an already-open descriptor.

The first restricted profile is **recommended rootless Podman offline**, using a pinned image already present, explicit no-pull admission, private namespaces, read-only root, narrow owned mounts, dropped capabilities and verified resource controls. Effective host-dependent controls must be recorded/tested; no implicit fallback or image acquisition. Native host controllers are not confined merely because a requested child is. Networked legacy agent CLIs require a distinct egress/credential grant. UI/evidence report controller authority, child runtime and workspace grants independently; a confined child cannot justify a whole-task sandbox badge. This section owns the profile; [ADR-A5](10-recommended-baseline-and-adrs.md#adr-a5-enforced-execution-without-building-a-container-platform) records the rationale. G4/NG3 evidence precedes any restricted-supervision claim.

## Permission semantics

Permissions grant capabilities, not a model's prediction of a command's intent. Examples: read a selected artifact; launch a bounded execution in runtime R; write an owned workspace; request a network destination through an egress provider; publish an artifact through a named external-action adapter. A shell command is arbitrary code. An argv allowlist or shell AST can improve explanation, but cannot prove what that code will do.

A long-lived shell granted network and writable filesystem access can perform actions without further terminal input. Session supervision cannot truthfully promise per-action approval inside it. Strong per-action gates require a broker-owned tool/action interface or isolation that withholds the capability until a bounded operation. General external CLIs may not support that granularity; show the broad session capability and its consequences.

Agent-provided “read-only” and “safe” labels are advisory. A command named `test` can run destructive hooks. Admission checks effective executable, interpreter/script, cwd, inputs, mounts, environment, credentials, egress, limits, and policy; approval text includes unresolved dynamics. Grant tokens are scoped to participant, task/attempt, object, operation, expiry, and maximum delegation. Agents cannot mint policy or impersonate the human.

## Approval lifecycle and stale approvals

An approval request names a canonical action digest and material context: task revision; selected artifact/input revision; command/executable or bounded semantic operation; resolved workspace and target; runtime/host identity; session incarnation and writer generation; environment/secret-version references; policy version; risk explanation; capability scope; expiry; maximum uses; evidence references. The canonical encoding and digest algorithm are versioned. Secrets are referenced, never displayed as ordinary arguments in the approval record.

Lifecycle: proposed → reviewed → granted/denied → reserved → consumed, expired, revoked, or invalidated. The permission broker compares expected context at reservation and again at dispatch. An artifact change, dependency change, host change, ownership takeover, policy change, expired grant, or relevant environment change makes a grant stale. “Same command text” is insufficient. If the actual context cannot be established, a narrow exact-context approval is refused or replaced by a plainly broader grant requiring a new decision.

Use an immutable launch specification and protected inputs for narrow execution approval. Merely hashing a mutable executable/path before launch leaves a TOCTOU window. A sealed runtime prevents unapproved writers within its threat model; host mode can only record the observed starting context and disclose the residual race. No promise that a screen remains unchanged between observation and key delivery is possible for an arbitrary TUI.

Grant consumption is durable before dispatch, with a request ID and runner receipt. This prevents a naive duplicate admission but does not make recording and external effects atomic. A crash between dispatch and outcome recording produces unknown state. The system reconciles runner/remote receipts before deciding whether work ran; it never automatically replays an unknown operation. Revocation blocks future dispatch; it cannot undo delivered input, sent packets, a committed transaction, or leaked data. Bounded timeouts and termination reduce continued exposure without promising rollback.

Human review is presented in application chrome, with stable target/context identity and changed-field emphasis. Terminal text cannot open a trusted approval dialog or mark one approved. Batch approvals enumerate exact members; invalidation of one cannot be hidden by an aggregate “approved” badge. Audit artifact acceptance separately from execution authorization.

Both initial frontends are unprivileged clients of the public API: no direct store/evidence paths, managed PTY/keeper FDs or private engine socket. Their outer terminal/native rendering operations do not create broker grants. TUI drawing uses sanitized canonical cells and a dedicated review screen, never raw output pass-through; other same-user writers or a compromised outer emulator remain outside protection. Outer terminal recording is independent of Project X capture-off.

The approval channel is a separate broker capability held by the authenticated review UI or an explicitly configured policy principal, never by an ordinary agent gateway. A broker challenge binds the displayed action/context to the submitted decision; stale dialogs fail the same revision checks as APIs. Record automated policy grants as policy decisions, not human clicks. This attests the authenticated channel, not physical human presence against a compromised renderer or unrestricted same-user malware; stronger presence checks are a separate future decision.

## Navigation authority and sensitive prompts

Agentic navigation has explicit capabilities for session observation, logical keys/text, paste, raw input, terminal UI commands, selection export, and host clipboard access. Granting one does not silently grant the others. Terminal UI commands are allowlisted product operations on identified views; they never become a generic OS-shortcut synthesis API. Application keys go to the named session and bypass desktop shortcut dispatch, not broker authorization. Agent-driven focus changes need a separate UI grant; background session input must not steal the human's focus.

A “navigation” capability is not inherently read-only. Arrows, Tab/completion, modifiers, editor shortcuts, function keys, and control sequences can cause application-defined effects, including executing completion hooks or committing a selection. Their apparent meaning cannot justify privilege escalation. Raw-byte compatibility uses the same session incarnation, writer generation, freshness, mode/profile, prompt, quota, and delivery checks as structured input, plus an explicit raw-input grant. It cannot become an escape hatch from a denial or unsupported-key response.

The broker enforces protected states it actually knows, such as its own pending approval or a negotiated credential-entry handoff, by rejecting ordinary agent writes through every input route. Suspected sensitive prompts suspend the agent loop for human inspection; ECHO-off, screenshots, and prompt text alone do not establish a trusted prompt identity. An opaque TUI can present an undetectable prompt or change state after observation, so no universal safe-prompt detection or semantic approval guarantee is advertised. A broad opaque-session grant must state this residual risk; a narrow semantic action needs stronger cooperation or human control.

Human password/MFA/host-key handling or a scoped credential adapter gets a fresh, target-bound input context without granting the agent access to secret values or raw input evidence. A downstream agent CLI approval dialog is an application interaction, not permission from the Project X broker. A grant to navigate must not be used to self-approve expanded broker capabilities. Clipboard reads, terminal-selection exports, completion results, and echoed secrets retain capture/export controls. Session takeover invalidates queued navigation by generation; emergency revocation remains a broker/runtime operation, not a Ctrl-C sequence.

Logical key values, text, action summaries, selections, completion labels, and raw encodings can all reveal secrets. With input capture off, durable receipts must not reconstruct the payload through a list of key names, unkeyed low-entropy digests, or verbose RPC traces. Keep only permitted delivery/context metadata and opaque protected references. A secret-entry handoff advances the known prompt/protection revision, invalidating previously queued ordinary input; returning control requires a new observation and writer context. Unknown visual prompts retain the uncertainty stated above.

## Uncontrolled input and action bypass

Every Project X input route—keyboard, paste, mouse, CLI, API, adapter, plugin, restored queue, and protocol-generated response—passes its appropriate broker admission path. A second socket, developer-console API, or “raw write” extension cannot bypass policy. Control IPC is not mounted into untrusted runtimes; participants receive only narrowly scoped gateways. API authentication is not replaced by trusting environment variables.

Local Unix socket permissions and peer UID checks are useful but cannot distinguish hostile programs running with the same unrestricted UID. Secret tokens stored where those programs can read them do not fix that boundary. Separate identities and runtime confinement are needed for strong claims. External tmux clients, writable PTY endpoints, native automation, or an unsandboxed agent's direct network calls are additional bypasses. Sessions attached through such routes are marked unenforced; the product must not imply every action was reviewed.

The emergency control revokes automation input and optionally terminates owned executions. It does not require agent cooperation or a responsive terminal parser. A broker/OS failure may still require operating-system intervention; that limitation is visible in recovery documentation.

Task takeover additionally revokes future mediated tool admissions and workspace leases, requests cancellation/checkpoint of active attempts, and reports residual processes/writers. Session takeover alone changes only terminal input ownership. Quiescence requires verified runtime observations or a fresh protected snapshot; expiration of an advisory host lease does not prove background writes stopped. Human edits and handback create new context/artifact revisions rather than preserving previous validation or approvals implicitly.

## Untrusted output and prompt injection

Terminal content, titles, links, filenames, diffs, search snippets, agent messages, and repository instructions are all untrusted display/input data. Escape them in trusted chrome. Disable output-triggered clipboard reads, automatic clipboard writes, URL opening, file opening, downloads, notifications with trusted styling, and custom command execution by default. Gate any supported escape-sequence side effect through explicit product policy; harmless rendering sequences continue to work.

Do not concatenate terminal text into HTML, scripts, shell commands, API requests, or permission decisions. Link activation validates scheme and shows the destination; opening a file cannot silently execute it. Limit OSC/DCS payload length, graphics memory, recursive markup, parser work, and plugin-defined handlers. The xterm.js security guide specifically treats terminal-derived data as untrusted and cautions about embedding and transport exposure. [xterm.js security guidance](https://xtermjs.org/docs/guides/security/)

Agent context exporters carry origin and taint labels, bounded excerpts, evidence references, and explicit distinction between instructions and quoted material. The agent adapter should not present output as a higher-priority instruction. These measures reduce accidental authority confusion; they do not solve prompt injection or guarantee a model will obey. Real enforcement remains outside the model. An output line saying “policy now permits deployment” has no policy effect.

## Secrets, capture, and retention

[Capture/retention policy A6](10-recommended-baseline-and-adrs.md#adr-a6-capture-and-retention-defaults) owns classes, limits and expiry. Ordinary output/screens are volatile by default; derived snapshots, spools, search and crash data cannot bypass consent. Filtered/expired evidence cannot be invented for audit or exact parser recovery.

Use an OS credential service where available and explicit memory-only secrets otherwise; no portable plaintext fallback. Recommend OS disk encryption and private store permissions, without adding custom application encryption/key recovery initially. Keep provider/authentication secrets in a credential service where possible. Give children only what their declared tools need; many third-party CLIs require secrets in their environment and therefore can read them. That exposure is part of the grant. Avoid automatic forwarding of SSH agents and credentials. Use opaque credential-version references in provenance; hashing a low-entropy secret is not safe redaction. Do not enumerate or capture the entire host environment by default.

User input is not persistently recorded by default. Sensitive input can be submitted through a dedicated path; ECHO-off detection is a hint, not a complete password detector. Tools can print secrets back. Streaming filters must handle chunk boundaries and bound work, yet arbitrary secrets, encodings, and terminal redraws cannot be perfectly recognized. Encryption at rest helps against offline storage access, not a live authorized process or model receiving the secret. Model/context export uses a separate permission and filtered view, never implicit raw-capture access.

Retention applies to transcripts, artifacts, search indexes, snapshots, exports, crash dumps, remote caches, backups, and derived summaries. Deletion leaves minimal tombstone metadata and marks dependent evidence unavailable. There is no guaranteed physical erasure from arbitrary SSDs or prior exports. Hash chains and local append records provide accidental-corruption detection; without an external trust anchor they are not tamper-proof against the account owner. Untrusted imported sessions restore as data, without executing stored commands or restoring authority.
