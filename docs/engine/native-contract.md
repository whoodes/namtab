# Native agent, work and observation contract

[Campaign](index.md) · [Core](compatibility-and-core.md) · [Runtime and recovery](runtime-persistence-security.md) · [Human journeys](frontend-contracts.md)

**Proposed native contract for product v1; unimplemented, private experimental wire version pending N0.** The existing small domain/protocol tests are useful foundations, not this API's completed implementation. Source rationale: research S02–S05, S15–S19, S31–S32. Policy rules below are Project X decisions.

## Objects and authority

| Object | Identity / lifetime | Meaning and authority |
| --- | --- | --- |
| Principal | Authenticated connection binding + principal ID + capability generation | Human, agent or adapter role. A display name/process title is not identity. Capabilities narrow operations, sessions, data, runtime/context, retention and expiry. |
| Workspace/context | Context ID + revision + runtime profile + source/artifact revision | Filesystem/worktree and environment contract. A Git worktree isolates edits, not processes or secrets. Shared writable contexts require an explicit coordination policy. |
| Task / attempt | Task ID; unique attempt ID; dependency revision | Work objective versus one execution attempt. Attempt status is a domain decision with supporting facts, not a terminal prompt guess. |
| Session | Session ID + incarnation + interpretation profile | Continuity of an interactive compatibility surface. Several tasks may reference it; no exclusive task ownership is inferred from screen position. |
| Process execution | Execution ID + runner incarnation + OS handle identity | A managed `argv` execution may use pipes or a fresh PTY. Child status and stream observations are facts within the runner's visibility. |
| Operation | Principal-scoped request ID + canonical payload binding + authority epoch | One requested mutation. Reusing an ID with different data conflicts. Transport retry is not permission to repeat an uncertain effect. |
| Observation | Session/incarnation + event frontier + semantic/geometry/profile revisions + completeness | Immutable view of what the engine has processed. It does not prove the application will remain in that state. |
| Artifact / validation | Authorized object identity + exact content/context revision + producer attempt + evidence range | An exit code is not artifact correctness. A later edit invalidates revision-bound validation; a truncated range cannot be cited as complete. |
| View / writer lease | Endpoint/view IDs; focus generation; writer and sizing epochs | Focus identifies a human interaction destination; authority permits a bounded action. Native agents address sessions explicitly and never acquire desktop focus implicitly. |

Keep causal relationships explicit: operation → admission → runner delivery → stream events → observation → validation decision. Use session-local increasing event numbers and service authority epochs; do not fabricate a globally ordered clock across independent sessions. Cross-session dependencies cite explicit frontiers/artifact revisions. Wall time is diagnostic; expiry uses a defined trusted clock boundary and is invalidated across service restart when continuity cannot be established.

One mediated application-input writer per interactive session; separate readers can observe with data grants. The canonical terminal reply lane has its own restricted authority and ordering. A human override atomically advances the writer epoch and rejects queued old-epoch input before dispatch. It does not retract already written bytes, revoke external credentials, stop all descendants or undo filesystem/network effects.

## Minimal operations and asynchronous outcomes

Use a small typed protocol, not an arbitrary remote object graph:

| Operation family | Required intent | Result and limits |
| --- | --- | --- |
| `CreateTask`, `StartAttempt`, `RunProcess` | Context revision; argv array; cwd/env policy; pipe/PTY choice; output/capture and runtime profile | Immediate execution/operation IDs; later started/exited/drained evidence. A shell command string is a separately authorized explicit shell execution, never inferred from argv. |
| `CreateInteractiveSession` | Shell/program argv; declared compatibility profile; geometry; runtime and capture policies | Session identity and negotiated capabilities. Existing-shell keystrokes are not equivalent to creating a managed process. |
| `Observe`, `Subscribe`, `ReadRange` | Explicit session or execution; requested projection; base/event cursor; data grant | Snapshot/delta/range plus completeness, gaps and availability. Credit-bounded; a stale cursor requests a new snapshot, not a guessed delta application. |
| `AcquireWriter`, `ReleaseWriter`, `TakeOver`, `Resize` | Expected epochs/context; endpoint; sizing policy where relevant | New epoch or typed rejection. UI attach/detach does not kill work. |
| `SendKey`, `CommitText`, `Paste`, `Mouse`, `FocusReport`, `SendRaw` | Distinct event type; explicit target; expected observation/profile/lease/focus/geometry; bounded payload | Receipt with encoded/delivered frontiers, later observed transitions. Raw is an explicit separately granted escape hatch, not a bypass. |
| `ViewCommand`, `ReadSelection` | Explicit endpoint/view, mapping/focus generation, bounded UI operation or identified range | Separate UI/observation/export capability; no application bytes, writer acquisition or implicit clipboard permission. Product focus changes are not terminal focus reports. |
| `QueryOperation` | Prior operation ID/namespace; read authority | Reconcile retained delivery/outcome metadata without resubmitting; expired or unavailable facts stay unknown. |
| `CancelOperation`, `SignalExecution`, `StopRuntime` | Target operation/execution/runtime identity, scope, escalation policy and grant | Cancellation requested/observed/no-longer-applicable/unknown. Process stop needs separate OS observations. |
| `RegisterArtifact`, `RecordValidation`, `RequestDecision` | Authorized immutable object/evidence/context binding | Inspectable provenance, freshness and decision state. Arbitrary process output cannot invoke these methods. |

