# Bounded engine control qualification

[Architecture](../architecture/index.md) · [H0 admission owner](h0-harness-contracts.md) · [Issue82](https://github.com/whoodes/namtab/issues/82) · [Fairness backlog48](https://github.com/whoodes/namtab/issues/48)

**ENG4 draft, based on merged PR81/main97741bb.** The unchanged parent [PR83](https://github.com/whoodes/namtab/pull/83) uses `namtab.engine-load/0`; this ENG5 child uses **`namtab.engine-load/1`** with [new admission policy](human-admission.md). It names a test/evidence profile, not a new runtime wire, grant, scheduler or full H0 acceptance. The owner authorized this isolated qualification slice after scoped MCP integration. Numerical gates below are predeclared for this experiment; results must name the exact source/tree, host, executable hashes and fresh evidence directory. Calibration is not final validation, and a gate failure is not permission to tune it to green.

## Admitted tuple

| Dimension | Finite experimental choice |
| --- | --- |
| Protocol/core | Existing `namtab.mk1/1`, scoped-observation/0 and `scoped_work_export_v0`; finite owned MK1 core, two80×24PTYs; controlled clean Bash, TERM=dumb, capture off |
| Populations | Three live public endpoints: two **scripted** agent declarations and one preconnected human declaration. Same-UID roles are not authenticated physical/model identities. Two managed attempts share one exact immutable task revision and retain distinct endpoint/request origins. One controlled PTY foreground job is counted separately. H0 agent-run/assignment populations are not implemented; no model is invoked, external activity remains unknown; subscriptions=0 |
| Existing admission | Keep2lifetimePTY/2lifetimeattempt/4dispatchconnection+1boundedhandshake/retirementtransport/15ordinary+1lifecycle identities, existing frame/input/receipt/storage limits. Do not infer those maxima can all be occupied safely under arbitrary workloads |
| Application load | Managed A advances a heartbeat; managed B and PTY B emit paced synthetic data only in noisy/stalled stages. Each producer emits under2KiB/50ms, combined application load below80KiB/s plus finite human probes. Each program exits after30s or an owned finish file. No CPU/disk benchmark; one64KiB synthetic registered artifact |
| Client load | Agent A rotates WorkExport/Session/Attempt, one polling request per≥100ms (at most10Hz total), serial on one transport. Workflow/control calls are separately finite. Agent B deliberately stops reading after at most8queued legacy read-only requests; its receive buffer request is4KiB, actual kernel value recorded |
| Stages | Two warmup interventions excluded from measured cohort; three repetitions of quiet/noisy/stalled, six interventions spread across at least750ms/stage at125ms planned intervals.54retained interventions/18percohort. Actual noisy queries are observed before measured noisy/stalled samples. Fixture45s deadline; no existing user session is contacted |
| Host | Qualification candidate: this shared Ubuntu22.04 x86_64 VM, kernel6.8.0-138-generic,4logicalCPUs/affinity0–3,16,721,039,360BMemTotal and≥2GiBMemAvailable at launch. Record actual tools/host load per run; do not alter affinity/nice/cgroups/cache/security to improve results. Matching these fields is not hardware attestation or a cross-machine performance claim |

The phase/finish files and reply/probe files are synthetic **fixture controls/evidence**, not Namtab scheduling, artifact acceptance or inferred shell semantics. Human calls do not hold the agent transport mutex; only that agent’s own calls serialize. Record actual read/takeover interval overlaps without inferring internal server contention. Raw public clients are not model-backed agents; this profile does not qualify MCP stdio overhead or native painting.

## Gates and measurement boundaries

| Gate | Required result / meaning |
| --- | --- |
| Safety | Exact epoch/endpoint/request/origin/task revision; no cross-stream attribution. One expected cursor reply per controlled query, independently checked against exact `ESC[3;7R` plus reserved/applied journal counts. No reply or input replay |
| Ownership/input | Each forced takeover binds exact session/generation. A preconstructed old-generation agent intent must reject with0delivered bytes and no canonical canary. Human text/Enter receipts have exact byte extents; real append-file effects are unique and ordered. Release/reclaim is a **new** intent, never a retry of stale input |
| Functional service | Every positive human takeover/text/Enter/probe and nonstalled polling read finishes within2s. All failures/refusals/timeouts stay in evidence/denominators; unexpected ones fail. Each nonstalled read scope progresses in every stage; both managed jobs must advance in every intervention/stage, exit0 and retain their own positive stream canary, not merely omit the peer’s. Explicit B resume must succeed on a new read, not only Hello. An intentionally unread consumer is excluded from the eligible response cohort, with queued-request IDs/disconnection recorded |
| Sampled resources |20Hz census includes the owned broker root plus all pidfd-verified worker/shell/supervised descendants. Record exact expected7PID/start identities in each sample, RSS/threads/FDs/CPU ticks and all9stage windows. Timestamps strictly increase, cover every stage at an evidence floor of10Hz with no gap>250ms, and summedRSS≤1GiB. These sampling-validity checks are not product latency SLOs. SumRSS double-counts shared pages, excludes the driver, and misses between-sample peaks/virtual allocation/swap. This is a test guard, **not** an enforced total allocation budget or full G6 memory qualification |
| Local control candidate | Per quiet/noisy/stalled cohort, takeover-RPC p95≤100ms, inherited from the provisional protective-control proposal. Report raw p50/p95/p99/max and counts with floor-rank method;54interventions cannot qualify rare tails. Clock starts before the public takeover call and ends at its bound receipt, not physical input, downstream application response or paint |

`--qualify` enforces both the recorded host tuple and100ms candidate, in addition to functional/safety/resource gates. It returns failure if the candidate fails. CI's explicitly named **functional** lane uses the same workload and2s safety/service gates, reports the100ms candidate separately, and never earns local-host qualification. This split is predeclared because hosted runners have a different host/load tuple. Neither lane silently widens capacities, changes thresholds or hides a failed candidate. A failing strict validation blocks the corresponding qualification claim and dependent work, even if functional CI passes.

No scheduler priority or universal fairness guarantee is implied by a fast RPC. This tests a preconnected human path under a small declared workload. Full queue/resource exhaustion, uninterruptible OS work and larger populations remain open.

## Queue and ownership census

- The child public server dispatches at most4connections, with one shared auxiliary handshake/retirement accepted transport (at most5accepted socket objects) and services one request/response at a time per connection; complete frames are bounded1MiB and writes time out after2s. Kernel socket buffers are separate; the stalled socket records actual receive-buffer size/unread bytes. No application subscriber queue is introduced.
- The private worker channel has8queued slots plus one serial in-flight exchange and a1response channel. Requests/responses retain131,072/1,048,576-byte caps. Each of2PTY pumps feeds one512-byte chunk synchronously;4managed stdout/stderr buffers retain65,536bytes each while continuing to drain.
- These are **source-audited bounds**, not measured queue occupancy/high-water or a production global allocation proof. The admitted C3/T2 cohort does not saturate the8-slot worker queue; existing stopped-worker/final-write regressions remain relevant but do not certify arbitrary overload scheduling.
- The parent /0 post-measurement probe exposed new-Human refusal when all four slots were occupied; its evidence stays unchanged. Child /1 first refuses a fifth Agent, then verifies a fresh Human handoff plus the existing protected Human read. This is conditional same-UID admission, not authentication or universal priority; see the [policy and refusal cases](human-admission.md).

The broker/core remain authoritative. Protocol clients do not access private persistence, PTY handles or parser state. The test audit reads only its fixture-owned journal metadata for reply counts; it does not confer permission. Input ownership fences future mediated bytes, not already completed effects or hostile same-UID writes.

## Reproduce and interpret

```sh
# Existing pinned local toolchain/dependencies; build own binaries and terminfo.
make engine-load-test
```

This target runs explicit `--qualify` on the admitted local tuple and prints/preserves a fresh ignored `.artifacts/engine-load-*` result. `make foundation-verify` includes the equivalent isolated **functional** journey plus damaged-result oracle regressions. No model, GUI, user socket discovery, saved trust or credentials are involved. Unknown/partial writes are never repaired or replayed. Cleanup uses only the fixture's owned handles/pidfds and preserves evidence.

A profile-owned finalizer requires clean identical before/after Git head/tree/index/diff and verifies all11executed fixture-source snapshots. It seals the owned metadata journal/runtime path and bounded recursive evidence hashes. Any execution, cleanup, dependency or source-seal failure demotes both overall verdict and qualification; a previously computed latency PASS cannot override it. This is two-observation provenance, not atomic filesystem or compiler attestation.

Read `checks.json`: exact source/binary hashes, host/populations, all intervention/read/call records, resource samples, stalled request IDs, query count, functional assessment, candidate verdict, explicit qualification status and cleanup outcome. A PASS label without the matching clean source/tree and complete evidence is insufficient. The program test cap is30s and outer guard45s; kernel cleanup may exceed a caller timeout and remains uncertainty, not arbitrary-PID kill permission.

Initial failed calibration is retained: its controlled program restored termios with the cursor still at row3/column7, so Bash printed `PPPPPPmk1$ `. An instrumented actual frame diagnosed the fixture-layout mismatch. The payload now emits CRLF before Bash resumes; the strict clean-prompt assertion is unchanged. Initial descendant-only memory samples omitted the broker root and are incomplete, not qualification. Root-inclusive sampling and later clean-revision validation replace neither original record nor its failure.

## Remaining qualification

This is a partial experimental A2/A3/#48 result, not full#46/H0 closure. Do not infer dozens-agent capacity, authenticated model principals, a production scheduler/global memory limit, physical human/IME/a11y/paint, full compatibility or recovery. After bounded workload/profile admission and broader human-control/fairness review, #49 still needs identity/revision/cut/completeness, credits/backpressure, gaps/resync and disconnect/cancel contracts for subscriptions/deltas/waits. They are **not implemented here**; native migration stays separate.


Adversarial review of the first candidate found false-pass oracles for failed managedA, incomplete/stale/reversed process census and cleanup after a successful journey, plus missing standalone final source/evidence binding. Original reports/results remain retained as incomplete for those claims. Tightened damaged-result and no-process finalizer regressions must reject each counterexample before fresh final-revision validation. No numerical control/service/RSS gate or capacity is relaxed.
