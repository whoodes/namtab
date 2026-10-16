# MK1: first hands-on prototype

MK1 is a **working human-feel evaluation**, not full NG1, a final prototype or a production terminal. This record admits bounded engineering against design baseline `a408dbc7e4a4eda51ba668fa93f0235514bd8786`; it does not certify unimplemented behavior. The [delivery contract](../engine/delivery-and-validation.md) retains the later qualification gates.

## Owner decision and responsibility

On 2026-10-16 the user explicitly authorized proceeding until a usable prototype, independent agent reviews and eligible normal PR merges into existing `main` (`Sentinel_f8b7e0e92f40819182c954f7212bdb55`). The later clarification (`Sentinel_552489188e4481919a859a3052a79f69`, corrected by `Sentinel_2bf401b9425c8191950e621a71161f81`) makes **MK1, for human feel**, the stopping milestone. This is the previously missing implementation/owner decision, beyond merely approving N0 document merges.

N0 is accepted **for this bounded design baseline**: issues 7–15 contracts and packet at the revision above, with the exclusions and admission below. This does not turn static fixtures into executed runtime proofs. Codex coordinates implementation; independent contexts review correctness/security/tests and human UX/docs/scope. The human owner retains product, legal, funding, security and later release decisions. A ten-active-working-day checkpoint triggers a status/re-scope review, not an automatic stop or promised delivery date.

Within MK1 only, the owner delegates eligible PR merging after two independent final-revision reviews, resolved blocking findings, exact-head local checks and hosted active-source/governance checks. Normal merge commits preserve focused history. No force push, protection changes or automatic interpretation of review as broader authorization. After a verified MK1 handoff **stop for user feedback**; later work needs a new decision.

## Admitted scope and explicit exclusions

- Existing x86_64 Ubuntu developer VM; local supervised execution, at most two PTY sessions and two managed argv attempts. Capture off by default. Managed stdout/stderr remain separate; PTY streams are merged. Clean-rc Bash/readline is the first interactive target; no inference of command boundaries or success from shell output.
- One project-owned canonical Rust terminal state/input/reply authority. Existing historical xterm/libghostty code supplies comparison evidence only. A minimal Ratatui/Crossterm host TUI and Electron desktop consume the same semantic observations and structured input operations.
- Real typing, command execution, live output, session/task selection, truthful process/connection status, typed agent operations/receipts, explicit artifact revisions, human takeover and live-owner reconnect. No paid model integration is necessary: a deterministic agent client demonstrates the actual protocol.
- A finite implemented subset of `px-dev-text-0`: width-one ASCII/Latin-1 text, bounded ANSI cursor/edit/mode behavior and classic input. Unsupported text/modes must remain explicit; no universal Unicode/TUI/keyboard compatibility. Capability reporting describes what is actually implemented, not every N0 proposed table row.
- Worker/core separation and resource bounds are implementation gates. Developer-host process supervision is **not** a hostile-code sandbox. Same-UID hostile processes, escaped descendants, password detection, security policy enforcement and production runtime isolation are outside MK1 assurance. No terminal text grants authority.
- No durable terminal checkpoint, deterministic execution replay, process resurrection, capture-history restore, full NG1 continuation corpus certification, remote transport, rich editor/pager support, broad accessibility claim, production packaging or performance superiority claim. Preserve their later roadmap obligations.

## Concrete admission tuple

