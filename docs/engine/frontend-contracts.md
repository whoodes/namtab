# Canonical model to frontend integration

[Index](../index.md) · [Shared human UX](../ux/11-human-experience-spec.md) · [Host TUI profile](../ux/13-dual-frontend-and-host-tui.md) · [Native contract](native-contract.md)

**Proposed integration, unimplemented.** This page owns the canonical projection/rendering seam and the native-process-to-interactive handoff. Human layout, shortcuts, review, appearance, configuration and states have one owner in UX 11; host decoding/TERM/SSH differences have one owner in UX 13.

## One model, two useful clients

Both clients consume public service projections: revisioned frames/history pages, native process/stream facts, tasks, receipts and authorized artifacts. No inner parser, independent mode tracker, private persistence/worker endpoint or managed PTY access. Disposable glyph/row caches, scroll position, selection and focus are presentation state; rebuilding them cannot alter canonical facts or send replies.

A frame/page contract needs session/incarnation, profile/data/geometry versions, base/target state revisions, processed event frontier, buffer and stable line/range identities, cell/grapheme/style/cursor data and completeness/availability. Deltas apply only to their exact base; a missing base returns resync rather than client-side repair. Large observations/history are paged and credit-bounded. Schema fields remain proposed until N0; do not copy private core structs into public bindings.

“Raw terminal view” means ordinary canonical presentation through admitted input, not unmanaged PTY passthrough. [Zero-agent UX and four lenses](../ux/11-human-experience-spec.md#information-architecture-and-default-layout) remain the product entrypoint. N1 needs only a useful selector, viewport, receipt/status and takeover in **both** clients; full Work/Review/search/windows follow their declared gates, not pane creation per agent.

## Input, focus and command routing

The [native contract](native-contract.md#observeactobserve-races-and-human-override) owns explicit targets, lease/freshness fences and receipts; [core input policy](compatibility-and-core.md#input-policy) owns encoding. Human [desktop keys/focus](../ux/11-human-experience-spec.md#three-action-planes-and-keyboard-behavior) and [TUI prefix/outer decoding](../ux/13-dual-frontend-and-host-tui.md#input-modes-and-nesting) normalize only facts their endpoint knows. UI commands, application intents and automation/control remain distinct. Focus never grants a writer lease.

A quoted TUI Ctrl+G must become a control-key intent, not C0 hidden in `CommitText`; the current synthetic foundation is not the completed native contract. Send key can express an inner supported intent when a host cannot physically distinguish it. No client may guess missing layout/consumed-modifier/IME/release facts to claim parity. [DF02/DF03](../engineering/16-test-and-ci-strategy.md#dual-frontend-contract-cases) tests endpoint races and nesting.

## Text, rendering, selection and accessibility

**Recommended N1 desktop renderer: virtualized DOM cells/runs with explicit grid geometry**, browser shaping/font fallback and an accessible reading projection. No browser emulator. Prove occupancy, cursor, selection and actual assistive interaction before optimizing. Canvas/WebGL is a later measured replacement for painting only if DOM limits justify it and the same semantic accessibility/selection contract survives. Do not build two renderers in advance or assume a DOM node per cell meets G6.

Canonical widths/cluster occupancy come from the [core text profile](compatibility-and-core.md#unicode-reflow-and-rendering-boundary); fonts/shaping may not reinterpret them. Selection anchors bind session/buffer/logical-line/column and revision, with explicit mapping or invalidation after erase/reflow/eviction. Copy/search cite authorized ranges and gaps; an application selection is separate from a view selection. [Human rendering/paste/search](../ux/11-human-experience-spec.md#rendering-selection-paste-and-scrollback) owns font/theme/clipboard/preview behavior, and [host capabilities](../ux/13-dual-frontend-and-host-tui.md#paste-resize-and-terminal-capabilities) owns outer width/color/paste limits.

IME preedit stays frontend-local; committed text goes once through the typed path. Candidate placement tracks canonical cursor/viewport. Desktop accessibility maps semantic controls/reading ranges to those facts; full-screen TUI cells cannot invent a native accessibility tree. Critical workflows need actual [accessibility tests](../ux/11-human-experience-spec.md#accessibility-and-performance-behavior), not only ARIA, headless grids or a linear log.

## Native-to-legacy journeys

**NJ1: native agent work, then human debugging.** Agent starts task T/attempt A, runs an argv test command with pipes, watches separate streams and an exact execution ID, and registers authorized artifact revision R. Human receives one Review item, inspects exit/drain facts and artifact diff, then opens a **new** PTY editor session at that context. A pipe-backed process cannot simply be converted into a terminal-attached process. Validation of R becomes stale if the editor changes it; task acceptance requires a new bound result.

The shared journey catalog is [J01–J09](../ux/11-human-experience-spec.md#end-to-end-journeys-and-transition-evidence): zero-agent shell (J01), delegation/review (J02/J03), stale approval (J04), takeover (J05), reconnect versus engine loss (J06), sensitive/remote interaction (J07), config migration (J08), cross-client handoff (J09). NJ1 above adds the native-pipe boundary; it does not duplicate that catalog. Observe–act–observe and N01–N16 govern opaque navigation throughout.

## States and transitions

[Human recovery/state table](../ux/11-human-experience-spec.md#takeover-resume-and-recovery-states) owns empty/loading/degraded/disconnected/unknown states. Native process exit also has a distinct **output draining** state: remaining streams/descendants are not implicitly complete. [Runtime recovery](runtime-persistence-security.md#precise-v1-recovery-contract) defines what each frontend may truthfully show after owner/service/keeper loss. No cached screen can restore authority or justify input into unknown interpretation.

## Measurement contract

Use the sole [G6 numerical baseline](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets) and [all-process method](../engineering/16-test-and-ci-strategy.md#all-process-performance). Actual human key→paint, native admission/receipts, observation freshness, resize and takeover include queueing/IPC/store/rendering. TUI-only, desktop-only and both share one canonical backend. Model/provider/application latency remains separate. N1's smaller declared workload is not a full G6 pass; no new result is claimed here.
