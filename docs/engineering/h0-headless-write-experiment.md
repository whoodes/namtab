# Bounded Rust headless write experiment

[H0 admission](../engine/h0-harness-contracts.md#remaining-h0-admission-packet) · [Write contract](../engine/native-contract.md#final-dispatch-and-suffix-fencing) · [Issue56](https://github.com/whoodes/namtab/issues/56)

## Admission recorded before implementation

Owner approval at **2026-10-19 18:53 UTC** admits this experiment and isolated tests, plus a reviewed **draft** PR, not its merge or full H0/H1. This is the A1/A2 entry for **`h0-pty-once/0`**, a separately versioned local proof. It does not extend `namtab.mk1/1`, replace N0's proposed public API, adopt new dependencies or select a provider/toolkit. A3 capacity/calibration, A4 broader residual criteria and A5 native-window feasibility remain open. No dozens/capacity/performance claim follows.

| Material tuple | Admitted boundary |
| --- | --- |
| Runtime / host | Existing supervised Ubuntu Linux, Rust1.91.0 locked workspace; same-build owned terminal worker and existing PTY helper, no new dependency/features/unsafe code. No sandbox or hostile-same-UID protection |
| Wire / schema | `namtab.h0-write/0`; Rust `protocol::h0_write` owns strict request/response DTOs, generated `schemas/experimental/h0-write-{request,response}.schema.json`. Anonymous Unix socket pairs connect two supervisor-issued scripted endpoints; **no public listening/authentication/SDK transport** |
| Identity | Exactly two issued principals/runs/namespaces, one exact context/task/attempt, one session/incarnation and exclusive assignment. Context/task are immutable digests of the recorded fixture manifest and task. IDs are locators; private endpoint handles, not client-supplied principal labels, establish the local binding. Epoch generated per experiment; no persisted tokens or reconnect credentials |
| Immutable provenance | Each admitted receipt retains epoch/principal/run/namespace/connection/control/capability, context/task/attempt, assignment/writer/guard, profile/geometry and canonical observation/mode revisions. A transfer changes the current assignment/writer generations; it never relabels old receipts |
| Populations | C≤2 issued endpoints, R=2 runs, K=0 managed commands, T=1 lifetime PTY, M=0 model requests; no inference about a shell command's semantic completion/output ownership |
| Resources | 4,096-byte frames before decode; bound reference strings≤64 graphic ASCII bytes; text≤256 printable ASCII bytes or one logical Enter; two queued application/reply entries total, one unresolved call; 32 decisions/namespace, two retained canonical reply decisions; generations/correlation counters≤65,535, terminal revisions≤65,535; no wrap/eviction |
| Time / lifecycle | Relative monotonic TTL1–250ms, checked at admission and immediately before the call; 20s monotonic write-admission lifetime (no hard kernel/cleanup latency guarantee), PTY output processed in≤512-byte chunks with ≤64KiB synthetic observation budget. Reconnect only by supervisor after disconnect, same run/new connection/control/writer fences. Run close/revoke irreversible; no replacement attempt within this profile |
| Storage / privacy | Capture off; no durable journal/checkpoint/replay. Frozen input and receipt bodies remain only in bounded RAM for this invocation; synthetic test output/evidence may be recorded explicitly. Output budget exhaustion fences input, not a hidden truncation-as-complete |
| Refusal / uncertainty | Invalid/version/shape/key/body mismatch/exhaustion refuses before effects. Known nonblocking zero is zero, partial preserves exact prefix; uncertain error has unknown extent. Partial/unknown fences **all byte lanes** and requires a new session. Missing receipt is unknown evidence, not proof of zero |

## Control and write contract

Observe → queue an exact bound intent → dispatch that retained decision → query is the experimental loop. Only printable text or Enter is admitted and encoded by the existing canonical worker. DTO schemas own shape; the Rust decoder/admission also enforces byte/identity/deadline/resource limits (schema-valid alone is insufficient). No raw-byte bypass, key-protocol expansion, shell-marker authority or protected-prompt inference is added. Client transfer requires current authority and the exact binding; close/revoke/disconnect/reconnect are restricted supervisor decisions in this proof. The latter are not client-authenticated product control APIs.

The client observes the last worker-reduced cut; this does not establish application/kernel quiescence or semantic command boundaries. In this fixture there is no protected credential prompt, inferred approval or arbitrary TUI control route. Worker/schema/executable digests are prelaunch observations under the same-UID trust assumption, not sealed executable attestation.

The queue/metadata caps are application-level bounds, not cgroup/RSS limits; OS socket buffers are inherited, with one synchronous request per endpoint. The proof is not a production isolation/resource-control admission.

The single state/writer mutex orders final validation, the **one** `PtyWriter::write_once` call and authority changes. This makes the check adjacent to the syscall; it provides no hard control-latency bound if the kernel stalls. Admission/encoding outside that boundary grants no write permit. An active call occupies the byte lane until its trusted completion is reconciled. Late ACKs refine only the original receipt. No automatic suffix, resend, replay or in-place re-arm exists, including after a known zero. Querying a decision never redispatches it; changing the body under a consumed key conflicts.

Canonical replies enter only from the owned worker's output reduction, never an application request. They use independent restricted authority and the same ordered lane, not the current run's identity. An unresolved application call blocks them. Partial/unknown reply delivery also requires a new session. A worker/output continuity failure fences the session; plausible screen content cannot repair it.

This experiment deliberately does **not** implement N0's durable commit-before-effect contract: RAM admission is lost on crash, restart has a fresh epoch/session and no recovery/resend. It is a bounded runtime slice, not an admission of crash-safe dispatch. The finite JS model permits suffix/re-arm paths; comparison here covers only full first-call, stale/fence/refusal, transfer and immutable-result cases. Its stronger/more general states are not runtime guarantees.

## Validation and evidence

The implementation is a separate `service::h0_write` module and `namtab-h0-probe`, not an MK1 wire extension. [Reproduction](#reproduce) and the exact comparison mapping in repository `tests/h0/README.md` distinguish the real PTY proof, injected writer evidence and finite-model subset. Local/hosted results and exact source identity are recorded in the associated draft PR and fresh ignored handoff; no historical report is refreshed. Required challenges: two independently attributed clients with the same sequence number; queued transfer/close/revoke/disconnect/deadline fences; reconnect old-binding rejection; partial/unknown/late completion; canonical reply ordering; exhausted bounds; actual fresh PTY input/handoff, with no automatic retry. Preserve negative runs and exact source/commands/versions in fresh ignored evidence. Old X4/EA1 results remain independently historical **FAIL**.


## Reproduce

On the prepared Ubuntu checkout, with the existing pinned tools/dependencies:

```sh
CARGO_HOME="$PWD/.cache/cargo" cargo build --locked -p project-x-service -p project-x-terminal -p project-x-platform --bins
# Select a NEW absolute path; existing directories refuse. Parent must exist.
target/debug/namtab-h0-probe --artifacts "$PWD/.artifacts/h0-example-UNIQUE" \
  --worker "$PWD/target/debug/namtab-terminal-worker" \
  --pty-helper "$PWD/target/debug/namtab-pty-helper"
.tools/node/node-v24.21.0-linux-x64/bin/node tests/h0/compare-model.mjs \
  .artifacts/h0-example-UNIQUE/report.json
CARGO_HOME="$PWD/.cache/cargo" cargo test --locked -p project-x-service --lib h0_write
```

The probe exclusively creates its workspace/artifact directory, uses only synthetic Bash builtins and reaps only its own child. It retains `report.json` and explicitly consented synthetic `synthetic-output.bin` in that directory. No user session, named socket or daemon is touched. The report pins prelaunch worker/helper/Bash/schema hash observations, environment/argv/workspace, profile, context/task digests, original receipt bindings and exact DSR reply bytes/source cut. The comparison independently recomputes the context/task digests and challenges altered manifest/reply/extent/attribution evidence. It is not executable attestation against hostile same-UID races. Normal engine output capture remains off.

`make foundation-verify` includes this probe and the limited audited model comparison, alongside existing MK1 assertions. Unit tests inject partial/unknown/late completion at the real Rust control seam; the actual PTY fixture proves only observed full single writes, handoff and canonical replies. There is no actual kernel-partial, durability, public authentication, fairness/latency/capacity or GUI qualification. Do not equate the finite model's suffix/re-arm behavior with this new-session-only runtime policy.

## Adversarial corrections and residual limits

Independent review challenged the actual call and evidence seams. A two-reply counterexample crossed the session deadline during the first write; each queued reply now checks monotonic expiry immediately before its own call. A private injected-clock regression advances time after the first call without sleeping. It is not a client clock capability or an OS timing proof. Earlier self-review also retained a counterexample where a refused nonholder transfer fenced another run; refusal no longer changes that run's queue.

The three-receipt comparison checks the admitted manifest constants/schema hashes, exact known fixture input lengths and session/incarnation/profile/geometry/principal/namespace/connection/control/capability/assignment/writer/guard pins. Rehashed capture/version changes, zero-length “full” results and stale/cross-session writers must fail. Hashes check consistency under the stated trust boundary; they cannot authenticate an entirely fabricated report. Hash observations precede both worker and PTY launch, with same-UID executable races still outside the guarantee. Detailed original findings, negative commands, corrected checks and exact final review SHAs remain append-only in the ignored task/review evidence and draft PR summary.
