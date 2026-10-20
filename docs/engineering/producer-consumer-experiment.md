# Bounded producer → consumer experiment

[Issue60](https://github.com/whoodes/namtab/issues/60) · [H0 admission](../engine/h0-harness-contracts.md#remaining-h0-admission-packet) · [Earlier write experiment](h0-headless-write-experiment.md)

## Admission before implementation

This separately admitted experiment follows the owner's producer→consumer authorization on 2026-10-20. It is **not** a whole-MVP freeze, H0 acceptance, a production API, or permission to merge. The native-window study under issue47 and retired-Electron cleanup have independent branches/evidence. Existing H0 admission/resource/profile questions remain open beyond this tuple.

| Tuple | Narrow boundary |
| --- | --- |
| API/profile | `namtab.flow/0`, profile `h0-flow-once/0`; strict Rust protocol DTOs and generated experimental schemas. Private supervisor-issued in-process endpoint handles; serialized requests are decoded before admission. No listening socket, public authentication, agent SDK or MK1 wire extension |
| Identity/context | Two scripted run endpoints (producer, consumer), plus a separate trusted supervisor human-control endpoint. One invocation epoch, immutable fixture/program/schema context digest, fixed task/attempt per run, connection1 and monotonic control generation. Client labels do not confer authority |
| Populations | C=2 scripted run handles +1 human handle; R=2, K≤2 managed command attempts total, T=0, M=0. No PTY or shell command inference in this slice; earlier PTY write evidence remains separate |
| Artifact contract | One producer output artifact, revision1, ≤256 printable ASCII bytes, exact epoch/context/producer-attempt/content SHA-256/size. Publication requires complete bounded stdout/stderr drainage and exit0, then fresh authority check. Consumer pins that exact reference, checked at queue and final launch; never “latest by name” |
| Programs | Same-build synthetic Rust `namtab-flow-fixture` producer (no descendants) and `/usr/bin/printf` consumer literal argv, empty inherited environment, fresh private working directory. Clients cannot supply programs, arbitrary argv, paths or environment. Consumer receives the artifact as one literal argv item, not shell interpolation or an automatic PTY write |
| Resources | Request≤4096bytes, response≤16384bytes, references≤64ASCII bytes; sequences1–16 per run, two retained attempt receipts, queue≤2, event journal≤128, output channel≤8chunks of256bytes, each output stream≤4096bytes, artifact≤256bytes, update page≤16events. No eviction, unbounded history or silent truncation-as-success |
| Time | Invocation admission≤20s, each attempt wall deadline≤5s; monotonic expiry checked at queue/final launch and artifact publication. Output collection polls in≤10ms slices. OS spawn/kill/wait may stall; no hard latency or descendant-stop guarantee |
| Control | Takeover is a trusted human operation with expected control generation. It fences queued run attempts and later artifact publication/launch, not an in-flight process's effects. No automatic resume/retry, suffix write, process restart or receipt relabelling. Active results retain original run/attempt/control provenance |
| Updates/privacy | Scoped event cuts: run sees its own process/stream facts and shared artifact/control events; consumer never sees producer private stderr events. Human sees both. Events carry origins, offsets/counts/digests, not full transcripts. Captureoff; bounded RAM artifact bytes are intentional dependency data, not general terminal capture. Explicit synthetic reports may be saved by the probe |
| Refusal/unknown | Unknown version/shape/scope/identity/dependency/generation/expired/exhausted refuses before launch. Reused sequence+different body conflicts; exact duplicate queries original receipt, never redispatches. Launch errors are conservatively unknown/no retry. Incomplete/truncated/failed output never publishes an artifact. Journal/stream exhaustion fences future actions and marks affected result incomplete |

No new dependencies, unsafe/FFI, sandbox/security policy, provider/model, GUI/toolkit or capacity population is admitted. Same-UID supervisor trust and ordinary Linux process privileges remain; context hashes are provenance observations, not sealed runtime attestation. Stdin is null; this slice does not change the earlier PTY `write_once` semantics or qualify terminal reply/input behavior.

## Planned proof and remaining gaps

Real managed producer/consumer execution must retain separate stdout/stderr origins, accept exactly the published artifact bytes and show scoped observations. Adversarial cases include wrong endpoint, stale generation/context/dependency, dedupe/conflict, missing artifact, nonzero/incomplete output, stream/journal exhaustion and takeover racing queued/active work. Takeover timing is descriptive evidence with workload/sample count, not an achieved SLO. An already launched child may finish after takeover; its result cannot publish a newly authorized artifact.

Outstanding H0 includes external transport/authentication, durable receipts/recovery, complete wire/resource admission, restricted runtime/hostile same-UID containment, calibrated load/fairness/dozens, real agent adapters, and actual native-window/IME/accessibility evidence. No result here closes #46/#47 or earns production/H1 qualification.

## Results

The implementation uses `crates/protocol/src/flow.rs` and `crates/service/src/flow/`. No new package, dependency, unsafe/FFI seam or second terminal parser is introduced. The fixed `namtab-flow-fixture` child emits synthetic data; `/usr/bin/printf` consumes exactly the admitted artifact as a literal argv item. Failure-only `nonzero` and `overflow` modes deliberately exercise result/limit handling within the same tuple. The probe creates a new private directory, runs only its own children and reaps them; it does not attach to or terminate a user session.

Initial isolated runs passed four normal journeys, sixteen takeover-during-running-output trials, and nine initial report-corruption challenges. Targeted protocol/lifecycle tests cover exact dependency and final-launch fencing, stale context/endpoint/control, immutable receipts, dedupe, unknown launch/no retry, bounded scoped pagination, stream/journal capacity and actual mutex ordering. Peer review identified and repaired a last-slot journal publication inconsistency: result+artifact event capacity is now reserved atomically, with explicit Failed/incomplete receipts at127/128 occupied slots while retaining real exit/drain facts;126 slots can complete. The independent report audit now checks complete paginated human/consumer cuts, exact immutable epochs/context/receipt origins, per-stream offset/digest/count consistency and the actual running-output takeover witness/acknowledgement. Twenty-two counterexamples include the reviewers’ reproduced corruptions. Final-revision evidence and two independent reviews belong to the associated PR; a test report's self-declared `checks[]` is not an oracle.

A preliminary x86_64 Ubuntu22.04/kernel6.8/4CPU run observed in-process human takeover call p50=22µs, p95=p99=126µs, n=16. Workload: one producer emits18artifact bytes plus2048stderr bytes in32writes with2ms pauses; two scripted run handles plus the supervisor, no parallel invocations. Timing covers serialize/decode/control/encode from a current observation, excluding1ms polling, process launch, external IPC, model work, PTY input and rendering. Small sample tails are descriptive, not an SLO or capacity/physical-human-input result. Later exact-revision records supersede these preliminary timing samples without rewriting them.

## Reproduce

On the prepared Ubuntu checkout, no new package/install or live daemon is needed:

```sh
make flow-build
python3 -B tests/h0/flow.py --bin-dir target/debug --artifacts "$PWD/.artifacts/flow-$(date +%s%N)"
```

The script runs four normal, sixteen noisy-takeover and two fault invocations, each with a fresh context and at most two children. It records commands, executable/schema/context digests, scoped observations, immutable receipts, output counts/hashes, nearest-rank latency samples and 22 rejected report corruptions. Raw stdout/stderr transcripts are not retained; the synthetic artifact is bounded dependency data in RAM. `make foundation-verify` includes this real journey, and a regression test verifies that its failure fails the lane. Source-only excludes only the historical native worker, not the owned Rust harness. Historical X4/EA1 remain FAIL and are not rerun by this experiment.

## Review and debt boundaries

Managed processes give an exact launch/stream association; this does **not** solve semantic attribution for commands in opaque persistent shells. Scope filtering intentionally shares the exact artifact/control facts but hides other-run stream events and receipts; the global cut can reveal event counts/timing and is not tenant isolation. Collection/hashing and bounded channel waits stay outside the authority mutex. Final spawn validation and the OS spawn share that mutex, so a stalled kernel spawn may delay takeover; no hard responsiveness bound is promised. Takeover never claims to stop external side effects.

Captureoff still permits intentional artifact bytes and bounded volatile receipts/digests; there is no durable journal, reconnect credential, replay or crash recovery. A successful child with complete output is process evidence, not task correctness; this fixture's expected-content validator is deliberately specific. Broader H0 #46/#47 and H1 public subscriptions/scheduling/adapter criteria remain open. No toolkit/provider, dozens population or whole-MVP scope is selected by this slice.
