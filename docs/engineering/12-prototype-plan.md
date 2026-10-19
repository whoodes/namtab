# Prototype entrypoint

**Current direction: [E5](../adr/0005-ai-first-harness-direction.md).** MK1 is delivered; this change is docs/planning with a draft PR only. No runtime work, frontend removal or merge is authorized.

[Index](../index.md) · [Scope](../engine/index.md) · [Canonical delivery plan](../engine/delivery-and-validation.md#ordered-milestones)

**MK1 engineering authorized; broader qualification remains planned.** The [MK1 admission](mk1-scope-and-admission.md) owns the first hands-on stopping point; finite core/worker and Linux process primitives are implemented with targeted tests, the [MK1 quickstart](mk1-quickstart.md) owns actual local launch and finite limits, without claiming full NG1. The current proposed sequence is [H0–H2](../engine/delivery-and-validation.md#current-harness-milestones); N/NG is retained qualification history, with deliverables, dependencies, success/stop criteria, staffing and open questions in that owner document. This page preserves earlier entrypoints without maintaining another plan.

## Order and stopping rules

Start with H0 contracts/workload and emulator/adapter decisions, then H1 coordinated scheduling/observations, then admitted H2 integration proof. Exact client count and native-window form remain proposals. MK1's two-client proof is historical and full NG1 remains unearned. Failure stops the dependent claim; no scope change relabels an old failure as success.

## PP1: falsifying the default early

Historical adoption prototype. Its surviving concerns are covered by [core state/input/replies](../engine/compatibility-and-core.md) and NG0/NG1. X4/EA1 stay failed and are not prerequisites to fix or waive for the owned engine. No SDK patch or alternate legacy authority is implicitly authorized.

## PP2–PP3: human and resource reference conditions

Historical labels now route to [shared UX](../ux/11-human-experience-spec.md), [host profile](../ux/13-dual-frontend-and-host-tui.md), [frontend integration](../engine/frontend-contracts.md) and [G6 budgets](07-validation-roadmap-and-risks.md#performance-and-resource-budgets). No numerical targets are duplicated here.

## PP4–PP6: failures that must remain visible

[Runtime recovery](../engine/runtime-persistence-security.md#precise-v1-recovery-contract) owns capture-off loss, checkpoint cuts and UI/engine/process distinctions; [security](../architecture/04-isolation-security-and-approvals.md) owns runtime and approval enforcement. NG3/NG4 require their fault/negative evidence, not a plausible restored screen.

## PP7–PP8: release and usability decisions

[CI4 release scope](16-test-and-ci-strategy.md#ci-lanes-and-practical-cost), [human validation](../product/01-product-and-supervision.md#product-validation) and [N/NG delivery](../engine/delivery-and-validation.md) replace the old release/usability prototypes. Formative studies identify blockers; they do not establish population-level superiority. License adoption remains pending.

## Foundation bootstrap after separate engineering authorization

Preserve the [current workspace inventory](14-repository-and-engineering-practices.md#concrete-proposed-layout-and-ownership) and historical experiments. The [target layout](14-repository-and-engineering-practices.md#concrete-proposed-layout-and-ownership) and [module contract](../engine/runtime-persistence-security.md#module-and-process-design) now include one cohesive finite `terminal` library/worker package. Extend schemas/import checks deliberately; add no empty microcrates. Each new capability adds its own relevant test evidence.

## Beyond the first certified baseline

N5 expansion is conditional on the [published qualification matrix](../engine/delivery-and-validation.md#compatibility-matrix), user need and maintenance budget. Structured remote helpers, platforms, graphics/bidi and schema migration are separate gates, not automatic v1 commitments.
