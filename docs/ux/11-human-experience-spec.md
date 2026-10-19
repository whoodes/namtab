# Human experience specification

> **Retained UX design; not full qualification.** [E5](../adr/0005-ai-first-harness-direction.md) owns current product scope. MK1 delivered a bounded interaction subset; this page owns proposed human behavior, not terminal semantics or runtime authority.

[Index](../index.md) · [Recommended baseline](../architecture/10-recommended-baseline-and-adrs.md) · [Navigation protocol](../engine/native-contract.md#observeactobserve-races-and-human-override) · [Prototype gates](../engineering/12-prototype-plan.md)

**Retained human interaction requirements and layout proposals; emulator form remains open under E5, with usability/accessibility/performance unqualified.** This specification describes human behavior alongside the agent contract. [E5](../adr/0005-ai-first-harness-direction.md) now specifies a custom emulator serving an AI-first harness; presentation form/renderer remain open. Electron is frozen as MK1 history; the host TUI is diagnostic/attach tooling. The retained layout/journey proposals below must be selected and tested for the eventual interface, not treated as a new framework decision. Shared task/review/authority behavior applies to admitted client profiles; window/font/DOM/clipboard examples below retain prior desktop rationale without selecting the new emulator stack. [13](13-dual-frontend-and-host-tui.md) owns the TUI mapping and honest capability differences. All examples are design scenarios, not observed experiments.

## Information architecture and default layout

With zero agents, an explicit terminal entry must remain useful through the broker. Existing-session attach must not create duplicate work. A steering entry exposes tasks, actual activity and exceptions without inventing an agent. Exact initial layout/entry behavior depends on the unresolved emulator presentation choice. No account, model provider, task creation, recording, shell hook, or container is required. Existing shell initialization runs under the user's authority; namtab does not rewrite it. A dismissible introduction explains the optional Work view. A small trusted status strip identifies host, runtime authority, input owner, capture policy, and live/history state. Do not start an agent or enable recording during onboarding.

One window selects a workspace. A workspace groups task records, artifact references, connection references, and terminal layouts; it is not itself a sandbox or a Git repository. Four lenses share selection without equating it with focus:

| Lens | Primary object | Default presentation |
| --- | --- | --- |
| Terminal | Sessions and their views | Tab strip, active terminal, optional user-created splits |
| Work | Tasks, groups, dependencies, active attempts | Stable grouped list with outcome, blocked reason, freshness, owner, and attention state |
| Review | Decisions and artifacts | Durable inbox plus selected review packet |
| History | Past events and available evidence | Searchable timeline with capture/expiry/gap labels |

Palette commands include Switch workspace and Go to task/group/agent, with escaped names plus stable identities. In Work, arrows traverse rows and expand/collapse groups, Enter opens the selected inspector, and Tab moves to its actions. These bindings apply only when the list owns focus, never inside a terminal.

Agents are a filter/inspector dimension, not the top-level container for all work. A task can outlive an agent and have several attempts. Groups are user-named task collections; dependency edges remain explicit. The inspector shows one selected task's attempts, artifacts, decisions and session links. A session drawer includes detached sessions. Selecting a task does not send input or open ten terminals.

```text
Zero agents
┌ Workspace: local ▾   Terminal | Work | Review(0) | History     Palette ┐
│ [shell 1] [+]                                                        │
│                                                                      │
│                         ordinary terminal                            │
│                                                                      │
├ local / host authority | You own input | Capture: memory | Live ──────┤
└ 120×36 · font/zoom · session status                                    ┘

Many tasks, one selected review
┌ Workspace: app ▾      Terminal | Work | Review(3) | History            ┐
│ Group/filter/search       Tasks                         Inspector      │
│ Build [8]            ▸ compile       running          outcome/attempt │
│ Investigate [12]     ▸ parser case   needs review      exact revision  │
│ Waiting [5]          ▸ docs          blocked: build    evidence/risks  │
│                      ▸ test         unknown           [Open terminal]│
├ Inbox: 2 approvals · 1 artifact changed       [Revoke automation…] ───┤
└ No automatic pane creation. Counts include off-screen work.           ┘
```

Progressive disclosure is outcome → artifact/diff and evidence → process/output detail. Unknown states remain visible in summaries. Large lists use stable ordering while keyboard focus or a review is active; new arrivals increment a count rather than move the row under the user. Offer explicit refresh/re-sort. Four live preview surfaces is the initial preview budget, not an arbitrary ban on additional user-created splits. More visible terminals consume the measured rendering budget.

## Three action planes and keyboard behavior

1. **Terminal UI commands** change tabs, panes, search, selection, zoom, or the inspector. They never become application bytes.
2. **Application input** includes logical keys, committed text, paste, mouse reports and explicit raw fallback, all passing through the same ownership/admission boundary as agent requests.
3. **Automation API** addresses typed session/task/view identities and capabilities. It does not type shortcuts into the desktop or bypass admission.

The following **desktop** keymap is a proposal to validate on Linux, not a universal convention. The host TUI instead has a configurable Ctrl+G prefix/quote scheme and a Send key palette because outer terminals may alias modifiers; [nested-input rules](13-dual-frontend-and-host-tui.md#input-modes-and-nesting) preserve the same action separation:

| Chord / context | Proposed behavior |
| --- | --- |
| Ctrl+Shift+P / T / N | Palette / new terminal tab / new window |
| Ctrl+Shift+C / V | Copy terminal selection / paste through the broker |
| Ctrl+Shift+F | Search; active session first, explicit workspace scope |
| Ctrl+Shift+W | Detach this view; session continues unless separately stopped |
| Ctrl+Shift+Space | Move focus from terminal to trusted chrome; remappable for desktop/IME collisions |
| Tab, arrows, Escape, function keys, Ctrl+C in terminal | Application input according to negotiated modes; Ctrl+C is not termination proof |
| Tab/Shift+Tab and F6 in chrome | Move among controls / major UI regions; F6 is not intercepted inside an application |
| Escape in palette/dialog | Close that UI layer only; no Escape byte or subsequent Enter release leaks into the application |

The palette exposes the effective shortcut, scope, target, and effect category. “Send next chord to application” bypasses one product shortcut through the ordinary broker; it cancels on focus/ownership changes and is never a sticky capture of future input. This permits applications that use Ctrl+Shift combinations. OS-reserved shortcuts cannot always be intercepted: show the collision and provide remapping. During IME composition, composition keys belong to the input method; committing composed text is distinct from forwarding every physical key. Do not let a global product shortcut destroy composition.

Selection, keyboard focus, current task, terminal foreground job and input ownership are independent. Trusted chrome names the session and writer, and separates controller authority from child runtime/workspace authority. Focusing an agent-owned pane does not take control; first attempted typing is blocked and offers **Take input**, with the attempted keys discarded rather than replayed after takeover. An agent may address its owned background session without changing human focus. Inspecting scrollback or another task never transfers a lease. Modal dismissal restores focus only to the same still-valid target; otherwise choose chrome.

For agent keys, retain [observe–act–observe](../engine/native-contract.md#observeactobserve-races-and-human-override), session/incarnation and mode generations, freshness checks, structured-key primary/raw fallback, and partial/unknown receipts. Human UI focus is not a substitute for those fields. Never retry a key, paste, click or approval because a spinner timed out. Opaque applications expose screen transitions, not universal semantic selection guarantees.

## Rendering, selection, paste and scrollback

For the selected emulator presentation, evaluate a system monospace default with explicit font selection/fallback, ligatures off initially, per-window zoom and configurable line height. Font shaping must not change the canonical cell-width interpretation. Test combining sequences, emoji/ZWJ, ambiguous widths, CJK, bidi text, fallback glyphs and mixed DPI; a correct byte stream does not guarantee a readable screen. Keep terminal theme/ANSI palette separate from chrome. Chrome targets WCAG 2.2 AA text contrast (4.5:1 for ordinary text); offer high contrast, reduced motion and cursor-blink control. An optional readable terminal palette visibly changes presentation, not evidence bytes or the program's original color choices. This is a target, not a compliance certification. [WCAG contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)

[Frontend integration](../engine/frontend-contracts.md#text-rendering-selection-and-accessibility) owns semantic rendering and accessibility requirements. The DOM-first recommendation is historical; H0/E5 leaves the renderer open. Admitted clients render canonical frames, never parse inner output. Stale frames disable dependent coordinates; terminal titles/links cannot alter trusted chrome.

Desktop normal drag selects terminal text unless the application has negotiated mouse reporting. Provide a documented remappable modifier to force local selection. Local terminal selection is different from an editor's internal selection. Copy joins known soft wraps, preserves hard line breaks, and copies text without terminal escapes. Empty selection yields a UI hint, not Ctrl+C. No automatic copying or clipboard reading. Opening an output hyperlink requires a visible destination and explicit action, with schemes restricted by policy.

Paste is a typed operation with the exact target and payload length. Bracketed-paste mode is encoded only when active; it does not prove an application will avoid execution. Single-line printable human paste can follow an explicit user preference. Multiline, control-bearing, or trailing-newline paste gets a preview with **Paste** and **Cancel**; transformations are visible, never silent trimming. Paste does not add Enter. Password entry is not saved in paste history or ordinary request metadata. Automated sensitive input requires a separately authorized secret handle; a visually suspected password prompt is not a safe inference. Input can be partially delivered; the UI presents that outcome for inspection rather than resending.

Active-session search covers the current bounded live buffer and scrollback. Workspace/history search covers only explicitly captured, retained material; “not captured,” “expired,” “gap,” and “zero matches in searched range” are distinct results. Regex is opt-in, bounded and cancellable. Search hits retain session/prefix or evidence references and are read-only until the user returns to live input. Output must not yank a scrolled reader to the bottom; indicate new output. Ordinary typing may return the active live terminal to its bottom after a clear indication. “Clear view,” “purge captured data,” and application shell-history management are different actions. A clear-screen escape is not a retention deletion request.

## Tabs, splits, windows and geometry

The host TUI initially uses virtual tabs and one primary viewport; it creates no OS window and does not control the host emulator’s tabs. Desktop tabs contain view layouts, splits show sessions, and sessions survive view closure according to [lifecycle policy](../architecture/10-recommended-baseline-and-adrs.md#adr-a4-linux-and-eventual-desktop-ports). No new pane is created just because an agent appears. Closing a view detaches it and offers a session-drawer link; stopping work is a separate explicit command. On last-window quit, show active-work count and choices **Keep running**, **Stop owned work and quit**, **Cancel**. Stop may remain pending/unknown; never promise all descendants or external effects ended.

Two windows may inspect one session. One view holds sizing authority; mirrors letterbox/scroll rather than repeatedly resizing the application. Changing sizing authority generates a new geometry revision. Resize is coalesced to the latest dimensions but applied in ordered parser/PTY events; invalidate affected observations, selections and queued coordinate actions. Resizing an alternate screen, IME composition or a scrolled selection cannot silently retarget input. Multi-window focus has a stable view identity plus generation, not just an index or title.

## Attention, review and approvals

The durable inbox groups attention by task/decision revision, with blocked cause, age, consequence and evidence availability. Ordinary completion is quiet by default. No focus stealing. The TUI uses a bounded inbox/status aggregate without unsolicited host bells/OSC notifications. Optional desktop notifications omit terminal content and secret-bearing titles by default; sounds off. Initial toast budget: at most three per minute, then an aggregate counter. Critical locally detected service failures use an aggregated persistent banner rather than an unlimited toast exception. Rate limits suppress interruption, not durable decisions. Allow per-group subscriptions and a quiet period. Show approval queue age and pause new decision-generating admissions when the configured attention budget is exhausted; existing processes need explicit policies, not imaginary pause semantics.

```text
Review: task parser case / decision revision 7
Outcome reported by agent: handles malformed input       [untrusted claim]
Artifact: patch digest …  Source: workspace snapshot …    [Open diff]
Validation: runner-observed / initial snapshot / output available
Current workspace differs: 2 files                       [Compare]
Action: execute validator in offline runtime, no network
Grant: this action + these revisions, one use, expires …
[Deny] [Request changes]                     [Allow once]
```

A review packet always separates reported summary, observed execution, enforced controls, artifact revision and current integration revision. A passing earlier check is not a current-tree verdict. “Current workspace” names the last observed manifest and time; a mutable host directory can change outside mediated control, so the UI cannot promise that a watcher observed every write. Stronger binding requires the sealed-input profile. Accepting an artifact is different from allowing execution; integration produces a new revision requiring its own checks. Expired or unavailable evidence stays visible. Reviewer comments bind to the reviewed revision.

Approval controls live in trusted chrome, never an application-drawn prompt. Opening a request focuses its heading, with no default Allow button and no global Y/Enter acceptance. Show concrete action, host/runtime, capabilities, capture/export, target revisions, expiry, unknowns and the breadth of a host-shell grant. Context changes disable the old Allow control and require reviewing changed fields; do not silently update a button's meaning. Dismiss leaves a request pending; expiry is not consent. A cooperative CLI may ask for approval through an adapter, but printed markers cannot grant authority. Password prompts use human direct input or scoped secret handles, never clipboard/screen scraping or generic approval shortcuts.

## Takeover, resume and recovery states

**Session input takeover** fences the old writer and drops its queued input; the new human lease does not stop the foreground job. **Task takeover** also closes future mediated tool/workspace admission and requests cancellation of active attempts. Present these separately:

```text
Task control: You
Future agent input/tools: revoked
Active attempts: stop requested (2), observed exited (1)
Other writers / remote effects: unresolved
[Inspect attempts] [Escalate stop…] [Record reconciliation]
```

Do not label this “all paused.” Stop escalation names the process/group/runtime and possible hangup/data-loss effects. A signal request, even Ctrl+C, is not exit evidence. Resume requires a new lease, fresh observations, explicitly selected runtime/capabilities and a handoff note identifying human edits; stale approval and input queues are never replayed. An ordinary view focus change is not task takeover.

| State / trigger | Human presentation and allowed next action |
| --- | --- |
| Empty ordinary workspace | Ready shell; optional Work introduction; no agents synthesized |
| Empty Work/Review | “No tasks” / “No decisions waiting”; create task or return to terminal |
| Loading/reconnecting | Last observed timestamp, pending read progress, cancel/view history; input disabled until ownership/state reconciled |
| Process exited, output draining | Show observed exit separately from remaining stream/descendant facts; bounded wait or explicit loss policy, never a completed-task inference |
| Confirmed launch failure | Specific failure and safe preparation actions; a new launch is explicit |
| Degraded output/GPU/disk | Named lost capability/range, useful surviving actions; no green complete-evidence badge |
| Unknown action outcome | “Delivery/outcome unknown”; inspect receipt/process/artifact; no generic Retry action |
| UI renderer crash | Reopen and reattach to surviving keeper/canonical engine; show gap if any |
| Supervisor restart | Reconcile keeper identities; invalidate old grants/leases; no automatic re-execution |
| Canonical engine lost | PTY may survive, interpretation unknown; restore only valid complete state/prefix or explicitly start a new session |
| Keeper lost | PTY/session loss and descendant uncertainty; inspect separately; no automatic duplicate launch |
| SSH disconnect | Last observed host/session and time; ordinary remote effects unknown; helper mode may reconcile documented receipts |
| Host reboot | Historical attempts interrupted/unknown; new attempt is a new identity, not resumed execution |

Recovery uses the same evidence distinctions as normal operation. A text inspector is a degraded inspection aid, not a substitute for a working accessible terminal or a valid full parser state. Capture-off sessions cannot promise state recovery from disk. A stale replica may remain available read-only with its timestamp. After canonical loss, application input stays disabled until valid state is restored; raw fallback is not an uncertainty bypass. Process inspection and separately authorized lifecycle controls remain available, and the user can create a new session. Already delivered input/effects remain unknown. A terminal reset cannot recover a TUI or its prior screen. Arbitrary human raw-mode recovery is deferred rather than introducing another bypass in the first release. Disk-full recovery must not issue durable grants it cannot record; emergency human control is visibly unaudited if only memory remains.

## Onboarding, migration and portable configuration

Start with an ordinary shell and a visible host-authority label. Agent setup is a later explicit flow: choose controller, workspace, runtime, available image, permissions and capture/export separately; show missing prerequisites without installing/pulling anything implicitly. Existing terminal preferences may be imported only through an allowlisted preview of font/colors/keybindings; shell hooks, credentials, startup commands, host trust and arbitrary terminal config expressions are excluded. Do not read another application's private session storage to discover sessions. Existing tmux/SSH work can be attached as an ordinary explicitly selected command with honest observation limits.

Versioned TOML contains portable presentation, shortcuts and named references, with machine-local connection paths/credentials resolved separately. Precedence: built-in defaults → user portable preferences → explicit machine-local overrides → validated harmless workspace display hints. Show each effective setting's source; project data never overrides permission, capture or executable-hook policy. Export excludes secrets, live leases and approval grants. Invalid settings show line/location and retain the last known valid configuration. Schema migrations preserve a backup; rollback may need an exported compatible configuration, not unsafe database downgrade. Redact previews without hiding which authority changed.

## Accessibility and performance behavior

Desktop chrome controls have semantic names/roles; all admitted clients require logical order and visible focus. Full-screen TUI cells do not provide a native accessibility tree. Test actual host/screen-reader pairs and a linear review/reading mode without assuming a text log proves editor accessibility. Tables expose stable row identity and status text; provide a linear accessible view if virtualization breaks assistive navigation. Use established widget keyboard conventions rather than inventing terminal-only navigation for review forms. [WAI-ARIA patterns](https://www.w3.org/WAI/ARIA/apg/patterns/)

Terminal screen-reader mode and browser accessibility are starting points, not proof. The successor presentation gate must test the selected interface in its declared profiles, including Linux Orca, full keyboard use, IME composition, zoom, high contrast and reduced motion on the actual package. Keep a reliable keyboard exit from terminal to chrome. Announce task/decision changes in bounded summaries; do not flood a live region with every terminal cell. Users can request output reading or silence it. Preserve an accessible representation of provenance/unknown states; color, animation and icons alone cannot encode them. Future VoiceOver/NVDA gates are platform-specific.

Use the [G6 numerical baseline](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets) and [all-process method](../engineering/16-test-and-ci-strategy.md#all-process-performance), including real outer-host paint. Hidden sessions still parse; lag exposes the processed frontier and blocks stale actions. Resize/selection/IME and human control must remain usable under floods. No benchmark has qualified this UX.

## End-to-end journeys and transition evidence

| Journey | Concrete sequence | Required transition evidence / failure handling |
| --- | --- | --- |
| J01 — Zero-agent daily terminal | Open shell → run pager → navigate/search → quit → edit file → select/copy → reopen scrollback hit → return live | Application keys stay application input; no account/recording; pager/editor modes and ownership correct; search history visibly read-only |
| J02 — Delegate a group | Create eight tasks → choose offline tool profile and capture policy → inspect dependencies → open Work → review one blocked item | Group counts match task states; no eight-pane explosion; attention remains bounded; grant/capture/runtime are separate choices |
| J03 — Review changed work | Open reported success → inspect exact diff/check → detect subsequent edit → request new validation → integrate | Old verdict stays tied to old revision; unavailable output labeled; integration has new identity |
| J04 — Stale or lost approval | Review revision 7 → target changes to 8 → old button disables → review 8 → connection drops after action reservation | No old grant dispatch; receipt reconciliation, no blind retry/Enter acceptance; unknown stays unknown |
| J05 — Human takeover | Agent owns editor → human focuses pane → Take input → edit → inspect still-running background job → resume with note | Old writer fenced, queued keys discarded; session takeover does not claim background work stopped; new agent observes before acting |
| J06 — Reopen after failure | Close window → reopen same session → crash engine → inspect recovery state | GUI loss can preserve execution; engine loss cannot invent canonical state; captured versus unrecorded recovery differs |
| J07 — Sensitive interactive session | Explicit SSH session → trusted host label → password input → Unicode/IME REPL → Ctrl+C → inspect process | No prompt text grants authority; secrets absent from metadata/history; IME not mangled; interruption request is not exit proof |
| J08 — Move configuration | Export theme/keybindings → import on another machine → resolve named runtime → open shell | No transferred grants/secrets, no auto-started agents, unsupported settings explained and recoverable |
| J09 — Switch frontend | Review in TUI → attach desktop read-only → explicitly take input → return to TUI and detach over SSH | Same task/evidence revisions, one client-bound writer, no replay of blocked keys, one size authority; outer TERM/prefix/retention limits visible |

These retained journeys apply to selected client profiles with declared physical/presentation capabilities, and are refined by DF01–DF08/G8. They supplement N01–N16, not replace them. In particular, menus, pagers, editors, REPLs, agent CLIs, wrong focus, stale observations, output floods, keypad/cursor modes, bracketed paste and Escape ambiguity remain release gates. Human success also requires recognizing which revision was checked, who controls input and whether effects may continue. Those are explicit G5/NG3 study tasks, not merely screenshot review.
