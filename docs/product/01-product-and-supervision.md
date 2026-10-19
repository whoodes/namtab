# Product and supervision

**Current direction:** [E5](../adr/0005-ai-first-harness-direction.md) prioritizes the AI-first execution/coordination harness and custom emulator; presentation form is open. MK1 is delivered. Older frontend examples do not require Electron or a second product frontend.

[Index](../index.md) · [Work model](02-work-evidence-and-coordination.md) · [Validation gates](../engineering/07-validation-roadmap-and-risks.md)

## Product thesis and assumptions

Build a standalone terminal where humans and agents share useful operating context without treating every task as a chat transcript or every agent as a pane to watch. A normal shell remains a first-class workspace. Agents must navigate existing interactive applications as well as launch commands: this is a primary product workflow, not a convenience hook. Parallelism should increase completed, reviewed work per unit of human attention, rather than merely increase active agent count.

Assumptions to validate: initial users are developers on Linux; agents can run as ordinary CLI processes; many useful outcomes are files, diffs, test results, or decisions; people need to interrupt and resume work without reconstructing context. Administrative and research workflows also matter, but not every operation has a Git revision or a deterministic result. The domain supports non-Git artifacts and external targets with explicit identity and freshness limitations.

The [human experience specification](../ux/11-human-experience-spec.md) owns concrete layouts, shortcuts, focus/selection, approvals, recovery states and J01–J09 journeys. It has the same release authority as the agent contract. The custom emulator serves that human contract; presentation form/renderer remain open, and the retained host TUI is diagnostic tooling.

## Personas and jobs

| Persona | Job and current pain | Product response |
| --- | --- | --- |
| Individual developer | Keep a shell, editor, debugger, and a few agents coherent | Ordinary terminal workspace, optional task association, quick takeover |
| Parallel-work supervisor | Review many attempts without reading every transcript | Artifact deltas, dependency view, decision queue, evidence drill-down |
| Reviewer or maintainer | Establish what changed, what was tested, and what is still unknown | Revision-bound review packet with contradictory evidence retained |
| Operations engineer | Work through SSH, interactive diagnostics, changing remote state | Visible host identity, cautious action boundaries, explicit connection uncertainty |
| Accessibility-focused user | Navigate high-volume work without tiny panes or constant visual monitoring | Keyboard navigation, structural announcements, text-first review, configurable density |
| Extension author | Integrate an agent, shell, validator, or runtime without forking the product | Narrow versioned adapters, conformance fixtures, declared capabilities |

Initial ownership is one local user. Multi-person simultaneous editing, enterprise role administration, and cross-organization sharing are future scope. Observer versus writer is still explicit because the same person may have multiple clients and agents.

## Scope and non-goals

Across the staged product scope (not all mandatory in N1): a custom emulator interface (form pending); Linux shells and inherited environment use; agentic navigation of CLI/TUI/REPL/editor interfaces; panes and windows; live and detached sessions; parallel tasks and dependencies; artifacts and review; attention routing; human control; bounded scheduling; local durable history; optional SSH and structured remote execution; replaceable adapters; explicit trust and capture settings.

Non-goals for initial releases: replacing Bash or POSIX; reimplementing editors/debuggers; a universal shell AST safety classifier; making arbitrary programs transactional; guaranteeing that a model is honest; automatically merging all changes; transparent process migration between hosts; exactly-once external side effects; recording every secret safely; deterministic execution replay; supporting every terminal graphics extension at launch. Windows and macOS are future ports, not implicit compatibility promises.

Both frontends use public broker APIs; neither reads private persistence or managed PTY internals, and neither owns permission enforcement. Semantic parity does not mean identical pixels, physical key representability or native accessibility.

Terminal use must remain worthwhile with no configured agents. Users can also launch agent CLIs without task integration. Conversely, managed background execution need not render a pane until inspection is useful.

## Agentic navigation is core

An authorized agent can inspect a session, move through shell history with arrows, request completion with Tab, cycle a supported completion menu, navigate interactive menus and pagers, edit within a terminal editor, and interact with a REPL or another agent CLI. Core input covers arrows, Tab/Shift-Tab, Enter/Escape, editing/page keys, F1–F12, supported Ctrl/Alt/Shift chords, explicit text, control sequences, and selection/paste. Each application's bindings and side effects remain its own; shell completion can run code, and a seemingly navigational key is not inherently read-only.

Two input action planes prevent accidental routing; the typed automation API addresses either explicitly and is not a third unmediated input path. Terminal UI commands address namtab windows, pane/view focus, scrollback, and view selection. Application input addresses a named session's PTY, independent of desktop focus. Selecting/copying terminal text does not select text inside an editor; sending an editor shortcut does not invoke a namtab shortcut. Host clipboard access is a separate capability. Agents do not have to steal the human's focused pane to drive other owned sessions.

