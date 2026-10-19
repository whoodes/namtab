# ADR E5: AI-first harness around the owned terminal engine

**Status: user-chosen direction, 2026-10-19; detailed architecture and delivery gates proposed.** Supersedes E4's two-initial-frontend commitment and baseline A1/A9's Electron default. Retains E4's project-owned semantics, authority boundaries and historical evidence. This decision does not authorize runtime implementation or merging the alignment PR.

## Decision

Build namtab as an **AI-first execution and coordination harness around a custom project-owned terminal engine**, with optimized agentic control intended to scale cleanly to dozens of agents. The custom terminal emulator serves that harness and useful human intervention. Electron is removed from active product scope. Ordinary Linux shells, CLI/TUI programs and interactive navigation remain explicit compatibility surfaces; they are not the source of native task or command semantics.

The shell runs programs; the terminal core interprets admitted terminal behavior; the harness coordinates identified work, execution, evidence and control. No layer turns terminal text or a model's claim into permission or verified success. Rust remains the existing owned backend/core implementation; it does not prove speed.

## Accepted, proposed and open

| Classification | Meaning |
| --- | --- |
| Accepted direction | Harness-first product, custom owned terminal engine/emulator, agentic control and coordination, dozens-agent ambition, excellent human intervention, Electron out of active scope |
| Retained invariants | Distinct task/attempt/execution/session identities; revision-bound contracts and evidence; one input/reply authority; explicit writer ownership; unknown effects never automatically retried; bounded resources; no frontend PTY/private-store authority |
| Proposed architecture | Session-ordered execution lanes with bounded shared workers; resource admission/fair scheduling; scoped event subscriptions/deltas/condition waits; adapter-bound agent integration; focused human steering and exception review |
| Open decisions | Emulator presentation form and renderer/framework; model-facing harness loop versus existing-agent adapter boundary; exact agent/process/PTY workload and hardware; numerical latency/resource gates; new wire/dependency admission |

A standalone native window is a **proposal**, not an approved requirement. A host-terminal control TUI is an existing diagnostic/attach tool, not a second mandatory product frontend. “Native” here means owned agent/work contracts; it does not silently select an OS window toolkit. An exact 32-client workload is also a proposal, not the definition of “dozens” or an achieved capacity claim.

## Harness responsibilities and boundaries

- **Work and coordination:** agent identity/capabilities; task definition and lifecycle revisions; attempts; selected dependencies; bounded delegation/messages; context/workspace ownership; artifact publication and revision-bound integration/review. DAG-ready, resource-admitted, process-exited, artifact-validated and task-accepted are separate facts. A shared contract grants neither a writer nor merge approval.
- **Execution and control:** managed argv/pipe execution for direct process/stream attribution; explicit PTY compatibility for interactive tools. PTY stdout/stderr merge; persistent shell output does not establish per-command boundaries or success. Keys/text/paste/mouse/focus capability proposals remain mode-aware and profile-gated; UI commands and application input stay separate.
- **Observation:** scoped subscriptions, deltas, explicit gaps/resync and waits on observable conditions are proposed to replace whole-state polling at scale. Each observation retains source, revision/cut, completeness and trust. App-internal meaning cannot be inferred universally from opaque TUI cells. Reported agent status is distinct from broker/process facts; no hidden thought/cognition is advertised.
- **Scheduling:** separate connected agents, active model calls, managed commands, PTYs and CPU/IO demand. Bound queues, retained state and subscribers; maintain per-session ordering, fair admission and a reserved human control path. Provider/model work, output floods, parsing, artifact hashing and storage must not monopolize control. Worker sharding/failure domains require measurements and review, not one process per agent by default.
- **Human supervision:** a clear steering interaction, task/agent identity, factual activity, blockers and decisions; focused terminal/evidence windows through progressive disclosure. Focus does not grant input ownership. Takeover fences future mediated input, not existing effects or all descendants. Actual keyboard, selection, accessibility, IME and paint quality must be tested for the selected interface.
- **Security and recovery:** untrusted output/messages are data; grants and explicit human decisions use trusted controls. Worktrees are not runtime isolation. Capture consent is separate from model export. Preserve no-capture uncertainty, reconnect versus engine/process loss, no-effects state replay and no deterministic execution-replay promise. Do not relax existing boundaries to achieve throughput.

