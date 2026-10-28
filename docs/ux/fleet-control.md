# NAT1 native fleet-terminal control

[E6 ownership](../adr/0006-engine-and-native-harness-workstreams.md) · [issue71](https://github.com/whoodes/namtab/issues/71) · [engine contract](../engine/attempt-control.md)

**Bounded NAT1 experimental draft:** owner resumed issue71 on2026-10-28. This separate branch depends explicitly on [ENG1 PR74](https://github.com/whoodes/namtab/pull/74) at **`a900b0c42137c2b887ed32b42131c2a97f4b5f77`** and consumes `namtab.attempt-control/0` / `attempt_cancel_v0` through the shared public client. The engine head remains unchanged. No parser, process authority, provider/model loop, renderer adoption or capacity expansion is added to the harness. No merge authority or stable public API follows.

## Admitted presentation and resource tuple

Existing Ubuntu22.04x86_64/Rust1.91.0/Cargo pins, winit0.30.13/X11 + softbuffer0.4.8 CPU surface and fontdue0.9.3 over the existing DejaVuSansMono font. No new dependencies/toolkit selection. Window bounds900×780–1600×1200 and finite Latin-1 rendering/IME-disabled limitations remain; this is not final shaping/accessibility/clipboard/scrollback qualification. Existing2PTY/2attempt/four-live-connection and15ordinary+one restricted lifecycle identities remain. UI control/input queues stay2/8; one optional confirmation target and a roster bounded by current2attempt/2terminal/4task counts add no unbounded pending work. Existing trace caps and no capture/automatic uploads remain.

The harness owns only work/terminal selection, focus, forms and confirmation/presentation caches. Engine snapshots remain authoritative; selecting a managed attempt does **not** infer an associated PTY. A disconnected projection is last observed, not fresh process truth or a reason to repeat an operation.

## Useful two-client loop

The Work view is a roster of actual attempts/tasks/origins and sessions, with one selected detail activity. It must not equate a declared endpoint with an authenticated agent run or expose a wall of panes. Show task revision, launch endpoint/request, observed process/cancellation facts and connection status separately. Model reasoning/turn status stays unobserved unless an adapter actually reports it.

Keyboard Up/Down or Home/End in Work changes selection; it is a UI action, never application arrows. Enter opens only a selected terminal, paused; selecting an attempt/task displays one detail projection. Ctrl+Shift+Tab retains terminal cycling. Selection/view/focus change pauses application input and invalidates local queued intent context. The terminal remains a projection of the engine's canonical screen; takeover confirms one session's current writer generation. Existing app key/input routes and real-focus fencing stay intact.

Ctrl+Shift+X opens a confirmation for one frozen epoch/attempt/origin tuple, with a direct-child-only warning. Enter once sends it; Escape cancels; auto-repeat cannot resubmit. Capability missing, changed selection/context, disconnect, synthetic held-key restoration or unknown result closes/refuses the action. The engine decides authority and effects. The harness cannot replace it with PID kill, Ctrl+C or namespace shutdown. Requested/already exited/unknown are visible, not an invented generic Done.

## Acceptance and limits

A fresh actual-window journey must use two explicitly scripted clients and one immutable shared task: observe independent attributed attempts/streams; navigate work and canonical terminals; cancel A while B continues; take over only the chosen terminal and reject stale writer input; show disconnected/failed/unknown truthfully. No fixture is called a model. Capture actual window pixels and targeted keyboard/focus evidence, preserving the original failed focus assertion and library-suppression limitations.

Retain2PTY/2attempt/four-connection bounds and disclose lifetime slot exhaustion. Record raw samples/workload/hardware for any latency statistic; sampled control RPC and input-to-receipt timing are not inference, renderer or all-process performance qualification. Wider fleet scheduling/targeted subscription capacity belongs to later engine gates; larger UI virtualization/final accessibility/IME belong to later harness gates. No exact32client or universal speed promise.

## Launch and actual validation

From a normally prepared source checkout (`make bootstrap docs-install install`, existing Rust toolchain), use:

```sh
make mvp
```

This builds and opens a **fresh** owned native run. Take input explicitly; closing detaches. The launcher prints its run directory and exact attach path. Status/stop use `python3 -B scripts/mvp.py status --run /absolute/printed/run` and `stop --run /absolute/printed/run`; older user daemons are never retroactively controlled. Literal argv is not an AI prompt; SCRIPTED DEMO is not a model.

| Interaction | Meaning |
| --- | --- |
| Ctrl+Shift+1 / 2 | Work / canonical Terminal view; change pauses input and dismisses confirmation |
| Work Up/Down, Home/End | Select attempt, terminal or immutable task by identity; no application key bytes |
| Work Enter | Open selected terminal paused; an attempt does not imply a terminal |
| Ctrl+Shift+Tab | Cycle terminals; input remains paused; Work selection follows an explicit terminal cycle |
| Ctrl+Shift+T | Take only the selected terminal with frozen observed epoch/generation; backend remains final authority |
| Ctrl+Shift+X, then plain Enter once | Confirm one frozen attempt/origin cancellation; receipt is request, exit observed separately |
| Escape, selection/view/resize/focus change | Dismiss cancellation; no hidden retarget or queued replay |
| Ctrl+Shift+E | Show selected task/attempt revision; one stream detail, not every agent's output |
| Ctrl+Shift+C | Explicit same-epoch live-owner reconnect; input/cancel intent not replayed |

Reserved Ctrl+Shift UI chords, including repeat, never become application input. Ordinary terminal key repeat remains application input only while focused/armed/owned. A confirmation is consumed before enqueue. The connector rechecks the local fence **after** blocking target/capability observation; the engine validates the immutable target/role. A UI fence is not atomic with OS focus/socket dispatch: an already-dispatched action can still take effect on its exact target. Takeover does not stop unrelated commands or all side effects.

For the supported native test path, run **one** fresh isolated journey:

```sh
make nat1-test
```

Its `mk1-build` prerequisite builds the same-worktree binaries and runs the existing project-local `tic` terminfo setup before the test. No undocumented manual directory creation is needed. It needs the actual accessible X11 DISPLAY, installed `tic`, ImageMagick `import`, libX11 and the documented font; no install, headless substitute or user-session discovery occurs. A fresh `.artifacts/nat1-<id>/checks.json`, trace and actual-window screenshots record attributed scripted work, selection/dismissal/focus loss, repeated typing/history/interruption, exact takeover/stale-writer refusal, one-shot human cancellation with independent owned pidfd exit/peer continuity, stale natural-exit dismissal and disconnected navigation/explicit resume. `make foundation-verify` runs headless contracts/model/real-process checks; hosted headless CI does **not** certify a graphical journey.

Initial evidence remains separate: the first manual invocation failed before launch because terminfo was absent; a later focus-mapping race caused Xlib fatal exit before result writing. Its exact own broker/window were separately verified and cleaned, recorded as FAIL—not qualification. The fixture now waits for an owned viewable focus target, turns Xlib errors into checked Python failures that reach cleanup, and tests a closed owned target. Another earlier full journey failed on an expired observer after the owned broker pause; the test explicitly replaces only that reader (no mutation replay) while the native client resumes its own same-epoch identity. These are harness/setup corrections, not changes to engine stop semantics or evidence deletion. Fresh clean-worktree reproduction and final-head checks/reviews are recorded in the PR/handoff.

No real model is run. Same-UID roles remain declarations, process labels are not cognition, and two attempts are not fleet-scale qualification. Historical failed focus/X4/EA1 records, physical held-key/IME/a11y gaps, no-capture/recovery limits and wider H0 decisions remain unchanged.
