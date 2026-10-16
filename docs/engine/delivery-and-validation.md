# Delivery, validation, traceability and review

[Campaign](index.md) · [Research ledger](research.md) · [Native design](native-contract.md) · [Core](compatibility-and-core.md) · [Runtime](runtime-persistence-security.md) · [Human experience](frontend-contracts.md)

All new-engine capabilities below are **planned/not run**. This research phase validates documentation and the unchanged existing workspace, not a new engine. Evidence for this phase belongs under fresh `evidence/owned-engine-research/` paths. Historical X4/EA1 evidence remains immutable and failing.

## Requirement traceability

S IDs refer to the primary-source ledger; P means an explicit product choice rather than a standards guarantee. V IDs identify meaningful validation families defined below. This table traces requirements to design and falsifiable evidence, not merely headings.

| Requirement | Source basis | Design owner | Validation / release condition |
| --- | --- | --- | --- |
| R01 Agent-native operations, not only terminal bytes | S17–S19; P | [Native objects/operations](native-contract.md#minimal-operations-and-asynchronous-outcomes) | V01: managed attempt/result/artifact workflow without prompt parsing; native advantage demonstrated |
| R02 Identity, capabilities and causal evidence | S17/S18/S31/S32; P | [Authority](native-contract.md#objects-and-authority) | V02: wrong identity/epoch/context and payload-conflict traces rejected |
| R03 One writer, human override, no rollback claim | S03–S05; P | [Races](native-contract.md#observeactobserve-races-and-human-override) | V03: queued/partial/in-flight dispatch interleavings and explicit remaining effects |
| R04 PTY versus pipes, merged streams, job control | S01 partial follow-up N14-01; S02–S06 | [Execution boundary](compatibility-and-core.md#execution-boundary) | V04: real shell/foreground jobs/signals/EOF/drain and separate pipe stream cases |
| R05 Optional integration, no semantic extraction guarantee | S15/S16/S30; P | [Fact levels](native-contract.md#observation-and-evidence-schema) | V05: spoofed prompt/OSC/native-looking output grants nothing |
| R06 Profile/TERM/terminfo truth | S07–S10 | [Profiles](compatibility-and-core.md#profiles-and-negotiation) | V06: every advertised terminfo capability and query matches its fixture |
| R07 Incremental parser and unsupported/malformed bounds | S07/S08/S20; P | [FSM](compatibility-and-core.md#parser-and-deterministic-application) | V07: partitions/fuzz/caps/ignore states and independent byte-action oracles |
| R08 Complete buffers/margins/tabs/modes/continuation | S08/S10/H01 | [Owned state](compatibility-and-core.md#complete-semantic-state) | V08: hidden-state continuation corpus, not screenshot-only equality |
| R09 Unicode/grapheme/width/reflow/bidi limits | S11–S14/S21/S22; P | [Text profile](compatibility-and-core.md#unicode-reflow-and-rendering-boundary) | V09: pinned Unicode tests plus independent occupancy/edit/resize fixtures |
| R10 Mode-aware human/agent keyboard and text | S10/S23/H01; P | [Input](compatibility-and-core.md#input-policy) | V10: aliases, all named keys, modifiers/consumption, negotiation/reset and unsupported facts |
| R11 Single reply authority and effect suppression | S08/S10/H01; P | [Replies](compatibility-and-core.md#single-reply-authority) | V11: identical/split queries, two views, crash/replay and uncertain delivery |
| R12 Versioned complete snapshots and safe migration | S20/S26/H01; P | [Checkpoints](runtime-persistence-security.md#complete-snapshots-and-journal-cuts) | V12: arbitrary-cut restore continuation; corrupt/unknown versions refused; schema-change migration gate |
| R13 Capture/privacy/recovery boundary | S02/S26/S29; P | [Recovery matrix](runtime-persistence-security.md#precise-v1-recovery-contract) | V13: no-capture owner loss unknown; authorized complete-tail restore only; consent/revocation faults |
| R14 Acyclic Rust/module/unsafe boundary | S20/S27/S28; P | [Modules](runtime-persistence-security.md#module-and-process-design) | V14: import/unsafe/dependency/FD boundary checks; no hidden emulator authority |
| R15 Bounded fairness/backpressure and responsive humans | S06/S27; P + existing G6 | [Scheduling](runtime-persistence-security.md#ordering-fairness-and-budgets) | V15: all-process latency/memory/flood/storage/slow-client workload, p50/p95/p99 |
| R16 Same canonical model in both frontends | S23–S25; P | [Shared views](frontend-contracts.md#one-model-two-useful-clients) | V16: equivalent revisions/input receipts; no frontend parser/reply or private endpoint |
| R17 Native/legacy journeys, accessibility and attention | S23–S25/S30; P | [Journeys/states](frontend-contracts.md#native-to-legacy-journeys) | V17: zero-agent shell, work/review/takeover/recovery; real keyboard/IME/screen readers |
| R18 Hostile output, prompt injection, secrets and OS isolation | S06/S24/S29/S32; P | [Security/approval model](../architecture/04-isolation-security-and-approvals.md) and [core threats](runtime-persistence-security.md#threat-model-and-mitigations) | V18: output cannot authorize; sandbox negative tests; hostile snapshots/floods; explicit same-UID limits |

## Compatibility matrix

This is the **proposed published qualification matrix**, not a claim that any row now passes. Exact builds, locale, TERM/terminfo, fonts, keyboard layout and runtime profile are recorded for each tested tuple. “Candidate” means a program selected for testing, not installed/verified in this campaign.

| Area | N1 slice | v1 required candidate profiles | Later / non-goal |
| --- | --- | --- | --- |
| OS/backend | Local Linux dev tuple; record this VM independently | Ubuntu 24.04 LTS x86_64 target; verify supported updates and 26.04 separately; Wayland/X11 desktop profiles | macOS PTYs/permissions and Windows ConPTY/native handles are separate certification paths |
| Shell/line editing | One declared Bash/readline clean-rc fixture + no-agent use | Bash/readline defaults and Ctrl/Alt navigation, job control, completion; controlled rc fixture; POSIX shell smoke; optional adapter on/off | Zsh/fish profiles later; no shell/theme prerequisite or arbitrary rc guarantee |
| Interactive programs | Small declared profile; do not imply editor compatibility | less paging/search/resize; Vim normal/insert/command modes/save/alternate screen; Python REPL and a deterministic local agent-CLI fixture | Version-specific tmux/nested TUI/full agent products after explicit tests; no opaque widget semantics |
| Output/control | Basic private text profile | Modern text VT/SGR/scroll/margins/tabs/saved state/alternate screen + advertised queries | Graphics, printer/device emulation, all ECMA controls and legacy encodings excluded |
| Input | Named/basic legacy keys, text/paste subset with explicit refusals | Arrows/Tab/Enter/Escape/F-keys/keypad; Ctrl/Alt/Shift; cursor/keypad/mouse/focus/paste; selected Kitty/MOK levels including stack/reset behavior | Missing host facts or protocol variants refused/unadvertised; no universal physical-key parity |
| Unicode/text | Declared ASCII + tested UTF-8 subset, no Unicode18 conformance claim | Pinned data/segmentation/width profile, combining/CJK/emoji/ZWJ/variation and invalid bytes; documented logical-order bidi limit | Full bidirectional terminal editing/complex-script fidelity only after mapping/accessibility gates |
| Frontends | Both useful minimal clients; one viewport and receipt/takeover view | Full critical Terminal/Work/Review/History workflows; desktop IME/selection/search/accessibility and tested TUI host profiles | Fancy shaders/graphics/pixel parity not required; TERM=dumb linear read/control only |
| Sessions/recovery | Local live-owner attach/detach/reconnect | Service/UI/engine/keeper failures classified, opt-in exact-version state+tail; default capture off | No process resurrection, cross-version live migration or deterministic execution replay |
| Remote | No structured helper requirement | Local opaque `ssh`; TUI-over-SSH only on an explicitly supported preinstalled host tuple | Structured authenticated helper later; no implicit install/network-latency guarantee |

No application/OS/version is retrospectively “supported” merely because a reference engine handles it. Pin actual executable provenance and deterministic configs in the later harness. Readline/less/Vim are compatibility probes; their licenses must be respected without linking/copying their implementation into the product.

## Test and evidence strategy

**V01–V03 native correctness:** independent model-based operation/lease/receipt lifecycle traces; inject clocks, crashes and storage failure. A local deterministic client, not a paid model, runs a task with two concurrent argv attempts, separate streams, one artifact revision, cancellation and a conflicting human takeover. The human can identify which attempt produced which artifact and which effect remains unknown. Failure: native code still needs prompt regexes to associate results, duplicate IDs rerun uncertain effects, or a human must inspect a pane per attempt. Comparative measures may include context bytes/interaction steps and operator errors; no productivity/speed claim without a declared study.

**V04–V06 compatibility boundary:** real PTY and managed-pipe integration, foreground/background/stopped jobs, raw/cooked/echo/ISIG changes, VINTR, EOF/EIO, child exit before drain, descendants holding FDs, exec failure, shell exit/restart, resize order and TERM mismatch. Markers that look like prompts, task facts or approval dialogs must remain data. Terminfo/DA/capability advertisements are generated/reviewed against a finite capability table, with negative tests for unadvertised operations.

**V07–V11 core:** byte/Unicode/resize/key/query fixtures have a provenance/license manifest, source citation or explicit de facto policy, hand-authored expected hidden state and continuation, and independent review. Include all original X4 continuation inputs and legacy aliases as new-engine regressions **without modifying old fixtures or their failing candidate tests**. Compare against pinned xterm/Ghostty/VTE/parser sources and selected real terminals, classifying disagreement as standards requirement, deliberate profile difference, implementation bug or unresolved question. Majority output is not truth. Do not normalize away cursor/mode/reply differences to green a test.

Property/chunk tests apply every short split and seeded long partitions, including observations/snapshots mid-sequence, partial UTF-8 and ESC/ST, CSI subparameters, strings, zero-width/ZWJ clusters, wide-cell edits and resize. Model invariants cover occupancy, cursor/margins, bounded stacks/history, monotone frontiers, one committed state/effect cut and stale-input refusal. Fuzz byte streams, event interleavings, input DTOs and snapshot import with CPU/memory budgets; retain minimized sanitized failures. Differential tests can find errors in the test oracle too; review conflicts before changing expectations.

Keyboard cases cover ordinary Ctrl-I/M/[, actual Tab/Enter/Escape, Ctrl-C not implying exit, Ctrl/Alt/Shift combinations, physical/logical/consumed distinctions, IME commit versus preedit, release/repeat availability, application cursor/keypad, Kitty per-screen stacks, MOK levels/formats/precedence and reset. Agent and both human clients must converge on the same canonical intent semantics for their advertised capabilities. Include wrong focus/endpoint/session, stale screen/geometry, delayed output between observation/encoding/delivery, partial/non-idempotent input and retry attempts. A TUI may correctly refuse a chord the host cannot identify.

**V12–V14 recovery/architecture:** snapshot at every small event cut, continue both original/restored states with adversarial tails and compare full state/effects under suppression. This includes unsupported/overlong-string discard state and pending grapheme/UTF-8, both screens, saved cursor, tabs, margins, modes and stack state. Crash at journal/checkpoint/object publication, authority admission, dispatch and acknowledgement cuts; inject full storage, truncated/corrupt/oversized versions, lost tails and unavailable keepers. Verify no replayed replies/launches/grants and no guessed recovery in no-capture mode. Import/unsafe/FD checks keep frontend/store/worker/core boundaries enforceable.

**V15–V18 whole product:** run TUI-only, desktop-only and both; full process accounting and actual paint measurement on declared hardware. Mix interactive humans with output/query floods, blocked storage, expensive Unicode, snapshot/search, slow/disconnected clients and cancellation. Component/E2E tests cover keyboard, palette collisions, selection/copy/paste, alternate screens, contrast/fonts/DPI/resize and every failure state. Real screen-reader/IME/manual visual tests complement headless goldens; headless tests cannot certify paint or accessibility. Runtime profiles need filesystem/network/socket/credential/descendant negative tests before unattended restricted claims.

[CI0–CI4](../engineering/16-test-and-ci-strategy.md#ci-lanes-and-practical-cost) owns lane triggers, budgets and quarantine policy; [fixture/artifact policy](../engineering/16-test-and-ci-strategy.md#sanitized-fixtures-and-ci-artifact-security) owns provenance, retention and safe evidence handling. V families refine the inherited T/N/DF catalog into owned-engine obligations, not a parallel CI workflow.

Routine CI validates the [active-source foundation and governance](../engineering/16-test-and-ci-strategy.md#ci-artifact-diagnosis-and-lane-separation). Obsolete historical qualification jobs were removed in PR33 under owner approval; their original FAIL evidence, pins and manual reproduction paths remain. The native fixture checks below validate review data, not a terminal implementation. Future owned-engine CI must establish its own V/NG criteria; neither expected historical failures nor foundation PASS certify an engine.

## Ordered milestones

| Milestone / dependencies | Concrete deliverable | Success gate / stop condition |
| --- | --- | --- |
| N0 — contracts and harness; first | Reconcile existing domain/protocol/input gaps; own event/state model; finite dev-profile table; independent initial goldens; source/data/license pins and parser-cap policy | NG0: reviewed contracts and traceability; no hidden parser state or copied legacy authority. Timebox unresolved standards details; defer unadvertised features rather than expanding all VT. |
| N1 — useful vertical slice; N0 | Pure reducer + worker, one pipe attempt, one PTY shell, native operations/receipts, memory-only live reconnect, small terminal/task/takeover views in **both** clients | NG1: [J01](../ux/11-human-experience-spec.md#end-to-end-journeys-and-transition-evidence)/[NJ1](frontend-contracts.md#native-to-legacy-journeys) and takeover demonstration; original continuation/alias inputs pass for live canonical state/late-view attachment in the **owned** profile (durable restore remains NG4); actual native fact advantage and useful shell. Not full v1 compatibility/recovery. Stop if success requires prompt inference as authority or direct frontend PTY bypass. |
| N2 — useful classic profile; N1 | Core modern-text state/input, declared Unicode/data tailoring, independent terminfo, less/Vim/readline/REPL and agent-fixture navigation | NG2: published targeted application matrix and V04–V11; no essential navigation/unknown-mode failure hidden. Narrow a profile transparently or fix it before claiming it. |
| N3 — reliable supervised use; N1/N2 incrementally | Concurrent sessions, bounded capture/store/receipts, restricted runtime, dual-frontend review/a11y, fairness and fault handling | NG3: V01–V03/V14–V18, G5/G6 for declared release tuple; no full storage/slow view starving human control. N1 does not wait for all 32-session qualification. |
| N4 — complete-state recovery; state contract starts N0 | Versioned exact-tuple snapshots/journal, import validation, suppression and keeper reconciliation; no-capture loss UX | NG4: V12/V13 arbitrary-cut continuation and all crash cuts; refuse incompatible/gapped state. Keep recovery unadvertised if evidence is incomplete rather than blocking all live-terminal iteration. |
| N5 — broader scope; v1 gates as applicable | More apps/platforms, richer native adapters/transport/text, first schema migration and sustained conformance maintenance | Each addition has its own profile/source/cost/migration gate. No marketplace, graphics or fleet work until a concrete user need and owner budget exist. |

N2/N3/N4 can overlap with explicit prerequisites; a useful N1 is intentionally earlier than full v1. v1 certification requires NG0–NG4 for the features it advertises and both frontends' critical workflows. None is measured/implemented in this campaign. No old failed engine must be fixed to begin a separately authorized owned-engine slice.

### N0 starting queue and readiness

The initial GitHub plan was read back on **2026-10-16**: five N0–N4 milestones, 24 issues, 17 parent links and 45 blocking edges. The private Project has two Ready cards and 22 Backlog cards; Blocked is a separate initial planning field, not an automatically synchronized guarantee. No N0 deliverable is completed by creating tracking. The order below resolves the issue graph without adding another roadmap or changing remote planning.

| Order / issue | Prerequisites | Concrete exit evidence |
| --- | --- | --- |
| 1a — [N0-01 identities/facts](https://github.com/whoodes/namtab/issues/7) | Owner-approved contract-drafting scope; access to current design | Reviewed identity/event-cut examples, trust levels and valid/invalid native-operation fixtures; no prompt-derived attribution |
| 1b — [N0-04 finite profile](https://github.com/whoodes/namtab/issues/10), in parallel | Same drafting scope; bounded N1 use case | Capability table with exact syntax/defaults, replies, classic aliases, exclusions and truthful TERM plan; enhanced input unadvertised until specified |
| 2 — [N0-02 receipts](https://github.com/whoodes/namtab/issues/8), [N0-05 state](https://github.com/whoodes/namtab/issues/11), [N0-08 provenance](https://github.com/whoodes/namtab/issues/14) | Respectively N0-01; N0-04; N0-04 | Receipt/expiry/unknown transition table; complete bounded continuation inventory; exact admitted primitive/data/source/license tuple with explicit gaps |
| 3 — [N0-03 ownership](https://github.com/whoodes/namtab/issues/9) and [N0-07 core fixtures](https://github.com/whoodes/namtab/issues/13) | N0-01/02; N0-04/05 respectively | First-write/takeover/partial-delivery traces; independent state/input/query goldens and oracle-challenge plan. Core fixtures need no invented native schema |
| 4 — [N0-06 public observations/input](https://github.com/whoodes/namtab/issues/12) | N0-01/03/04 | Reviewed base/target/credit/resync examples and dual-client positive/refusal cases. Quoted Ctrl-G is a key intent or explicit refusal, never hidden control text |
| 5 — [N0-09 modules/IPC](https://github.com/whoodes/namtab/issues/15) | N0-02/05/06/08 | One cohesive proposed core/worker seam, failure/resource domain and import/authority checklist; preserve existing experimental namespaces |
| 6 — [N0-10 NG0 review](https://github.com/whoodes/namtab/issues/16) | All N0-01–09 outcomes; named owner/reviewer decision | Revision-bound R/V traceability, reviewed fixtures/schema examples, preservation checks, unresolved limits and explicit owner acceptance of the bounded N1 plan |

**Ready means eligible for scoped contract drafting, not ready to execute engine work.** These tasks still need a real owner, review availability and an authorized timebox. Existing synthetic named-key/schema/receipt checks do not satisfy NG0 or live frontend/PTY requirements. A passing audit PR is not N0 acceptance or N1 implementation permission. N0-07's core fixtures can proceed independently of N0-06; shared native/frontend fixtures must also be reviewed against N0-06 before NG0 closes.

Before NG0 acceptance, the owner must resolve staffing/review responsibility and timebox; the exact N1 feature/data/Unicode/cap tuple and smaller measurement workload; native wire/version/admission details; and dependency/data licensing admission. Project-license adoption remains separate and is not required to review private design work. Detailed options stay in the [remaining-decision register](#risk-register-and-remaining-decisions), not a second set of issue defaults. Configuration-version validation is required early; N5's broader migration backlog does not defer incompatible-version refusal or promise cross-version terminal restore.

## Staffing, expertise and effort

Planning ranges, **not measurements, commitments or vendor quotes**. Assumptions: Linux text-first scope above; experienced full-time Rust/systems and UI engineers; at least one terminal/job-control/Unicode specialist; reusable OS/Unicode/font/storage infrastructure; no new shell or universal graphics/bidi/platform support. Independent security/accessibility review and recruiting are additional calendar constraints. Team members share conformance/fuzz work; do not hide testing as free work.

| Outcome | Plausible staffing / calendar range | Interpretation |
| --- | --- | --- |
| N0 contract/harness/bootstrap | 2–3 engineers, 3–5 weeks | Ready-to-implement contracts and small fixtures, not a terminal release |
| N1 useful dual-client/native slice | 3–4 engineers, another 8–14 weeks | Approximately 2–4 months after N0; deliberately narrow compatibility and memory-only recovery |
| N2–N4 bounded Linux v1 | 4–6 engineers, roughly 9–18 months total from engineering start | Dedicated core/OS, frontend/a11y and test/recovery ownership; estimates include correctness work, not broad ecosystem maturity |
| Broad cross-platform/ecosystem quality | 5–8 engineers, roughly 18–36+ months total | Separate platform/renderer/integration owners; scope dominates range |
| Sustained ownership | Reserve 2–4 engineering FTE across core/conformance/security/frontend maintenance | More if profile/platform count grows; independent reviews still needed |

These new **whole-product** ranges differ from report 19's historical core-only adoption/fork estimates. They are not evidence that Rust is faster to implement or execute. A solo expert can build a narrower demonstrator, but has review, accessibility and sustained conformance bottlenecks; do not advertise the multi-frontends/recovery/security v1 range as a solo deadline. Missing expertise, full cross-version archival compatibility, graphics or broad bidi can exceed the upper range. Re-estimate after N1 with observed defect/corpus/renderer costs.

## Risk register and remaining decisions

| Risk / severity | Owner role and mitigation | Bounded question / trigger |
| --- | --- | --- |
| RE1 Compatibility grows without limit / high | Core/product: finite profiles, advertise only tested capabilities | N0 feature table; new app failure does not authorize implementing all xterm |
| RE2 Native model degenerates into a PTY API / high | Domain: pipe/structured work and exact evidence in NG1 | Stop slice if every workflow still requires prompt parsing |
| RE3 Hidden continuation or unsafe restore / high | Core/store: owned state inventory, arbitrary-cut tests, effect suppression | NG4 incomplete state → recovery remains unavailable |
| RE4 Two frontends duplicate modes/authority / high | Client/core: public projection, shared input fixtures, import checks | Reject client-side inner parser or independent reply generation |
| RE5 Unicode/font/width disagreement / high | Core/frontend: frozen data, occupancy cases, real host tests | Unusable editing blocks that host/profile; full bidi remains separate |
| RE6 Prompt injection/same-UID bypass / high | Security/runtime: explicit trust and enforced execution profile | No sandbox claim without negative tests; never fix with output markers |
| RE7 Floods/storage/snapshot stalls / high | Runtime/test: bounded quanta/caps/reserves, full-process stress | Human starvation or silent parser gaps fail NG3 |
| RE8 Receipt/delivery uncertainty / high | Service/keeper: durable reservation, epochs, no automatic retry | Missing acknowledgement must not become “not executed” |
| RE9 Team/test/maintenance cost / high | Product/maintainers: timeboxes, shared corpus, gate feature breadth | Two N1 checkpoints without usable shell/native benefit → reassess plan/staffing, not silently adopt a legacy authority |
| RE10 Dependency/source/license drift / medium-high | Dependency owner: exact pins/notices/data and upgrade corpus | Unreviewed corpus/font/code cannot be copied into product |
| RE11 Accessibility is deferred / high | Frontend/a11y: real early users and J01–J09/NJ1 tests | Critical inaccessible workflow blocks frontend qualification |
| RE12 No-capture mistaken for durable recovery / high | Store/UX: visible policy, separate failure boundaries | Owner loss without valid state/tail remains unknown |
| RE13 Workspace/shared Git/caches cross-contaminate attempts / high | Workspace/runtime: serialized administration, explicit immutable revisions, confined mounts | T1/T7 and sealed-input negative tests; watchers/worktrees are not isolation |
| RE14 Human review or notification backlog defeats scale / high | Product/scheduler: bounded inbox, exact-context batching, attention-aware admission | G5/NG3: review minutes, queue age and missed material context, not pane/agent count |
| RE15 Provider spend/delegation escapes limits / medium-high | Scheduler/adapters: parent ceilings, distinguish hard mediated limits from advisory reports | T10/NG3; unmediated CLI calls cannot acquire a fictitious hard spend cap |
| RE16 Update/migration destroys live work or evidence / high | Store/distribution: preflight/backup, compatible keeper lifecycle and explicit drain | T9/T11/T12 and CI4 package/logout/rollback checks; no automatic old-grant restore |

The sole current open-decision register follows. Closed for design: owned Rust semantics/FSM; no legacy core authority; both initial clients; native pipe + PTY paths; classic default with explicit enhancements; one reply owner; no-capture default; exact-version v1 restore only; one proposed cohesive new package; no stable plugin ABI. The [dated primitive selection](../engineering/15-dependencies-and-supply-chain.md#n0-primitive-and-data-selection-2026-10-16) supplies an exact finite draft tuple, not owner/engineering admission. Pending: admission of its feature/target/notice closure and limits, matching later Unicode18 algorithms/data; DOM painting viability; host width/IME/a11y; OS runtime enforcement/PTY lifecycle; detailed MOK variants; full checkpoint cost and G6. Funding/team assignments and the project license remain owner decisions. No new software or license is adopted here.

If a gate fails, first distinguish a test/oracle error, a profile difference and a real design defect. Fix or explicitly reduce an unadvertised scope. Never weaken an advertised critical workflow silently. If owned-state security/performance or sustainable ownership cannot be demonstrated within the declared funding/timebox, pause the affected release and make an explicit product/funding decision. Reconsidering ownership would require a new owner decision, not an engineer quietly embedding a legacy emulator.

## Second review and repairs

Performed in this campaign after drafting, separately challenging the proposed happy path:

| Challenge | Repair / remaining limit |
| --- | --- |
| Hidden legacy-first assumption | Native pipe/structured attempts need no terminal surface; arbitrary shell lines remain opaque. NG1 requires native evidence advantage. |
| Impossible semantic extraction | Typed observed/reported/inferred/unavailable facts; no readiness/command-success oracle from prompt text or TUI images. |
| Reconnect confused with restart | Live presentation attach, engine restore and OS process loss have separate contracts; N1 need not persist raw state. |
| Exactly-once or deterministic-execution implication | Receipt reservation is distinct from OS delivery; unknown/partial effects are not retried; deterministic reduction has explicit fixed-input/version assumptions. |
| Frontend state duplication | Only disposable presentation caches; no inner parser/mode authority; shape/paint cannot alter canonical widths. |
| Inherited attach wording still forwarded bytes | Updated execution spec to presentation snapshot plus **semantic deltas**, preserving one inner parser. |
| N1 fixture promise exceeded its tiny profile | Added saved cursor, vertical region, tabs and alternate buffer to the dev profile; NG1 tests live continuation/late attach, while durable snapshot restore remains NG4. |
| Revision fence implied an atomic application transaction | Explicitly documented unread-output/internal-widget races; exact backend cut is not atomic TUI state/delivery. |
| Full-state checkpoint handwave | Include discard/partial UTF-8/grapheme/parser state, keyboard stacks, both buffers and effect cuts; no hidden decoder object. No cross-version v1 promise. |
| Privacy contradiction | Checkpoints/partial parser/history are raw-equivalent; metadata excludes payload/content-derived hashes by default; outer recording/swap caveats stated. |
| Dedupe metadata retained secret argv or forgot expired IDs | Added volatile binding/default-safe metadata, explicit input-audit consent, namespace/sequence fencing and conservative refusal when old binding is unavailable. |
| Small-team abstraction explosion | One future core+worker package, existing service/domain/platform boundaries, built-in adapters, no plugin ABI or distributed event bus. |
| Perfection before use | N1 is useful at a narrow declared profile; full app coverage, 32-session target and durable recovery follow independently. |
| Old failures erased by direction change | Historical source/evidence/gates remain unchanged and red; new owned-engine acceptance is separately named and unimplemented. |

Residual uncertainty is concentrated in the explicit experiments above. Documentation/links/workspace verification establishes campaign coherence and preservation, not the new engine's runtime guarantees. Campaign command results and inspected-file hashes remain in `evidence/owned-engine-research/verification.json`. The later [consolidation review](../index.md#consolidation-review-2026-10-15) has separate evidence and does not rerun/rewrite these results; no external publication or provider CI run occurred.
