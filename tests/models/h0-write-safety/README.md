# H0 lifecycle/write-safety model — test-only profile `h0-write-model/0`

Hand-authored 2026-10-19 from the [H0 run/assignment rules](../../../docs/engine/native-contract.md#proposed-h0-run-and-assignment-lifetimes), [receipts](../../../docs/engine/native-contract.md#receipt-admission-retention-and-reconciliation) and [per-write ordering](../../../docs/engine/native-contract.md#final-dispatch-and-suffix-fencing), after owner-approved PR54. No external corpus, measured process output or upstream implementation was copied. This is a finite executable design model, **not** production dispatch, an agent SDK, native wire schema, authentication, storage or OS experimentation.

Run with the pinned Node (no installation required on the prepared checkout):

```sh
.tools/node/node-v24.21.0-linux-x64/bin/node --test tests/system/harness-write-model.test.mjs
```

The normal `tests/system/*.test.mjs` discovery includes it; no new CI lane, dependency, package or production import is added.

## State, trusted inputs and bounds

- `model.mjs` is a pure copied-state transition function. `invariants.mjs` audits obligations without importing the model's guard/helpers. The system test supplies fixed expected outcomes, deliberate broken variants and bounded graph exploration.
- Two initially issued runs (RA/RB), two owned request namespaces, one exact context, one exclusive assignment, attempts A1→A2, one session and one application writer (RA/RB/H). Run issuance/authentication and control/keeper fact classification are **assumed**, not proven by labels. Claim/transfer/release/run-close events represent already-authorized ordered decisions; their own receipt transactions are abstracted, not implemented. The submit/key subset models byte-intent decisions only.
- At most four decisions/namespace, eight admitted application operation records plus two fixed canonical replies; generations1–8, clock ticks0–6, deadline/namespace expiry5, frozen synthetic ASCII payload0–3bytes, at most three OS-call *tokens* per operation and one unresolved call/session. Rejected valid next decisions consume a sequence; malformed/wrong-owner/future keys do not. Exhaustion is explicit refusal, no wrap/reuse. Retained identity alphabets are RA/RB, NA/NB, A1/A2, T1/T2 and C1; unknown/oversized symbols fail preflight without consuming a key. These integers and strings are model values, not native wire fields or product capacities.
- Assignment transfer preserves immutable attempt/task/context pins; replacing the attempt retires A1 and taskT1. Target/mode/protection/observation/continuity dependencies are represented only by one session-guard generation plus a known-state flag. No key encoder, parser, prompt detector, protected-prompt authentication, view focus/geometry/IME model or complete N0-03 field matrix is claimed.
- Authorization and completion are separate steps. An injected result is zero, a known prefix/full extent, or unknown **with the call declared ceased**. Lost ACK retains the unresolved token/queue until reconciliation. External reality is not read. Terminal reply tokens use fixed allowlisted bytes, distinct canonical query IDs and the same queue; they do not borrow application run/writer authority.
- Partial/unknown abandonment taints continuation. Explicit trusted fresh/risk-tolerating re-arm advances the session guard, closes old queued intents/replies and never reopens the original operation. It does not prove the application's internal parser is safe; that risk is assumed explicitly tolerated for the new context.
- Missing volatile binding prevents repeat submission. Missing receipt never proves zero effects; retained synthetic state supports the model audit only, **not a durable payload-retention/privacy design**. Epoch change closes old submission; historical lookup is separate. Namespace retirement/clock continuity/real receipt quotas and expiry/storage failures remain outside this profile.

## Evidence and falsifiers

| Obligation | Executable challenge |
| --- | --- |
| Run/assignment identity and immutable attempt pins | Claim CAS race, compatible transfer/release, stale attempt request and principal/namespace mismatch; a new holder never relabels old results |
| Same-key reconciliation, not redispatch | Duplicate before/after delivery, changed-body conflict, missing binding/receipt, future/unknown/old-epoch keys, empty encoding |
| Every first/suffix call checks authority | Run close, capability revoke, assignment transfer/attempt change, writer takeover/disconnect, session fence and epoch change before first call or between partial writes |
| No retroactive effect guarantee | Fence while a call token is unresolved, then independently inject zero/prefix/full/unknown; late result refines the original receipt, never starts a suffix |
| No byte-lane interleaving | Partial operation retains queue; replies/other intents wait for unresolved call; tainted state needs explicit re-arm and fresh bindings |
| Retry does not create certainty | Recovery policy chooses query for missing/unknown evidence; a new key is a distinct intent, not semantic server dedupe or proof that retry is safe |
| Audit independence and sensitivity | Broken run, assignment and suffix guards must produce audit failures; forged offsets/result attribution/binding changes are challenged independently |
| Finite ordering coverage | Breadth-first exploration to depth6 across write, result choices, close, transfer, revoke, takeover and lost ACK, with state+depth deduplication;20,000-node hard cap fails rather than truncates |

Graph diagnostics report depth-indexed states and explored edges (including refusals), not all possible executions or capacity. Directed tests cover other transitions excluded from the exploration alphabet, including reconnect, receipt loss, deadlines, re-arm and stale attempts. A test time is harness execution time, **not terminal latency**. The model and auditor share a written specification and can share a misunderstanding; fixed goldens, omission variants and independent adversarial reviews reduce that risk without eliminating it.

## Explicit gaps

No production conformance claim: there is no actual service/keeper authentication, durable commit/dedupe, parallel memory race, IPC, syscall, cancellation/process group, PTY, model/provider, renderer, sandbox, latency/fairness or human input experiment. The actual implementation needs its own independent traces, fault injection and every-write seam review. Liveness/starvation, arbitrary depth, all Unicode/control sequences, multi-session scheduling, multi-context runs and full receipt/permission lifetimes are not proven. This model does not complete issue46/47, H0, NG1 or an old X4/EA1 gate.
