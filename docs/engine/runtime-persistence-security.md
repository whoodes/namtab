# Rust runtime, persistence and security boundaries

[Campaign](index.md) · [Native operations](native-contract.md) · [Core state](compatibility-and-core.md) · [Delivery gates](delivery-and-validation.md)

**Accepted-for-design direction; proposed implementation details below.** No new crate, runtime, database or snapshot has been implemented. Source constraints: S02–S06, S20–S29, S31–S32 and historical recovery evidence H01.

## Module and process design

Keep the existing seven Rust/four TS packages and historical source intact in this research phase. The next authorized engineering work should add **one** cohesive owned package, provisionally `crates/terminal`, containing a pure library and `project-x-terminal-worker` binary. Do not repurpose/delete the experimental `crates/engine` worker or its gate. No lexer/width/checkpoint/wrapper microcrates, plugin ABI or service per method.

| Boundary | Proposed responsibility / imports |
| --- | --- |
| `domain` | Pure work/evidence/lease/approval transitions. No terminal parser, OS, database or GUI dependency. Extend existing models after reconciling semantics, not copying them. |
| `protocol` | Versioned private/public envelopes, bounds and schema export. Native agent/client DTOs are distinct from private worker/keeper messages. Large IDs are bounded decimal strings for JS interoperability. |
| `terminal` library | `lexer`, `text`, `screen`, `modes`, `input`, `reduce`, `snapshot`, `observe` modules. Own semantic types/versions. No OS handles, clock, network, storage, GUI, grants or direct output callbacks. |
| `terminal` worker binary | Bounded private transport, pure-core ownership, session scheduling, DTO↔core mapping. Uses `protocol`; core modules may not import it. One package with module checks is sufficient initially. |
| `platform` and keeper | OS process/PTY/pipe/worktree/runtime adapters and stable handle observations. Keeper holds master FDs and controller epochs; not terminal semantics or human UI. |
| `service` | Authentication, domain policy, admission/receipt store, context/artifact coordination, scheduling, private worker/keeper orchestration and public subscriptions. Does not instantiate a second terminal interpreter. |
| `client` / `console` / desktop + TS protocol | Public IPC, caches, intent and presentation only. No private endpoints, persistence files, PTY FDs or core imports. Migrate current reference input types deliberately into generated public protocol before live frontend use. |

The intended compile graph stays acyclic: console → client → protocol; desktop → TS protocol; service → domain/platform/protocol; platform → domain; terminal worker → protocol + its pure library. The library uses only owned types and reviewed algorithm/serialization primitives. Core↔wire projection is a translation boundary, not two independent terminal states. Add one future package/boundary rule when implementing it; current checks still correctly enforce the current seven-package workspace.

Rust owns core/service/keepers/TUI; TypeScript owns Electron UI. Start with safe Rust and reviewed safe OS wrappers. Existing EA1 unsafe is historical; the new pure core forbids unsafe. Any unavoidable low-level launch/PTY syscall or font FFI must be confined, documented with lifetimes/FD/allocator/threading invariants, independently tested and reviewed. No private native-emulator ABI, transmuted snapshots, callback pointers or dynamic in-process plugins. Dependency licenses remain separate from the unadopted project license.

N1 uses one dedicated owned-worker process multiplexing a small admitted session set, with one serial owner per session. A worker fault can lose all sessions mapped to it; report that failure domain. A bounded pool can follow measured fault/resource requirements, not one process/thread per agent. Service/keeper asynchronous I/O may use Tokio; CPU parsing, snapshot encoding, search, hashing and font work do not block that event loop. Tokio/OS/Unicode versions are future reviewed pins, not new dependencies installed here.

## Ordering, fairness and budgets

The keeper sequences observed PTY output, acknowledged geometry operations and delivery records. The engine consumes events in that order. Cross-process readiness is not an atomic picture of the application; record requested/applied cuts and uncertainty. Native pipe streams retain independent offsets and service observation order. Engine replies and authorized application input have distinct identities but one explicitly ordered final write queue, with no byte-level interleaving of partially delivered operations.

Use bounded per-session/per-principal/global input, output, effect, observation and evidence budgets. Service admission reserves capacity; partial writes retain an exact prefix and may block later writes to avoid combining inputs. Parser work is scheduled in bounded quanta with fair round-robin/weighted quotas; human input, takeover/revoke and failure controls have reserved queue capacity. Human priority does not allow an old-mode encoding to overtake unprocessed output or revoke already delivered effects. Limit monopolization by control/reply floods as well as bulk output.

Coalesce render deltas and cancel obsolete search/index/snapshot work; never coalesce away parser bytes or committed control events. Slow readers get `resync_required`/disconnect rather than unlimited state copies. Backpressure can block the program and change timing. If a capture sink fails while parsing continues, capture becomes gapped; if canonical input is actually missing, interpretation becomes unknown. Full storage refuses new evidence reservations before execution or explicitly records degraded policy under the existing grant; it never silently evicts pinned evidence.

