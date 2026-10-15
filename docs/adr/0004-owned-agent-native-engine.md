# ADR E4: own an agent-native engine with terminal compatibility

Date: **2026-10-15**. **Owner-chosen direction; accepted for design.** Implementation, dependency choices, compatibility certification and performance remain pending. [Campaign executive summary](../engine/index.md) · [Research](../engine/research.md) · [Delivery gates](../engine/delivery-and-validation.md)

## Decision and scope

Build a project-owned Rust agent-native engine. Own typed session/work/input/observation contracts, VT compatibility semantics, parser continuation, input policy and versioned complete-state representation. Native operations model managed work and evidence directly; unmodified Linux shells/CLIs/TUIs use an explicit PTY compatibility boundary. Both the host-terminal TUI and Electron remain first-class, including excellent zero-agent terminal behavior for published profiles.

Reuse OS APIs, safe wrappers, Unicode algorithms/data, font/shaping/rendering primitives, serialization and SQLite where justified. A lexer primitive is eligible only if its full bounded continuation is publicly representable under a reviewed license. An existing emulator must not quietly become the native semantic authority. Recommended starting point is a small owned FSM and semantic reducer, not an emulator fork or a bespoke OS/font stack.

The pure core owns no process/grant/dispatch authority. Service/domain govern permissions, receipts and task evidence; keepers own PTY/pipe handles and observed process facts. One canonical interpretation/input-policy owner and one reply source per session serve both frontends. Raw fallback, logical keys/text/paste, terminal UI commands and native operations remain separate routes. Text never grants authority.

No universal compatibility, opaque-TUI understanding, process resurrection, guaranteed termination, perfect security or deterministic execution replay follows. Complete-state continuation is conditional on a fixed version/profile/event history and validation; initial cross-version restore is refused. Capture-off owner loss can become unknown. A useful memory-only live slice precedes full v1 qualification.

## Rationale and rejected alternatives

The owner chooses long-term control of native semantics and compatibility policy. Earlier assessment recommended core adoption to reduce semantic maintenance, and the subsequent public-C seam failed classic legacy aliases. That mismatch and the old serializer failure are informative counterexamples, not proof that existing cores are generally broken or that full ownership is automatically cheaper.

Adopting a legacy emulator behind a native API is no longer the selected direction. Rewriting all OS, Unicode, shaping and rendering infrastructure is unnecessary. A large maintained emulator fork carries a different semantic/upstream burden and is not selected implicitly. Unbounded compatibility expansion, one microcrate/service per concept and a stable plugin ABI are rejected for the initial team. Staffing and long-term conformance/security costs are explicitly budgeted in the campaign.

## Supersession and preservation

This ADR supersedes **E1's adoption choice**, **E3's recommendation to seek a public SDK policy seam as the next product step**, and adoption-specific portions of baseline ADR-A1/A2, repository target and PP1/EA roadmap. It retains their single-owner, evidence, authority, dual-frontend and uncertainty invariants. E2's separation of continuity/capture/process recovery remains valid; its Ghostty-codec mechanism is historical and replaced by the owned-state design.

Original xterm X4 and libghostty EA1 tests/results stay **FAIL**, with their existing source, fixtures, evidence and pre-campaign six-commit history preserved (the campaign itself is the seventh commit). They are historical candidate gates, not v1 definitions for the new owned engine. No test expectations, CI jobs or source are changed by this decision. Existing passing foundation checks are not owned-engine certification.

## Consequences and next step

The project accepts responsibility for the complete text-terminal compatibility surface, input/Unicode/reflow policy, owned state/migrations, conformance/fuzz/security infrastructure, rendering/accessibility and sustained maintenance. Rust enables a low-overhead design but proves neither latency nor safety. Both frontends share canonical facts and require real usability qualification.

Use N0 → N1 → N2/N3/N4 gates, then expand profiles only with evidence. The future repository target adds one cohesive pure-library/worker package while preserving experimental references; no module move, dependency install, SDK patch or implementation occurs in this research phase. A later engineering authorization starts the bounded slice, not a broad production rewrite. Apache-2.0 remains a recommendation only.
