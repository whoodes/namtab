# Scoped pull observations and MCP export

[Architecture](../architecture/index.md) · [Agent guide](../agents/index.md) · [Issue78](https://github.com/whoodes/namtab/issues/78) · [Broader observations backlog49](https://github.com/whoodes/namtab/issues/49)

**ENG2 merged in PR79/main ee5fdfc.** The ENG3 MCP consumer extension below merged in PR81/main97741bb. `namtab.scoped-observation/0`, advertised as `scoped_observe_v0`, adds bounded pull projections over private experimental `namtab.mk1/1`. This is a precursor to49, not subscriptions/deltas/waits, full H0 admission, a stable SDK or fleet capacity. No native-harness feature, provider/model, renderer, dependency or security-policy change.

## Why this slice

Legacy `Observe` builds every canonical screen and bounded attempt stream, then refreshes registered artifact revisions. Filtering afterward (including legacy MCP) does not avoid that work. The new broker path projects the selected scope directly under one state lock, without calling full `snapshot` or `file_revision` for the original Work/Session/Attempt scopes. Formatting one selected stream remains bounded; plain Work does not clone cells/stream bodies, scan stream loss or read files. Legacy Observe semantics remain unchanged, including artifact refresh.

## Admitted wire and resources

Request: `{"op":"observe_scoped","epoch":"EXPECTED_BROKER_EPOCH","scope":{"kind":"work"}}`. Session/attempt selectors use `{"kind":"session","session":"session1"}` or `{"kind":"attempt","attempt":"attempt3"}`. IDs are nonempty, at most80UTF-8 bytes, with no control characters. The expected epoch must match the connected broker. Missing/stale/invalid targets return explicit read errors without a mutation receipt or fallback projection.

| Projection | Included | Intentionally not attested |
| --- | --- | --- |
| Work | Bounded endpoint/role/connection metadata, capabilities, immutable task contracts, session ownership/process/revision summaries and attempt origin/state/cancellation summaries | No terminal cells, stdout/stderr, artifact freshness or receipts. Absence is an explicit view type, not an empty whole-workspace snapshot |
| Session | One exact existing `Session`, including canonical screen/context | No other session/attempt data or opaque application semantics |
| Attempt | One exact existing `Attempt`, immutable launch origin/task revision and bounded sanitized stdout/stderr | Not full retained capture, task acceptance or a conversion to a PTY |

Response has an optional **`observation`** distinct from `snapshot`, containing `version`, `profile`, `epoch`, `endpoint`, `degraded` and tagged `view`. A scoped success has no full snapshot, mutation receipt or resume credential. The outer response ID correlates one exchange. The profile attests a lock-time broker projection, **not** a global event/causal cut, subscription cursor, application/OS transaction or durable recovery point. Screen/mode revisions and writer generations retain existing meanings.

This read API does not expand admission:2lifetime PTYs,2lifetime managed attempts,4dispatching public connections,15ordinary+1restricted lifecycle retained identities, existing frame/ID/receipt/journal/input/output limits. Requests/replies remain within1MiB; Rust client socket operations retain2s timeouts. Selected streams use the existing65,536-byte sanitized projection bound and truncation flag. Work `source_truncated` describes the raw source cap; without formatting the omitted body it does not certify a complete text preview. Arrays remain bounded by existing broker admission. The separate [ENG5 draft](human-admission.md) explicitly adds one auxiliary handshake/retirement transport, not a fifth dispatcher. No subscriber queues, event retention or background subscription workers are introduced.

Read scope is **not** an ACL/security boundary: existing same-UID roles can already observe the workspace. It grants no writer/process authority and does not prevent a caller requesting legacy full state or exporting data externally. Terminal output/task text remains untrusted data. Capture-off and provider-export consent are unchanged.

## Public client and CLI

`Client::observe_scoped(ObservationScope)` discovers support once per live transport through legacy Observe. That bootstrap **can rehash artifacts and copy full state**. Subsequent scoped pulls use the advertised capability; absence yields Unsupported without a scoped dispatch or silent full-state fallback. Explicit reconnect/resume creates a new transport and renegotiates. No read or uncertain operation is automatically replayed.

The client validates response/profile/version/epoch/endpoint and exact requested view/ID. Missing, mixed, unrequested or misbound projections fence the connection. A normal missing-target refusal is distinguishable from transport corruption. Generic raw scoped calls with another epoch are rejected locally before ID allocation/write; the broker independently validates raw wire callers. Malformed discovery is fenced rather than trusted as capability evidence. Capability support is not permission.

On merged ENG2 and this branch, with an explicitly chosen run/socket:

```sh
target/debug/namtab-agent --socket /absolute/chosen/socket work
target/debug/namtab-agent --socket /absolute/chosen/socket session session1
target/debug/namtab-agent --socket /absolute/chosen/socket attempt attempt3
```

Each **one-shot CLI** performs full bootstrap and consumes a retained endpoint identity. Repeatedly spawning it is not the optimization or a scale proof. Use a long-lived Rust client or explicit JSON-line protocol client for repeated scoped pulls. The raw JSON-line route requires callers to discover support and supply the exact epoch. Human launch/run paths are owned by the [user guide](../users/index.md); never discover/attach to user sessions for a test.

## ENG3 MCP consumer admission

[Issue80](https://github.com/whoodes/namtab/issues/80) migrates repeated MCP reads only (merged PR81). It uses `namtab.mcp-local/1` over unchanged `namtab.mk1/1` and `namtab.scoped-observation/0`. A separately advertised **`scoped_work_export_v0`** capability gates the additive `work_export` scope/view. Older clients never request its new tag; older brokers missing either scoped capability are explicitly refused by the new bridge before tools become available. No automatic legacy fallback, reconnect/resume or replay. The former `/0` MCP profile remains historical; this is a private experimental profile revision, not a stable SDK.

`Bridge::connect` performs exactly one checked full Observe (success with no error/receipt/resume, bound snapshot) for capability discovery on its single live transport. It can copy/rehash full state once. Subsequent `namtab_observe` calls dispatch one scoped read:

| MCP arguments | Broker scope | Export semantics |
| --- | --- | --- |
| `{}` | `work_export` | Existing MCP work fields and fresh registered artifact metadata; no terminal grids, stream bodies, endpoints, receipts or resume credentials |
| `{"session":"EXACT_ID"}` | `session` | Exactly the selected canonical session/context, same export as before |
| `{"attempt":"EXACT_ID"}` | `attempt` | Exact immutable origin/task revision and existing4,096-code-point stdout/stderr previews with source/preview truncation flags |

`work_export` shares **the exact legacy artifact-refresh helper**: at most16registered files, existing1MiB/file/path checks, SHA revision, validated revision, stale or `unavailable`. It registers nothing and reserves no mutation receipt/journal record. Refresh is outside the state mutex, followed by one locked metadata projection: no sealed filesystem/global causal cut or correctness/acceptance proof. A concurrent file edit can immediately obsolete the observed digest, as before. Session/Attempt reads do not claim artifact freshness and do not perform unrelated file checks.

The work export additionally scans at most the existing bounded2attempt streams to preserve legacy MCP `source_truncated`, including sanitized-output expansion loss. Its shared UTF8 replacement/character-width budget matches legacy65,536-byte formatting without building formatted bodies. This differs deliberately from **plain Work's raw-source-only flag**; mapping that flag directly would underreport loss for control-heavy streams. Plain Work stays unchanged. Work export still pays bounded rehash/scan costs, not zero work or guaranteed low latency.

The outer broker request ID remains the exact scoped request ID in MCP's epoch/endpoint binding. Transport/profile/view/selector/epoch mismatches fence the public client; missing targets stay read refusals. No cached or partially refreshed work export is substituted on error. A fresh manually started bridge uses a new endpoint and rediscovers once; it does not resume an old request namespace or repeat work.

The tool set, preview filtering and manual workspace/provider/tool approval/export-consent boundary remain unchanged. No provider call, credential access or new export destination is introduced. [Agent configuration and retained real-model blocker](../engineering/external-agent-mvp.md) remain authoritative. Read scope is not an ACL. **Native and diagnostic polling stay on legacy Observe.** One-shot agent CLI discovery cost is unchanged.

### Supported proof and limits

```sh
make mcp-scoped-test
```

This builds same-worktree binaries/local terminfo and runs an owned headless broker plus real MCP stdio bridge, a scripted peer and human endpoint. It never attaches to an existing run or invokes a model/GUI. An owned serial socket proxy records actual broker request/response frame bytes, redacts resume capabilities, and proves one discovery followed by exact scopes. Controlled artifact edits/deletion, sanitized loss, separate origins, takeover/stale zero bytes, peer progress and explicit fresh-bridge rediscovery are checked.

The admitted workload uses2quiet PTYs/2attempts/3live public connections (four retained identities after fresh bridge), one synthetic1MiB artifact, B's bounded control-heavy stream plus20ms heartbeat. Two warmup iterations precede24retained iterations/scope; order alternates. Raw samples disclose host/kernel/debug binaries and exact hashes. MCP elapsed includes stdio/Python/proxy overhead; direct legacy RPC has a different path. Actual broker wire bytes and reserialized MCP size are labeled separately. No latency superiority, human key-to-paint, server-only CPU, fairness, SLO, model or larger-fleet claim follows from24samples. Structural correctness/omission is gated; timing superiority is not.

The first fixture's oracle wrongly required the optional outer response endpoint. The scoped header is the authoritative exact endpoint; an outer endpoint, if present, must agree. Preserve that failed record and test both absent-valid and present-wrong cases rather than treating a rerun as proof. Runtime tests validate both header and target; malformed responses still fence.

## Validation and measurement

`make scoped-test` builds current binaries and runs a fresh isolated **headless** journey, with two SCRIPTED clients, a human observer,2PTYs,2attributed managed attempts and a temporary fourth measurement connection. It checks exact scopes, bounded noise, real CLI use, stale/missing refusal, targeted human takeover while B continues, stale-owner zero bytes and no read mutation journal. Exact legacy/scoped payload comparison occurs only at quiescent targets, not across changing output cuts. No GUI/model/user session is involved.

The workload retains100paired legacy/Work RPC samples after10warmup pairs, interleaved AB/BA. Raw roundtrip times and actual wire-line bytes are saved with hardware/kernel/build/harness hashes and connection/output/artifact tuple. Timing includes Python encode/send/read/decode; legacy samples include a registered synthetic1MiB artifact rehash. It is not server-only CPU, human key-to-paint, model inference, all-process resource/fairness or dozens-agent qualification. CI asserts structural/byte/data correctness, **not** a latency superiority threshold. One-shot bootstrap is outside these steady-state samples and disclosed separately.

`make foundation-verify` includes this journey plus failure-propagation regression. Protocol/broker/client tests exercise typed omission, nonmutation, artifact-refresh separation, sanitization, selector/epoch refusal, malformed peers, same-connection caching and reconnect negotiation. Historical X4/EA1 FAIL and original focus/Ctrl-C/setup records remain unchanged. Report exact source/tree/dirty status and preserved failures; initial working-tree runs do not certify the final commit.

Remaining dependencies, **not implemented here**: H0#46 admits bounded mixed workload/profile/resources; #48 qualifies responsive human control/fairness before broader load claims. Then #49 needs event/cut identity, credit/backpressure, deltas/gaps/resync, canceled waits, calibrated larger populations, scheduler fairness and later native/other consumer integration. Scoped reads remove unrelated observation work; they do not solve those gates.


The separately admitted [bounded-load qualification](bounded-load.md) tests engine-side control under finite noisy/stalled clients. It does not replace these scoped-read semantics, raise limits or qualify native painting.