The primary interface accepts structured logical keys with modifiers; an explicit raw-byte fallback handles declared compatibility needs. Raw bytes have the same target, ownership, context, prompt, and retry constraints, and require their own granted capability. Unsupported key combinations are reported instead of silently remapped. Detailed mode/encoding and delivery rules belong to the [navigation contract](../engine/native-contract.md#observeactobserve-races-and-human-override).

Navigation follows observe-act-observe: obtain an identified observation and current input modes, propose one bounded action, check authority and freshness, deliver through the runner, and inspect transition evidence before continuing. Completion menus, editor modes, selected rows, and approval/password prompts may be ambiguous. A successful write is not evidence that the intended application transition occurred. The product supports navigation without a cooperative application adapter, but provides no universal semantic navigation guarantee for opaque TUIs.

Human override remains immediately reachable. Taking session ownership stops further agent input admissions; task takeover separately handles background tools and edits. Sensitive prompts hand control to the human or a specifically authorized credential/action adapter. Agent navigation must not become a way to answer approval dialogs automatically. Acceptance covers ordinary applications and adversarial focus/staleness/load cases in [N01–N16](../engineering/07-validation-roadmap-and-risks.md#agentic-navigation-acceptance-cases).

## Task, artifact, and decision views

The default supervision surface is a task list or graph, not a wall of small terminals. Each task presents outcome, owner, current attempt, dependencies, artifact revision, validation status, attention needed, cost/resource summary, and the confidence/provenance of its status. Live terminals are a selectable inspection surface. The desktop permits a conventional pane layout; the initial host TUI favors a stable list/inspector and one application viewport, with virtual tabs. No frontend creates a wall of panes automatically.

A review packet answers: what was requested; what changed since the last review; what exact revision was checked; which evidence supports each claim; what failed or is unavailable; what decision is needed; what action would follow. Agent prose is shown as a claim with its source. A green exit code is not silently converted into acceptance.

Decisions are durable records: accept an artifact, request changes, choose a dependency version, allow an execution capability, or resolve a conflict. Artifact acceptance and permission to execute are different decision types, with different invalidation rules. A reviewer can accept a documented risk while the validation remains incomplete; the interface must not label that as a passed test.

Example: agents A and B independently produce revisions A1 and B1. The human reviews A1 with test evidence. A then creates A2. The previous review stays in history, but the current task shows A2 as unreviewed. Integrating A1 with B1 creates C1; tests on the individual inputs do not automatically validate C1. These rules belong to the [domain model](02-work-evidence-and-coordination.md).

## Attention is a constrained resource

Separate FYI activity from requests that block progress. The queue groups repeated failures by common cause and shows dependency impact, age, risk, expected review effort, evidence completeness, and resource/cost pressure. Priority combines user policy and observable impact; an agent's self-declared urgency is merely a claim. Users can pin an item, snooze it with an expiry, or redirect it to a different task.

Never make every completed command a notification. Default policy: one inbox item per material decision/context revision, deduplicated updates underneath, quiet summaries for successful delegated work, immediate interruption only for locally configured urgent events. A blocked task does not manufacture urgency by retrying. Batches require the same policy class and individually inspectable context; heterogeneous actions cannot hide behind one blanket approval.

Approval throughput is part of scheduling. When decision arrival exceeds service capacity, stop admitting more approval-producing work, preserve an emergency/control reserve, and show the backlog rather than lowering standards. Planning example, not measured behavior: 20 tasks/hour needing three minutes of review each consumes one full reviewer-hour. Fifty simultaneous agents cannot remove that limit. Measure decision arrivals, service time, queue age, repeated context switches, and accepted artifacts per human hour.

Policy templates may authorize bounded low-risk capabilities in advance. A template names scope, budget, expiry, runtime enforcement, and revocation behavior; it does not infer consent from a similar earlier command. All bulk decisions remain traceable to their individual action digests.

## Interaction and takeover journey

1. Start a shell immediately; task assignment is optional. The UI states host, environment profile, capture mode, and current input owner.
2. Create a task with acceptance conditions and attach a workspace snapshot or mutable directory. Delegate bounded attempts with resource and authority limits.
3. Watch outcomes and decision requests. Inspect an artifact or precise evidence span without losing task context.
4. Take control of a session using an explicit takeover action. Agent input queues are invalidated by a new ownership generation. Already delivered bytes cannot be recalled.
5. Investigate, interrupt, or modify the work. Hand back with a new context revision and concise human note. Agents must refresh before sending more input.
6. Accept a particular artifact revision, integrate it, and preserve or expire the evidence according to policy. Closing a terminal is separate from canceling its task or deleting its workspace.

Opening a pane never steals ownership. Stopping an agent does not imply stopping its descendants or undoing its effects. A persistent “revoke automation input” control is available independently of the focused pane; termination uses the execution lifecycle in document 03.

## Accessibility and human performance

All task, artifact, approval, ownership, and error actions must be keyboard reachable with visible focus and stable ordering. Status must use text and shape as well as color. Screen readers get structured task changes and selectable output summaries, not a live announcement of every terminal cell. Announcements are rate-limited and user-configurable; full output remains inspectable.

Support zoom, high contrast, reduced motion, remappable keys, IME composition, discoverable shortcuts, selectable text, and plain-text evidence exports. Keep application shortcuts distinguishable from keys forwarded to a TUI. The user chooses when an application shortcut takes precedence, and can escape terminal focus without sending control bytes. Dangerous actions never depend on interpreting colored terminal text as trusted chrome.

Task summaries must expose omissions. “Three failures grouped” expands to three identities; hidden stale evidence remains countable. Diff viewers show deleted lines, binary artifacts, unusual filenames, and changed symlinks safely. Do not bury authority or a changed target host behind a collapsed section.

## Product validation

Proposed study: within-subject comparison against ordinary panes plus an existing agent-capable terminal, using matched tasks and planted stale evidence, failed validations, and ambiguous TUI prompts. Measure time to correct review, missed material defects, false approvals, interruption recovery, and subjective workload. Agent count is an experimental variable, not the success metric.

Release gates include no silently actionable planted stale approval and keyboard/screen-reader completion of critical workflows. G5/NG3 uses a small formative sample to find blocking errors; it cannot establish population-level productivity or a statistical critical-miss rate. Predeclare task-specific completion thresholds and interventions before testing. A later comparative study must set adequate sample size/confidence and miss-rate bounds before claiming improved reviewed outcomes per human hour. See [current delivery/study gates](../engine/delivery-and-validation.md#ordered-milestones), [UX journeys](../ux/11-human-experience-spec.md#end-to-end-journeys-and-transition-evidence) and [technical gates](../engineering/07-validation-roadmap-and-risks.md).
