**Current increment note:** the separately admitted [MCP/lifecycle profile](external-agent-mvp.md) adds explicit status/stop for new runs and reserves one of16identities (15ordinary + one restricted lifecycle). Original MK1/PR66 admission/results below retain their dated scope. No live-model or fleet qualification follows.

# Try MK1 locally

**Retired app cleanup, 2026-10-20 ([issue61](https://github.com/whoodes/namtab/issues/61)).** Current checkout retains `make mk1-build`, `make mk1-daemon`, `make mk1-tui` and `make mk1-agent`. The Electron app/launcher/workspace/dedicated tests are removed. All Electron commands and UI instructions below are historical: use the intact [pre-cleanup source at eb7bbb2](https://github.com/whoodes/namtab/tree/eb7bbb2ddcecac646bcf4f8a45eac13d8af57bf0), in a separate checkout, with that revision’s reviewed artifact requirements. No existing local runtime, cache or live session is removed or stopped by source cleanup. Historical X4/EA1 findings and required license records are unchanged.

**Scope update — [E5](../adr/0005-ai-first-harness-direction.md), 2026-10-19.** Reproduction instructions below describe the delivered finite MK1, including its historical Electron client. Electron is frozen outside active product scope; the TUI remains diagnostic/attach tooling. Do not stop/restart existing user sessions as part of documentation work.

[Scope and owner admission](mk1-scope-and-admission.md) · [Design guide](../index.md) · [Core limits](https://github.com/whoodes/namtab/blob/main/crates/terminal/README.md) · [Broker limits](https://github.com/whoodes/namtab/blob/main/crates/service/src/mk1/README.md)

MK1 is the first hands-on human-feel slice. The commands below use real local processes, the owned terminal worker and one shared backend; no model, playback or mock terminal is required. This is not full NG1, a secure sandbox, a general-purpose terminal release or performance qualification. Use ordinary non-sensitive development commands; default capture is off, but live screens/streams are present in RAM.

## Prepare and launch

Run from the repository root on this Ubuntu x86_64 VM. Existing prerequisites are the pinned Rust 1.91.0 toolchain, Python 3.10+, Node/pnpm for repository checks, `tic` from ncurses for the private terminfo entry, and the separately verified Electron 44.8.0 Linux runtime. Nothing installs a global terminfo entry or starts a network server. Native build/launch helpers respect `CARGO_TARGET_DIR`; cross-target execution is explicitly unsupported.

```sh
cd /home/w/codex-dir/project-x
make mk1-build
# Terminal 1: keep the shared backend running.
make mk1-daemon
# Terminal 2: use your existing host terminal.
make mk1-tui
# Terminal 3, in the graphical session: open the desktop client.
make mk1-desktop
```

The default public socket is `.artifacts/mk1/socket`, inside an owned mode-0700 directory, with socket mode 0600. All clients use this same endpoint. The helper refuses a changed Electron executable hash. It does not install/download Electron or bypass its sandbox. The verified runtime on this VM is `.tools/electron/44.8.0/runtime/electron`; vendor ZIP/SHASUMS and notices are retained beside it. On another machine, prepare and verify that official artifact before launching. A missing display or normal-sandbox prerequisite is a blocker, not grounds for `--no-sandbox`.

An existing socket is refused; **do not blindly remove it or its journal**. It may represent a live owner. After an abnormal stop, use a fresh private run directory with the explicit `namtabd` arguments in the broker README, and give that public socket to both clients. Linux socket pathname length is at most 107 UTF-8 bytes; use a short path inside this project. Reconnecting a view differs from restarting the daemon, worker or OS process.

## First feel: agentic work, with a useful shell

The TUI creates the first clean-rc Bash session if there is none, takes its unowned writer and sizes it to the available viewport. Otherwise it attaches read-only: **Ctrl-G, then t** explicitly takes input. Type a simple command such as `printf 'hello from MK1\n'` and press Enter. Arrows, Tab, Backspace, Home/End and supported modifiers are application input; Ctrl-C is delivered as terminal input, not a promise that all work/descendants stopped.

The desktop starts with shared tasks and attributed activity. **Define a task** records a bounded instruction and declared artifact paths for protocol clients to observe; it does not launch a model or schedule work. A selected contract has an immutable revision, and attempts explicitly bind that revision. The separate **Run a local command** disclosure invokes clean Bash with the entered shell syntax through a managed attempt; it is explicit human local execution, not interpretation of an agent goal. Literal argv stays available in advanced controls.

**Shell** activity exposes Create shell, select session, Take input / Force takeover, Release and explicit Resize. Click the terminal viewport (which focuses its keyboard field) or the labelled Terminal keyboard field before typing. Ctrl-G then Escape returns keyboard focus to session controls; Ctrl-G twice sends literal application Ctrl-G. Select screen text outside that field and use ordinary Ctrl-C for browser copy; inside the field Ctrl-C is application input. It shows the same canonical screen as the TUI. Focus is distinct from ownership; switching sessions or losing window focus clears queued stale input. A passive view does not resize or take ownership automatically.

TUI controls use **Ctrl-G followed by**:

| Key | Terminal UI action |
| --- | --- |
| `t` / `l` | Human takeover / release; old-generation future input is fenced, existing side effects continue |
| Tab / `n` | Next session / create second shell |
| `r` | Explicitly resize the selected owned session to the host viewport |
| `a` | Literal managed-argv JSON form; Enter launches, Escape cancels |
| `b` | Toggle task/artifact/receipt review; application input is disabled in review focus |
| `c` | Explicit reconnect to the live owner after disconnect; never replay an uncertain mutation |
| `q` | Detach the TUI; leave the daemon/session running |
| Ctrl-G | Send a literal application Ctrl-G; prefix timeout/unknown commands send no application input |

In TUI task review, Tab selects the next managed attempt; Up/Down and PageUp/PageDown scroll its evidence. Task state, exact launch endpoint/request, revision-bound instructions and explicit artifact references precede bounded stdout/stderr. The TUI displays the contract bound to the selected attempt read-only; a contract without an attempt is not listed here. Define contracts through the portal or typed CLI. This is a small review view, not a full diff/search workflow.

Outer host terminals lose some physical key/layout/modifier distinctions and have Escape ambiguity. The TUI refuses unavailable facts instead of reconstructing them. The desktop knows more browser key identities but is not full IME, accessibility or layout qualification.

## Native work and safe handoff

Use **Run a local command** in the desktop for explicit clean-Bash shell syntax, its advanced literal-argv option, or **Ctrl-G a** in the TUI for a JSON argv array. An array such as `["/usr/bin/printf", "native stdout\n"]` is executed directly, with distinct stdout/stderr and actual process exit/drain state. It is not implicitly interpreted by a shell. Use short finite commands: MK1 has no managed-attempt cancellation operation or UI, and takeover of a PTY does not stop managed work. These facts differ from a persistent PTY shell, where the engine does not invent per-command exit status or command boundaries.

For real scripted agent integration without paid models, keep two typed CLI connections open alongside either or both frontends (four connections total):

```sh
make mk1-agent
```

Enter one operation JSON object per line. The client supplies the version, monotonic request ID and outer envelope; the positional `run TASK PROGRAM...` shortcut is ad hoc, so use JSON `run` with `task_revision` for a shared contract. These are protocol clients, not autonomous models. The shared task capability is `shared_tasks_v1`; absent capability means unsupported, never implied legacy support:

```json
{"op":"observe"}
{"op":"define_task","task":"native-demo","goal":"Produce two independent observations for human review.","expected_artifacts":[]}
{"op":"observe"}
```

From either client, observe that contract's **actual** `revision`, then submit `run` with `task:"native-demo"`, `task_revision` set to that exact value and the desired literal `argv`. Missing/stale revisions refuse. A run receipt's typed `attempt_started` result identifies the attempt; its `origin.endpoint` and `origin.request_id` distinguish identical numeric IDs from different clients. Separate streams/exit belong to that managed attempt. A free-form task label without a registered contract remains explicitly ad hoc. Contract goals/expected paths are data, not execution grants, scheduling or proof of artifact production.

An agent can explicitly release a PTY writer and another can take the observed new generation. Human takeover fences both old writers' future input; existing processes/effects continue. Both clients show shared-task activity rather than inventing model progress. A contract has no automatic success/completion state.

For application control, first observe the actual session ID/generation/revisions, take an unowned writer with `force:false`, then submit `input` with `policy:"observed"` and those exact facts. Do not copy stale example IDs, infer mode from output or automatically retry a lost input response. Named keys/text/paste share the same worker encoder as human input. A human can explicitly force takeover; a declared agent cannot force it. Connection/declared-role and managed-process facts are not evidence of an LLM thinking or a task being semantically complete.

Artifact registration explicitly associates a bounded workspace file with a task label. SHA-256 revisions and human-marked reviews refer to the observed file, not a sealed tree, causal proof or test success. A later file change marks the review stale. The desktop exposes registration/review; the TUI exposes the resulting facts in its task review view.

## Supported boundaries and stopping point

- Four immutable RAM task contracts, each with a single-line goal of at most 512 UTF-8 bytes and at most two declared relative artifact paths; no contract editing, autonomous task dispatcher, dependency scheduler or model integration. Define a new task for a changed instruction.
- Four connected clients and sixteen retained identities per daemon; prefer explicit live reconnect over creating identities repeatedly. The clients show declared agent connection facts, not model activity. Receipts/journal also have finite caps documented in the broker contract.
- Two total PTYs and two total managed attempts per daemon, including finished records; explicit capacity errors do not imply success. This deliberately small lifetime budget is for MK1 evaluation.
- Width-one ASCII/Latin-1 except soft hyphen, monochrome frontend projection, selected cursor/mode/key/query behavior. No history/search/reflow, wide/grapheme/bidi support, mouse navigation, general Vim/less compatibility or arbitrary enhanced keyboard protocol. Unsupported enhanced negotiation fences input until a supported reset/new session. Display replacement markers remain observations, not valid committed input.
- Bracketed paste must be enabled by the application; at most 8,180 content bytes fit the single-write envelope. No silent unbracketed multiline paste, raw-byte bypass, automatic suffix resend or guessed prompt safety. Partial/unknown delivery fences further input; a new session is the bounded recovery path.
- After any refused/uncertain application input, both frontends pause and discard queued suffixes. Inspect the application (a command may be incomplete), then explicitly Take/Re-arm; TUI uses Ctrl-G t. Focus/observe/reconnect never silently clear the pause. Queued recovery gestures are discarded too. Same-UID delayed software input cannot be attested as a physical human gesture.
- Ordinary human-live input tolerates echo-only screen changes while retaining original owner, generation, mode and geometry fences. Exact agent input stays revision-bound. Neither policy promises application understanding or absence of observation/action races.
- No output/input content is journaled. Metadata receipts reserve before effects; completion persistence failure is uncertainty. Live reconnect is supported; daemon/worker loss does not restore screens/processes or authorize old mutation replay.
- Developer-host filesystem/network access is unenforced; same-UID matching does not attest a real human or protect against hostile same-UID programs. Detach/takeover do not stop prior work. Abrupt daemon death may leave descendants/effects; MK1 is not containment or crash-recovery qualification.

Use MK1 to assess typing feel, visibility of ownership/status, navigation between two sessions/tasks and the usefulness of truthful native evidence. After verified handoff, implementation stops for user feedback. Broader compatibility, hardening, polish and full NG1 remain separate work.

## Detach, stop and evaluate again

To leave work running, detach the TUI with Ctrl-G q and close the desktop window. To end a test, first finish or interrupt your commands and explicitly exit each shell; observe its exited state. Close clients, then use Ctrl-C in the daemon terminal. Killing the daemon is not an orderly all-process shutdown: descendants/effects may survive and a socket may remain. Inspect and clean up only your known processes separately; never infer termination from a closed window.

Completed attempts and sessions still consume the lifetime budget. For another evaluation, use a **new** owned run directory instead of deleting an old socket/journal. After the build, these explicit commands select the same fresh endpoint (replace `mk1-next` with another unused short name if needed):

```sh
# Terminal 1
./target/debug/namtabd --socket "$PWD/.artifacts/mk1-next/socket" --workspace "$PWD" --worker "$PWD/target/debug/namtab-terminal-worker" --pty-helper "$PWD/target/debug/namtab-pty-helper" --terminfo "$PWD/.artifacts/terminfo"
# Terminal 2
./target/debug/namtab-tui --socket "$PWD/.artifacts/mk1-next/socket"
# Terminal 3: use only the previously verified official runtime.
./.tools/electron/44.8.0/runtime/electron apps/desktop/mk1 --socket "$PWD/.artifacts/mk1-next/socket"
# Optional separate terminal
./target/debug/namtab-agent --socket "$PWD/.artifacts/mk1-next/socket"
```

These examples use the default native `target/debug`; adjust all binary paths together if `CARGO_TARGET_DIR` was selected. A fresh daemon has empty RAM state and new authority, not restored work or a guarantee that previous side effects stopped.

## Reproduce engine evidence

After `make mk1-build`, the active CI lane runs both real harnesses in fresh isolated namespaces, preserving any live evaluation session:

```sh
python3 -B tests/mk1/e2e.py --bin-dir "$PWD/target/debug" --terminfo "$PWD/.artifacts/terminfo" --artifacts "$PWD/.artifacts/mk1-use-a-fresh-id"
python3 -B tests/mk1/agents.py --bin-dir "$PWD/target/debug" --terminfo "$PWD/.artifacts/terminfo" --artifacts "$PWD/.artifacts/mk1-agents-use-a-fresh-id"
```

The first checks whole-command bursts, exact repeats, foreground interruption before a finite natural completion, task selection, suffix fencing, live reconnect and deliberate re-arm. The second checks two concurrent scripted agents against one immutable contract, precise endpoint/request attribution, duplicate/conflict/stale-revision refusal, explicit agent handoff and real TUI takeover while another PTY emits 32 KiB over a finite interval. File observations/reviews remain separate from sealed producer causality.

The agent harness retains raw monotonic RPC samples: 30 observations, control receipts and logical-End input receipts in each quiet/noisy phase (180 total), environment, commands and binary hashes. “Quiet” means the other PTY is idle; shared VM load is recorded, not controlled. Debug-build samples include client overhead, worker/service work and required metadata fsync; p50/p95/p99 are descriptive small-sample values, not achieved SLOs, input-to-photon/render/model latency or comparative speed. Its takeover-to-observed-rearm timing includes polling and the outer-screen test observer. No observed measurement justifies weakening durability/freshness gates. Real Electron screenshots/journeys remain separate from headless CI; human feel and broader accessibility/compatibility await evaluation.
