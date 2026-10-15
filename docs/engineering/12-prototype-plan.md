# Ordered prototype plan and decision gates

[Design index](../index.md) · [Owned-engine scope](../engine/index.md) · [ADR E4](../adr/0004-owned-agent-native-engine.md) · [Canonical milestones](../engine/delivery-and-validation.md#ordered-milestones)

The owner chose a project-owned Rust agent-native engine on 2026-10-15. This is research/design only; engineering requires a later bounded authorization. Historical PP1/EA adoption steps and their rationale remain in reports 19–23 and the prior Git revision; they are superseded as the next product plan. Original X4 and libghostty EA1 remain FAIL, unchanged. Do not fix or waive them to start the new path.

## Order and stopping rules

The one authoritative engineering sequence is [N0–N5 and NG0–NG4](../engine/delivery-and-validation.md#ordered-milestones). This page maps earlier product gates without duplicating a second milestone plan.

1. N0: native identity/receipt/event contract, owned complete-state inventory, finite compatibility profile, source/data pins and independent fixtures.
2. N1: useful pipe-backed native work and PTY shell, owned reducer, takeover/live attach and minimal useful views in both clients. Prove native evidence value without prompt parsing.
3. N2: explicitly advertised classic terminal/input/Unicode profile and real candidate application navigation.
4. N3: parallel fairness, review/accessibility, resource/fault/runtime enforcement and declared release workload.
5. N4: opt-in compatible complete-state recovery and corruption/crash/consent cases. State design begins at N0; full durability need not delay N1.
6. N5: broader platforms/apps/transport/profiles/migrations only after a concrete need and validation budget.

## PP1: falsifying the default early

PP1's old bridge/source-family exercise is historical. Its invariant survives: one interpretation/reply authority, correct input and hidden-state continuation, no frontend PTY bypass. The owned engine must pass its own independent corpus, including prior counterexample inputs, with native operations beyond a PTY API. An owned lexer or dependency must expose all bounded partial state; no private emulator authority or screenshot-as-checkpoint. Repeated failure to show a useful N1 triggers a scope/staffing/design review, not quiet adoption of another core.

## PP2–PP3: human and resource reference conditions

Both frontends remain required. Use [human journeys and measurement contract](../engine/frontend-contracts.md) and [G6 reference budgets](07-validation-roadmap-and-risks.md#performance-and-resource-budgets): eight cores, 16 GiB, SSD, 1080p with exact CPU/GPU/fonts/scaling recorded; 32 sessions/eight active attempts, 8 MiB/s aggregate for ten minutes at full qualification. N1 measures a smaller declared workload and makes no G6 pass claim.

Keep provisional p95 input-to-paint 50 ms, decision update 250 ms, admission-revoke 100 ms and total product memory 1 GiB. Report p50/p95/p99/max, queues and actual paint; TUI-only, desktop-only and both on one canonical backend. Include all product processes/copies and report outer host/tmux/child-tool resources separately. Quotas are included allocations, not simultaneous full-capacity promises. No measured result follows from Rust or this documentation.

## PP4–PP6: failures that must remain visible

[Runtime/recovery/security](../engine/runtime-persistence-security.md) owns authority epochs, partial/unknown delivery, checkpoint cuts and no-capture loss. UI restart is not engine restart; a restored terminal is not a restored process. Existing runtime/worktree/isolation requirements still need negative tests. Restricted profiles fail admission when enforcement is unavailable. Output, prompt markers and native application statements never grant authority. Do not begin fleet/plugin expansion to avoid these local boundaries.

## PP7–PP8: release and usability decisions

[Validation and staffing](../engine/delivery-and-validation.md) owns actual release tuples, both clients' accessibility and artifact/provenance review. Preserve project-local locks, notices/SBOM review, offline basic terminal use and inert configuration migration. Apache-2.0 remains unadopted. Comparative productivity claims need a declared user study; a native workflow demonstration alone is not evidence of universal superiority.

## Foundation bootstrap after separate engineering authorization

Preserve current source and history. Add one cohesive future terminal library/worker package under the [module plan](../engine/runtime-persistence-security.md#module-and-process-design); extend boundaries/generated schemas deliberately. Existing seven Rust/four TS packages are a foundation and historical experiments, not the owned engine. Avoid empty microcrates or deleting failed probes. Build N1 before expanding the test matrix to every program/platform; add tests for the capability being claimed.

## Beyond the first certified baseline

Broader compatibility, graphics/bidi, cross-version migration, structured SSH helpers and macOS/Windows are separately scoped work. No universal compatibility, deterministic execution replay or process resurrection promise. New implementation and installations are outside this research campaign.