Long operations return a handle and publish status; waiting does not hold a writer lease or event-loop thread unnecessarily. `WaitFor` predicates are narrowly typed (specific execution exit, observed frontier, artifact revision, explicit adapter event), with timeout/cancellation. No generic “wait until terminal ready” predicate claims application readiness. Input context binds session/incarnation, host/helper and service authority epoch, endpoint/writer generation, observation and freshness policy, geometry/buffer generation, mode/profile/capability revision, known prompt/protection revision, action/approval reference, deadline and request ID. View-derived requests also bind mapping/focus generation; direct session input never requires human focus. Absent fields are not wildcard authority. Capabilities separately state representability, negotiation, authorization and endpoint limits.

Input batches are bounded sequences of operations, not atomic TUI transactions. Revalidate each not-yet-started event; fix its encoding before the first byte and never re-encode a partial suffix under changed modes. A takeover may discard the remaining suffix, leaving application state uncertain.

Illustrative shape, **not** an implemented wire schema or stable method spelling:

```json
{
  "protocol": "px-native/0.x",
  "request_id": "principal-A:request-17",
  "operation": "run_process",
  "task": "task-4",
  "attempt": "attempt-9",
  "expected": { "context": "ctx-2", "revision": "31", "authority_epoch": "7" },
  "execution": { "argv": ["tool", "--check"], "io": "pipes", "profile": "host-approved" },
  "output_policy": "bounded-memory"
}
```

Authentication/capabilities come from the connection, not a caller-supplied principal label. An admitted response supplies an operation/execution ID and receipt cursor; later events bind exit, stdout/stderr ranges and drain status. The service resolves an authorized cwd/env/runtime policy from the context. A human can open a terminal for that context separately; no shell line or ANSI prompt is needed to identify the managed execution.

Receipt facts form a progression with branching uncertainty:

```text
received → rejected
        → admitted → not_dispatched / expired / revoked
                   → dispatch_started → delivered(prefix) → observed_transition
                                      → partial / delivery_unknown
managed execution separately: created → running → exit_observed → streams_drained
task separately: domain acceptance lifecycle (not derived from process or operation status)
```

These are distinct fields/events, not one misleading `success` Boolean. `delivered` means the runner's specified OS write boundary, not that a program consumed, accepted or acted on the bytes. `observed_transition` cites subsequent state; the cause may be inferred if the application also emits unrelated output. A process exiting does not prove all output has drained or every descendant is gone. A task can remain unresolved after process exit.

Metadata admission and its receipt reservation must be recorded atomically before dispatch, or the operation is rejected. OS delivery and journal persistence cannot be made one transaction. A lost acknowledgement produces `unknown` and reconciliation by ID; never automatically replay non-idempotent Enter, paste, launch or signal operations. Receipt retention is reserved; after expiration the service rejects reuse/retry as expired/unknown, rather than forgetting and executing it again. Scope and storage failures are typed errors, with bounded sanitized diagnostics.

N0 should make this implementable with a server-issued authority namespace plus bounded monotonically accepted request sequence/expiry. Old namespaces/expired or already accepted sequence numbers cannot become new mutations after receipt compaction; reconciliation is a separate read. Parallel work can still dispatch asynchronously after ordered admission. Durable receipt metadata need not retain literal argv, environment or input: these may contain secrets. Keep full payload binding in authorized volatile state by default; persist it only under an explicit input-audit policy with secret references. If the binding is unavailable, refuse a repeated mutation rather than pretending to compare it or repeating it. A content hash is not automatic redaction.