[G6](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets) owns all numerical workload/latency/memory allocations. N1 measures a smaller declared workload; N3 qualifies the full reference without counting independent caps as simultaneous guaranteed capacity. [Product resource admission](../engineering/07-validation-roadmap-and-risks.md#resource-scheduling-and-overload) separately accounts attempts, parent/provider limits and human attention.

## IPC, configuration and extension scope

Keep framed, size-bounded JSON over authenticated local Unix sockets as the initial private/public transport, using existing schema generation. Negotiate protocol major/minor, feature/profile versions, frame limits and endpoint capabilities before operations. New native/owned-worker contracts use distinct namespaced experimental versions; never reinterpret existing EA1/public-v1 messages as the new semantics. Reject duplicate keys, unknown mutation fields, overlong/deep structures and unsupported major versions before authority evaluation; parse/schema validity is not authorization. Cancel subscriptions/waits separately from admitted effects. Use the [typed error families](../engineering/14-repository-and-engineering-practices.md#error-cancellation-and-logging-contracts), with separate gap/receipt outcome fields and sanitized diagnostics; do not introduce a second error-code vocabulary.

Create sockets in an owned restricted runtime directory; validate peer credentials and connection-to-principal binding, prevent inherited capabilities via unintended FDs/environment, and authenticate keeper/worker controller epochs. Peer UID alone is not agent identity. A remote bridge later needs authenticated encrypted transport and explicit host/context identity; do not expose an unauthenticated TCP port or auto-install a helper. Ordinary SSH/remote TUI patterns remain in the UX contract.

Versioned TOML configuration remains declarative, with portable keymaps/themes/profile choices. Separate machine-local paths/secrets and runtime grants. Migrations validate, preserve a backup and fail closed on unsupported security settings; no executing shell snippets during config load. Built-in typed adapters first; later out-of-process versioned adapters receive scoped capabilities. No arbitrary native code loading, generic workflow DSL or premature stable plugin ABI.

## Complete snapshots and journal cuts

Define three different objects: a **presentation snapshot** for live attach; a **complete terminal-state snapshot** for a compatible reducer; and a **product checkpoint** binding that state to runner/store cuts and capture consent. Neither state object includes an OS process image. Ordinary reconnect needs the first object only.

The complete state includes every family in the core table, both screens/history, decoder/FSM continuation, discarded-string terminator state, pending grapheme/reflow context, typed negotiation stacks, profile/error/limit behavior and semantic counters. Derived caches such as glyph atlases and search indexes are omitted only when they can be rebuilt without changing semantics. Snapshot at a committed reducer event cut; a partial escape sequence can be the state **at** that cut. “Wait for parser ground” is not required or a complete solution.

Recommended v1 envelope: explicit product/snapshot schema/codec/engine/profile/Unicode/width versions, session/incarnation, event and stream/geometry cuts, configuration digest, required feature bits, bounded lengths/cell/history counts, privacy classification and stored-byte integrity digest. Use explicit typed JSON initially, no compression, with quota-checked decoding; benchmark size/copy/parse cost before choosing a binary codec. A serializer library's version or Rust struct memory layout is not the format. Integrity detects corruption, not an authenticated owner or trustworthy data. Never restore executable commands, FD numbers, leases, approvals, secret capabilities or callbacks from it.

The optional durable journal records ordered output bytes, input delivery metadata, geometry/options and state-relevant controls under the chosen capture policy. Separate control metadata from content; default metadata must not contain terminal payload, content-derived hashes, credentials or environment values. Content digests and artifact references need their own authorized evidence context. Native task output/artifact retention is explicit, not implied by the word “managed”.

Treat literal argv/environment/application input as potentially secret too. Persist safe identity/status/sequence facts by default, not the raw request body or an unconsented payload hash. Volatile payload binding plus namespace/sequence fencing permits conservative duplicate refusal after restart/retention expiry; stronger durable input audit needs separate consent and secret-reference handling. No perfect automatic secret classifier is assumed.

For checkpoint C at event E: serialize an immutable bounded state view; verify all required retained/durable tail events after E; write a temporary object, verify limits/integrity, durably publish object and journal reference using the storage protocol. Account for directory/file sync assumptions and crash/full-disk cuts (S26). Retain the previous valid checkpoint until commit. A reference cannot become usable before its object and required prefix are durable. A memory-only keeper tail remains bounded and can be lost; do not call it crash-proof.

On restore, validate identity/version/profile/quotas/completeness, replay the exact ordered tail in **no-external-effects mode**, and reconcile keeper delivery frontiers. Generate a new authority epoch and require fresh input ownership. Historical queries may update internal query state as specified, but never re-emit old replies, input, launches, clipboard or window effects. Reply/operation IDs and delivery-unknown outcomes remain in the separate receipt log; identical future live queries remain valid. A state hash or plausible screen does not authorize replay of an uncertain external write.

## Precise v1 recovery contract

| Failure / policy | v1 target behavior | Non-guarantee |
| --- | --- | --- |
| Late attach, frontend restart or disconnect; owner alive; capture off | Fresh presentation snapshot + deltas from its cut; new view is read-only until ownership granted | No need for full output replay; no promise of retained old scrollback beyond quotas |
| Engine worker dies; capture off | Mark interpretation unknown, fence all application input, keep separately observed process status; offer new session/authorized process controls | No reconstructing arbitrary live TUI from nothing, repaint guessing or raw-input bypass |
| Worker dies; authorized valid exact-version checkpoint + complete retained tail | Restore/replay without effects, verify cuts and reobserve, then allow fresh ownership | Missing bytes/options/identity, corrupt/oversized state or uncertain delivery stays unknown |
| Service restarts, keeper/engine survive | Reconcile receipts/controller epochs and current state before admission | Old grants/leases/queued actions do not resurrect automatically |
| Keeper/PTY/process loss, reboot | Report observed exit/loss or unknown outcome and preserve authorized evidence | A terminal snapshot cannot restore processes, files, remote side effects or old FDs |
| Capture revoked / retention expired / unsupported schema | Stop future authorized persistence, apply retention/deletion rules, expose unavailable recovery | No recovery promise from filtered or gapped capture; no guaranteed physical erasure from backups/OS media |

**v1 complete-state guarantee to earn:** continuation equivalence for admitted profiles, exact snapshot schema/engine/profile/data tuple, ordered event tail and validated resource bounds, including arbitrary partial-parser cuts. This is an acceptance criterion, not current proof or all-input formal verification. Initial cross-version loading is refused. Version numbers reserve migration discipline; they do not promise every future format can restore every past live process. A first schema change must deliver a separately reviewed offline migrator with old fixtures/continuation tests or explicit unsupported status. Presentation/evidence metadata should remain inspectable without executing historical engine binaries.

No-capture means no namtab durable raw-equivalent state by default. Cells/history/partial parser snapshots can contain secrets; memory-only is not a claim of no swap, core dump, outer-terminal recording or host administrator access. Engine-loss recovery without durable capture may later use a separately surviving authorized memory snapshot/tail, but v1 does not depend on that extra replica or silently promise it.

A compatibility-worker failure does not by itself invalidate an independently observed pipe-process exit or the service's task/receipt metadata. Mark the lost interpretation and its dependent actions/evidence unknown; retain other authoritative facts. Unrelated native pipe attempts need not acquire a terminal dependency merely because the product is called a terminal engine.

## Threat model and mitigations

[Security and approvals](../architecture/04-isolation-security-and-approvals.md) owns the product threat model, grants and verified runtime profiles. This table specializes that model for owned-core/state/IPC attack surfaces; it does not define another approval lifecycle.

| Threat | Boundary / proposed response | Residual risk |
| --- | --- | --- |
| Hostile escape/Unicode/font/output floods | Pure bounded core, arithmetic/geometry/string caps, fair scheduling, process memory/CPU containment, fuzzing; browser/font parsing outside engine semantics | Safe Rust does not bound CPU/allocations or prove logic correct; process isolation is not a full sandbox |
| Prompt injection / spoofed integration / fake approvals | Output is labeled data; native control uses authenticated typed methods; shell markers remain hints; separate trusted chrome and protected decision channel | Agents can still reason incorrectly from hostile data; renderer text cannot establish human presence |
| Secret entry and capture | Scoped observation grants, explicit protected handoff, no default input logging, raw-equivalent checkpoint consent, bounded retention/export | Echo-off/prompt heuristics cannot identify every secret; outer host and uncontrolled programs may capture it |
| Agent escapes or uncontrolled filesystem/PTY writes | Runtime profile, worktree/mount policies, restricted FDs/sockets/network, mediated keeper channel; enforce and test before claiming restricted execution | Same-UID unrestricted processes may bypass coordination; tasks/leases are not an OS security boundary |
| Forged peers / stale authority / replay | Restricted sockets, peer binding, epochs, operation IDs, bounded receipts, fresh challenge and context-bound approvals | Mutually hostile code with the human's full UID authority is outside this assurance; stronger isolation requires OS enforcement |
| Malicious snapshot/config/plugin | Version/length/structure/integrity validation, no object deserialization into capabilities, inert imports, built-in adapters first | Authenticated content can still be hostile; checksum is not provenance or authorization |

Rootless restricted runtimes remain the existing recommended enforcement direction, subject to host prerequisites and negative tests. Host-approved unrestricted execution is a separately labeled profile, never described as sandboxed. No system configuration or isolation experiment is part of this campaign. Shutdown/takeover stops future mediation only to its tested boundary; existing child/remote side effects require separate controls and evidence.
