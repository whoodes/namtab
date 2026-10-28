# NAT1 native fleet-terminal control

[E6 ownership](../adr/0006-engine-and-native-harness-workstreams.md) · [issue71](https://github.com/whoodes/namtab/issues/71) · [engine contract](../engine/attempt-control.md)

**Owner-resumed NAT1 scope, not implemented in this ENG1 branch:** consume the reviewed ENG1 contract in a separate native harness branch/PR, explicitly stacked on ENG1 and PR68. No new parser, execution/policy authority, model/provider loop, renderer adoption or capacity expansion. Existing native X11/font/IME/a11y limits remain.

## Useful two-client loop

The Work view is a roster of actual attempts/tasks/origins and sessions, with one selected detail activity. It must not equate a declared endpoint with an authenticated agent run or expose a wall of panes. Show task revision, launch endpoint/request, observed process/cancellation facts and connection status separately. Model reasoning/turn status stays unobserved unless an adapter actually reports it.

Keyboard Up/Down in Work changes selection; it is a UI action, never application arrows. Open a selected terminal explicitly; Ctrl+Shift+Tab retains terminal cycling. Selection/view/focus change pauses application input and invalidates local queued intent context. The terminal remains a projection of the engine's canonical screen; takeover confirms one session's current writer generation. Existing app key/input routes and real-focus fencing stay intact.

Cancel opens a confirmation for one frozen epoch/attempt/origin tuple, with a direct-child-only warning. Enter once sends it; Escape cancels; auto-repeat cannot resubmit. Capability missing, changed selection/context, disconnect, synthetic held-key restoration or unknown result closes/refuses the action. The engine decides authority and effects. The harness cannot replace it with PID kill, Ctrl+C or namespace shutdown. Requested/already exited/unknown are visible, not an invented generic Done.

## Acceptance and limits

A fresh actual-window journey must use two explicitly scripted clients and one immutable shared task: observe independent attributed attempts/streams; navigate work and canonical terminals; cancel A while B continues; take over only the chosen terminal and reject stale writer input; show disconnected/failed/unknown truthfully. No fixture is called a model. Capture actual window pixels and targeted keyboard/focus evidence, preserving the original failed focus assertion and library-suppression limitations.

Retain2PTY/2attempt/four-connection bounds and disclose lifetime slot exhaustion. Record raw samples/workload/hardware for any latency statistic; sampled control RPC and input-to-receipt timing are not inference, renderer or all-process performance qualification. Wider fleet scheduling/targeted subscription capacity belongs to later engine gates; larger UI virtualization/final accessibility/IME belong to later harness gates. No exact32client or universal speed promise.
