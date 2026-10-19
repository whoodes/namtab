# Acceptance catalog and provisional performance budgets

[Index](../index.md) · [Current delivery and risks](../engine/delivery-and-validation.md) · [CI strategy](16-test-and-ci-strategy.md)

## Status and measurement discipline

This page owns cross-cutting quality dimensions **G0–G8**, suite catalog **T1–T15**, navigation cases **N01–N16**, and the inherited numerical **G6 reference proposal**. They are proposed criteria, not passing results or a second roadmap. [H0–H2](../engine/delivery-and-validation.md#current-harness-milestones) owns current proposed delivery; N/NG is retained qualification history; its R/V table traces sources to design and validation. [CI0–CI4/DF01–DF08](16-test-and-ci-strategy.md) owns execution lanes and dual-client cases.

Choose the advertised profile first, then applicable cases. No test catalog makes every shell, protocol or platform mandatory at the first slice. Every result must state source/build, hardware, workload, capture mode, sample count, percentile method and evidence. Stale-authority/correctness invariants do not become optional when latency is poor. Randomized testing supports bounded claims, not universal proof.

## Resource scheduling and overload

Start with bounded admission and fair queues, not cluster orchestration. Allocate CPU/memory/PIDs, output/storage budget, runtime slots, workspace/resource leases, network permission, provider-call budget, and human-review budget separately. Request declared estimates, account actual usage, and reserve headroom for interactive human work and emergency control. A task cannot avoid its parent ceiling by recursively spawning agents.

Use per-project weighted fairness plus aging, user-set priorities, maximum concurrent attempts, and all-or-none multi-resource admission. Dependency-ready does not mean resource-ready. Do not start work while holding one scarce lease and indefinitely waiting for another. Priority inheritance may be needed for a dependency blocking an urgent human task, but never permits skipping approval or isolation checks. Expose why an attempt is queued and which limit is binding.

Host organized sessions may have advisory resource control; hard controls depend on verified cgroup/runtime support. Never promise a CPU/memory/PID cap merely because the scheduler asked for it. Linux cgroup v2 exposes controllers and whole-cgroup termination, but availability/delegation and confinement of membership must be checked. [cgroup v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html)

Preemption options are cooperative checkpoint, revoking future tool/input admission, or explicit runtime termination. Freezing a process may hold locks, expire credentials, or leave remote transactions active; it is not a safe universal pause. A blocked agent can release a scheduler slot only if resource accounting still includes its resident processes. Quotas for output and evidence avoid a chatty task filling disk. Repeated restart/failure loops get bounded retries and a decision item.

Provider tokens/cost need their own accounting. A hard cap is meaningful only where all calls are mediated and the provider's reservation semantics support it. External agent CLIs can spend through their own credentials; delayed usage reports provide advisory limits, not exact spending enforcement. Network or provider outages use bounded retries with jitter for retry-safe reads; unknown execution outcomes follow reconciliation rather than retry.

Human decision backlog feeds admission control. Do not increase agent concurrency while review work accumulates without bound. Queue targets can include oldest high-risk decision age and expected review minutes. Users may deliberately accept a backlog, but the system never treats silence or a deadline as approval.

## Proposed measurable gates

G identifiers are quality dimensions. H0 selects their applicable successor evidence; old NG mappings below are retained qualification references, not current two-client gates.

| Quality dimension | Required evidence / current delivery mapping |
| --- | --- |
| G0 — Design feasibility | NG0 finite profile, owned state/contracts, source/license pins, staffing and bounded harness; no hidden legacy authority |
| G1 — Classic terminal | NG1 narrow shell slice, then NG2 published app/profile matrix with terminal/PTY/job-control/Unicode behavior and no agent dependency |
| G2 — Audit and authority | NG1/NG3 lifecycle/lease/approval/revision tests; exceptions visible; output cannot grant authority |
| G3 — Durability and recovery | NG1 live-owner attach versus NG4 complete terminal recovery; NG3 domain/receipt faults; reconciled or explicit unknown, no duplicate effects |
| G4 — Enforced isolation | NG3 each claimed runtime passes filesystem/network/socket/credential/descendant/control-unavailability negative tests; fail admission if unsupported |
| G5 — Human and accessibility | The selected emulator's critical keyboard/actual screen-reader workflows pass predeclared formative criteria; comparative productivity needs a separate adequate study |
| G6 — Load and responsiveness | Numerical baseline below; NG3 qualifies full declared workload, N1 reports its smaller measured workload without a full G6 claim |
| G7 — Agent navigation | N01–N16 for published inner profiles; outer key aliases disclosed; structured/raw share authority; UI commands emit zero managed application-input bytes; stale not-yet-started input rejected |
| G8 — Client authority and foundation | Applicable DF01–DF08 cases preserve shared truth, attach/takeover, capability/accessibility and import/schema boundaries; H0 selects clients, not two mandatory products |

## Performance and resource budgets

**Historical G6 reference workload; provisional, unmeasured, not the current approved harness target.** H0 must select the successor agent/process/PTY populations and hardware/limits. In particular 32 sessions below is not an approved exact 32-agent/client requirement. Reference qualification machine: dedicated Linux x86_64 desktop/VM, **8 vCPU, 16 GiB RAM, SSD-class local storage, 1920×1080 display**. Record exact CPU/GPU/drivers/fonts/scaling, virtualization, OS and display/host profiles; this is not inspected current-host hardware. The old qualification compared two frontends; new evidence covers the selected emulator, actual protocol/load clients and optional diagnostics. [All-process methodology](16-test-and-ci-strategy.md#all-process-performance) owns accounting and actual-paint measurement.

| Target / allocation | Provisional value and interpretation |
| --- | --- |
| Full qualification load | 32 sessions, eight active managed attempts, aggregate 8 MiB/s output for ten minutes |
| Local input to visible paint | p95 ≤50 ms; include queueing, admission, encoding, parsing, IPC and real paint |
| Decision queue update | p95 ≤250 ms |
| Automation revoke admission | p95 ≤100 ms; separate from process/remote cessation |
| Application memory | ≤1 GiB total product processes/copies, excluding child programs and separately reported OS cache |
| Output queues | ≤4 MiB/session, ≤128 MiB globally at the reference concurrency |
| Live history | ≤32 MiB/session within a shared 256 MiB cap |
| Cold start | Minimal clean-shell readiness ≤1 second, without network/model initialization; arbitrary user rc startup reported separately |
| Visible presentation cadence | Aim for 60 Hz when hardware permits; a presentation target, not a parser scheduling clock |
| Later vision scale | 128 inspected sessions; requires a new memory/output budget and human study before any readiness claim |

Queue/history limits are **included allocations**, not independent promises of simultaneous full occupancy. All admitted harness/keeper/core/adapter/emulator/diagnostic processes and view caches count; no Node engine fallback belongs to the owned shipping design. Measure the outer emulator/multiplexer separately and as total deployment footprint; it is not free. Tighten admitted concurrency or buffers explicitly if baseline memory leaves too little headroom.

Report end-to-end **p50/p95/p99/max**, not only the target percentile, for control/observations/input/rendering and takeover; report model inference/provider/application time separately. Include floods, query storms, disk saturation, cap crossings, slow clients, cancellation, fairness, resize and IME. Hidden surfaces may skip paint, never parser bytes. No Rust speed-superiority claim or achieved benchmark follows.

Capture quotas/retention, artifact-copy budgets and metadata reserve have one owner: [A6](../architecture/10-recommended-baseline-and-adrs.md#adr-a6-capture-and-retention-defaults). Test gaps/backpressure and evidence reservation, not silent eviction of pinned material.

## Test architecture and fault injection

| Suite | Important scenarios | Required observation |
| --- | --- | --- |
| T1 — Domain and evidence | Task revisions, dependency cycles, integration creates new subjects, expired evidence, exceptions, dirty/ignored/LFS/submodule inputs | No pass or acceptance migrates silently to a different subject; quality/availability preserved |
| T2 — Terminal parser | Every split of UTF-8/escape sequences, invalid bytes, long OSC/DCS, carriage-return progress, modes, alternate buffer, cursor/query replies | Bounded parser work; equivalent results for equivalent byte/resize histories within one engine/profile |
| T3 — Interactive compatibility | Bash/Zsh/Fish/sh, nested tmux, editors/pagers/curses, REPLs, raw mode, `/dev/tty`, EOF and echo | Correct termios/job-control behavior; no fabricated command boundaries or stream separation |
| T4 — Resize and text | Rapid resize, disconnect during resize, two observers, reflow, CJK/combining/emoji, IME, bidi filenames | Single sizing authority; stale coordinates rejected; byte/cell/grapheme distinction maintained |
| T5 — Ownership and approval | Takeover during partial write, stale generation, terminal dialog spoof, changed executable/context/host, expired grant, delayed callback | No further broker admission under old authority; delivered prefix and unknowns visible |
| T6 — Process lifecycle | Foreground pipelines, background jobs, double-fork, PID reuse, leader exits first, descendants hold slave, signal races | Correct target identity, no collateral signals, truthful residual-descendant reporting |
| T7 — Security boundaries | Path traversal, symlink/hardlink races, inherited FDs, host sockets, alternate API input, prompt injection, hostile links, oversized images | Actual enforced profile matches claims; no output-triggered authority or unapproved side effects |
| T8 — Secret/capture policy | Secrets across chunks, ECHO-off, secret echo, raw capture off, snapshots/search/export/crash dumps, retained backups | Policy labels and permissions respected; known filter limits explicit; no claim of perfect redaction |
| T9 — Storage/recovery | Kill GUI/broker/parser/runner, power-loss model, disk full/read-only, corrupt/truncated object or spool, clock rollback, GC race | Recover committed prefixes or explicit gaps; never infer no effects from missing results |
| T10 — Transport/API | Split frames, duplicate IDs, slow subscribers, lost ack, competing brokers, stale controller generations, SSH partition/rekey/host change, remote reboot, protocol mismatch | Bounded resources, dedup/resume behavior, no blind retry, identity and authority fencing preserved |
| T11 — Plugins/migrations | Hung/crashed/malicious adapter, capability escalation, old client, unknown enum/critical field, failed migration/rollback | Core terminal survives; fail closed for security semantics; backups usable |
| T12 — Human/load/distribution | Notification storm, huge diff, 32/128 session views, keyboard/screen reader, GPU fallback, install/upgrade/remove | Usable review/control under load; explicit lifecycle; no deletion of unowned work |
| T13 — Agentic navigation | N01–N16: keys, modes, interactive application journeys, focus/identity, partial input, sensitive prompts, floods | Observe-act-observe works without agent-specific hooks; no byte-delivery/semantic-success conflation |
| T14 — Retained client-boundary cases | Applicable DF01–DF08: client/API truth, writer endpoint, input decoding, sizing, review, detach/SSH, accessibility; old TUI/Electron pairing is historical | Same authority/evidence, honest physical/rendering differences, no private endpoint or persistence access |
| T15 — Foundation and supply chain | Import DAG, Rust/TS schema drift, duplicate Crossterm, locked tools, hostile build scripts, packaging/SBOM gaps | CI boundary checks block forbidden edges, source/ABI/notice gaps explicit; no privileged untrusted-PR job |

Use paired pipe and PTY fixtures to show their different behavior rather than forcing equality. Use property-based chunking tests, subprocess/job-control integration tests, protocol conformance, GUI/accessibility testing, and sandbox negative tests at their appropriate boundaries. Differential comparison with reference terminals identifies differences; a majority vote is not necessarily the specification. Do not normalize away bugs in fixtures just to match a candidate engine.

Inject crashes before/after durable approval consumption, before/after runner admission, before/after spawn, after an external side effect but before result recording, during object promotion, and during migration. No database transaction can atomically contain an arbitrary remote side effect. Include an external test service that counts operations to demonstrate unknown outcome handling, not to claim exactly-once execution.

## Agentic navigation acceptance cases

These are future tests, not live experiments or claims of implemented support. Each case pins application/version/configuration, terminal/keyboard profile, locale/encoding, transport, runtime, capture policy, target identity, and expected evidence. Run applicable cases over local PTYs and again over certified SSH/tmux paths. Use controlled disposable fixtures in that future test effort; no current host application is driven during documentation work.

For each positive journey, require prior identified observation → bounded action → actual encoding/delivery receipt → new observation and justified transition claim. Negative cases assert zero bytes for a rejected not-yet-started event; an already started event instead reports its actual partial/unknown delivery. No assertion may treat a write acknowledgment, elapsed sleep, matching screen hash, or Ctrl-C as universal proof of application success or termination.

| Case | Planned interaction / fault | Acceptance and evidence |
| --- | --- | --- |
| N01 — Shell history and completion | Published shell profiles (Bash/readline for v1; Zsh/Fish when separately qualified) with pinned bindings: arrows through history, cursor editing, Tab completion and supported reverse/candidate cycling, Ctrl/Alt shortcuts | Correct mode-aware key delivery and observed completion/history transition under that shell's configuration; no built-in universal completer or assumption completion is read-only |
| N02 — Menus and interactive CLIs | Controlled curses/menu and fzf-like selection: arrows, Tab/Shift-Tab, page keys, supported function keys, confirm/cancel | Selected-item/result evidence matches intended bounded steps; opaque highlighting alone is labeled weaker than a known fixture/result |
| N03 — Pager | less-like paging, PageUp/PageDown/Space, search, next match, quit | Application navigation changes the pager; terminal history scroll does not send pager input, and application keys do not move namtab focus |
| N04 — Terminal editor | Pinned Vim/Neovim or Emacs terminal profile: movement, editing/selection, Escape and multi-key shortcuts, configured F-key action, save/exit | Observe editor transitions and verify resulting artifact separately; a redraw or status line is not proof of save; no universal binding assumption across editors |
| N05 — REPL | Pinned Python and SQLite-like REPLs: history, completion where configured, multiline editing/paste, result inspection | Follow each program's input/multiline policy; no extra Enter or duplicate evaluation after a delayed result; result/process evidence distinct from delivery |
| N06 — Agent CLI | Opaque agent CLI plus one supported adapter profile: menu navigation, Tab, modifiers, completion, approval/authentication interruption | Opaque navigation remains usable without hooks; structured protected-prompt state blocks ordinary automation; downstream approval never grants broker authority |
| N07 — Keys and input modes | Normal/application cursor, numerical/application keypad, Return/keypad Enter, F1–F12, editing keys, representable modifiers; unsupported key/release request | Pinned encoder/byte fixtures match the active profile; mode changes invalidate queued incompatible input; unsupported requests emit zero bytes and do not auto-fallback |
| N08 — Layouts and encoding | Non-US layout, AltGr, IME, accented/CJK/combining text, explicit legacy aliases, unsupported encoding | Agent text preserves declared characters; logical keys do not depend on host layout; no accidental Ctrl-Alt shortcut from composition; aliases/limits disclosed |
| N09 — Wrong focus or remapped view | Two sessions, two clients/views, human focus change, pane remapping, stale session incarnation or writer generation | Direct session input stays on its explicit background target without focus theft; stale view-derived or wrong-incarnation requests emit zero bytes; UI focus/selection commands emit no PTY bytes |
| N10 — Stale screen and races | Output arrives between observation/admission/dispatch; resize, alternate-buffer replacement, mode change; output hidden in transit | Known stale preconditions reject not-yet-started input; partial delivery is visible; receipts identify actual dispatch prefix; no claim that checking freezes application state |
| N11 — Non-idempotent input and lost acknowledgment | Lost receipts, duplicate request ID/body, changed body under same ID, expired ledger, disconnect during Escape/Enter or a multi-event batch | No re-write on a reconciled duplicate; conflicts/unknowns explicit; no blind new-ID retry; per-event delivered prefixes retained under permitted capture without asserting atomic batch execution |
| N12 — Selection and paste | View selection versus editor selection; authorized text transfer; multiline/unbracketed paste; bracket delimiters/control bytes in payload; partial paste | Correct action plane and source authorization; observed paste mode and transformation in action identity; unsafe ordinary payload rejected; partial paste remains uncertain; no implicit clipboard access |
| N13 — Password and approval prompts | ECHO-off, MFA/host key, broker approval, agent CLI permission dialog, spoofed prompt, stale prompt after takeover | Known protected state blocks both structured/raw writes; suspected prompts hand off; secret path has no routine payload capture; no assertion of universal visual prompt detection or automatic consent |
| N14 — Output floods | G6 output workload while navigating, stale observation stream, slow parser/subscriber, capture gap, human override | Bounded queues, explicit stale/gap/degraded results, no unbounded observe/retry loop; parser completeness distinguished from capture; reserved control stays responsive without silently weakening freshness |
| N15 — Escape ambiguity and transport | Lone Escape, Alt/meta chord, split escape sequences, delayed SSH/tmux delivery, enhanced protocol enabled/disabled | Verify declared wire profile and explicit ambiguity handling; do not claim one local write or a timing delay makes legacy Escape semantics unambiguous |
| N16 — Interrupt versus termination | Ctrl-C under cooked/raw/enhanced keyboard modes; program catches/ignores interrupt; foreground pipeline and surviving descendants | Key receipt never marks a process terminated; separately observe job/process result and explicitly authorize signal/termination escalation; emergency ownership revocation requires no Ctrl-C success |

G7 applies to the [published qualification matrix](../engine/delivery-and-validation.md#compatibility-matrix), not every candidate named in this catalog. It requires the baseline structured key set, text/control input, selection/paste, explicit raw fallback, and ownership/receipt rules. Optional enhanced keyboard reporting is tested only when advertised; its absence cannot justify failing all core navigation or inventing equivalent key-release behavior. A profile limitation is documented; a missing mandatory capability blocks that profile's core certification. Application journeys test specific configured bindings, not a promise to navigate every opaque interface semantically.

## Staged roadmap

Use [H0–H2 current proposals](../engine/delivery-and-validation.md#current-harness-milestones). N/NG and P/PP are historical delivery references. MK1 delivered only its finite slice; current coordination/scale is unqualified, compatibility stays scoped and exact recovery remains later. The G/T/N catalog retains meaningful criteria without imposing Electron or an unchosen replacement interface.

## Risk register

The [current RE risk register and open decisions](../engine/delivery-and-validation.md#risk-register-and-remaining-decisions) is authoritative. The old R1–R22 planning table is retained in Git at `a5fb6dd`; its material workspace/attention/provider/update risks are incorporated there. No empirical report or fixture was removed.

## Exit from documentation to engineering

The [prototype entrypoint](12-prototype-plan.md) routes to the same N/NG plan. Only separately authorized bounded engineering may earn those gates. Original X4/EA1 source and FAIL results remain historical counterexamples; passing foundation, documentation or preservation checks cannot certify the owned engine.