The existing [native contract](../engine/native-contract.md), [coordination policy](../product/02-work-evidence-and-coordination.md), [core](../engine/compatibility-and-core.md), [runtime/security](../engine/runtime-persistence-security.md) and [human UX](../ux/11-human-experience-spec.md) retain those detailed rules. They are proposed product contracts except where the bounded MK1 implementation is explicitly evidenced. E5 controls product scope where older frontend/stage wording conflicts.

## Existing baseline and preservation

MK1 was delivered and merged in [PR44](https://github.com/whoodes/namtab/pull/44), main `ff15a18b81c78ebcc539d521ec6ca00ebcd14b69`, reviewed candidate `48be5db04b7f2316173a6c534c7b0200e48ecfe1`. It includes an owned finite Rust core/worker, real Linux PTY and managed execution, experimental typed operations/receipts, immutable task declarations, attributed attempts, ownership fences, host TUI and Electron demonstration. These are **delivered bounded facts**, not full NG1 qualification, a model integration or dozens-agent evidence.

MK1 admits four live clients, two lifetime PTYs/two managed attempts and four immutable RAM contracts. Its broker mutex/shared worker, whole snapshots and artifact rereads need reassessment for scale; increasing constants does not prove fairness or capacity. The two scripted-agent journeys and descriptive small-sample RPC timings remain evidence only for their recorded workload/build. They are neither LLM workflows nor rendering/model latency results.

Freeze Electron as historical MK1 source/evidence. Keep `apps/desktop`, related packages, locks, tests, launch commands and reproducibility references; this documentation step removes no runtime or CI assertions. Preserve original eight commits, 201 protected evidence files, original native pin and failed X4/EA1 records. Existing sessions and the legacy staged worktree are not cleanup targets. No project license or repository/Project visibility change follows.

## Delivery and decision discipline

The [current delivery table](../engine/delivery-and-validation.md#current-harness-milestones) owns H0–H2 proposals and issue mapping. H0 reviews contracts, scale tuple and interface choice; H1 proves coordinated scheduling/observation; H2 proves admitted real-agent integration and the selected emulator interaction. Compatibility qualification (#3) remains scoped; exact recovery (#5) stays later; broader future work (#6) and historical evidence (#24) remain separate. Bring coordination (#4) forward rather than deferring it behind a dual-frontend gate.

N0 design and MK1 delivery remain historical accepted outcomes. Old N1/NG1's dual-client qualification is **unearned and superseded as the current gate**, not silently completed. Residual N1 criteria get explicit review/dispositions and successor references. No dates, owners, capacity or numerical SLO are invented. A future runtime step requires separate authorization; the current authorized result is an issue-linked **draft** docs/planning PR, not a merge.

## Rejected approaches and next decisions

- Continuing Electron or requiring two product frontends would contradict the chosen scope and divide effort. Retained MK1 diagnostics still serve reproduction and existing users.
- Calling the two-client proof a dozens-agent harness, or a scripted client an autonomous model, would misstate evidence.
- A new global scheduler, microservice/plugin ecosystem or public ABI before a bounded contract/measurement gate would add unearned complexity.
- Discarding sound Rust contracts/core tests or historical failures would lose useful correctness evidence. Reuse is conditional on source and semantic suitability, not automatic qualification.

Before dependent implementation: H0 #46 must define contracts, a reproducible mixed workload (connected versus runnable agents/processes/PTYs), admitted dependency/wire tuples and measurable latency/resource/failure gates. Headless H1/adapter work need not await presentation #47; #47 is required before selected-emulator integration. Review and separately authorize the smallest useful slice; issue dependencies alone grant no execution authority. Actual model/provider credentials, spending, legal/license and security-policy choices remain owner decisions.
