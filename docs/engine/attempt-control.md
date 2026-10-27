# ENG1 exact-attempt control contract

[E6 ownership](../adr/0006-engine-and-native-harness-workstreams.md) · [issue70](https://github.com/whoodes/namtab/issues/70) · [native harness consumer](../ux/fleet-control.md)

**Proposed experimental contract, paused/unimplemented in this branch:** `namtab.attempt-control/0`, capability `attempt_cancel_v0`, on the existing experimental MK1/1 wire. This is a design for issue70, not resource/schema admission, a stable public API, full H0 or a capacity claim. A partial untested ENG1 prototype is preserved separately; cleanup72 does not include it. Depends on unmerged PR68/9bcb627; no merge is authorized.

## Typed target and authority

`CancelAttempt { epoch, attempt, origin_endpoint, origin_request_id }` binds a unique attempt in the exact broker lifetime and its immutable launch origin. Epoch is64hex characters, bounded identities use the existing80byte limit, and launch request ID is a positive safe integer. The broker validates the full tuple and role under authoritative state. A declared human may target it; an agent may target only its own launch. Same-UID/developer-host trust remains unqualified; this is not human attestation.

The caller must observe `attempt_cancel_v0` before dispatch. Missing support is refusal, not a signal/PID fallback. MCP's existing nine-tool bridge gains no tool implicitly. No PTY writer, task acceptance, provider request or entire namespace is canceled by this operation.

## Receipt, observation and failure semantics

Ordinary reservation/dedupe/journal rules apply. `applied` means a cancellation request was registered, not that a child/descendant stopped. A typed `CancellationRequested { attempt, status }` result distinguishes requested, already requested and already exited. Reused request IDs cannot bind a different target; a new request is not automatic retry permission after uncertainty.

The managed attempt snapshot retains `cancellation { requested_by, request_id, status }` and observed exit code/signal. The existing monitor uniquely owns Child. A bounded per-attempt flag is checked there; no caller PID is stored as authority and no second waiter races the handle. Status `exit_observed` describes a wait result, not proof Namtab caused it or the task succeeded. Signal9 can be observed without claiming all side effects stopped. Pipe drain, direct-child exit, descendant lifetime and task acceptance remain different.

Wrong epoch/origin/role refuses before cancellation publication. Unknown process/signal outcome remains unknown; no hidden retry or namespace-wide stop. Degraded ordinary mutation admission can refuse this targeted request. Protective shutdown from PR68 is a separate human action with different scope and uncertainty semantics, never an automatic fallback.

## Validation contract

Tests must use fresh isolated resources: two attributed attempts, cancel A while B continues, exact side effects/streams and observed exits; wrong/stale epoch and launch origin, other-agent refusal, duplicate/conflicting IDs, already-exited/requested cases and no peer/PTY/grant change. Tests/models do not prove arbitrary OS scheduling or descendant containment. Include this real headless journey in active CI with runner failure propagation.

Measure only named boundaries under recorded workload/build/hardware. One control receipt sample is not a latency distribution, fleet throughput or a production SLO. No provider/model/trust change is needed or permitted for this slice. Existing source/data/pins/history and owner sessions remain intact.
