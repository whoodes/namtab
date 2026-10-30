# Bounded request-driven session watch

[Architecture](../architecture/index.md) · [Scoped pulls / MCP export](scoped-observations.md) · [Issue87](https://github.com/whoodes/namtab/issues/87) · [Broader49](https://github.com/whoodes/namtab/issues/49)

**ENG6 draft, based on main51cf9d1 after83/86.** This is a separately admitted finite precursor to49, not full H0 acceptance, push/event subscriptions, cell deltas, condition waits, durable cursors or a stable SDK. #46/#48 remain open beyond the previous narrow experiments. Current owner authorization admits only the profile below; no larger workload, renderer, model or security policy follows. MCP and native/diagnostic clients remain unchanged.

## Contract and identity

Capability **`session_watch_v0`**, profile **`namtab.session-watch/0`**, additive operation `session_watch` on experimental `namtab.mk1/1`. One watch selects one exact existing session on one bound ordinary transport. Restricted lifecycle connections refuse it. Roles remain same-UID claims, not authenticated humans/models; selection is not an ACL or grant.

Request envelope: `operation={op:"session_watch",epoch:EXPECTED_EPOCH,request:SELECTION}`. The ordinary outer request ID correlates one exchange; read calls have no mutation receipt/dedupe/replay contract. Supply current values, not guessed examples:

| Selection | Response / effect |
| --- | --- |
| `{kind:"open",session:EXACT_ID}` | Baseline at sample1, fresh opaque connection-owned subscription ID; refuses a second active watch |
| `{kind:"next",subscription:ID,cursor:APPLIED_CURSOR,credit:1}` | One locked sample: `replaced` with full selected view and exact `from` cursor, or `unchanged` with **no view body** |
| `{kind:"resync",subscription:ID}` | Explicit new baseline, next sample cut; no automatic fallback |
| `{kind:"close",subscription:ID}` | `closed`, no observation, same last cursor; releases watch only |

A cursor is `{sample,revision}`: a positive, monotonically increasing **per-watch projection sample ordinal** (maximum2^53−1), plus broker-generated `sha256:` opaque content revision. The revision covers the entire selected published Session (including owner/generation/process), shared degraded state and canonical-output-pending flag. Hashing alone grants no authenticity or authority; treat equality metadata with the same confidentiality as output. It is not a portable checkpoint format or a client-verified signed proof.

A sample is taken under the broker State lock. It is **not** a global event number, byte-stream/OS/application causal cut, immutable task artifact or complete event interval. Cursor state advances after a complete size-bounded reply can be formed, **before socket delivery is known**. A lost/partial response cannot be silently retried.

## Completeness, gaps and freshness

Every frame binds version/profile/epoch/endpoint/subscription/session/cursor. `coverage=latest_published_view` accompanies baseline/replacement/unchanged; `canonical_pending` reports whether canonical output is still pending, and the Session's existing `input_known`/diagnostics remain visible. A published view can lag incoming bytes. No apparent screen or watch cursor authorizes application input: existing owner/generation/mode/revision/final-write checks still apply.

**`history_complete` is always false.** This profile keeps no intermediate event history. Producers change canonical state normally; readers sample latest state when they supply credit. A→B→A between samples can yield `unchanged`; that means the sampled projection matches, never that no transitions occurred. This coalescing limitation is explicit on every reply, including a resync baseline.

A well-formed wrong base/sample/revision returns `resync_required` with `coverage=no_observation`, no view, no pending/degraded assertion and no sample advance. It **latches** until explicit resync; sending the server's last cursor afterward does not bypass the latch. A malformed/invalid request, stale epoch, unknown session/watch, unsupported lifecycle identity, exhausted open/cursor budget or oversized projection returns a read refusal without advancing the active cursor or adding a mutation journal record. Malformed outer frames retain existing MK1 codec behavior. Errors are never converted to cached/full legacy state.

Close cancels future watch observations, not execution/input/side effects. There are no long waits to cancel. Calls are serial; close cannot preempt a currently blocked exchange. Disconnect, retirement or response-write failure drops transport-local watch state. An explicit endpoint resume preserves existing endpoint/ownership/receipt semantics but **cannot restore a watch**: old subscription IDs refuse; open a new baseline. No automatic replay, reconnect, retry or resubscription.

## Credit and resource admission

| Resource | Bound / limitation |
| --- | --- |
| Execution / transports | Existing2lifetimePTY/2attempt/4dispatch+one shared bounded admission/retirement transport;15ordinary+1lifecycle retained identities unchanged |
| Watch population | One active per ordinary transport; at most4dispatching plus at most1retiring handler's watch;16successful Opens per transport lifetime, no eviction of a live watch |
| Credit / buffering | `next.credit` must equal1; each Open/Resync/Close also authorizes only its single RPC reply. No accumulated credits, publisher workers, retained event queue or background sends. One serial response in flight per connection; kernel socket buffers remain separate |
| Reply | Whole watch response ≤131,072encoded bytes **including newline**; bounded streaming hash/count checks include JSON escaping, with explicit oversize refusal before cursor commit |
| Metadata | Bounded identifiers/cursor and one fingerprint only; no retained terminal-body/event queue. Temporary full replacements are bounded by reply checks; this is not a global allocator/memory quota |
| Timing | Existing public1MiB request/2sresponse-write and checked-client2s transport behavior. Broker locks/OS scheduling can still delay/fail; no hard responsiveness/hostile-peer fairness guarantee |

Watch hashing still serializes the selected scope. Unchanged sampling avoids cloning/exporting its view body, not all computation; no speed/CPU superiority follows. Full baseline/replacement also pays bounded encoding checks. Watches are read-only phases for existing Human-admission retirement, so an unread Agent watch cannot acquire mutation protection merely by subscribing. Existing all-protected/busy/exhausted refusal cases remain.

## Client, privacy and ownership

The Rust client exposes `session_watch_open`, `session_watch_next(&applied_frame)`, `session_watch_resync`, `session_watch_close`. Support is discovered once per live transport via the existing checked legacy Observe, which can initially rehash artifacts/copy full state. Missing capability refuses before watch dispatch; no legacy fallback. Caller credit must acknowledge the bound frame it has applied (epoch/endpoint/subscription/session plus cursor), not merely a coincidentally equal cut/hash from another watch. That is a consumer claim: the library cannot prove an application actually updated its replica. A stale local application base refuses before ID allocation/write and requires a deliberate recovery decision, never an implicit baseline.

Strict client checks bind header/selector/cut/base/response kind, bound the incoming watch frame, reject mixed snapshot/receipt/credentials, and fence malformed peers. Ordinary read refusal is distinguishable from corruption. A resync-required reply is no data, not a successful view. This profile extends actual Rust MK1 runtime types; larger N0 schemas/generated TS contracts are separate and unchanged.

Capture-off, untrusted output and provider consent remain unchanged. Watch operations do not journal output/cursors, read files, refresh artifacts, grant ownership, emit terminal replies or contact a provider. Session data remains untrusted. Explicit `work_export` still refreshes artifact revisions through its existing bounded path. New watch capability does not add an MCP tool, background export, agent cognition or private PTY/persistence access.

## Executable proof and residuals

```sh
make session-watch-test  # same-worktree binaries; fresh owned headless fixture
make foundation-verify  # active-source aggregate, not historical worker certification
```

`namtab.session-watch/0` fixture uses two scripted clients, two quiet/noisy PTYs, two managed attempts sharing an immutable task revision with distinct origins, one preconnected Human and a temporary fourth unread Agent. No model/GUI/user session. Combined steady Next rate≤10Hz,12samples/target; paced output remains under the earlier80KiB/s fixture envelope and programs self-expire30s. Journey25s guard and2sfunctional read/control gates are experimental; not production SLOs. Sampled owned7process RSS guard≤1GiB is not global enforcement/instantaneous peak. Source/binary/fixture/evidence seals and cleanup are required.

The populated quiet-grid byte comparison measures actual NDJSON baseline versus unchanged frames; an empty/small scope need not shrink. The unread burst is **127whole compact resync requests**,32,662bytes in the exercised ID range, hard32KiB batch guard. Original attempts with128frames failed before sending: default34,584bytes, compact32,920. Those failures remain; final count is127, not a claimed128-frame result. No runtime/resource/service threshold was relaxed. A stopped consumer is observed as unread, not inferred to be blocked at a particular kernel instruction.

Gates include ownership/pending/degraded changes with stable screen revision; no cross-scope output; wrong credits/epoch/transport/base; latched gaps and explicit baselines; close/reconnect/new namespace;16Open budget; slow-consumer control with actual unique file effects and exact stale-writer0byte refusal; positive distinct attempt streams/results; single terminal reply authority; artifact freshness on explicit export; and no watch mutation journal. Negative oracles reject silent baselines, bad cuts/coverage and skipped cleanup. Unit fault injection is not real OS/concurrency proof. Initial compile/fixture failures are retained; clean final revision evidence is separate.

Broader49 remains: true event/cell deltas, condition waits and cancellation, retained event cuts/completeness, byte/queue credit windows, larger calibrated populations, scheduler fairness and native/MCP consumers. #85 remains cooperative shell visibility **design only**. No new work follows a passing finite watch check automatically.
