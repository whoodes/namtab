# H0 harness admission and proof proposal

**Recommended for review, 2026-10-19; not accepted, implemented or measured.** Supports [issue46](https://github.com/whoodes/namtab/issues/46) under [E5](../adr/0005-ai-first-harness-direction.md). PR52 merged the direction; the current authorization covers H0 design and test-only contract examples, not a scheduler, runtime, renderer, model integration or future PR merge. [Native contract](native-contract.md) continues to own identities/receipts/input; [coordination policy](../product/02-work-evidence-and-coordination.md) owns task acceptance and integration. This page owns the proposed H0 admission/workload and finite proof selection, not another wire API.

## Recommended boundary and native value

Use an **external agent with a narrow native adapter first**. The agent owns planning, model turns, conversation/context compression and its provider connection. Namtab owns authenticated work identities, dependency admission, execution/control, observations, artifact/review bindings and human intervention. It can coordinate work without implementing a second model reasoning loop. A later harness-owned loop must earn a separate product decision, cancellation/context/export/cost design and provider admission.

| Alternative | Benefit | Cost or refusal boundary |
| --- | --- | --- |
| Native adapter for an existing agent — recommended first | Exercise real model-directed operations while keeping one execution/evidence contract | Agent must actually route relevant tools through namtab. Unmediated tools/provider calls remain explicitly unenforced; labels/ACP support alone prove no mediation |
| Namtab-owned model loop | Direct ownership of prompts, tools and model-call scheduling | Adds provider APIs, context strategy, retries, budgets, secret/export handling and model evaluation before the execution harness is proven; defer |
| Opaque agent CLI in a PTY | Useful compatibility with an unmodified program | Screen/process observations only; no invented tool IDs, per-command results, cognitive status or hard model-spend limit |

An ACP client adapter could carry prompt/turn updates and delegate supported terminal calls; an MCP tool adapter could expose native operations to a participating agent. Neither protocol becomes the authority or is selected as a dependency here. In particular, ACP v1 release kills a running command, unlike namtab detach; a read-only/detach grant cannot implement that mapping. MCP tool descriptions/annotations are not internal grants. Pin one actual agent/protocol/version and audit its real tool routes before H2, rather than promising every agent works. Current primary observations and limitations are in the [H0 source ledger](research.md#h0-focused-source-review-2026-10-19).

## Work identity and admission overlay

Reuse the native namespace, exact revision, issued mutation-key and receipt rules. Do not introduce another command ID derived from pane order or terminal output.

| Record or operation family | Proposed H0 clarification | Falsifier / future proof |
| --- | --- | --- |
| Agent run | A service-issued run ID identifies one admitted agent execution lifetime, bound to an authenticated principal, adapter/version, context and capability generation. Several connections may observe it; a reconnect is not a new run. A replacement agent gets a new run. This is distinct from a PTY process title, task or model request | Two agents may use the same display name and endpoint-local request number without sharing identity or receipts |
| Work assignment | Bind run, exact task-definition revision, expected lifecycle revision, attempt and context/dependency selection. Claim/transfer is a broker transition with a new assignment generation; it grants no terminal writer, workspace isolation or acceptance | Concurrent claims conflict at the task/assignment boundary; two legitimate attempts may coexist only under the declared task policy |
| Managed execution | Existing `RunProcess` proposal binds task/attempt/context before launch; execution ID exists even on launch failure. Output binds execution/runner/stream/offset; pipe stdout and stderr have independent offsets | Interleaved streams never inherit the latest task/pane or another request's origin. Exit and drain remain distinct |
| Interactive compatibility | Existing session incarnation, canonical profile/cut and writer generation govern `SendKey`/text/paste. PTY output is one merged stream. A persistent-shell command label is a claim, not a managed execution | Shell prompts/OSC, inferred widget state or a reported tool completion cannot establish command boundaries or acceptance |
| Observation / wait | Explicit task/attempt/execution/session projection and authorized range, base/cursor, bounded credit, completeness and trust. `WaitFor` uses observable typed conditions, not “agent finished thinking” | Expired base, missing range or slow subscriber yields a gap/resync requirement. No old screen authorizes fresh input |
| Coordination / integration | Typed messages name sender/run, target task revision, causal reference and bounded purpose. Delegation intersects parent capabilities/budgets; immutable artifact publication and integration select exact revisions | Message text cannot grant authority. Changed dependency selection stales a new admission; running pinned work remains historical, not retroactively rewritten |
| Handoff / stop | Assignment transfer, terminal writer takeover, cancellation request and process/runtime stop are separate operations with receipts | Closing an old writer fences future mediated input, not descendants, other sessions or external effects; unknown delivery never authorizes automatic retry |

Agent-run and assignment spelling, admission atomicity and decoder limits require a reviewed native schema revision before implementation. They are **proposed additions**, not fields secretly added to `namtab.mk1/1` or the existing N0 DTOs. Existing type/schema generation remains unchanged. Run/assignment IDs are locators, not bearer credentials; service authentication and capabilities bind every use. Same-UID developer-host role declarations still do not attest a physical human or contain hostile processes.

### One useful coordination journey

1. An authorized coordinator defines parent task T/revision1 with artifact/validation requirements and selected producer/consumer child tasks TP/TQ. Attempts A/B bind those exact child definitions in separate contexts. The coordinator delegates intersected capabilities, not shared credentials or unrestricted workspace access; no task depends on its own unfinished result.
2. Runs RA/RB admit different argv executions under their own issued request namespaces. They may both use sequence1. Process/stream facts remain attached to each execution and context; no cross-stream total order is invented.
3. B's consumer task TQ waits on the selected artifact/validation from producer TP without occupying a PTY or blocking the event loop. A publishes revision R; a message reporting “done” alone cannot satisfy that dependency. A newer R2 does not silently replace R for an existing attempt.
4. Human selects an exception, inspects exact R/context/evidence and takes a specified session writer. B's independent execution may continue emitting output. Focus changes are read-only. Old-generation input must not cross the final dispatch boundary.
5. Integration produces a new revision and requires fresh validation. Process exit, adapter turn completion, artifact publication and task acceptance remain four different observations/decisions. Unknown operation effects reconcile by key; they are not retried under a new ID for convenience.

This is a future semantic proof, not a claim that MK1 implements delegation, dependency scheduling or a model. Existing MK1 evidence supplies only finite shared declarations, direct attempts/receipts and explicit writer handoff.

## Workload populations and measurement proposal

“Dozens” describes desired useful agent coordination, not a mandate for the same number of PTYs, active commands or model requests. Publish each population independently:

| Symbol | Count and accounting |
| --- | --- |
| C | Authenticated connected protocol endpoints; distinguish real agents, scripted load clients and human/diagnostic views |
| R | Admitted agent runs, including disconnected-but-continuing, waiting and runnable runs; not equal to C. Report broker-recorded runnable, dependency-blocked, resource-queued and observation-waiting cohorts separately; adapter-reported activity is not that admission state |
| K | Simultaneously active managed executions, plus queued launches and descendant/stream-drain state separately |
| T | Live interactive PTYs, their writers and subscribers; completed retained sessions counted separately |
| M | In-flight mediated model requests. External/unmediated calls are unknown, not zero or hard-capped by namtab |
| S / Q | Active subscriptions/waits and resident queue/credit/history/receipt bytes, including disconnected retained state |

Read-only inspection on this VM reported **4 logical CPUs, 16,329,140 KiB MemTotal, Linux x86_64 kernel6.8 / glibc2.35**. It is shared, not a dedicated benchmark machine; available CPU/GPU/storage throughput and load were not qualified. No benchmark was run in H0. A future run must record exact OS/tool/binary/dependency hashes, CPU affinity/load, memory, storage, display/host, geometry/fonts, input/capture/profile tuples and all-process accounting. The inherited [G6](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets) used a different 8-vCPU reference machine.

**Proposed experiment matrix, not capacity or implementation admission:**

| Study | Candidate population and purpose | Required distinction |
| --- | --- | --- |
| Baseline | Reproduce MK1's admitted C4/K2/T2/M0 limits only in a separately authorized isolated run | Existing recorded results are not a new same-host baseline; do not enlarge lifetime caps in this design PR |
| Idle/waiting fan-out | Candidate R=4,12,24,48 scripted runs, all waiting; one connection/run plus two human/diagnostic endpoints (C=R+2); two scoped subscriptions/run; K0/T0/M0 | Parameter sweep brackets dozens without choosing32 or qualifying48. A separate disconnect cohort retains runs while lowering C; include authorization/retention costs |
| Useful concurrent work | Candidate R24/C26, all24 initially runnable with four finite operations/run; K=1,2,4; T0, then2; M0. Separately test four runnable/twenty dependency-waiting runs, then release a declared dependency cohort | Hold the96-operation work fixed while varying concurrency; report eligible/resource-queued/blocked counts, queue delay, completions and per-run fairness, not just aggregate RPC throughput |
| Flood / slow reader | Candidate R24/C26/K4/T2/M0; one finite1MiB/s stream and one paused subscriber for60s | Synthetic workload chosen to expose blocking; sweep volume before claiming a load ceiling. Backpressure can stall the producer; do not drop canonical bytes silently |
| Actual agent | One explicitly admitted model-backed agent, then two sharing a task/context policy; separate synthetic load | Real model/provider time/export/cost reported separately. No credentials, provider, paid calls or model concurrency are selected now |

For the CPU/IO subtuple, propose quiet-output jobs first, then separately bounded CPU hashing of fixed synthetic data for10s and16MiB temporary-file writes/flush per command (≤64MiB total at K4). Pin the eventual fixture implementation, work/rate, storage location and tool versions; report warm-cache/flush behavior and existing host load without clearing system caches, saturating a user's live namespace or calling it a dedicated CPU/IO test. These are design candidates, not executed programs or admitted host load. Vary output rate separately so a CPU or filesystem bottleneck is distinguishable from parsing/transport.

For attention, propose five selected task packets with one changed artifact/context and one unavailable evidence range, then a bounded synthetic decision-arrival stream (two requests/minute for10minutes). Measure inbox/notification updates, age, context invalidation and exact per-item dispositions; synthetic requests do not measure human comprehension. A separately consented hands-on study must measure review time, missed material context and intervention errors before claiming the UI removes a human audit bottleneck. CPU/IO fixture rates, actual human participation and all study limits remain pending owner/engineering admission.

Use five measured repetitions after a declared warm-up, at least10,000 samples per high-rate control/observe metric where feasible, fixed-duration rate recording and raw monotonic samples/histograms. These are proposed statistical inputs; report sample counts and an insufficient-tail warning when rare actions such as takeover have fewer observations. Report per-run p50/p95/p99/max, distributions and failures/refusals separately, not a percentile of percentiles. Timed-out/refused requests cannot disappear from latency reports. Scripted output is reproducible; process/model execution is not promised deterministic.

Measure admission wait→receipt, dispatch→OS acknowledgement, observation freshness, and physical human input→actual paint separately; include IPC, serialization, durable reservation, hashing, parsing and client costs. Model inference/remote network/application response is separate. A fast refused operation is not completed useful work. Compare scoped subscriptions with whole-snapshot polling for equal observed facts, gaps and workload; report bytes/CPU/copies, not an invented speedup.

### Provisional gates and finite resources

The following are **recommendations for owner/engineering review**, not selected production limits or achieved results. Reuse G6's human-facing provisional p95 ≤50ms input→paint, ≤100ms protective-control admission and ≤250ms decision update, plus ≤1GiB product-process memory, rather than inventing new speed promises. Whether those fit the smaller inspected VM is unknown. Always report p99/max; these proposals assign no p99 SLO. Painting cannot be qualified by an RPC-only/headless run.

For an initial stress profile, propose ≤16 queued ordinary operations/principal and256 globally, eight separately reserved protective-control slots, ≤1MiB queued canonical output/session within8MiB global, ≤64KiB subscription credit/stream within8MiB global, and shared live history ≤256MiB. These are included allocations under the total product budget, not additive guaranteed maxima. Human/control reserve admission must not depend on an ordinary full queue or model call; exhausted persistent storage still needs the existing protective authority-decrease path with honest durability. Receipt retention/binding and decoder budgets remain the native contract's separately admitted tuples, not resettable queues.

At a cap: refuse new work with its admission/receipt stage, backpressure canonical ingest, or expire/coalesce an **observation** with an explicit gap/resnapshot. No silent evidence eviction, lost canonical input, implicit widening of grants or hidden retry. Round-robin or weighted deficit scheduling with bounded work quanta is a starting **algorithm proposal**, not selected code. Charge parsing, reply generation, snapshot/serialization and artifact reads; avoid a lock around expensive work. Retain per-session order and one reply owner. If bounded processing/OS calls cannot release the human path, stop the affected design rather than claim preemption of an uninterruptible call.

Fail the later gate on cross-attribution, unbounded allocation, stale-writer effect, undisclosed gap, lost/duplicated reply, observation/authority confusion or starvation of a continuously eligible run. Under a finite declared workload, require service of each eligible queue within the reviewed bound; pending admission must specify that bound before a performance pass can be earned. Numerical throughput and fairness bounds are **pending calibration/admission**, so this H0 proposal cannot certify H1. Record a rejected/degraded tuple rather than silently reduce test load.

## Test-only contract examples and admission record

`tests/system/harness-design.test.mjs` checks finite hand-authored traces for distinct run/operation/execution attribution, revision-bound dependency evidence, refusal versus dispatch, unknown-effect reconciliation, writer generations and population accounting. Negative counterexamples alter those relationships. It is a **test-only design oracle**, not a service/dispatcher, a complete model checker, generated wire schema, scheduler proof or runtime conformance suite. No production module imports it and it starts no process, PTY, model or listener. Future H1 must challenge it with independent actual-event traces and real failure injection.

| Admission item | Current disposition / required next evidence |
| --- | --- |
| Existing source baseline | PR52 merge914d079c7acf77c9fba4afcc623d975371351420, preserving finite MK1 and historical pins/failures; no product license change |
| Public/private wire | Existing MK1/1 and experimental N0 namespaces retained unchanged; H0 run/assignment/adapter mapping is conceptual, not wire-admitted |
| Dependencies/data | Existing lock/toolchain tuple only; no ACP/MCP/window/render/model SDK installed or selected. New features/targets/notices/security/export closure need separate admission |
| Runtime/profile | Existing Ubuntu supervised developer-host profile, capture off, trusted same-UID limitations; no sandbox or broader compatibility/recovery claim |
| Workload and resource tuple | Above populations/caps/G6-derived thresholds are proposals; exact hardware, workload and fairness bounds require owner/engineering acceptance after measurement design review |
| Residual qualification | [Criterion register](h0-residual-criteria.md) retains each original outcome/gap and proposed successor; no full NG1/NG3 acceptance |
| Owner decisions | Accept/revise external-agent-first responsibility, choose presentation scope under#47, admit workload/wire/dependencies and authorize a bounded implementation separately. Issue46 stays open until required decisions are recorded |

Recommended next engineering authorization, **not granted here**: one bounded headless coordinated managed-execution/observation slice after accepting the necessary H0 contracts/limits, followed by selected-interface feasibility. No renderer or model-provider choice should block independent contract review; either blocks its dependent integration proof.
