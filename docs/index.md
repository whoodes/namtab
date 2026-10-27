# namtab design guide

**Build an AI-first execution and coordination harness around a custom owned terminal engine, intended to scale cleanly to dozens of agents.** [ADR E5](adr/0005-ai-first-harness-direction.md) owns the 2026-10-19 direction and supersession; [manifesto](product/manifesto.md) states priorities. Electron is outside active product scope. The retained host TUI is diagnostic/attach tooling. External-agent-first responsibility and native-window feasibility before toolkit selection were approved with PR53. Final presentation/renderer and exact load/admission remain open; no 32-client target or capacity is chosen.

**MK1 is delivered**, merged in [PR44](https://github.com/whoodes/namtab/pull/44) at `ff15a18b81c78ebcc539d521ec6ca00ebcd14b69`: a finite owned core/worker, real local execution, experimental task/receipt contracts and TUI/Electron demonstrations. This is not full NG1, a model integration or dozens-agent qualification. Historical X4/EA1 remain **FAIL**; original material uses [Apache-2.0 with third-party exceptions](licensing.md). PR55 merged the finite model and PR57 merged the separately bounded [headless write experiment](engineering/h0-headless-write-experiment.md). Neither completes broader H0 acceptance. The separately authorized [producer→consumer experiment](engineering/producer-consumer-experiment.md) extends bounded headless evidence; it does not earn H0 acceptance or merge authority.

## Reading order

1. [Direction and open choices](adr/0005-ai-first-harness-direction.md), then [executive scope](engine/index.md): native value, owned/reused components, useful slice versus v1 and non-goals.
2. [Human experience](ux/11-human-experience-spec.md): steering, focused activity, review and takeover. The [prior host profile](ux/13-dual-frontend-and-host-tui.md) is retained diagnostic/history guidance, not a second product commitment.
3. [Native contract](engine/native-contract.md) → [compatibility core](engine/compatibility-and-core.md) → [runtime/recovery](engine/runtime-persistence-security.md): authoritative identities, state and failure boundaries.
4. [H0 admission/workload](engine/h0-harness-contracts.md), [criterion dispositions](engine/h0-residual-criteria.md) and [interface decision analysis](ux/h0-interface-decision.md): concrete recommendations and owner decisions, not implementation. [Delivery](engine/delivery-and-validation.md) owns the H0–H2 roadmap, traceability and risks.
5. For engineering preparation, read [repository practices](engineering/14-repository-and-engineering-practices.md), [dependency inventory](engineering/15-dependencies-and-supply-chain.md), [CI strategy](engineering/16-test-and-ci-strategy.md) and [contributor workflow](engineering/contributing.md).

The [external-agent/lifecycle increment](engineering/external-agent-mvp.md) owns the new bounded bridge/control profile, blocked live-model evidence and proposed next fleet-terminal slice. It does not replace the native contract or earn broader H0 admission.

[E6](adr/0006-engine-and-native-harness-workstreams.md) now owns the two-workstream boundary: AI-first engine and native harness, one repository. [ENG1 control](engine/attempt-control.md) and [NAT1 interaction](ux/fleet-control.md) own proposed, paused feature contracts; the E4/E5 semantics and historical evidence remain.

## Documentation ownership

Each row has one authoritative owner. Other pages summarize and link rather than redefine its contract. A scope conflict requires updating the owner and its traceability entry together; a later date alone does not make a paragraph authoritative.

| Decision or contract | Authoritative location | Supporting detail |
| --- | --- | --- |
| Current direction and supersession | [ADR E5](adr/0005-ai-first-harness-direction.md), retaining E4 owned semantics | [Baseline decision register](architecture/10-recommended-baseline-and-adrs.md) |
| Product scope, owned/reused components, v1 exclusions | [Engine scope](engine/index.md) | [Personas/attention](product/01-product-and-supervision.md), [manifesto](product/manifesto.md) |
| Native identities, operations, receipts, observations, writer fencing | [Native contract](engine/native-contract.md) | [Artifact/validation/dependency policy](product/02-work-evidence-and-coordination.md) |
| PTY/pipe semantics, parser/state, Unicode, input encoding, query replies | [Compatibility and core](engine/compatibility-and-core.md) | [Process ownership, optional adapters and remote modes](architecture/03-execution-terminal-and-remote.md) |
| Runtime/module graph, ordering, scheduling, IPC, terminal checkpoints/recovery | [Runtime and persistence](engine/runtime-persistence-security.md) | [Domain storage, exports and extension lifecycle](architecture/05-architecture-storage-and-extensions.md) |
| Threat model, grants, approval lifecycle, runtime isolation, secret handling | [Security and approvals](architecture/04-isolation-security-and-approvals.md) | [Terminal-specific threat mitigations](engine/runtime-persistence-security.md#threat-model-and-mitigations) |
| Capture classes, retention and platform/distribution/license defaults | [Baseline A4–A8](architecture/10-recommended-baseline-and-adrs.md#adr-a4-linux-and-eventual-desktop-ports) | Recovery mechanics remain in the runtime contract |
| Human information architecture, appearance, shortcuts, review, states, configuration | [Human UX](ux/11-human-experience-spec.md) | [Host TUI/SSH/paste/TERM profile](ux/13-dual-frontend-and-host-tui.md) |
| Canonical frame/renderer integration and native-to-legacy handoff | [Frontend integration](engine/frontend-contracts.md) | Shared UX is not a second engine contract |
| Repository paths, current versus target packages, schemas, errors/logging, releases | [Repository practices](engineering/14-repository-and-engineering-practices.md) | [Logical commit workflow](engineering/contributing.md) |
| Proposed dependency admissions, current pin references and upgrades | [Dependencies](engineering/15-dependencies-and-supply-chain.md) | Lockfiles and dated provenance records establish actual artifacts |
| Quality dimensions, T/N acceptance catalog and numerical performance budgets | [Acceptance and budgets](engineering/07-validation-roadmap-and-risks.md) | [CI/DF cases and all-process measurement method](engineering/16-test-and-ci-strategy.md) |
| Compatibility qualification matrix, milestones, V/NG gates, risk register, open decisions | [Delivery](engine/delivery-and-validation.md) | [Prototype entrypoint](engineering/12-prototype-plan.md) routes to the same sequence |
| H0 admission, external-agent responsibility and workload proposal | [H0 proposal](engine/h0-harness-contracts.md) | Native contract remains owner of public identity/receipt/input rules |
| Criterion-level legacy disposition review | [H0 residual register](engine/h0-residual-criteria.md) | Original issue criteria and pinned MK1 evidence are retained |
| Presentation-form alternatives and feasibility recommendation | [H0 interface analysis](ux/h0-interface-decision.md) | Human UX/renderer contracts remain distinct; no framework selected |
| Current research facts, revisions and evidence gaps | [Source ledger S01–S32/H01 and H0](engine/research.md) | [Requirement traceability](engine/delivery-and-validation.md#requirement-traceability) |

## Status and terminology

- **Owner-chosen / accepted for design:** a current decision; not implementation authorization or proof.
- **Recommended / proposed:** a technology default or detailed contract to validate. **Pending** identifies an unresolved choice or empirical gate.
- **Empirical result:** applies only to its recorded source/build/environment and tested boundary. The repository evidence map (`evidence/README.md`) identifies immutable phase records; in this site, use the history links below.
- **Historical / superseded:** retained rationale and results, not an active instruction or next step. File numbers 01–23 preserve provenance; they do not prescribe reading or milestone order.

Use **owned terminal core** for the pure reducer/state/input library; **terminal worker** for its process; **service/broker** for domain authority; **keeper** for PTY/pipe handles and observed process facts. The product engine includes native work/control, not just VT emulation. A frontend's presentation cache is never another interpretation owner. “Raw terminal view” means canonical terminal presentation, not a PTY bypass. “Raw input” is a separately granted operation.

**H0–H2** is the current proposed harness sequence. **N0–N5 / NG0–NG4** preserve prior qualification rationale; N0 design/MK1 are accepted only at their recorded scope, not full NG1. Compatibility/recovery/future items retain explicit successor links. **G0–G8** are cross-cutting quality dimensions, **T/N/DF/V** identify test catalogs, and **CI0–CI4** identify lanes. Old **P/PP**, **X1–X6** and **EA** adoption progressions are historical. Private experimental wire versions, a future product v1, snapshot schemas, Unicode versions and upstream library versions are distinct namespaces.

## Design invariants

These identifiers are the common vocabulary for all documents. They are requirements on a future implementation, not guarantees this document has tested.

| ID | Invariant |
| --- | --- |
| I1 | Ordinary shell and TUI operation does not require an agent or shell adapter. Enhanced semantics degrade to explicit uncertainty. |
| I2 | Tasks, attempts, sessions, processes, artifacts, and claims have distinct identities and lifecycles. A pane disappearing does not complete a task. |
| I3 | Every claimed validation names its recorded input revision and execution context, explicitly marking unknown or mutable inputs instead of inventing exact binding. Evidence availability, trust, completeness, and freshness are independent properties. |
| I4 | Terminal bytes, shell markers, agent messages, and repository instructions are data. None creates authority or proves success. |
| I5 | One broker-owned input lease governs namtab writers to a session. This is not protection against arbitrary same-user host processes. |
| I6 | Approval binds to an explicit action and its material context. Material context changes invalidate it; unknown delivery or outcome is never silently retried. |
| I7 | Worktrees separate edits; runtime boundaries constrain authority. Neither the other nor a PTY constitutes a security sandbox. |
| I8 | UI, logging, plugins, and slow subscribers cannot consume unbounded resources. Lost observation is visible, never silently presented as complete. |
| I9 | Durable history supports evidence inspection and state reconstruction. It cannot promise deterministic execution replay or restoration of dead processes. |
| I10 | Clients and plugins share versioned, capability-scoped contracts. Extensions cannot grant themselves permissions or mutate authoritative storage. |
| I11 | No network service, model subscription, or particular agent is required for basic terminal use. Remote/cloud features are explicit opt-ins. |
| I12 | The interface identifies which capabilities are observed, cooperatively reported, or enforced. Strong enforcement is advertised only for a verified runtime profile. |
| I13 | Agentic navigation is core: application input and terminal UI commands have separate authority/routes, every action targets an explicit session/view context, and observed transitions are distinguished from successful byte delivery. |
| I14 | Harness clients and the selected emulator interface share authoritative identities/evidence with honest capability differences. No client owns broker authority, private persistence or managed PTYs. A second product frontend is not required; retained diagnostic TUI outer I/O is separate. |

## Historical record

The MkDocs **History** section is deliberately outside the current plan. Existing paths remain stable so original citations and command records still resolve. Read these pages as dated records even where their preserved text says “current” or “next.”

- [Technology comparisons](research/06-technology-distribution-and-compatibility.md), [prior source/decision ledger](research/08-research-and-decisions.md), [prior reviews](reviews/09-self-review-and-coverage.md).
- [Xterm adapter/gates](engineering/17-xterm-adapter-risk-and-gates.md) and [foundation results](engineering/18-foundation-gate-results.md): X4 is a serializer-continuation failure, not proof of canonical-parser failure.
- [BUILD/ADOPT/REDESIGN assessment](engineering/19-engine-build-adopt-redesign-assessment.md), [organization record](engineering/20-workspace-organization.md), [EA1 worker](engineering/21-ea1-native-worker-results.md), [initial history](engineering/22-local-history-and-validation.md), [input investigation](engineering/23-legacy-input-investigation.md): bounded public-C proof plus a failed classic-alias requirement; no product certification.
- [ADR E1](adr/0001-engine-ownership-and-presentation.md) and [E3](adr/0003-legacy-input-public-policy.md) preserve superseded adoption choices. [E2](adr/0002-terminal-continuity-and-checkpoints.md) preserves the continuity rationale; current mechanics are owned by the runtime contract.

## Consolidation review — 2026-10-15

Audited all 37 documentation pages, README, evidence index, navigation and contributor guidance against campaign commit `a5fb6ddf15ca14e20449d4d8c74799f8ac61e6f2`. Consolidated repeated navigation/state/recovery exposition into the native/core/runtime owners; narrowed execution/storage pages to their distinct responsibilities; replaced duplicate P/PP roadmaps and old risk registers with references to N/NG and the current risk register. Kept the T/N/DF catalogs, numerical budgets, human UX, source ledger and traceability rather than discarding edge cases. History is grouped separately, with original empirical reports/evidence unchanged.

Second review challenged accidental legacy authority, frontend state duplication, replay/retry promises, privacy, zero-agent UX and solo-team maintenance cost. Repairs include one renderer default (DOM first, acceleration measured), no Node fallback in the owned runtime, one current module graph, and clear evidence/proposal namespaces. Fresh inspection hashes and check results live in repository path `evidence/docs-consolidation/`; this is a documentation audit, not a runtime result. No new research, dependency/runtime change or engine work occurred.

MK1 admission and delivery are retained in the [scope record](engineering/mk1-scope-and-admission.md) and [quickstart](engineering/mk1-quickstart.md). The finite model, headless write and producer→consumer experiments are merged at their separately admitted bounds. The [native feedback increment](engineering/native-mvp.md) adds a partial real X11 Work/Terminal window over the public broker API; it does not complete H0, select the final renderer or qualify capacity/accessibility. Historical Electron source remains only in Git history and retained evidence.
