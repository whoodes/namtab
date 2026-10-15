# Domain storage, API evolution and extensions

[Index](../index.md) · [Runtime/process graph](../engine/runtime-persistence-security.md#module-and-process-design) · [Native API](../engine/native-contract.md) · [Security](04-isolation-security-and-approvals.md)

**Accepted-for-design; full store/IPC implementation pending.** This page owns domain journal/projection durability, evidence-object publication, subscriptions, API evolution and extension/migration lifecycle. The engine runtime specification owns canonical event application and terminal-state recovery.

## Architecture shape

Use one modular local application with purposeful lifetime/authority boundaries, not a service per conceptual object. The [runtime module/process graph](../engine/runtime-persistence-security.md#module-and-process-design) is authoritative. Native pipes bypass VT interpretation; PTY output goes to the owned core. Clients and adapters use public projections; keepers hold OS endpoints; the broker owns admission. A later remote helper is a separate authenticated boundary, not a reason for local microservices.

## Module ownership and extension seams

[Repository practices](../engineering/14-repository-and-engineering-practices.md#concrete-proposed-layout-and-ownership) maps source paths/maintainer responsibilities; [runtime](../engine/runtime-persistence-security.md#module-and-process-design) maps semantic imports and process ownership. There is no second graph here. Initially combine work supervision, permissions, artifacts, scheduling and store in the service. Parser isolation and keeper lifetime justify processes; plugin flexibility alone does not.

## Event and request contracts

One broker is authoritative for a local state namespace; acquire an exclusive OS-backed state lock and reject a second writer rather than permitting two supervisors to allocate authority. A runner fences broker reconnection with its own authenticated ownership generation and rejects requests from an older controller. Remote helpers own their own fencing state. Local revocation during a partition cannot be presumed delivered remotely; show the remote authority/outcome as unconfirmed until reconciled. Clock-based lease expiry alone is not a distributed split-brain solution.

An event envelope contains event ID, schema version, producer identity and incarnation, producer-local sequence, broker journal sequence, object ID/revision, causal/request ID, wall and monotonic observation time, trust/representation labels, and typed payload. Large bytes live in bounded chunks, referenced by digest/stream offsets. Wall-clock order is not causality. Broker sequence orders accepted records locally; independent hosts have separate sequence spaces.

[Native receipts](../engine/native-contract.md#minimal-operations-and-asynchronous-outcomes) own mutation admission, payload binding and unknown outcomes. Storage's proposed local receipt horizon is 24 hours, distinct from longer action metadata retention in A6. Expired namespaces/sequence IDs are rejected after compaction, never treated as new work; tombstones cannot retain secret input.

Subscriptions specify a journal incarnation/cursor, filters, batch limits, and credit. Delivery is at least once within retained history; consumers deduplicate by event ID and advance their cursor after processing. A cursor older than retention returns an explicit gap and a consistent projection snapshot with its cursor. The client then subscribes after that cursor, preventing a snapshot/stream race. Filtered sequence gaps are distinguished from lost history. Slow clients receive bounded disconnect/resume errors.

Raw output streams have their own epochs/offsets and capture acknowledgments; journal sequence is not a byte offset. UI state can be current while persistence lags, and a durable event can reference evidence that the client lacks permission to read. Show these distinctions instead of a generic “synced” badge.

## Storage and durability

Accepted-for-design: a local SQLite metadata database with an append-only-within-retention domain journal and transactional projections, plus content-addressed artifact/output files and policy-gated keeper spools. Ordinary capture-off sessions have no output disk spool; durable admission/process metadata must not hide output, input values or secret-bearing argv. Journal retention/deletion and exports follow ADR-A6 rather than implying permanent append-only retention. This is selective event sourcing: retain important state transitions for audit, while caches, search indexes, view layout, and sampled metrics need not all be replayable events. Start with one store writer, batching output references without delaying revocation admission. Before journal retention removes a rebuild prefix, commit a versioned domain checkpoint at the cut cursor and retain its following events; this is metadata state, not a terminal-engine checkpoint. Rebuild from that checkpoint plus retained events, with purged history/evidence explicitly unavailable. Active object state cannot disappear merely because an old transition aged out. Physical disk admission budgets include WAL, temporary objects and migration backups; N3 storage qualification fixes the measured overhead ceiling and bounds long-lived readers/checkpoint delays.

SQLite WAL supports local reader/writer concurrency but has a single writer and host-local shared-memory requirements; do not put the live database on a network filesystem. Each remote runner owns its own local journal, replicated through protocol messages rather than shared database files. Durability depends on configured synchronization and working storage hardware. [SQLite WAL documentation](https://www.sqlite.org/wal.html), [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html)

For an artifact: stream to a quota-checked temporary object, compute/verify digest, flush as required by the durability class, atomically promote within the storage filesystem, sync directory metadata where required, then commit the database reference. A crash before the reference leaves a collectible orphan; a reference with missing/corrupt bytes is explicitly unavailable and cannot satisfy evidence requirements. Garbage collection honors active references/retention and does not delete user workspaces by inference. These operations do not form a universal transaction across files, databases, remote hosts, and external actions.

Durability classes are visible: volatile observation, locally persisted capture, and committed decision/action admission. Decision grants and consumption require the durable class before dispatch. Output may use bounded batches with an advertised loss window; an unacknowledged tail is unknown after a crash. A checksum-valid spool prefix can be imported after broker recovery, preserving stream identity and deduplicating records. Mid-record truncation discards only the incomplete suffix and records uncertainty.

If even reserved control storage is exhausted, deny new grants/dispatches and revoke future admissions in memory/at live runners where reachable. Emergency stop does not wait for a successful journal write; report that its audit persistence failed. Broker unavailability blocks new mediated grants/actions, while already admitted processes may continue. Restart invalidation of live authority prevents an unpersisted revocation from silently resurrecting a grant. None of this undoes effects or contains an unrestricted host process.

Terminal-state checkpoint objects use the same durable object/reference discipline, but their completeness, privacy and recovery semantics are owned by [the runtime contract](../engine/runtime-persistence-security.md#complete-snapshots-and-journal-cuts). A domain checkpoint is not a terminal checkpoint.

Search and summaries are derived, access-controlled projections. They expose the same capture/retention limits as their inputs. Retention removes derived copies as well as objects where feasible; exports and external backups remain outside guaranteed recall. Export format includes schema versions, evidence digests, gaps, availability, trust labels, and redaction policy, without secrets or executable restore hooks by default.

## Reconnect and crash recovery

The [runtime recovery matrix](../engine/runtime-persistence-security.md#precise-v1-recovery-contract) owns UI loss, worker loss, keeper loss and capture boundaries. Domain startup rebuilds projections from the retained checkpoint/tail, checks integrity and reconciles live runner receipts. It does not execute historical commands or restore live grants. Old leases and narrow unconsumed approvals are invalidated across uncertain restart; boot/monotonic context prevents a backward wall clock extending permission.

Service packaging must preserve independently owned keepers if persistence is advertised; ordinary child spawning alone does not do this. Logout/update behavior is owned by [platform/distribution policy](10-recommended-baseline-and-adrs.md#adr-a4-linux-and-eventual-desktop-ports). Historical interpretation playback is data reconstruction, never deterministic application execution replay.

## Versioned APIs and plugins

[Runtime IPC](../engine/runtime-persistence-security.md#ipc-configuration-and-extension-scope) owns framing, authentication, payload caps and endpoint negotiation. This section owns evolution and extension lifecycle.

Handshake advertises protocol major/minor, supported methods, feature flags, schema ranges, limits, runtime controls, interpretation profile, and deprecation windows. Additive optional fields are minor changes; changed meanings or required fields need a new major/feature. Unknown enum values survive round-trip and display as unknown, not success. Unknown security-critical fields/features fail closed. Initially ship one private 0.x protocol pinned to bundled components, without a public ABI commitment or prior-prototype compatibility promise. Support the previous released public client during a documented transition only after a public contract exists; maintain schema exports/migration backups meanwhile.

Generated public wire/config DTOs have a single Rust source and reviewed schema→TS generation; private engine/keeper schemas are separate. No frontend reimplements authoritative state transitions. Typed errors distinguish admission/delivery/outcome and cancellation stage. [Contract/workflow details](../engineering/14-repository-and-engineering-practices.md#schema-ownership-and-duplicated-state)

Initial operations cover task/attempt CRUD with revision fences, artifact/evidence reads, session attach/snapshot/subscribe, lease acquire/revoke, input submit, execution propose/status/cancel, and decision/approval operations. Only authorized human/policy channels can grant approvals. All interfaces use the same enforcement; a CLI is not inherently trusted more than a GUI.

### Navigation API boundary

[Native operations/receipts](../engine/native-contract.md#minimal-operations-and-asynchronous-outcomes) is the sole operation contract; [core input](../engine/compatibility-and-core.md#input-policy) is the encoding contract. Proposed method spelling is not a frozen wire API. UI commands target client/view generations; application input targets explicit sessions/epochs; lifecycle controls have separate authority. No missing target defaults to the focused pane, and raw fallback cannot bypass denial. Public frame subscriptions carry presentation data, not private engine or PTY access.

Plugins run out of process with manifests naming identity, version, protocol range, capabilities, network/filesystem needs, resource budget, and configuration schema. They cannot read the database directly, supply executable HTML to the trusted review surface, write PTYs, or replace permission enforcement. First extensibility targets are agent adapters, validators, and workspace/runtime adapters. Dynamic UI extensions, a public marketplace, arbitrary in-process native plugins, and programmable scheduling are deferred until concrete needs justify their risk.

Signatures/provenance help identify packages; they do not make plugins safe. Uninstall revokes grants and stops future calls, while preserving evidence attribution. A plugin crash/hang produces bounded errors and degraded semantic fidelity. Adapter/provider outages cannot freeze local terminal input. Extension conformance suites test partial messages, cancellation, duplicate callbacks, stale revisions, capability denial, and version mismatch.

## Migrations and maintainability

Version database schema, event payloads, artifact manifests, protocol, configuration, and terminal snapshots separately. Forward migrations are transactional where possible, with preflight disk-space checks and a consistent backup. A failed migration leaves a recoverable old store; no best-effort partial startup with mixed authority semantics. Rollback uses a compatible backup/export, not blindly opening a newer database with older code. Do not migrate live runner protocol state in place without negotiated support.

Keep historical event bytes or a documented canonical export; apply versioned upcasters when reading. Projection rebuilds must preserve original provenance and trust labels. Purged raw evidence cannot be reconstructed by a migration. Configuration imports are validated data, with unknown privileged fields rejected and prior effective settings inspectable. Recommend versioned TOML for portable UI/keybindings and named references; local secrets/paths/trust and live grants are separate. Project-local hints cannot enable capture, hooks or permission changes. [Configuration UX](../ux/11-human-experience-spec.md#onboarding-migration-and-portable-configuration)

Use one product release tuple with separate wire/config/store/interpretation versions and locked Cargo/pnpm workspaces; generated code and frontend import boundaries are CI gates. [Repository practices](../engineering/14-repository-and-engineering-practices.md), [dependency inventory](../engineering/15-dependencies-and-supply-chain.md), [invariant/CI map](../engineering/16-test-and-ci-strategy.md#invariant-to-test-and-ci-map)

Give each module an owner, public interface, dependency direction, operational metrics, and failure contract. Use the [current decision register](10-recommended-baseline-and-adrs.md) and focused ADRs; [08](../research/08-research-and-decisions.md) is historical research, not the active open-decision list. Add an abstraction after two concrete implementations or a clear mandatory boundary, rather than creating a generic provider hierarchy for every type. Terminal engine and runtime boundaries are exceptions justified by compatibility and security, even before a second implementation exists.
