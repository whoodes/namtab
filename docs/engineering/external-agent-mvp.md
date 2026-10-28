# External-agent bridge and lifecycle increment

[Native launch](native-mvp.md) · [H0 admission gaps](../engine/h0-harness-contracts.md#remaining-h0-admission-packet) · [Issue67](https://github.com/whoodes/namtab/issues/67)

This separately admitted increment builds on merged PR66/main `fb428384426be7127e40b7ce3b472c6cfea8e78b`. It enables an existing agent to call Namtab's public execution/control tools and adds explicit run lifecycle controls. **The product remains an owned terminal engine and human control surface for agent fleets. MCP and shutdown are enabling plumbing, not fleet qualification or the milestone finish line.** No new model loop, final toolkit, production API freeze or H0/H1 acceptance follows.

## Admitted experimental tuple

| Boundary | Narrow implemented profile / limit |
| --- | --- |
| Host/dependencies | Existing Ubuntu22.04 x86_64/Rust1.91.0/Cargo lock, native X11 tuple and Python3.10. No dependency additions, SDK source copying, renderer replacement or lock repinning |
| Adapter | `namtab.mcp-local/0`, stdio only; MCP2025-11-25 or2025-06-18. JSON-RPC initialize→initialized→tools; no HTTP, model sampling, roots, prompts, subscriptions or task protocol. No claim of universal MCP-client conformance |
| Resources | One serial broker connection, one bounded line at a time:65,536input bytes,262,144response bytes,128accepted request IDs per bridge lifetime. No unbounded worker pool or retry queue. Broker operations retain their2s transport timeout; blocked stdin/stdout applies OS backpressure rather than spawning work |
| Identity | Exact broker epoch and endpoint, separate MCP IDs and broker request IDs, typed task revision/attempt origin/receipt. Declared Agent role under same-UID peer checks is not authenticated model identity or H0 principal/run admission |
| Execution | Existing two lifetime PTYs, two lifetime managed attempts, four simultaneous public connections, captureoff. No provider-request population admission or dozens-agent capacity result |
| Lifecycle namespace | New broker reserves **one of16retained identities** for restricted `LifecycleHello`; **15ordinary lifetime identities + one reusable lifecycle identity**. Total retained and live connection caps do not grow. This explicitly reduces ordinary admission from the original16; it is not a measured capacity improvement |
| Persistence/security | No adapter storage or resume credential export; no new credential setup. Broker metadata journal and live projections are distinct from durable capture. No sandbox, hostile-same-UID, descendant containment, crash recovery or deterministic replay guarantee |

Oversized or unfinished line framing closes the bridge without processing its suffix. A malformed complete JSON line produces a parse error without dispatch; a later distinct complete request can still be processed. Batches are refused. Valid request IDs are bounded strings or safe nonnegative integers; reuse is refused even if the payload matches. There is no implicit retry, reconnect or operation replay. A fresh bridge is a new endpoint, not permission to repeat uncertain work. The parser is the existing serde_json parser within the line cap, not an adversarially qualified codec.

MCP initialization itself makes no model request. `tools/list` advertises only the supported operations. Invalid request shapes/unknown tools produce protocol errors; known-tool argument validation produces `isError:true` without dispatch. Execution refusals/partial/unknown results also remain errors. The bridge validates receipt endpoint, request ID and operation plus snapshot epoch/endpoint/version; an inconsistent response fences the connection. Response-cap exhaustion preserves the operation binding and reports uncertainty, never truncates JSON into apparent success. Stdout is exclusively protocol data.

## Agent integration and observation

| Tools | Actual authority and meaning |
| --- | --- |
| `namtab_task`, `namtab_run` | Define an immutable task contract, then start literal argv with that exact revision. Start/delivery is not exit, task acceptance or correctness. Poll the returned attempt identity |
| `namtab_observe` | Default work facts omit terminal grids and streams. Select **one** attempt for stdout/stderr previews (4,096code points each with preview/source truncation flags), or one session for its canonical screen/context. Not full capture or inferred TUI intent |
| `namtab_create_terminal`, `namtab_claim_terminal`, `namtab_release_terminal`, `namtab_input` | Real PTY through the broker, exact writer generation and observed screen/mode revisions. Agent claims never force human takeover; no HumanLive/raw-input bypass. Broker remains final writer, encoder and reply authority |
| `namtab_artifact`, `namtab_receipt` | Observe a bounded file revision or this bridge's known mutation receipt. No human approval operation is exposed. File observation is not sealed producer causality |

Only calls actually sent through this bridge are mediated and attributed. An agent displayed inside a PTY is not thereby integrated. Its own shell, file edits, model requests, web, other MCP tools and subprocesses remain external/uncontrolled unless separately mediated. Tool descriptions/annotations and output text confer no Namtab grants. Output, task text and terminal content remain untrusted data, including potential prompt injection; no universal model-level defense is claimed.

The native Work view renders broker facts, not model reasoning/turn status. Its chrome now says **model turns not tracked**, covering both scripted and external clients without pretending to detect a model. Literal argv entry remains a command form, not agent steering. A conversational fleet portal is still work to do.

For a **fresh, explicitly chosen, nonsecret** run:

```sh
make mvp
python3 -B scripts/mvp.py agent-config --run .artifacts/mvp-PRINTED_ID
```

`agent-config` prints a supported one-process Codex MCP configuration and **starts nothing**. It does not install a server, read auth stores, change persistent configuration or authorize a provider. The human must approve the chosen provider/workspace/tool exposure first. Basic terminal use requires no model account. Optional Codex uses existing CLI-managed authentication; Namtab never reads/copies it. Other MCP clients need their own actual interop validation.

## Detach, status and explicit stop

New launches record the broker epoch in their own small `launch.json`; no resume credential is persisted. Window close still detaches. These commands need no display and never discover sockets or accept a PID:

```sh
make mvp-status RUN=.artifacts/mvp-PRINTED_ID
make mvp-stop RUN=.artifacts/mvp-PRINTED_ID
```

Status uses the restricted lifecycle identity repeatedly rather than consuming ordinary endpoints. Stop verifies the recorded epoch, checks `human_shutdown_v1`, and requests a protective authority decrease. This is a same-UID **declared human lifecycle purpose**, not proof of a physical human. That endpoint cannot run commands, create/claim terminals or input bytes; it cannot resume work identities. An agent endpoint cannot request shutdown. One lifecycle connection at a time and the unchanged four-live-connection cap still apply; a full connection budget can refuse it. Close an explicitly owned client connection rather than guessing/killing a process.

The initiating shutdown response, including an uncertain durable receipt, is tied to its exact endpoint/request before the listener enters cleanup. Queried/rejected shutdown-shaped receipts cannot trigger that transition. Final writer authority is cleared under the same state lock as delivery. Already admitted operations can finish their bounded effect path; new grants are refused. The server drains admitted mutations, stops its worker, and waits for still-owned **direct** process handles before normal return/removal of its own socket inode. No process group, escaped descendant, completed external effect or full output drain is certified.

Worker/journal/receipt pressure cannot create a new grant. A protective shutdown can proceed even when receipt persistence fails; its state is **unknown**, and actual socket removal is a separate observation. The launcher waits up to5s for that exact socket to disappear. Replacement, timeout or lost response is uncertainty, not permission to retry, remove a socket, signal a numeric PID or restart. Kernel waits can outlive the caller timeout. Files/evidence remain. Older PR66 daemons/records do not support this handshake; they are left unchanged and require separate operator review. This task never contacts the owner's existing live sessions.

## Evidence, approval blocker and reproduction

The deterministic MCP fixture executes real public IPC, managed argv, PTY input and human takeover. It checks separate streams, exact contract/origin/receipt bindings, reused-ID refusal, no forced claim, stale zero-byte input refusal, EOF detach, repeated status, wrong-epoch/Agent shutdown refusal and direct-child exit with retained files. This is protocol/runtime evidence, **not a model**.

```sh
make mvp-build
python3 -B tests/mvp/adapter.py --bin-dir target/debug --artifacts .artifacts/mcp-NEW-ID
python3 -B tests/mvp/adapter.py --native --bin-dir target/debug --artifacts .artifacts/mcp-native-NEW-ID
```

Use unused paths under the project and adjust for CARGO_TARGET_DIR. The native variant requires actual X11 and targets only its verified window; it retains the original focus failure and narrower synthetic-key/XI2 qualifications. Foundation CI runs the real **headless** MCP/lifecycle journey, all-target Rust checks, fourteen launcher cases through the existing system-test wrapper and failure-propagation regression. It does not run a GUI or model/provider. Original X4/EA1 FAIL remains unchanged.

**Actual Codex0.162.1/OpenAI attempt:** owner authorized existing CLI, synthetic-only data and no auth changes. A fresh nested synthetic workspace, ephemeral session, disabled project instructions/command tools/plugins/web, read-only sandbox and fixed printf-only test gate were used. The bridge catalog has9tools; the live test gate exposed3. Real model events show a `namtab_task` attempt, but Codex rejected it with **MCP tool call requires approval, but approval policy is never**. The gate records only initialization/tool discovery: **no task/run was forwarded**. The original harness report remains FAIL (it later timed out awaiting the nonexistent attempt). Live model-directed Namtab execution is **BLOCKED/UNVALIDATED**, not a fixture PASS. No denied call was retried and no permission bypass was applied. Initial source copies did not include the live runner/gate; this provenance gap is retained, with future-run copies added rather than retroactively attesting that run.

The installed CLI supports interactive `-a on-request`; official docs support MCP mode `prompt`. Proposed smallest next action is **one temporary interactive invocation** in a fresh synthetic workspace with the same fixed gate, read-only sandbox and prompt mode, so the owner can approve the specific task/run **once** at its prompt. The owner subsequently approved **one** such temporary interactive invocation. It reached a **folder-trust screen**, before any tool approval: the installed CLI explicitly says the trust decision will be saved. No prompt was answered. Persistent folder trust remains outside the granted scope, so this is a second concrete blocker, not permission to choose Trust. The session subsequently disappeared with no result/tool-call record. The configured interval elapsed, but the exact exit cause is **unknown**, not a proven timeout or successful workflow. The remaining empty fixture-owned broker was verified by exact epoch and stopped through its public lifecycle endpoint; no user session was touched. No trust/tool prompt was answered, alternative invocation launched or session restarted. The dated termination diagnosis and source-provenance limits remain in the ignored handoff. Interactive CLI session metadata may be retained normally; no saved trust/approval/credential was authorized or supplied. Do not apply automatic approval, save an allow rule, retry the denied call or infer new authority from general model-test consent. `agent-config` is a general manual integration example, not approval to retry the blocked fixture. The manual `tests/mvp/live_codex.py --allow-live-codex-synthetic …` path is never a CI step and still needs the current provider/tool authorization; its existence is no grant.

The source ledger for this increment is the official [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [lifecycle/version negotiation](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), [tools/errors](https://modelcontextprotocol.io/specification/2025-11-25/server/tools), [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp), [noninteractive mode](https://learn.chatgpt.com/docs/non-interactive-mode), and [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference), retrieved2026-10-27; local `codex --version`, `codex --help`, `codex exec --help` establish the installed CLI tuple. Source guidance is not account entitlement or interop proof. No SDK or copied specification code is added.

## Fleet-terminal successor and remaining work

ENG1 PR74 and NAT1 PR75 subsequently merged the bounded exact-attempt operation and native roster/selection/targeted takeover/cancellation loop. See the [implemented architecture](../architecture/index.md), [engine control contract](../engine/attempt-control.md) and [fleet interaction owner](../ux/fleet-control.md). The original proposed-next-slice checklist is superseded rather than maintained as another roadmap.

Keep the2PTY/2attempt/four-live-connection envelope; connected endpoints, runs, commands, PTYs and model requests remain different populations. Broader scheduling/fairness, targeted subscription scale, full wire/resource admission, actual model mediation, rendering/accessibility and recovery gates remain open in the [current roadmap](../engine/delivery-and-validation.md) and [H0 residuals](../engine/h0-residual-criteria.md). Prior latency samples are workload-bound, not new SLOs or dozens-agent qualification.
