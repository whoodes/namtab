# ENG1 exact-attempt control contract

[E6 ownership](../adr/0006-engine-and-native-harness-workstreams.md) · [issue70](https://github.com/whoodes/namtab/issues/70) · [native harness consumer](../ux/fleet-control.md)

**ENG1 bounded experimental implementation:** owner resumed issue70 on2026-10-28 after merged cleanup73/main `448b091`. `namtab.attempt-control/0` / capability `attempt_cancel_v0` extends the experimental MK1/1 wire; it is not a stable API, full H0 or fleet-capacity admission. The original partial patch/index is retained unchanged in its old worktree; this implementation uses the cleaned baseline in a separate branch. NAT1 is a separate consumer draft. No merge or next-scope authority follows.

## Admitted tuple and resources

| Boundary | ENG1 limit / explicit refusal |
| --- | --- |
| Host / dependencies | Existing Ubuntu22.04x86_64/Rust1.91.0, unchanged Cargo/pnpm/tool pins; no new library, provider or isolation profile |
| Wire | `namtab.mk1/1`, advertised `attempt_cancel_v0`, profile `namtab.attempt-control/0`; shared Rust public operation/receipt DTOs, not generated N0 wire or worker RPC |
| Target | Exact64hex epoch, attempt/origin identifiers bounded to80bytes, positive safe-integer launch request ID; unknown/extra authority fields refused by strict decoder |
| Namespace | Existing2lifetime managed attempts,2PTYs,four live connections,15ordinary+one restricted lifecycle identity; no freed/reused capacity after cancellation |
| Transport/storage | Existing1MiB frame,65,536-byte serialized mutation ceiling,2s client timeout,4,096retained decisions per endpoint and32MiB metadata journal. One bounded cancellation record/atomic flag per attempt, no new pending queue or durable output capture |
| Process | Existing unique direct-child handle/monitor; async request sets a flag, monitor polls at20ms then uses its owned kill/wait boundary. Poll interval is not a response/termination deadline; kernel wait can outlive caller timeout |
| Unknown/recovery | Degraded or unknown attempt refuses new cancellation; no automatic retry/reconnect/PID/namespace fallback. Live receipt queries preserve original binding. Restart loses live operation continuity; no automatic replay or durable cancel recovery |

The public client re-observes support and the immutable target before dispatch, validates receipt/result binding, and fences malformed replies with unknown effects. Capability is support, not authority. Raw protocol clients must negotiate too; unsupported old brokers refuse the unknown operation. A lost response cannot be repaired by silently issuing another cancellation.

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

## Reproduce and interpret

```sh
make mvp-build
python3 -B tests/mvp/attempt_control.py --bin-dir target/debug --artifacts .artifacts/eng1-feedback-001
make foundation-verify
```

Use a fresh artifact path each time. The isolated fixture starts its own broker and two **scripted** public agents with a shared exact task revision. It checks malformed/stale/other-owner targets, duplicate/conflicting request IDs, A's independent exit/signal observation, B's continued writes **after** that observation, bounded noisy stdout and B's natural exit. It does not attach/discover user sessions or invoke a model. The aggregate discovers the real journey and tests failure propagation. Unit/fake-peer cases cover unsupported capability, stale observations, receipt misbinding, unknown/degraded refusal and completion-journal failure; injected metadata faults are not real disk/OS crash qualification.

The receipt is a bounded command-registration fact; the snapshot's cancellation origin remains the first requester even after later human observations/repeated requests. A completion-persistence failure can leave the receipt unknown while later live observation proves direct-child exit. Neither fact implies task acceptance, all output drained, all descendants stopped, durable recovery or fleet scalability. Existing X4/EA1 FAIL and native focus/IME/a11y limits remain unchanged. Any timing value in the fixture is a named local sample, not an achieved p50/p95/p99 budget.
