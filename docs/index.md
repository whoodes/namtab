# namtab documentation

Choose the guide for your task. These are three audiences of one engine/harness project, not parallel specifications.

## Reading order

1. **[Use namtab](users/index.md):** prerequisites, launch, controls, agent configuration, lifecycle, troubleshooting and current limits.
2. **[Work on namtab](agents/index.md):** agent onboarding/source map, applicable instructions, build/test workflow and evidence discipline. [Contributor policy](engineering/contributing.md) remains the single workflow rule owner.
3. **[Architecture](architecture/index.md):** implemented engine and native harness, shared contracts and data/control flow; larger design and open qualification are labeled separately.

The owner-chosen direction is an AI-first harness around the owned terminal engine. Main includes the bounded native feedback/lifecycle and ENG1/NAT1 increments (PR74/75); real-model Namtab mediation and dozens-agent capacity remain unvalidated. A merged experiment is not full H0/H1 or production acceptance. Electron is removed, and the host TUI is diagnostic. The [manifesto](product/manifesto.md) explains the product tradeoffs.

## Documentation ownership

| Question | Canonical owner |
| --- | --- |
| Current use, setup, controls and lifecycle | [User guide](users/index.md); technical limits/evidence linked there |
| Working-agent orientation and source map | [Agent guide](agents/index.md); rules in [contributing](engineering/contributing.md) |
| Actual engine/harness structure | [Architecture](architecture/index.md); decisions [E4](adr/0004-owned-agent-native-engine.md), [E5](adr/0005-ai-first-harness-direction.md), [E6](adr/0006-engine-and-native-harness-workstreams.md) |
| Native task/attempt/artifact/control design | [Native contract](engine/native-contract.md); implemented [exact-attempt API](engine/attempt-control.md) is a separate experimental subset |
| Parser/state/input/replies and compatibility | [Owned core](engine/compatibility-and-core.md) |
| Runtime, persistence, recovery and threat boundaries | [Runtime contract](engine/runtime-persistence-security.md); broader [security profiles](architecture/04-isolation-security-and-approvals.md) |
| Client presentation and interaction | [Frontend contract](engine/frontend-contracts.md), [NAT1 controls](ux/fleet-control.md), [host diagnostic limits](ux/host-terminal-profile.md); broader [human UX](ux/11-human-experience-spec.md) |
| Delivery, traceability, sources and unresolved admission | [Roadmap](engine/delivery-and-validation.md), [H0 residuals](engine/h0-residual-criteria.md), [source ledger](engine/research.md) |
| Repository, dependencies, tests and supply chain | [Repository practices](engineering/14-repository-and-engineering-practices.md), [dependencies](engineering/15-dependencies-and-supply-chain.md), [tests/CI](engineering/16-test-and-ci-strategy.md) |
| Legal scope and notices | [Licensing](licensing.md), root LICENSE and retained upstream full texts |

Edit the owner and cross-reference it. Entry guides summarize/route rather than duplicate contracts. Plans use proposed/accepted-for-design; empirical claims name actual evidence/revision. Source code and lockfiles describe actual versions, not a future selection table.

## Status and terminology

The **engine** includes work/control/execution as well as the owned core. **Core** is the finite parser/state/input reducer; **worker** is its private process; **broker** owns admission and dispatch; the **native harness** owns human navigation/presentation through public contracts. Proposed independent **keepers**, SQLite projections, richer wire schemas and portable checkpoints are not the implemented MK1 runtime.

H0–H2 is the harness roadmap; ENG1/NAT1 are bounded workstream increments. N0–N5/NG0–NG4 retain prior qualification rationale: recorded N0 design/MK1 acceptance did not earn full NG1. I1–I14 below are design invariants, G0–G8 quality dimensions, T/N/DF/V test catalogs and CI0–CI4 lanes. Old P/PP/X/EA progression labels are historical. MK1/1, MCP adapter0, attempt-control0, future product API and snapshot/Unicode versions are distinct namespaces. Documentation and issue readiness never authorize actions.

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

Retain empirical findings and source/provenance, not a second active roadmap. [Xterm/X4](engineering/17-xterm-adapter-risk-and-gates.md), [foundation results](engineering/18-foundation-gate-results.md), [BUILD/ADOPT assessment](engineering/19-engine-build-adopt-redesign-assessment.md), [EA1](engineering/21-ea1-native-worker-results.md) and [legacy-input investigation](engineering/23-legacy-input-investigation.md) explain failures/alternatives. X4 reconstruction failure is not proof of canonical parser failure; original EA1 alias failure is not an owned-engine qualification gate. ADR E1–E3 preserve superseded choices. Active reference/test fixtures and exact source archives remain.

## Cleanup inventory — 2026-10-28

Issue76 streamlines the merged74/75 baseline. Remove obsolete research06/08, self-review09, prototype router12, pre-EA1 organization20 and local-history22 narratives; their recoverable prose remains in [the reviewed Git revision](https://github.com/whoodes/namtab/tree/5c99e34bc6534a09f0511557e1f6f1154d46572c/docs). No new archive copy is created. Unique host decoding/paste/TERM/SSH/focus/accessibility constraints are consolidated in the [host profile](ux/host-terminal-profile.md), replacing the retired dual-frontend page. README and this index route three audiences; details remain at the owners above.

Keep all historical evidence/pins/source archives, licenses/notices, locks/requirements, schemas/generated bindings, active fixtures/goldens/test assets, terminfo and build inputs. Historical JSON manifests still name removed prose as dated inventories; they are not live-file requirements and are not rewritten. Ignored/untracked artifacts/caches/worktrees are inventory-only and are not deleted. Measured before/after file/byte and consumer records are in the cleanup PR/handoff. No runtime behavior, dependencies or qualification changes.

## Consolidation review — 2026-10-15

The earlier pass is [recoverable in Git](https://github.com/whoodes/namtab/blob/5c99e34bc6534a09f0511557e1f6f1154d46572c/docs/index.md#consolidation-review--2026-10-15); its verbose chronology is superseded by the current owner map and cleanup inventory above. Protected empirical records remain unchanged.
