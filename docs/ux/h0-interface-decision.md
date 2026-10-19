# H0 custom emulator presentation analysis

**Recommended direction for owner review, 2026-10-19; no interface/framework selected.** Supports [issue47](https://github.com/whoodes/namtab/issues/47). [E5](../adr/0005-ai-first-harness-direction.md) already chooses the AI-first owned-engine harness and excludes Electron. This page owns the presentation-form comparison and feasibility proposal. [Human UX](11-human-experience-spec.md) owns reusable interaction requirements; [frontend contracts](../engine/frontend-contracts.md) own canonical projection/input semantics. No runtime, renderer or dependency implementation follows this draft.

## Recommendation and alternatives

Recommend a **local native-window presentation feasibility study**, after a separate owner choice, with the engine/harness independently usable through its public contract. It best fits a custom emulator without Electron while preserving direct keyboard/IME, focused terminal rendering and human steering. This remains a **proposal**, not an interpretation that the user already chose a native window. Keep the existing host TUI as diagnostic/attach tooling; do not turn the study into two mandatory product frontends.

| Form | AI-first harness and human strengths | Costs / limitations | Disposition |
| --- | --- | --- | --- |
| Native window, canonical cell/text rendering and semantic controls | Direct window/input lifecycle; integrated steering, selected task/activity and classic terminal; can avoid browser runtime | Must own or integrate shaping/font fallback, selection, accessibility tree, IME and platform lifecycle; low-level window/GPU libraries do not supply these | Recommended feasibility candidate; renderer/toolkit **pending** |
| Host-terminal TUI as sole interface | Existing code, SSH and keyboard familiarity; no graphical session prerequisite | Outer terminal owns fonts, input decoding, physical/layout distinctions, painting and accessibility. Nested Escape/prefix/mouse/paste collisions cannot be recovered by an API; cannot prove standalone-emulator feel | Retain diagnostic/attach path; not recommended as complete target without an explicit reduced-form owner decision |
| Local browser presentation without Electron | Browser text/IME/accessibility primitives and potential remote view | Requires reviewed authenticated bridge/origin/CSRF/session policy or another native host; browser tab/focus/clipboard and process lifecycle constraints remain. A browser page is not automatically a native terminal | Fallback proposal only; no loopback server, new permissions or Electron substitution authorized |
| Headless harness only | Best independent engine/agent contract and measurement surface | No satisfactory hands-on terminal or human steering by itself | Valid intermediate H1 engineering boundary, insufficient H2 product outcome |

Do not pick a renderer by assuming Rust is faster. A mature integrated Rust UI toolkit may reduce custom widget/accessibility work; low-level windowing plus custom rendering offers terminal control but expands maintenance. [winit](https://github.com/rust-windowing/winit) handles window/events and requires other rendering facilities. [AccessKit](https://github.com/AccessKit/accesskit) supplies accessibility infrastructure, not an automatically correct terminal reading model; its documentation identifies platform/feature limits. These are examples to evaluate, **not selected dependencies or a required stack**; the [dated source ledger](../engine/research.md#h0-focused-source-review-2026-10-19) records revisions and evidence limits. No framework comparison result or package license admission is claimed.

## A harness view, not a wall of terminals

The default populated view should answer: what is running, what changed, what needs my decision, and what does my next action control? Group by task/outcome; expose agents/runs as identities and filters. Do not require a pane per connected agent. Hide raw protocol/evidence/debug views behind deliberate inspection, without hiding uncertainty or errors.

```text
namtab / workspace                         Decisions 2   Control: observing
[ Steer selected task or admitted agent… ] [Review target / Send]
Task / run              Observed activity         Exception / next step
parser / RA             execution E7 draining     review artifact R3
input / RB              waiting for R3 validation dependency unchanged
Selected: parser@rev1 / attempt A7 / context C2
[progress and exact artifact / decision summary]
[Open activity window] [Review evidence] [Take session input…]
```

This is a design wireframe, not a screenshot or a shipped model prompt. With no admitted model adapter, label the field **Draft task/steering request — model delivery unavailable**; allow a local contract draft, not pretend a prompt invokes an autonomous agent. With an admitted adapter, show exact selected task/run/context and capabilities before delivery. Accepted steering, adapter acknowledgement and the eventual observed result are distinct. Editing a task creates a revision; it does not silently rebind running attempts. A request to a running agent is a bounded message, not terminal keystrokes or forced reasoning.

One selected activity window can show a canonical terminal, managed output, artifact or review packet. Keep trusted task/run/session/owner/capture/gap chrome outside hostile output. On explicit zero-agent terminal entry, ordinary local shell use remains possible within the admitted profile; no task/model/account required. Empty task lists explain that no agent exists rather than showing fabricated idle agents. A launch/admission failure is actionable without opening a debug dump.

## Three planes and ownership

| Plane | Exact route and feedback |
| --- | --- |
| Harness/UI actions | Switch task/session, open activity/evidence, request takeover, review/send steering. Typed public control operations; no application bytes. Selecting/focusing does not acquire a writer |
| Application input | Explicit session/incarnation plus current writer/observation/profile/focus/geometry bindings. Canonical mode-aware keys, committed text, paste, mouse/focus where admitted. No private PTY handle, renderer parser or automatic query replies |
| Agent automation | Capability-bound native operations and observations, independent of human viewport focus. Adapter events are attributed reports; broker/process facts and human decisions have their own trust |

Maintain an explicit focus target and generation for view-derived input. Session switch, window focus loss, stale mapping, reconnect or refused/unknown input discards queued tails and requires deliberate inspection/re-arm; no automatic resend. Agent direct-session input does not steal human focus. A protected prompt routes through trusted human handoff policy; Enter/paste in app content cannot approve a broker grant. Ctrl-C is input, not guaranteed termination. Takeover fences future mediated writer effects, not all active tasks or external side effects.

Retain arrows, completion/Tab, modifiers/shortcuts, function keys, selection/paste and interactive CLI/TUI/REPL/editor navigation for each advertised profile. Prefer structured logical key intents with known physical/layout/IME facts only when available; separately granted raw bytes never bypass mode/unknown/protection/ownership fences. Native-window events still require consumed-modifier, layout, composition, repeat/release and Escape handling tests. Opaque TUIs have no universal semantic navigation guarantee.

## Concrete journeys and states

| Journey | Required transition evidence / falsifier |
| --- | --- |
| Two runs, one dependency | Human sees RA/RB and exact task revisions, managed execution origins and R3 validation requirement. RB waiting is a recorded dependency state, not inferred from an unchanged screen. New R4 makes the selection explicit, not silently current |
| Intervene during output flood | Human opens one activity window, observes its explicit session and requests writer takeover while another session emits output. Old-generation input refuses at final dispatch; the other job need not stop. Input/control latency and visible paint measured separately |
| Native work to shell | Select artifact/context; explicitly open a new PTY there under its runtime grant. A pipe process cannot become a PTY. Edit creates a new artifact revision; old validation is shown stale |
| Disconnect/reconnect | Disconnected view shows last cut, no input authority and unresolved submitted operations. Reconnect fetches current authorized state and queries receipts, not replay. Worker/engine loss is distinct from frontend loss; capture off cannot promise restore |
| No model / protected / unavailable | No adapter: task draft only. Known protected target: dedicated fresh human context. Unknown interpretation: refuse dependent application input and offer inspection/lifecycle choices. Unavailable evidence is neither success nor failure |

Progressive disclosure must leave current selection, factual status, blockers, decision count and ownership visible. The review packet selects exact task/artifact/validation/context revisions, explains staleness/gaps and keeps permission approval distinct from task acceptance. Batch decisions require equivalent review context and individual dispositions; a notification is not durable review. Bound notification rate and inbox age, include missed-material-context tests, and never auto-accept to reduce attention load.

## Falsifiable interface feasibility gate

After the owner selects a form, compare a small number of credible stack candidates using the **same canonical frames and human journeys**, not polished mockups. Pin dependencies/features/targets/notices before executing them. Reuse existing core/public-client seams; no second mutable terminal state or premature public renderer/plugin ABI. Record implementation/maintenance size and platform limitations alongside measurements.

1. **Real input/paint:** launch the selected presentation on an actual available graphical host if graphical; no headless playback substitution. Type shell commands, completion/history/navigation and repeated/interrupted input; switch task/session, lose/regain focus, reconnect and take over. Verify no wrong-target/stale/queued-suffix effects. Save actual screenshots and source/build/host bindings.
2. **Text and accessibility:** canonical cell occupancy, saved cursor/both buffers, resize/wrap behavior, font fallback/combining/wide glyphs within the advertised profile; correct selection/search/copy provenance, bracketed paste and clipboard consent. Test actual IME and screen-reader/OS combination on critical steering/review/terminal paths, not merely an accessibility API's presence. No feature advertised if its gate fails.
3. **Resource and responsiveness:** apply the [H0 workload](../engine/h0-harness-contracts.md#workload-populations-and-measurement-proposal), measure all-process input→paint/control/observation p50/p95/p99 and memory under floods/slow readers. No model work or snapshot/hash/render pass may monopolize input/control. G6-derived numbers remain proposals until admitted.
4. **Lifetime/security:** malicious titles/OSC/links/bidi cannot alter trusted controls or trigger grants/clipboard; no UI/private-store/PTY authority. UI restart/attach cannot create a second reply owner, resize owner or grant. Runtime containment is separately qualified, not earned by native rendering.
5. **Switch/stop:** reject a candidate if it needs a hidden parser/mode cache, unsafe byte bypass, unavailable critical input/IME/a11y, unbounded state, unmaintainable private APIs or a new security permission outside admission. Report the exact failure and consider another candidate/form with owner review; do not resurrect Electron or silently weaken acceptance.

No candidate is implementation-proven by this document. Host TUI may still be useful over SSH, but outer TERM/terminfo/colors/fonts/Unicode/key decoding and tmux/SSH prefixes remain endpoint limits. Backend decoding cannot recover facts that the host never transmitted. Remote helper/graphical forwarding is not selected or configured.

## Owner decision packet

Recommend approving **external-agent-first native integration**, **independent headless H1 work after contract/resource admission**, and **native-window feasibility as the first presentation study**. That package does not select a renderer/toolkit, provider, exact agent count, legal/license terms, paid service or security policy. Alternatives remain documented above. Issue47 remains open until the owner records the form decision or an explicit blocker; H2 selected-emulator acceptance depends on actual feasibility evidence. Merging this analysis alone would accept documents, not grant runtime implementation or earn that gate.
