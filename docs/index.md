# Project X: an agent-first terminal

Start with the [draft manifesto](product/manifesto.md) and [contributor workflow](engineering/contributing.md) for product priorities and reviewable changes.

> Current gate: [EA1 worker results](engineering/21-ea1-native-worker-results.md) fail required legacy control aliases; app work is stopped.

> Prior engine decision: the bounded [BUILD/ADOPT/REDESIGN assessment](engineering/19-engine-build-adopt-redesign-assessment.md) recommends official libghostty-vt for the next prototype, with owned semantic/cell rendering in both frontends. The original xterm serializer X4 remains FAIL; bounded A/B proofs do not certify a product. [ADR E1](adr/0001-engine-ownership-and-presentation.md) and [ADR E2](adr/0002-terminal-continuity-and-checkpoints.md) supersede the former replica/language choice, preserving product requirements. No dependent application implementation was started.


Design baseline: 2026-10-15. Status: reviewed recommended baseline; accepted-for-design decisions remain unimplemented and measurement-gated. Linux is the first platform. The product has two initial frontends: a host-terminal TUI and a standalone Electron desktop emulator, sharing a durable local supervisor and automation interfaces. Its success depends on the human interface as much as process management.

Project X should let a person supervise many agents through tasks, changed artifacts, decisions, and inspectable execution evidence while continuing to use ordinary shells and terminal applications. Existing terminal products already offer agent orchestration, notifications, APIs, and approval workflows. Our proposed advantage is a more coherent relationship between work, evidence, validation freshness, and human authority; this is a product hypothesis to test, not a novelty claim.

