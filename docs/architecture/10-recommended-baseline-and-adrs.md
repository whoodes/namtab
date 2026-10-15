# Recommended baseline and architecture decisions

> Current decision: [ADR E1](../adr/0001-engine-ownership-and-presentation.md) supersedes the xterm browser-replica/Node-host choice with recommended public libghostty-vt and owned presentation. [ADR E2](../adr/0002-terminal-continuity-and-checkpoints.md) separates continuity/capture/process recovery. The [completed assessment](../engineering/19-engine-build-adopt-redesign-assessment.md) establishes bounded proofs only; shipping qualification and dependent product work remain gated.


[Index](../index.md) · [Stack evidence](../research/06-technology-distribution-and-compatibility.md) · [Human UX](../ux/11-human-experience-spec.md) · [Ordered prototype plan](../engineering/12-prototype-plan.md)

Decision date: 2026-10-15. This baseline is best justified for the stated terminal, agent-navigation, human-audit, and small-team requirements; it is not universally optimal. **Accepted-for-design** means a working design decision, not approval to implement. **Recommended** names the default technology/policy to pursue. **Pending measurements** identifies a veto gate. None is implementation-proven. This document resolves prior alternatives; it does not authorize engineering work in this session.

## Executive decision summary

| Area | Recommended baseline and status | Material condition |
| --- | --- | --- |
| Initial human interfaces | Rust Ratatui/Crossterm host TUI **and** Electron/TypeScript/React desktop; recommended | Same broker semantics; PP1–PP3/G8 certify both with honest host capability differences |
| Terminal core/renderer | Official pinned libghostty-vt public C API, one native owner; Project X cell/semantic rendering for both clients; recommended next prototype | Development SDK/codec; real render, key, ABI, accessibility, security/resource and upgrade gates open; old serializer X4 stays failed |
| Systems boundary | Rust supervisor/store/CLI/TUI/keepers and isolated native engine worker; TS Electron UI; accepted-for-design direction | Future audited FFI exception in one worker package only; no native pointer/launch authority in renderer |
| Session lifetime | Independent PTY keeper per session, multiplexed engine host, one local supervisor; accepted-for-design | Keeper survival and engine recovery are tested separately; no resurrection after reboot |
| Initial platform | Ubuntu 24.04 LTS x86_64 userland, Wayland primary and X11 compatibility; recommended | Certify current 24.04 updates and 26.04 LTS; no current-VM capability claims |
| Later platforms | Same frontend/domain on macOS and Windows; native process/credential/runtime adapters; recommended | Platform-specific certification, not POSIX emulation by assumption |
| Restricted execution | Rootless Podman offline OCI profile with pinned, pre-existing image and narrow mounts; recommended | G4 before unattended restricted-work claims; missing prerequisites block admission |
| Capture | Ordinary terminal output stays in bounded memory; opt-in managed/filtered/raw capture with explicit retention; recommended | No promise to recover unrecorded screen state or redact arbitrary secrets |
| Storage/protocol | SQLite local journal/projections, evidence objects, keeper metadata/spool; framed JSON over local authenticated IPC; accepted-for-design | Narrow schemas and bounded payloads; no distributed database or public ABI promise |
| Repository/build | Current six Rust/four TS foundation; revised target adds one justified worker package and keeps desktop/protocol shipping TS; recommended | No production package migration yet; reference xterm/Node packages remain assessment-only |
| Extensibility | Built-in adapters and out-of-process versioned executables first; accepted-for-design | No marketplace, in-process native plugins, arbitrary UI plugins, or generic workflow engine |
| Project license | Apache-2.0 recommended, owner decision pending | No legal license file added; dependencies still require release-specific review |

## ADR-A1 — Frontend and language boundary