| Item | MK1 admission and limit |
| --- | --- |
| Rust/tooling | Existing pinned Rust/Cargo 1.91.0, edition 2024; Node 24.21.0/pnpm 10.32.1 for existing builds; lockfiles remain reviewed. No legacy core becomes authority. |
| Core/data | Owned bounded byte FSM and UTF-8 continuation using checked Rust std conversion; no external VT parser or Unicode algorithm/data dependency. Width-one profile only. Both screens, partial sequences and unsupported-state flags belong to the core. |
| OS wrapper | `rustix =1.1.5`, default features off, selected `std,pty,termios,process,stdio,event,net` (`pty` implies `fs`). `stdio` enables safe helper FD duplication, `event` bounded polling and `net` kernel peer-UID checks; no new registry package/version is introduced. Existing source/notice hashes are in the [N0 provenance selection](15-dependencies-and-supply-chain.md#n0-primitive-and-data-selection-2026-10-16). Actual spawn/FD/job-control behavior still needs live tests. |
| Serialization/UI | Existing serde 1.0.229, serde_json 1.0.154, sha2 0.10.9; Ratatui 0.30.2 + Crossterm 0.29.0. Existing feature/notice closure remains in the locks and N0 provenance record. No framework/plugin expansion for the demo. |
| Desktop | Official Electron 44.8.0 linux-x64 ZIP SHA-256 `926679355f8bf8d521fc6e1a2dc1c71f11cf176a3a1444f9afbd9e1cdfeddb11`; verified against vendor HTTPS SHASUMS before execution. MIT/Chromium notices retained locally. Same-origin checksum is not independent signature or reproducible-build verification. No new JS runtime package is necessary to launch the verified binary. |
| Geometry/resources | Default 80×24; maximum 160×48 per session; two PTYs/two managed attempts; bounded input/chunks/queues/client count. Exact executable caps must appear in tests and launch docs before handoff; exhaustion refuses/fences rather than silently dropping application input. |
| Wire | Explicit private experimental `namtab.mk1/1` contract, separate from N0 proposed schemas and legacy `public-v1`. Typed operations/results; mutation IDs and receipts; incompatible versions refuse. No stable public ABI or silent reuse of an incompatible envelope. |
| Privacy/recovery | No raw terminal output or application input persisted. Minimal metadata receipt journal may persist IDs/dispositions, never command content. Frames/output remain bounded RAM. Frontend reconnect to a live owner differs from daemon/worker/process loss; old mutations are never automatically retried. |
| Evaluation | This VM, one then two sessions, one then two managed attempts, both clients attached, synthetic output flood and human takeover. Record actual environment and end-to-end timings where measured; no new arbitrary SLO or achieved percentile claim. [G6 budgets](07-validation-roadmap-and-risks.md#performance-and-resource-budgets) remain provisional larger-workload goals. |

Admission permits private prototype use of this reviewed-purpose dependency tuple, not legal/security clearance for distribution. No project license, spending, new credentials, host security/access policy or system configuration is adopted. A new material prerequisite needs its own decision.

## GUI fact and engineering checkpoints

A disposable **real on-screen** Electron 44.8.0 window loaded and painted on this VM with sandbox, context isolation and web security enabled, Node integration disabled; it closed with exit 0. X11 `xwininfo` identification failed; Wayland selection is inferred, not independently compositor-verified. This proves launch feasibility, **not** the application, input, IME or accessibility. Exact commands, checksum/provenance limits and own-window screenshot are in ignored `.artifacts/mk1-start-20261016T213422Z/desktop-preflight.{md,json}`; no security bypass was used.

1. Implement/test bounded owned core and real PTY/managed-process seam independently; keep source-only CI covering all new owned code.
2. Join them through one supervisor/worker authority and explicit MK1 wire. Test stale ownership, partial/unknown delivery, reply ownership, floods and lifecycle before exposing application input.
3. Connect useful TUI and desktop views, a deterministic typed agent and receipt/artifact status; test actual shell I/O and takeover across both clients.
4. Run two independent adversarial reviews of each final shipping revision, exact-head active/governance checks, and real end-to-end launch/demo evidence. Fix blockers, integrate normally, verify `main`.
5. Hand off short reproducible launch instructions, actual results and limitations, then **stop for human feel feedback**. A useful MK1 can satisfy this without claiming full NG1 or changing historical FAIL results.

Changes to the N0 proposed seam needed for responsive MK1 must be explicit, versioned and reviewed; for example human continuous input may tolerate ordinary output revisions while still fencing owner/focus/mode/geometry changes. Agent input remains observe–act–observe. Do not silently weaken stale-input protection, infer protected prompts, replay uncertain input, or equate input takeover with cancelling prior side effects.
