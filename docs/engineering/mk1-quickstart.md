# Try MK1 locally

[Scope and owner admission](mk1-scope-and-admission.md) · [Design guide](../index.md) · [Core limits](https://github.com/whoodes/namtab/blob/main/crates/terminal/README.md) · [Broker limits](https://github.com/whoodes/namtab/blob/main/crates/service/src/mk1/README.md)

MK1 is the first hands-on human-feel slice. The commands below use real local processes, the owned terminal worker and one shared backend; no model, playback or mock terminal is required. This is not full NG1, a secure sandbox, a general-purpose terminal release or performance qualification. Use ordinary non-sensitive development commands; default capture is off, but live screens/streams are present in RAM.

## Prepare and launch

Run from the repository root on this Ubuntu x86_64 VM. Existing prerequisites are the pinned Rust 1.91.0 toolchain, Python 3.10+, Node/pnpm for repository checks, `tic` from ncurses for the private terminfo entry, and the separately verified Electron 44.8.0 Linux runtime. Nothing installs a global terminfo entry or starts a network server.

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

## First feel: zero-agent shell

The TUI creates the first clean-rc Bash session if there is none, takes its unowned writer and sizes it to the available viewport. Otherwise it attaches read-only: **Ctrl-G, then t** explicitly takes input. Type a simple command such as `printf 'hello from MK1\n'` and press Enter. Arrows, Tab, Backspace, Home/End and supported modifiers are application input; Ctrl-C is delivered as terminal input, not a promise that all work/descendants stopped.

The desktop exposes Create shell, select session, Take input / Force takeover, Release and explicit Resize. Click the terminal viewport (which focuses its keyboard field) or the labelled Terminal keyboard field before typing. Ctrl-G then Escape returns keyboard focus to session controls; Ctrl-G twice sends literal application Ctrl-G. Select screen text outside that field and use ordinary Ctrl-C for browser copy; inside the field Ctrl-C is application input. It shows the same canonical screen as the TUI. Focus is distinct from ownership; switching sessions or losing window focus clears queued stale input. A passive view does not resize or take ownership automatically.

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

In TUI task review, Tab selects the next managed attempt; Up/Down and PageUp/PageDown scroll its evidence. Task state and explicit artifact references precede bounded stdout/stderr. This is a small review view, not a full diff/search workflow.

Outer host terminals lose some physical key/layout/modifier distinctions and have Escape ambiguity. The TUI refuses unavailable facts instead of reconstructing them. The desktop knows more browser key identities but is not full IME, accessibility or layout qualification.

## Native work and safe handoff

Use the desktop managed-argv form or **Ctrl-G a** in the TUI. An array such as `["/usr/bin/printf", "native stdout\n"]` is executed directly, with distinct stdout/stderr and actual process exit/drain state. It is not implicitly interpreted by a shell. Use short finite commands: MK1 has no managed-attempt cancellation operation or UI, and takeover of a PTY does not stop managed work. These facts differ from a persistent PTY shell, where the engine does not invent per-command exit status or command boundaries.

For an agent interface without paid-model integration, keep one typed CLI connection open:

```sh
make mk1-agent
```

Enter one operation JSON object per line. The client supplies the version, monotonic request ID and outer envelope:

```json
{"op":"observe"}
{"op":"run","task":"native-demo","argv":["/usr/bin/printf","hello from an agent\n"]}
{"op":"observe"}
```

For application control, first observe the actual session ID/generation/revisions, take an unowned writer with `force:false`, then submit `input` with `policy:"observed"` and those exact facts. Do not copy stale example IDs, infer mode from output or automatically retry a lost input response. Named keys/text/paste share the same worker encoder as human input. A human can explicitly force takeover; a declared agent cannot force it. Connection/declared-role and managed-process facts are not evidence of an LLM thinking or a task being semantically complete.

Artifact registration explicitly associates a bounded workspace file with a task label. SHA-256 revisions and human-marked reviews refer to the observed file, not a sealed tree, causal proof or test success. A later file change marks the review stale. The desktop exposes registration/review; the TUI exposes the resulting facts in its task review view.

## Supported boundaries and stopping point

- Four connected clients and sixteen retained identities per daemon; prefer explicit live reconnect over creating identities repeatedly. The clients show declared agent connection facts, not model activity. Receipts/journal also have finite caps documented in the broker contract.
- Two total PTYs and two total managed attempts per daemon, including finished records; explicit capacity errors do not imply success. This deliberately small lifetime budget is for MK1 evaluation.
- Width-one ASCII/Latin-1 except soft hyphen, monochrome frontend projection, selected cursor/mode/key/query behavior. No history/search/reflow, wide/grapheme/bidi support, mouse navigation, general Vim/less compatibility or arbitrary enhanced keyboard protocol. Unsupported enhanced negotiation fences input until a supported reset/new session. Display replacement markers remain observations, not valid committed input.
- Bracketed paste must be enabled by the application; at most 8,180 content bytes fit the single-write envelope. No silent unbracketed multiline paste, raw-byte bypass, automatic suffix resend or guessed prompt safety. Partial/unknown delivery fences further input; a new session is the bounded recovery path.
- Ordinary human-live input tolerates echo-only screen changes while retaining original owner, generation, mode and geometry fences. Exact agent input stays revision-bound. Neither policy promises application understanding or absence of observation/action races.
- No output/input content is journaled. Metadata receipts reserve before effects; completion persistence failure is uncertainty. Live reconnect is supported; daemon/worker loss does not restore screens/processes or authorize old mutation replay.
- Developer-host filesystem/network access is unenforced; same-UID matching does not attest a real human or protect against hostile same-UID programs. Detach/takeover do not stop prior work. Abrupt daemon death may leave descendants/effects; MK1 is not containment or crash-recovery qualification.

Use MK1 to assess typing feel, visibility of ownership/status, navigation between two sessions/tasks and the usefulness of truthful native evidence. After verified handoff, implementation stops for user feedback. Broader compatibility, hardening, polish and full NG1 remain separate work.
