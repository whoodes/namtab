# Bounded Rust headless write experiment

[H0 admission](../engine/h0-harness-contracts.md#remaining-h0-admission-packet) · [Write contract](../engine/native-contract.md#final-dispatch-and-suffix-fencing) · [Issue56](https://github.com/whoodes/namtab/issues/56)

## Admission recorded before implementation

Owner approval at **2026-10-19 18:53 UTC** admits this experiment and isolated tests, plus a reviewed **draft** PR, not its merge or full H0/H1. This is the A1/A2 entry for **`h0-pty-once/0`**, a separately versioned local proof. It does not extend `namtab.mk1/1`, replace N0's proposed public API, adopt new dependencies or select a provider/toolkit. A3 capacity/calibration, A4 broader residual criteria and A5 native-window feasibility remain open. No dozens/capacity/performance claim follows.

| Material tuple | Admitted boundary |
| --- | --- |
| Runtime / host | Existing supervised Ubuntu Linux, Rust1.91.0 locked workspace; same-build owned terminal worker and existing PTY helper, no new dependency/features/unsafe code. No sandbox or hostile-same-UID protection |
| Wire / schema | `namtab.h0-write/0`; Rust `protocol::h0_write` owns strict request/response DTOs, generated `schemas/experimental/h0-write-{request,response}.schema.json`. Anonymous Unix socket pairs connect two supervisor-issued scripted endpoints; **no public listening/authentication/SDK transport** |
| Identity | Exactly two issued principals/runs/namespaces, one exact context/task/attempt, one session/incarnation and exclusive assignment. IDs are locators; private endpoint handles, not client-supplied principal labels, establish the local binding. Epoch generated per experiment; no persisted tokens or reconnect credentials |
| Immutable provenance | Each admitted receipt retains epoch/principal/run/namespace/connection/control/capability, context/task/attempt, assignment/writer/guard, profile/geometry and canonical observation/mode revisions. A transfer changes the current assignment/writer generations; it never relabels old receipts |
| Populations | C≤2 issued endpoints, R=2 runs, K=0 managed commands, T=1 lifetime PTY, M=0 model requests; no inference about a shell command's semantic completion/output ownership |
| Resources | 4,096-byte frames before decode; text≤256 printable ASCII bytes or one logical Enter; two queued application/reply entries total, one unresolved call; 32 decisions/namespace, two retained canonical reply decisions; generations/correlation counters≤65,535, terminal revisions≤65,535; no wrap/eviction |
| Time / lifecycle | Relative monotonic TTL1–250ms, checked at admission and immediately before the call; experiment hard deadline≤20s, PTY output processed in≤512-byte chunks with ≤64KiB synthetic observation budget. Reconnect only by supervisor after disconnect, same run/new connection/control/writer fences. Run close/revoke irreversible; no replacement attempt within this profile |
| Storage / privacy | Capture off; no durable journal/checkpoint/replay. Frozen input and receipt bodies remain only in bounded RAM for this invocation; synthetic test output/evidence may be recorded explicitly. Output budget exhaustion fences input, not a hidden truncation-as-complete |
| Refusal / uncertainty | Invalid/version/shape/key/body mismatch/exhaustion refuses before effects. Known nonblocking zero is zero, partial preserves exact prefix; uncertain error has unknown extent. Partial/unknown fences **all byte lanes** and requires a new session. Missing receipt is unknown evidence, not proof of zero |

## Control and write contract

Observe → queue an exact bound intent → dispatch that retained decision → query is the experimental loop. Only printable text or Enter is admitted and encoded by the existing canonical worker. No raw-byte bypass, key-protocol expansion, shell-marker authority or protected-prompt inference is added. Client transfer requires current authority and the exact binding; close/revoke/disconnect/reconnect are restricted supervisor decisions in this proof. The latter are not client-authenticated product control APIs.

The single state/writer mutex orders final validation, the **one** `PtyWriter::write_once` call and authority changes. This makes the check adjacent to the syscall; it provides no hard control-latency bound if the kernel stalls. Admission/encoding outside that boundary grants no write permit. An active call occupies the byte lane until its trusted completion is reconciled. Late ACKs refine only the original receipt. No automatic suffix, resend, replay or in-place re-arm exists, including after a known zero. Querying a decision never redispatches it; changing the body under a consumed key conflicts.

Canonical replies enter only from the owned worker's output reduction, never an application request. They use independent restricted authority and the same ordered lane, not the current run's identity. An unresolved application call blocks them. Partial/unknown reply delivery also requires a new session. A worker/output continuity failure fences the session; plausible screen content cannot repair it.

This experiment deliberately does **not** implement N0's durable commit-before-effect contract: RAM admission is lost on crash, restart has a fresh epoch/session and no recovery/resend. It is a bounded runtime slice, not an admission of crash-safe dispatch. The finite JS model permits suffix/re-arm paths; comparison here covers only full first-call, stale/fence/refusal, transfer and immutable-result cases. Its stronger/more general states are not runtime guarantees.

## Validation and evidence

Implementation and results are pending at this admission revision. Required challenges: two independently attributed clients with the same sequence number; queued transfer/close/revoke/disconnect/deadline fences; reconnect old-binding rejection; partial/unknown/late completion; canonical reply ordering; exhausted bounds; actual fresh PTY input/handoff, with no automatic retry. Preserve negative runs and exact source/commands/versions in fresh ignored evidence. Old X4/EA1 results remain independently historical **FAIL**.
