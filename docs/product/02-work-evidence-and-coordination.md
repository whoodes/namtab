# Work, evidence, and coordination

[Index](../index.md) · [Approval enforcement](../architecture/04-isolation-security-and-approvals.md) · [Storage](../architecture/05-architecture-storage-and-extensions.md)

## Objects and authority

**Proposed domain policy, not implemented in full.** [Native objects/authority](../engine/native-contract.md#objects-and-authority) owns identities, incarnations, capabilities and client-bound writer leases. This page owns task acceptance, artifact/validation quality, dependency selection and workspace integration. IDs are stable opaque values; paths/PIDs/names are display attributes, not authority. Revisions are immutable records.

Domain-specific records include TaskRevision (acceptance requirements, dependency selection and budget), ArtifactRevision (bytes/tree manifest or versioned external reference), Validation (subject/context/check/evidence/limits), Claim (attributed participant statement), Decision (authenticated disposition) and Approval (broker-owned revocable permission, governed by [security](../architecture/04-isolation-security-and-approvals.md#approval-lifecycle-and-stale-approvals)). Both frontends consume these records; neither reimplements their state machine.

One agent may own several attempts; an attempt may use several sessions and executions. A terminal can predate its task. Neither a shell prompt nor an adapter's “done” event closes the task. Source authenticity proves who reported an event, not that its contents are true.

## State and acceptance

Task states: draft, ready, active, blocked, awaiting-review, accepted, canceled. An accepted task may acquire a new revision requiring new work; historical acceptance remains attached to the old revision. Attempts have queued, starting, running, waiting, stopping, succeeded, failed, canceled, and unknown outcomes. “Succeeded” for an attempt means it produced its declared result, not that a human accepted it.

Session connectivity, process liveness, agent activity, and task progress are separate projections. An alive process can be blocked; a disconnected process can continue; a dead process may have completed an irreversible action. Only authoritative transitions with expected object revisions update durable state. Deduplicated state-event delivery is harmless; repeated application input is not. Conflicting state transitions return a conflict with the current revision.

Acceptance requirements are a versioned checklist of predicates over named artifact revisions, evidence, and explicit human decisions. Examples: a selected test specification passed against the current snapshot; a named reviewer accepted a diff; an unresolved integration conflict is absent. A policy can permit an explicit exception with rationale, but cannot rewrite failed evidence into a pass.

## Exact evidence

An EvidenceRef includes producer identity, execution/session ID, stream epoch, byte offsets or artifact digest, representation, capture policy, timestamps, completeness, and retention state. An output citation refers to stored bytes, not a summary's approximate line numbers. For a PTY, the stream is `pty`; stdout/stderr identities do not exist after they share the slave. Pipe executions have separately indexed streams, without a provable total ordering between them.

Terminal screen observations additionally record rows/columns, active buffer, cursor/modes, parser and Unicode-width profile versions, output offset, and capture time. A screen is an interpretation of an observed prefix, not proof that a TUI is waiting for a particular answer. A screenshot without these attributes is useful illustration but weak execution evidence.

[Native observation/receipt fields](../engine/native-contract.md#observation-and-evidence-schema) bind navigation evidence to target/frontier/profile/geometry/ownership. An action links prior observation → intent/encoding/delivery → later evidence → transition claim, with sensitive values retained only under authorized capture. Application widget focus remains unknown without cooperation.

Byte delivery, visible change, semantic success, and task acceptance are separate claims. A Tab write can be complete while completion is still running; an editor redraw does not prove a file was saved; an unchanged grid does not prove no action occurred. A failed or unknown navigation transition must not advance an acceptance predicate. The [observe-act-observe loop](../engine/native-contract.md#observeactobserve-races-and-human-override) defines these evidence relationships without inventing deterministic application state.

Raw, filtered, and derived evidence are different representations. Raw byte retention is explicit and restricted. A redacted transcript is exact only about its retained representation; it must not claim to preserve the missing original bytes. Offsets in filtered text map through a transformation manifest where possible; omitted spans are marked. Agent summaries cite these representations and cannot upgrade their completeness. Secret handling is defined in [04](../architecture/04-isolation-security-and-approvals.md).

Ordinary sessions default to memory-only output. A live observation may be useful without becoming retained evidence; later audits cannot cite missing original bytes as if captured. Capture consent and expiry are explicit in [ADR-A6](../architecture/10-recommended-baseline-and-adrs.md#adr-a6-capture-and-retention-defaults). Decision metadata can outlive its supporting output, with availability marked accordingly.

Evidence quality is a tuple, not one confidence score: attribution (runner/adapter/unattributed); capture completeness (complete/gapped/not captured); subject binding (sealed/initial snapshot/mutable/unknown); environment closure (declared/partial); and availability (retained/expired/restricted). Hashes detect content change, not correctness, harmlessness, or independent trust. A signature on a remote receipt establishes attribution to that signer, not honesty of a compromised host.

## Revision-bound validation

A validation key names: artifact/tree digest; base revision if relevant; selected dependency revisions; check specification and executable/arguments or shell script digest; runtime image/toolchain identity; effective nonsecret environment and secret-version references; input/output mount policy; locale/timezone/terminal profile where relevant; network policy and external-state assumptions; validator/policy versions. Record the actual executed context, not merely the requested one. A failed prerequisite is “not run,” not “passed.”

For Git projects, a commit ID alone is inadequate when dirty, untracked, ignored, LFS, submodule, generated, or external files are consumed. The snapshot manifest records included paths, bytes, file modes, symlink targets, submodule materialization, and exclusions. Paths need lossless byte representations on Linux. Large-file pointers and their materialized inputs are distinguished. Unsupported special files are rejected or explicitly excluded. Snapshotting a directory while another process writes it is not atomic; a scan/watch cycle cannot prove otherwise.

Three binding levels prevent impossible guarantees:

| Level | Permitted claim | Limitation |
| --- | --- | --- |
| Sealed inputs | Check executed against these immutable, runtime-mounted inputs | Only declared closure; networks, clocks, services, and writable outputs may still vary |
| Initial snapshot | Check launched from a recorded input snapshot copied into a writable workspace | Mid-run mutations may affect what was actually read; start/end hashes cannot exclude transient changes |
| Mutable/unknown | Check observed in this directory/context at this time | Cannot certify an exact consumed source revision |

Sealed checks mount source read-only and write to declared output locations; tools requiring arbitrary source mutation fall back to an honestly labeled weaker level or a redesigned check. Do not claim sealing merely because a copied directory was hashed. The runtime must prevent all writers within the stated threat model; host/root compromise is outside that claim.

Current validation is a query comparing the validation key and required quality against the current review subject. Changes to source, dependencies, check specification, effective environment, runtime, or relevant policy can make it stale. Evidence expiry makes a result unauditable, which is distinct from staleness or failure. Both block acceptance when the acceptance requirement demands current inspectable evidence. External services require a freshness window and stated limitations, never an assertion that a local snapshot freezes the world.

Example: task T7 selects source S10 and dependency D3. Check C passes on S10/D3 under runtime R2. Source S11 or dependency D4 is a different subject. A summary mentioning C is historical support only. Integrating S10 into tree M1 also creates a new subject. A human can accept S11 with an explicit exception, but the system cannot display C as validating S11.

## Dependencies and coordination

Use a task DAG with typed edges: requires-artifact, requires-validation, requires-decision, and exclusive-resource. Each edge selects a concrete revision or a versioned selection policy. Ready-state calculation detects cycles, missing producers, invalidated selections, and unavailable evidence. Do not auto-follow a moving “latest” during an execution. New upstream revisions create a re-evaluation item; an already running pinned attempt may finish and remain useful historically.

Agents publish bounded messages and artifact proposals to a task mailbox. Messages carry sender, target task revision, causal reference, and purpose; they do not directly edit another agent's memory or grant authority. Delegation inherits the intersection of parent capabilities and explicit child limits, shares the parent budget ceiling, and has a maximum fan-out/depth. Parent exit follows a declared continue/cancel/escalate rule. Cancellation propagates as a request, not assumed completion.

Lease directories, test databases, ports, devices, and integration slots explicitly where the runtime can honor exclusivity. Advisory leases prevent cooperative collisions; they cannot fence arbitrary host programs. Use resource generations, expiration, renewal, and deterministic acquisition order or all-or-none reservation to avoid deadlocks. Expiration revokes future admissions; effects of already running processes require runtime termination or isolation, not a database flag.

## Filesystem collaboration and integration

Default agent editing uses a separate owned worktree/directory per attempt. Git documents shared repository data with per-worktree HEAD/index; it is an editing mechanism, not a trust boundary. [Git worktree documentation](https://git-scm.com/docs/git-worktree)

Alternatives: shared mutable tree is fast but conflicts and provenance are weaker; worktrees are efficient but share Git administration/configuration; independent clones reduce shared administration but cost more; filesystem snapshots/overlays can support sealed inputs but require platform-specific validation. Non-Git directories use copy/snapshot manifests. Runtime profiles in document 04 determine actual authority.

Serialize shared Git administration through a repository coordinator for managed operations. Worktree writers can still touch shared hooks/configuration if that storage is exposed; strict runtimes expose source snapshots without the common Git directory. Integration creates a new artifact revision from explicit input revisions, records conflict resolution, and runs relevant checks again. No last-writer-wins merge of code, dependency selections, or human decisions.

Cleanup is a separately authorized lifecycle: identify owned roots, stop/reconcile executions, preserve unreviewed artifacts, check dirty/untracked state, then remove only the owned resources. Never infer ownership from a directory name alone. A force-delete option is an explicit destructive action, not routine garbage collection.
