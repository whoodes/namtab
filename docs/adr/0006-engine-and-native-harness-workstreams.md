# E6 — Engine and native harness are separate first-class efforts

**Status:** owner-chosen direction,2026-10-27; accepted for design. Detailed ENG1/NAT1 contracts below are proposed bounded scopes, paused for cleanup before further implementation; not admitted or qualified APIs. This refines [E5](0005-ai-first-harness-direction.md), retaining [E4](0004-owned-agent-native-engine.md)'s ownership of semantics and the removal of Electron from active scope. One repository/workspace remains; no broad moves or new public plugin ABI.

## Decision and actual boundary

Namtab has two efforts, not a backend service with an incidental UI:

- **AI-first terminal engine:** authoritative terminal semantics/state, supervised execution, attribution, task/artifact coordination, ownership/concurrency, persistence/recovery boundaries, resource admission and measured control responsiveness. The pure terminal reducer is only one engine module.
- **Native harness:** makes that engine useful for humans operating agents: work/terminal navigation, steering and task visibility, interaction, targeted takeover/cancel/lifecycle UX, rendering and accessibility. It consumes explicit versioned contracts; it never becomes another parser, grant authority or execution-state store.

The broker's state is authoritative. The harness can retain selection, focus, forms, confirmation targets, presentation caches and the last observed frame; it must label stale/disconnected/unknown. It cannot turn model prose or a visible shell into an engine fact. Model turns, Namtab-mediated operations, external tools and process exit remain distinct. Native steering of an external agent requires an actually supported adapter; a literal argv box is not a model prompt.

| Actual code owner at PR68's reviewed9bcb627 | Responsibility / exclusion |
| --- | --- |
| `crates/terminal` | Owned finite parser/state/input/replies and worker; no grants/PTY ownership or semantic understanding of arbitrary apps |
| `crates/service/src/mk1`, `crates/platform/src/local_process.rs` | Current supervised process handles, work/receipt/ownership authority and bounded execution; not a qualified scheduler/sandbox/recovery implementation |
| `crates/protocol/src/mk1.rs` | Experimental typed public wire/capabilities; distinct from the N0 proposed public schemas and private terminal-worker RPC |
| `crates/client/src/mk1.rs`, `mcp/`, `namtabctl` | Shared client/adapter and operator integration: translate intents, validate response bindings, no private state/PTY access or independent grant decisions |
| `crates/console/src/native`, `namtab-window` | Native harness projection, keyboard/focus/selection/confirmation and rendering; no VT parser, process kill, authoritative task policy or invented model status |
| `scripts/mvp.py` | Explicit developer launch/attach/status/stop plumbing through public contracts; not a process supervisor, hidden socket discovery or PID authority |

Thus PR68's `LifecycleHello`/protective `Shutdown`, receipt semantics and resource reservation belong to **engine**; `namtabctl` and launcher commands are **client/harness integration**. MCP is an **external-agent adapter**, not a model loop or engine authority. Native UI focus fencing supplements the engine lease; it does not replace final engine validation. Existing Ratatui TUI remains diagnostic, not a second mandatory product interface.

## Version and dependency discipline

Use versioned operation/profile/capability contracts plus immutable epoch/operation/target bindings, not imports of private service structures. Current private development wire is `namtab.mk1/1`; extensions must be advertised before use and fail closed when absent. A capability is support, not permission. A later public1.0 wire/SDK needs separate compatibility/migration admission; these increments do not freeze the whole MVP.

Proposed ENG1 would add `namtab.attempt-control/0` / `attempt_cancel_v0`; proposed NAT1 would consume it. Neither capability is in this cleanup revision. Engine tests can run without a display or harness implementation. Harness components can test projection with fixtures, but actual integration must use the engine and independently observe effects; mocks cannot qualify the runtime. New DTO fields remain additive/defaulted where possible, with explicit unsupported cases. No speculative microservices/crates or language split is introduced.

**Integration lineage,2026-10-28:** PR68's reviewed `9bcb627` merged into main as `0856034620643a2cf55a4b9a55b48ee32311f0f1`, preserving its tree. The direction-only prerequisite `8eaca56` and [cleanup72/PR73](https://github.com/whoodes/namtab/pull/73) were reviewed at `78e94a7`, originally targeting PR68's branch. PR73 integration retargets actual main and preserves those commits through a normal merge; its current revision/checks are in the PR. Uncommitted ENG1 code remains frozen in its own worktree; no NAT1 source increment exists. Neither is included or qualified by cleanup. **Cleanup completion does not resume feature work; new owner scope is required.** Integration uses actual final-head checks/review, not a prospective tree as evidence. This dated lineage does not freeze live GitHub PR status into a product contract.

## Two milestones and backlogs

- [Engine ENG1](https://github.com/whoodes/namtab/milestone/9): exact-target managed-attempt cancellation and peer continuity, [issue70](https://github.com/whoodes/namtab/issues/70). [Engine backlog](https://github.com/whoodes/namtab/issues?q=is%3Aissue+is%3Aopen+label%3Aworkstream%3Aengine).
- [Native harness NAT1](https://github.com/whoodes/namtab/milestone/10): attributed roster, canonical terminal navigation and targeted controls, [issue71](https://github.com/whoodes/namtab/issues/71). [Harness backlog](https://github.com/whoodes/namtab/issues?q=is%3Aissue+is%3Aopen+label%3Aworkstream%3Aharness).
- [Integration alignment69](https://github.com/whoodes/namtab/issues/69), H0/H1/H2 and issues46/47/50/51 retain cross-track qualification/owner prerequisites. They are not silently earned by either milestone. Private Project2 holds separate workstream views without changing visibility or splitting repositories.

Original N/NG and failed X4/EA1 evidence remain historical; no gate is relabeled as passing. Original criteria are retained, with current dispositions appended rather than rewritten as success. Merged bounded issues can close only at their actual scope. No due dates, staffing commitments, funding, provider trust, license or production capacity are selected here.

## Bounded slices and integration acceptance

**Engine ENG1:** cancel one exact managed attempt using epoch+attempt+launch-origin binding; human or exact launch owner only. Record requested/already requested/already exited versus independently observed direct-child exit/signal or unknown. Existing monitor remains the unique Child owner; no caller PID, process-group claim or automatic namespace shutdown. Two isolated attempts prove A cancellation while B continues, with negative target/role/epoch/dedupe cases. [Operation contract](../engine/attempt-control.md) owns details.

**Native harness NAT1:** an attributable work roster and one focused canonical activity window. Keyboard selection is not input ownership; view/target changes fence queued input. Human takeover targets one exact session; a selected cancel confirmation freezes one engine attempt target. Missing capability, stale context, disconnected or uncertain effects are explicit. No UI-side cancellation/policy or inferred model success. [Harness contract](../ux/fleet-control.md) owns interaction.

Retain2PTY/2managed-attempt/four-live-connection/15ordinary+1lifecycle bounds. These prove a useful two-client control loop, not dozens-agent scalability. Larger cohorts, targeted subscriptions, scheduler fairness, full recovery, terminal fidelity, IME/a11y and release isolation stay open. Workload/hardware/build/sample count accompany every latency/throughput statement; control receipt latency is not inference or input-to-paint latency. Numerical budgets remain proposed until their particular gate is measured.
