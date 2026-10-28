# ENG2 scoped pull observations

[Architecture](../architecture/index.md) · [Agent guide](../agents/index.md) · [Issue78](https://github.com/whoodes/namtab/issues/78) · [Broader observations backlog49](https://github.com/whoodes/namtab/issues/49)

**Unmerged engine experiment**, based on merged PR77/main0772251. `namtab.scoped-observation/0`, advertised as `scoped_observe_v0`, adds bounded pull projections over private experimental `namtab.mk1/1`. This is a precursor to49, not subscriptions/deltas/waits, full H0 admission, a stable SDK or fleet capacity. No native-harness feature, provider/model, renderer, dependency or security-policy change.

## Why this slice

Legacy `Observe` builds every canonical screen and bounded attempt stream, then refreshes registered artifact revisions. Filtering afterward (including current MCP) does not avoid that work. The new broker path projects the selected scope directly under one state lock, without calling full `snapshot` or `file_revision`. Formatting one selected stream remains bounded; Work does not clone cells/stream bodies or read files. Legacy Observe semantics remain unchanged, including artifact refresh.

## Admitted wire and resources

Request: `{"op":"observe_scoped","epoch":"EXPECTED_BROKER_EPOCH","scope":{"kind":"work"}}`. Session/attempt selectors use `{"kind":"session","session":"session1"}` or `{"kind":"attempt","attempt":"attempt3"}`. IDs are nonempty, at most80UTF-8 bytes, with no control characters. The expected epoch must match the connected broker. Missing/stale/invalid targets return explicit read errors without a mutation receipt or fallback projection.

| Projection | Included | Intentionally not attested |
| --- | --- | --- |
| Work | Bounded endpoint/role/connection metadata, capabilities, immutable task contracts, session ownership/process/revision summaries and attempt origin/state/cancellation summaries | No terminal cells, stdout/stderr, artifact freshness or receipts. Absence is an explicit view type, not an empty whole-workspace snapshot |
| Session | One exact existing `Session`, including canonical screen/context | No other session/attempt data or opaque application semantics |
| Attempt | One exact existing `Attempt`, immutable launch origin/task revision and bounded sanitized stdout/stderr | Not full retained capture, task acceptance or a conversion to a PTY |

Response has an optional **`observation`** distinct from `snapshot`, containing `version`, `profile`, `epoch`, `endpoint`, `degraded` and tagged `view`. A scoped success has no full snapshot, mutation receipt or resume credential. The outer response ID correlates one exchange. The profile attests a lock-time broker projection, **not** a global event/causal cut, subscription cursor, application/OS transaction or durable recovery point. Screen/mode revisions and writer generations retain existing meanings.

No admission expands:2lifetime PTYs,2lifetime managed attempts,4live public connections,15ordinary+1restricted lifecycle retained identities, existing frame/ID/receipt/journal/input/output limits. Requests/replies remain within1MiB; Rust client socket operations retain2s timeouts. Selected streams use the existing65,536-byte sanitized projection bound and truncation flag. Work `source_truncated` describes the raw source cap; without formatting the omitted body it does not certify a complete text preview. Arrays remain bounded by existing broker admission. No subscriber queues, event retention or background subscription workers are introduced.

Read scope is **not** an ACL/security boundary: existing same-UID roles can already observe the workspace. It grants no writer/process authority and does not prevent a caller requesting legacy full state or exporting data externally. Terminal output/task text remains untrusted data. Capture-off and provider-export consent are unchanged.

## Public client and CLI

`Client::observe_scoped(ObservationScope)` discovers support once per live transport through legacy Observe. That bootstrap **can rehash artifacts and copy full state**. Subsequent scoped pulls use the advertised capability; absence yields Unsupported without a scoped dispatch or silent full-state fallback. Explicit reconnect/resume creates a new transport and renegotiates. No read or uncertain operation is automatically replayed.

The client validates response/profile/version/epoch/endpoint and exact requested view/ID. Missing, mixed, unrequested or misbound projections fence the connection. A normal missing-target refusal is distinguishable from transport corruption. Generic raw scoped calls with another epoch are rejected locally before ID allocation/write; the broker independently validates raw wire callers. Malformed discovery is fenced rather than trusted as capability evidence. Capability support is not permission.

On this branch, with an explicitly chosen run/socket:

```sh
target/debug/namtab-agent --socket /absolute/chosen/socket work
target/debug/namtab-agent --socket /absolute/chosen/socket session session1
target/debug/namtab-agent --socket /absolute/chosen/socket attempt attempt3
```

Each **one-shot CLI** performs full bootstrap and consumes a retained endpoint identity. Repeatedly spawning it is not the optimization or a scale proof. Use a long-lived Rust client or explicit JSON-line protocol client for repeated scoped pulls. The raw JSON-line route requires callers to discover support and supply the exact epoch. Human launch/run paths are owned by the [user guide](../users/index.md); never discover/attach to user sessions for a test.

Current MCP and native/diagnostic polling remain on legacy Observe; this increment does not migrate them or claim end-user UI latency improvement. MCP's targeted preview already limits its exported payload, but still pays full broker observation cost. Consumer migration is a later independently reviewed slice.

## Validation and measurement

`make scoped-test` builds current binaries and runs a fresh isolated **headless** journey, with two SCRIPTED clients, a human observer,2PTYs,2attributed managed attempts and a temporary fourth measurement connection. It checks exact scopes, bounded noise, real CLI use, stale/missing refusal, targeted human takeover while B continues, stale-owner zero bytes and no read mutation journal. Exact legacy/scoped payload comparison occurs only at quiescent targets, not across changing output cuts. No GUI/model/user session is involved.

The workload retains100paired legacy/Work RPC samples after10warmup pairs, interleaved AB/BA. Raw roundtrip times and actual wire-line bytes are saved with hardware/kernel/build/harness hashes and connection/output/artifact tuple. Timing includes Python encode/send/read/decode; legacy samples include a registered synthetic1MiB artifact rehash. It is not server-only CPU, human key-to-paint, model inference, all-process resource/fairness or dozens-agent qualification. CI asserts structural/byte/data correctness, **not** a latency superiority threshold. One-shot bootstrap is outside these steady-state samples and disclosed separately.

`make foundation-verify` includes this journey plus failure-propagation regression. Protocol/broker/client tests exercise typed omission, nonmutation, artifact-refresh separation, sanitization, selector/epoch refusal, malformed peers, same-connection caching and reconnect negotiation. Historical X4/EA1 FAIL and original focus/Ctrl-C/setup records remain unchanged. Report exact source/tree/dirty status and preserved failures; initial working-tree runs do not certify the final commit.

Remaining49/H0 work: event/cut identity, credit/backpressure, deltas/gaps/resync, canceled waits, calibrated larger populations, scheduler fairness and consumer integration. Scoped reads remove unrelated observation work; they do not solve those gates.