**Agentic terminal navigation is a core feature.** Agents must be able to operate existing interactive shells, CLIs, TUIs, REPLs, and editors through arrows, Tab/completion, shortcuts/modifiers, control sequences, function keys, and selection/paste. A session-bound [observe-act-observe contract](architecture/03-execution-terminal-and-remote.md#observe-act-observe-and-transition-evidence) separates terminal UI commands from application input and makes uncertain transitions inspectable. Structured keys are the primary interface; explicit raw bytes provide a controlled compatibility fallback. Neither interface promises universal semantic navigation of an opaque application. See the [API boundary](architecture/05-architecture-storage-and-extensions.md#navigation-api-boundary) and [acceptance cases](engineering/07-validation-roadmap-and-risks.md#agentic-navigation-acceptance-cases).

Local commit order and per-stage checks: [history record](engineering/22-local-history-and-validation.md).

Current workspace organization and historical evidence boundaries: [cleanup record](engineering/20-workspace-organization.md).

## Read the design

| Document | Owns |
| --- | --- |
| [01 — Product and supervision](product/01-product-and-supervision.md) | Personas, scope, workflows, attention, accessibility |
| [02 — Work, evidence, and coordination](product/02-work-evidence-and-coordination.md) | Domain objects, revisions, validation, dependencies, artifact integration |
| [03 — Execution, terminal, and remote sessions](architecture/03-execution-terminal-and-remote.md) | Core agentic navigation, key encoding/modes, observation/action loop, PTYs, ownership, SSH |
| [04 — Isolation, security, and approvals](architecture/04-isolation-security-and-approvals.md) | Threat model, runtime profiles, enforcement, stale approvals, secrets |
| [05 — Architecture, storage, and extensions](architecture/05-architecture-storage-and-extensions.md) | Modules, events, durability, recovery, APIs, plugins, migrations |
| [06 — Technology, distribution, and compatibility](research/06-technology-distribution-and-compatibility.md) | Stack alternatives, reuse, packaging, licensing, support matrix |
| [07 — Validation, roadmap, and risks](engineering/07-validation-roadmap-and-risks.md) | Measurable gates, fault injection, scheduling, staged delivery, risk register |
| [08 — Research and decisions](research/08-research-and-decisions.md) | Official-source findings, limitations, decision ledger, open questions |
| [09 — Self-review and coverage](reviews/09-self-review-and-coverage.md) | Two review passes, corrected contradictions, coverage, final checks |
| [10 — Recommended baseline and ADRs](architecture/10-recommended-baseline-and-adrs.md) | Executive decision summary, concrete defaults, rationale and vetoes |
| [11 — Human experience specification](ux/11-human-experience-spec.md) | Ordinary terminal, Work/Review UX, keyboard/focus, wireframes, journeys, recovery states |
| [12 — Ordered prototype plan](engineering/12-prototype-plan.md) | PP1–PP8, success/failure criteria, ordered later bootstrap |
| [13 — Dual frontend and host TUI](ux/13-dual-frontend-and-host-tui.md) | Initial TUI/Desktop parity, configuration, nested keys, host/inner TERM, SSH and accessibility |
| [14 — Repository and engineering](engineering/14-repository-and-engineering-practices.md) | Proposed Cargo/pnpm layout, ownership/DAG, schemas, contracts and contributor practices |
| [15 — Dependencies and supply chain](engineering/15-dependencies-and-supply-chain.md) | Proposed direct dependencies, Rust TUI comparison, licenses/unknown versions and pinning |
| [16 — Tests and CI](engineering/16-test-and-ci-strategy.md) | Layered tests, dual-client contracts, invariant/CI map, fixtures and full-process budgets |
| [19 — Engine BUILD/ADOPT/REDESIGN](engineering/19-engine-build-adopt-redesign-assessment.md) | Current comparative proofs, ownership estimates, recommendation and milestones |
| [17 — Bounded xterm risk](engineering/17-xterm-adapter-risk-and-gates.md) | Exact source family, public/proposed/private APIs, patch budget, X1–X6 and replacement |

Start with 10 for the executive recommendation, 11 and 13 for the human product, and 12 for the ordered plan. The current engineering continuation is 14 → 15 → 16, with 17 owning the terminal feasibility veto. Deeper paths: product → 01, 02; agentic navigation/API → 03, 05, 07; systems → 03, 05, 06; trust → 02, 04; research and review → 08, 09. Each document links back here. The source ledger distinguishes retrieved documentation from design proposals.

## Design invariants

These identifiers are the common vocabulary for all documents. They are requirements on a future implementation, not guarantees this document has tested.

| ID | Invariant |
| --- | --- |
| I1 | Ordinary shell and TUI operation does not require an agent or shell adapter. Enhanced semantics degrade to explicit uncertainty. |
| I2 | Tasks, attempts, sessions, processes, artifacts, and claims have distinct identities and lifecycles. A pane disappearing does not complete a task. |
| I3 | Every claimed validation names its recorded input revision and execution context, explicitly marking unknown or mutable inputs instead of inventing exact binding. Evidence availability, trust, completeness, and freshness are independent properties. |
| I4 | Terminal bytes, shell markers, agent messages, and repository instructions are data. None creates authority or proves success. |
| I5 | One broker-owned input lease governs Project X writers to a session. This is not protection against arbitrary same-user host processes. |
| I6 | Approval binds to an explicit action and its material context. Material context changes invalidate it; unknown delivery or outcome is never silently retried. |
| I7 | Worktrees separate edits; runtime boundaries constrain authority. Neither the other nor a PTY constitutes a security sandbox. |
| I8 | UI, logging, plugins, and slow subscribers cannot consume unbounded resources. Lost observation is visible, never silently presented as complete. |
| I9 | Durable history supports evidence inspection and state reconstruction. It cannot promise deterministic execution replay or restoration of dead processes. |
| I10 | Clients and plugins share versioned, capability-scoped contracts. Extensions cannot grant themselves permissions or mutate authoritative storage. |
| I11 | No network service, model subscription, or particular agent is required for basic terminal use. Remote/cloud features are explicit opt-ins. |
| I12 | The interface identifies which capabilities are observed, cooperatively reported, or enforced. Strong enforcement is advertised only for a verified runtime profile. |
| I13 | Agentic navigation is core: application input and terminal UI commands have separate authority/routes, every action targets an explicit session/view context, and observed transitions are distinguished from successful byte delivery. |
| I14 | Two initial human frontends share broker semantics and evidence, with honest capability differences. Neither owns authority, opens private persistence or reaches managed PTY/keeper/engine internals; outer TUI terminal I/O is separate. |

## Baseline boundaries

Recommend Rust Ratatui/Crossterm host TUI and Electron/TypeScript/React desktop, sharing one canonical native engine worker and immutable semantic/cell views. Official pinned libghostty-vt is the recommended next prototype; revised xterm is a fallback. Owning rendering/accessibility is now an explicit investment, not a waived fidelity gate. Both clients remain initial scope; TUI-only shipping must need neither Electron nor a separate Node engine runtime. Current JS/xterm packages remain reference harnesses. EA1–EA4/PP1–PP8 still gate production adoption. Ubuntu 24.04/26.04 x86_64 remains the certification target; this VM is 22.04. Apache-2.0 remains a recommendation only.

Actual bounded foundation: seven Rust and four internal TS packages, generated public/private schemas and local checks. The additional native worker is an EA1 experiment, not a production engine adoption. Reference Node/xterm packages remain; no frontend/runtime migration has occurred. [14](engineering/14-repository-and-engineering-practices.md) distinguishes current contents from the revised target.

P1 delivers both useful frontends with zero agents: ordinary shells/TUIs, core agent navigation, owned local sessions and bounded reconnect. P2 adds tasks/review and restricted supervision using the recommended verified rootless Podman offline profile; basic G4 is now required at P2, while P3 expands sealed validation. Host terminals remain clearly labeled host authority. Ordinary output is memory-only by default; capture/retention is explicit. Navigation keeps G7 and N01–N16 as core release gates. No cloud fleet, universal workflow framework or public plugin ABI is required.

The initial phase began with an empty directory; the prior continuation produced thirteen documents. This dual-frontend continuation read all thirteen, researched current primary sources, and rechecked `/AGENTS.md`, `/home/AGENTS.md`, `/home/w/AGENTS.md`, `/home/w/codex-dir/AGENTS.md`, and this project for instructions; none were present. This session preserves/revises the existing documents and adds five focused frontend, repository, dependency, testing and xterm-risk documents. Research used public primary sources and official source files. That prior documentation phase performed no implementation, installation or live experiment. The later authorized engineering continuation is recorded separately; no publication, private-session/credential inspection or unrelated-project change was performed. See the [completed review/check record](reviews/09-self-review-and-coverage.md#verification-record-and-residual-limits).