## Observation and evidence schema

An observation envelope contains identities/epochs, engine/profile/Unicode versions, processed input/output/resize event cuts, capture classification, geometry/active buffer, cell/grapheme/styles/cursor and supported mode metadata. Large history/artifact projections are paged with stable IDs; snapshots and deltas share base/target revisions. Agents request bounded task/stream/cell/range projections instead of receiving every pane or unbounded transcript. Keep model-specific tokenization/inference outside the core, and preserve evidence IDs when selecting or summarizing context. A slow subscriber may skip presentation frames and receive `resync_required`; it may not silently skip canonical parser input.

Every status carries one of:

- **Observed by an authorized component:** OS child exit, bytes read, canonical cell/mode state, broker grant/lease, immutable artifact digest, completed validation with exact inputs.
- **Reported by a cooperative application/adapter:** command boundary, progress, native plan or semantic widget. Record producer identity/schema and trust; authenticate the channel but do not assume the producer tells the truth.
- **Inferred:** prompt detection, OCR, likely TUI selection, apparent readiness or causal attribution. Record method and evidence, never elevate to permission or OS fact.
- **Unavailable/unknown:** capture denied, retention gap, state lost, output not drained, stale geometry or unreachable runner. These are valid results.

Native structured output uses a separately authenticated, bounded side channel, associated with an execution/attempt and constrained event schemas. It can supply domain-specific data without reverse parsing ANSI. It cannot change the supervisor's permissions or self-certify an artifact's acceptance. OSC prompt markers use the weaker compatibility hint path even if they resemble native event names.

## Observe–act–observe, races and human override

1. Observe explicit session S/incarnation I at frontier F, geometry G, mode/profile P. Select a bounded action justified by that evidence. Unknown opaque-TUI focus is still unknown.
2. Submit the target, writer epoch, capability, action ID and freshness policy. The broker rechecks context/approval and the engine serializes encoding against canonical mode state. Coordinate actions require matching geometry and screen revision; stronger screen-bound actions require exact frontier. Output churn returns `stale`, not silent revalidation.
3. At the runner's final input queue, recheck writer/authority epoch immediately before the first write. Takeover and dispatch use one ordered control channel; acknowledgements establish what was fenced. A write already underway may finish partly, so report its prefix or uncertainty. No global “atomic user click” claim.
4. Observe the resulting frontier/receipt. If the expected transition is absent, distinguish delivery unknown, no change, unrelated output, unsupported input and wrong inferred app state. Do not repeat a non-idempotent action to make the screenshot look right.

Strict screen-bound actions can starve on a continuously updating screen. Offer an explicit owner-approved session-level interaction grant with weaker freshness and visible uncertainty, or human control; do not quietly widen the action's authority. Human input may use that interactive policy, while sensitive approvals remain revision-bound protected UI operations. Keyboard modifiers, release/repeat, IME and lost host facts follow the [input profile](compatibility-and-core.md#input-policy).

There is no OS primitive that atomically compares an application's screen/widget state with delivery of a key. The backend's processed frontier can be current to its event cut while newer output is unread or the application changes internally without output. An exact revision fence is therefore a real backend guarantee, not an atomic TUI transaction. Cooperative typed application operations can supply stronger semantics; opaque interaction retains this race and must use an appropriate uncertainty grant or human intervention.

## Coordination and integration limits

Parallel agents own separate attempts/processes/interactive writers by default. Dependencies become ready only at recorded revisions. A group budget constrains admissions, output, retained evidence and concurrency; it is not a claim to control an agent's unmediated provider/network calls. Conflicting worktrees/artifact publication need an explicit integration decision. One agent cannot mutate another's writer context by changing a task label.

ACP/MCP adapters are optional later consumers of this contract, not foundational dependencies. Map cancellation and lifecycle semantics explicitly: ACP v1 `terminal/release` kills a running command (S17), while Project X detach does not. Foreign output truncation must become a gap, and foreign task completion must retain its trust level. No automatic mapping from an external permission request to an internal grant. Native APIs remain useful with a deterministic local client and no model provider.