**Recommended, pending PP1–PP3/G8.** Deliver two initial frontends. Use Rust Ratatui/Crossterm for a polished configurable host-terminal control TUI; use Electron for a consistent packaged browser runtime and TypeScript/React desktop interface. Both have Terminal/Work/Review/History and the same broker decisions. [13](../ux/13-dual-frontend-and-host-tui.md) owns host input/TERM/paste/SSH limits and legitimate presentation differences; no Zsh, prompt theme or shell hook is required. The review model needs accessible tables, large diffs, structured provenance, rich search, and predictable multi-window behavior as much as a terminal surface. Native GTK/VTE is attractive for a Linux-only terminal, while Tauri reduces the bundled browser choice; both add different integration risks for these combined requirements. Comparative evidence is in [06](../research/06-technology-distribution-and-compatibility.md#desktop-and-terminal-alternatives).

Use Rust for domain/admission, SQLite ownership, workspace/process/PTY operations, CLI/TUI and the proposed native engine worker. TypeScript owns Electron/React human UI and public client code. The worker calls pinned libghostty-vt through a small audited C boundary; Zig is an upstream build/debug dependency, not a new application framework. The separate Node engine host is no longer the recommended shipping path. Node 24.21.0 remains the local build/assessment tool, and Electron has its own embedded runtime. The current workspace is not migrated yet; see ADR E1.

The new cost explicitly includes owned terminal rendering, font/selection/IME/accessibility integration and native FFI qualification, alongside Rust/TS and Electron maintenance. This reverses the previous preference to avoid a renderer after the replica mechanism failed. Neither Ghostty app maturity nor a public C header makes our frontend accessible or the FFI safe. Full core ownership remains a serious funded alternative with staffing/compatibility estimates in 19; it is not selected merely to fix a serializer.

## ADR-A2 — One interpretation authority, replaceable presentation

**Superseded technology choice; current accepted-for-design invariant.** One canonical interpretation owner per session supplies cells, cursor, mode/profile and ordered reply proposals. Both Electron and host TUI render immutable semantic/cell views through public broker subscriptions. The native libghostty-vt worker is the recommended next prototype under [ADR E1](../adr/0001-engine-ownership-and-presentation.md); no frontend parses inner output independently or receives private endpoints.

Logical keys, committed text/IME, paste, mouse, raw fallback, UI commands and generated terminal replies retain separate routes. Encoding uses current canonical modes; Rust admission checks session/incarnation/endpoint/lease/observation/profile/geometry/focus generations and rechecks relevant revisions after asynchronous work. A native agent never steals desktop focus. No generic browser onData or high-level library input callback is a PTY tunnel. Mode 1035 and keypad mode both affect tested keypad Enter; outer host aliases remain explicit.

The original xterm 6.0.0/headless 6.0.0/serialize 0.14.0 family is verified and its serializer X4 still fails. Independent canonical continuations pass; revised xterm with one owner and owned rendering is a viable fallback whose full key/mode seam remains open. Same-build Ghostty snapshots pass the bounded corpus but are not complete durable product recovery. See [19](../engineering/19-engine-build-adopt-redesign-assessment.md) for exact proofs and [ADR E2](../adr/0002-terminal-continuity-and-checkpoints.md) for capture/continuity limits.

The original two-entry/two-file, two-patch/200-line no-semantics-fork adoption budget remains. Owned renderer work is now explicitly accepted for design; it is no longer an automatic veto by itself. Quality, accessibility, input correctness, quotas and maintenance can still veto a candidate. A long-lived fork or complete owned engine needs a new funded ownership decision, not an adapter label. Tauri is only a cause-specific frontend comparison; neither a GTK-only scope change nor a second parser is an automatic fallback.

## ADR-A3 — Supervision and actual process guarantees

**Accepted-for-design, pending PP4.** One Rust supervisor owns domain state, permissions, schedules, and storage. Small keepers own individual PTY masters/process observations and survive UI loss independently. One engine host multiplexes interpretation with per-session fair work limits; do not launch Node/Chromium per agent or session. A process boundary is justified by authority/lifetime, not by every row of the module table.

Separate durable action metadata from optional output spooling. Default memory-only sessions must not leak output into a keeper spool. Keepers retain bounded live prefixes and delivery metadata needed for reconciliation under capture policy. Broker restart invalidates live authority; keepers fence an old/new controller. Engine crash affects interpretation, keeper crash may destroy the PTY, and host reboot loses processes. A terminal can be re-viewed after either frontend is lost only while the needed runner/state survives. “Persistent session” always names that failure boundary.

Native agents use the scoped Project X request/evidence API: launch a known process, inspect stdout/stderr in pipe mode, navigate a PTY, publish artifacts/claims, and propose decisions. Agent output remains untrusted. Legacy agent CLIs use an ordinary managed PTY with optional adapters; tool-level guarantees cannot be inferred from their prompts. A host-side controller is not sandboxed merely because its requested child runs in a restricted runtime. Confining the controller itself requires a verified runtime and gateway profile.

For owned managed launches, promise only tested identity/launch receipts, observed wait status, ordered admitted input, output availability labels, writer fencing, and verified runtime controls. Foreground jobs, detached descendants, remote effects, arbitrary host writers, signal handling, and compromised controllers remain separate concerns. PTY stdout/stderr stay merged; unknown delivery is reconciled rather than retried. Session takeover revokes future input; task takeover also revokes future mediated tool/workspace admissions and requests stopping active attempts. **Neither means all side effects have stopped.**

## ADR-A4 — Linux and eventual desktop ports

**Recommended, pending PP7.** Build against Ubuntu 24.04 LTS userland for x86_64. Certify maintained 24.04 and 26.04 installations; For Electron, Wayland is primary and X11 a tested compatibility path. Host TUI profiles are independently certified and do not require a desktop display server. This is a product support target, not a claim about this VM or every Ubuntu flavor. Native package first; no silent startup-file changes or required account. Exact dependency/kernel patch floors are fixed from the prototype build and security inventory, rather than invented here. Canonical's lifecycle page documents both LTS lines. [Ubuntu lifecycle](https://ubuntu.com/about/release-cycle)

Package aligned core/CLI/TUI (including the qualified native engine) and desktop artifacts under one product tuple; both frontends are initial release scope. Package Linux user-level supervisor/keeper lifecycle explicitly; background service starts on demand, not automatically at login. Closing a desktop view or detaching a TUI releases its endpoint leases and unsent input while processes can continue; desktop last-window quit offers keep-running versus explicit termination. TUI detach restores outer terminal modes on normal exit; a killed client may need manual host recovery. Logout stops local owned work by default, with any unattended/linger mode an informed opt-in. Update only after compatible runners are retained or work is intentionally drained. Do not inherit a service-manager kill policy that contradicts advertised GUI persistence.

Later: macOS Apple Silicon first, Windows 11 x64 first, each with an OS version still supported by the selected Electron release. macOS needs its own PTY/session/credential and sandbox/VM decisions. Windows uses ConPTY and Job Objects where applicable; Ctrl-C, descendants, resize, paths, ACLs and IPC require native contracts. WSL is a separate Linux remote/runtime target, not equivalent to native Windows process control. Restricted Linux containers on macOS/Windows require an explicit Linux VM/remote host; do not relabel them native host confinement. [Microsoft ConPTY](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session), [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

## ADR-A5 — Enforced execution without building a container platform

**Recommended, pending PP5/G4.** First restricted profile: rootless Podman, explicit image digest already available locally, private runtime namespaces, no network, no privileged capabilities, no host IPC/desktop/agent/container sockets, read-only root, owned writable workspace/temp/output mounts, and measured CPU/memory/PID controls. No image acquisition is hidden in an ordinary run: admission uses a no-pull policy and a missing image becomes a preparation decision. Podman documents rootless prerequisites and configurable run controls; actual enforcement is host-dependent. [Podman rootless mode](https://docs.podman.io/en/latest/markdown/podman.1.html#rootless-mode), [Podman run](https://docs.podman.io/en/latest/markdown/podman-run.1.html)

The runtime adapter overrides/reconciles effective defaults, records the resulting mounts/identity/seccomp/network/resource configuration, and rejects an unavailable required control. A seal is a specific read-only source/input profile, not every container. Editing uses an owned writable snapshot/workspace; shared Git administration and host service sockets are not mounted. Scope “no network” to the verified workload namespace; the runtime manager's image acquisition and credential access are a separate operation.

Native controllers can ask for offline child execution without giving that child provider credentials. A networked legacy agent CLI may need broader egress and a credential inside its runtime; that is a distinct explicit grant with exfiltration risk, not an application-layer domain allowlist. First release does not build a universal LLM proxy or promise to mediate every downstream tool call. Badges report controller authority, child runtime and workspace authority separately; an offline child does not make the whole task or host controller “restricted.” A stronger VM profile is later work for a different threat model. If strict support is unavailable, keep ordinary human host terminals available and block restricted task admission; only a new explicit host-mode grant can choose weaker authority.

Move basic G4 validation into P2 for the restricted supervision MVP. P1 remains a useful ordinary human terminal; P3 adds sealed validation and richer isolation/credential cases. This deliberately changes the earlier plan that left all strict execution until P3.

## ADR-A6 — Capture and retention defaults

**Recommended, pending PP4/PP6.** Privacy expectations for an ordinary terminal outweigh the convenience of automatic disk transcripts. This supersedes the earlier filtered-by-default ordinary capture policy.

| Data class | Recommended initial default | Limit / implication |
| --- | --- | --- |
| Ordinary terminal output/screens | Live bounded memory only; no persistent transcript or raw snapshot | Per-session scrollback cap 32 MiB within a shared 256 MiB budget; memory is not secret-proof |
| Opt-in filtered session/managed output | Seven days after explicit session/workspace capture consent | 256 MiB rolling per session, 8 GiB shared transcript cap; transformations/gaps labeled |
| Raw validation evidence/screens | Explicit per-run permission; 24-hour default | A review pin can extend to 30 days with reserved space; never silently renew |
| Artifact review copies | 30 days by default, 8 GiB artifact-store budget | Owned worktrees/user files are not automatically removed by this policy |
| Decision/action metadata | 180 days by default, subject to deletion policy; initial 512 MiB logical store budget plus 64 MiB emergency storage reserve | No routine input values, command secrets, or hidden output; evidence can expire earlier; WAL/backup/compaction overhead measured separately |
| Search/export/crash data | Same or shorter authority/retention than input; uploads off | No unfiltered snapshot, index, core dump, or RPC trace bypass |

Control metadata capacity also constrains new admissions: an unavailable required retention reservation blocks new work or requires an explicit retention change, rather than silently purging promised audit records. The reserve is not a guarantee that failing hardware can persist an emergency action. Managed reservations and pins count against the same transcript budget; if retention cannot be honored, reject admission or ask for a concrete retention change. Ordinary output can roll only under its selected policy. Evidence expiry means unavailable/unauditable, not failed or retroactively successful validation. Shortening retention explains affected review packets. Existing recordings are never made raw by a later default change.

Use private store permissions and recommend OS disk encryption; do not add home-grown application encryption/key recovery to the first release. This does not protect a live compromised account. Credential storage uses an OS credential service when available, otherwise explicit memory-only entry; never a portable plaintext token fallback. Exact credential-service integration remains a port-specific gate. Consent to output capture is separate from model/context export.

## ADR-A7 — APIs, adapters, and portable configuration

**Accepted-for-design.** Framed JSON over local authenticated IPC, bounded binary attachments, typed request/receipt schemas, and negotiated capabilities. One private 0.x protocol initially; pin bundled components together. Support the previous released client only after a public versioned contract exists. Do not promise a permanent public native ABI or preserve every prototype command. Retain documented exports/migration backups so interface evolution does not strand evidence.

Built-in adapters first; out-of-process agent/validator executables next. No arbitrary UI plugins, shared-database access, generic scheduler language, or marketplace. Integration can use CLI/RPC without importing a product SDK. One optional shell adapter follows demonstrated demand; opaque PTYs and native navigation remain complete without it.

Portable user configuration: versioned TOML for theme/font/keybindings/layout preferences, named connection/runtime references, and capture defaults. Local paths, credentials, host trust, capabilities, approval history, active leases, and live sessions are not portable grants. Project-local files are untrusted proposals with a small harmless display/task-hint subset; they cannot enable capture, install hooks, run startup commands, or expand permissions. Validate imports, show effective precedence and changes, apply display settings live, and require new grants for security-relevant changes. See [UX onboarding/configuration](../ux/11-human-experience-spec.md#onboarding-migration-and-portable-configuration).

## ADR-A8 — License and dependency maintenance

**Recommended; owner decision pending.** Recommend Apache-2.0 for Project X source because an explicit patent-license model and permissive integration fit an extensible developer tool. MIT is a credible simpler alternative; copyleft is not rejected as bad, but would express a different distribution objective. No LICENSE file or legal license grant is created. Apache's text is the primary reference, not a blanket dependency clearance. [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0.txt)

Release review covers Chromium/Electron/Node, libghostty-vt/Zig/bundled code (or xterm/addons if shipped), React, Rust crates, SQLite bindings, fonts, runtime tooling and third-party notices. Track a small dependency bill, supported release updates, and vulnerability handling. Budget one upstream engine upgrade rehearsal in PP1 and a clean-machine offline package/rollback trial in PP7. A solo/small team should ship fewer supported platforms/profiles before carrying multiple engines or a long-lived terminal fork.

## ADR-A9 — Two frontends, one authority

**Accepted-for-design.** Neither frontend owns task/grant state or accesses private persistence/managed PTY internals. Local focus/selection/layout are client projections; writer identity includes client endpoint so two views of one human do not imply concurrent input. Both route application keys, raw fallback and approvals through the same broker. TUI outer decoding limitations are negotiated, not patched with guessed inner bytes; desktop/TUI native agent observations come from the canonical engine independently of rendering. Optional prompt integration remains cooperative data. Full parity and legitimate differences are specified in [13](../ux/13-dual-frontend-and-host-tui.md), with DF01–DF08/G8 in [16](../engineering/16-test-and-ci-strategy.md#dual-frontend-contract-cases).

## ADR-A10 — Small workspaces and measured engineering gates

**Recommended.** Keep Cargo + pnpm/node_modules and the actual six-Rust/four-TS bounded foundation. The revised shipping target in ADR E1 adds one justified unsafe/native worker boundary and avoids a Node engine runtime; it does not require empty new packages now. Generated bindings, one product version, separate schema/profile versions and dependency-aware checks remain. See [14](../engineering/14-repository-and-engineering-practices.md), [15](../engineering/15-dependencies-and-supply-chain.md), [16](../engineering/16-test-and-ci-strategy.md) and [19](../engineering/19-engine-build-adopt-redesign-assessment.md) for current versus proposed infrastructure.

## Coherent stopping point

The design now has a default rather than an equal-weight list of possibilities. The unresolved questions are bounded feasibility, human validation, security certification, and owner licensing/staffing decisions. PP1 is deliberately first because failure could change the frontend/core recommendation; later prototypes must not conceal that failure by weakening I1–I14. All prototypes below remain future authorized engineering work, not experiments performed during this research session.
