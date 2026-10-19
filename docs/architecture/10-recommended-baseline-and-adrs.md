# Baseline decision register

[Index / document owners](../index.md#documentation-ownership) · [Current ADR E5](../adr/0005-ai-first-harness-direction.md) · [Current milestones](../engine/delivery-and-validation.md#ordered-milestones)

**Scope revised 2026-10-19 under E5; prior consolidation 2026-10-15 retained.** This register records defaults and rationale, not a second architecture specification. Detailed design recommendations remain measurement-gated; the finite MK1 implementation is delivered. Historical A identifiers remain stable for citations; adoption-specific choices are superseded by E4; E5 supersedes dual-frontend scope and the Electron default. Actual artifact versions are recorded in locks and dated evidence, not implied by a recommended technology.

## Executive decision summary

| Area | Decision / status | Contract owner and unresolved condition |
| --- | --- | --- |
| Native engine | AI-first harness and project-owned Rust work/control/compatibility core; owner-chosen | [E4](../adr/0004-owned-agent-native-engine.md), [scope](../engine/index.md); MK1 finite subset delivered; broader qualification unearned |
| Human interface | Custom emulator serving the AI-first harness; form/renderer pending | [E5](../adr/0005-ai-first-harness-direction.md), [UX](../ux/11-human-experience-spec.md); Electron frozen as MK1 history, TUI diagnostic/attach only |
| Canonical state/input | One owned interpreter and reply source, semantic/cell presentation | [Core](../engine/compatibility-and-core.md), [frontend seam](../engine/frontend-contracts.md); no frontend parser or PTY access |
| Runtime/lifetime | Rust service, independent keepers, multiplexed owned terminal worker | [Runtime](../engine/runtime-persistence-security.md); process boundaries do not establish a sandbox |
| Native execution | Typed managed pipes plus explicit PTY compatibility | [Native contract](../engine/native-contract.md), [execution](03-execution-terminal-and-remote.md); no shell-prompt success oracle |
| Platform/distribution | Ubuntu 24.04 x86_64 build baseline, 24.04/26.04 qualification; later native ports | A4 below; current Ubuntu 22.04 VM is not certification |
| Restricted execution | Rootless Podman offline profile; recommended | [Security profile](04-isolation-security-and-approvals.md#filesystem-and-runtime-profiles); negative host tests before any restricted claim |
| Capture/retention | Memory-only ordinary output; explicit capture classes | A6 below; capture consent is not consent to model export |
| Storage | SQLite journal/projections + policy-scoped objects; accepted for design | [Domain storage](05-architecture-storage-and-extensions.md#storage-and-durability), [terminal recovery](../engine/runtime-persistence-security.md#complete-snapshots-and-journal-cuts) |
| IPC/extensions | Bounded local authenticated JSON; built-in then out-of-process adapters | [Runtime IPC](../engine/runtime-persistence-security.md#ipc-configuration-and-extension-scope), [extension lifecycle](05-architecture-storage-and-extensions.md#versioned-apis-and-plugins); private versioned interfaces, no stable plugin ABI |
| Repository | Cargo + pnpm; owned finite terminal package now included | [Repository target/actual](../engineering/14-repository-and-engineering-practices.md); no package migration yet |
| License | Apache-2.0 recommended only | A8 below; owner adoption and release-specific dependency clearance pending |

## ADR-A1 — Frontend and language boundary

**Prior dual-frontend/Electron recommendation superseded by [E5](../adr/0005-ai-first-harness-direction.md).** Rust owns the existing core, service/domain, OS and public CLI boundary. The custom emulator's presentation form and renderer/framework remain open; neither a native window nor a new UI toolkit is selected. Retain TS/pnpm/reference and Electron MK1 source/tests for reproducibility; do not infer shipping dependencies from their presence. Host TUI is diagnostic/attach tooling. Human UX and client authority constraints apply to the selected interface; old DOM/React/Forge options are historical rationale, not current defaults.

## ADR-A2 — One interpretation authority, replaceable presentation

E4 replaces adopted emulator semantics with owned parser/state/input. The [core contract](../engine/compatibility-and-core.md) owns modes, replies and complete semantic state; the [native contract](../engine/native-contract.md) owns admission/receipts. Every admitted client renders/consumes authorized projections; no second product frontend is required. X4/EA1 remain historical failures, not new-engine criteria or implicit fallback permission.

## ADR-A3 — Supervision and actual process guarantees

[Runtime/recovery](../engine/runtime-persistence-security.md) owns process boundaries and failure cuts; [execution](03-execution-terminal-and-remote.md#process-and-pty-ownership) owns process-group/PTY guarantees. A surviving keeper is not a surviving interpreter, and neither can resurrect a dead process. Session/task takeover fences future mediated actions; it does not stop all side effects. Native adapters report attributed facts; opaque agent CLIs remain ordinary programs.

## ADR-A4 — Linux and eventual desktop ports

The exercised MK1 tuple is Ubuntu 22.04 x86_64, not a support certification. Existing Ubuntu 24.04/26.04 release targets remain unqualified compatibility proposals; H0 must declare the next actual OS/hardware/display or headless workload. Do not couple that choice to Electron or assume a graphical native window. Native packaging and later macOS/Windows paths require their own renderer, PTY/process, accessibility, IPC and dependency review. They are not part of this alignment.

Lifecycle semantics remain: closing/detaching presentation differs from stopping owned work; unsent input is discarded, old leases never resurrect implicitly, and an explicit process action cannot promise all descendants/remote effects stopped. MK1 keeps authority across live reconnect as documented; future service-manager/logout policy must be admitted separately. No startup/security/global configuration changes are implied.

## ADR-A5 — Enforced execution without building a container platform

The [security specification](04-isolation-security-and-approvals.md#filesystem-and-runtime-profiles) owns the recommended rootless Podman offline profile, effective controls, no-pull admission and explicit host fallback. It is a recommendation until host-negative tests pass. N1 provides host-approved terminal use; N3 must qualify isolation before restricted-supervision claims. Sealed validation is a separately tested input profile, not a property of every container. Controller, child runtime and workspace authority have distinct badges.

## ADR-A6 — Capture and retention defaults

**Recommended; policy enforcement pending N3/N4.** Privacy expectations for an ordinary terminal outweigh the convenience of automatic disk transcripts. This supersedes the earlier filtered-by-default ordinary capture policy.

| Data class | Recommended initial default | Limit / implication |
| --- | --- | --- |
| Ordinary terminal output/screens | Live bounded memory only; no persistent transcript or raw snapshot | Live quotas are owned by [G6](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets); memory is not secret-proof |
| Opt-in filtered session/managed output | Seven days after explicit session/workspace capture consent | 256 MiB rolling per session, 8 GiB shared transcript cap; transformations/gaps labeled |
| Raw validation evidence/screens | Explicit per-run permission; 24-hour default | A review pin can extend to 30 days with reserved space; never silently renew |
| Artifact review copies | 30 days by default, 8 GiB artifact-store budget | Owned worktrees/user files are not automatically removed by this policy |
| Decision/action metadata | 180 days by default, subject to deletion policy; initial 512 MiB logical store budget plus 64 MiB emergency storage reserve | No routine input values, command secrets, or hidden output; evidence can expire earlier; WAL/backup/compaction overhead measured separately |
| Search/export/crash data | Same or shorter authority/retention than input; uploads off | No unfiltered snapshot, index, core dump, or RPC trace bypass |

Control metadata capacity also constrains new admissions: an unavailable required retention reservation blocks new work or requires an explicit retention change, rather than silently purging promised audit records. The reserve is not a guarantee that failing hardware can persist an emergency action. Managed reservations and pins count against the same transcript budget; if retention cannot be honored, reject admission or ask for a concrete retention change. Ordinary output can roll only under its selected policy. Evidence expiry means unavailable/unauditable, not failed or retroactively successful validation. Shortening retention explains affected review packets. Existing recordings are never made raw by a later default change.

Use private store permissions and recommend OS disk encryption; do not add home-grown application encryption/key recovery to the first release. This does not protect a live compromised account. Credential storage uses an OS credential service when available, otherwise explicit memory-only entry; never a portable plaintext token fallback. Exact credential-service integration remains a port-specific gate. Consent to output capture is separate from model/context export.

## ADR-A7 — APIs, adapters, and portable configuration

The [runtime contract](../engine/runtime-persistence-security.md#ipc-configuration-and-extension-scope) owns wire/negotiation scope; [storage/extensions](05-architecture-storage-and-extensions.md#versioned-apis-and-plugins) owns compatibility and extension lifecycle; [UX configuration](../ux/11-human-experience-spec.md#onboarding-migration-and-portable-configuration) owns declarative TOML precedence/import behavior. Private 0.x contracts ship as a pinned tuple. No prototype API is a permanent public ABI. Project content is data and cannot expand grants or enable capture/hooks.

## ADR-A8 — License and dependency maintenance

**Recommendation only; owner decision pending.** Apache-2.0's explicit patent-license model and permissive integration fit the intended extensibility. MIT is a credible simpler alternative; copyleft expresses a different distribution goal. No LICENSE or legal grant is added. [Apache text](https://www.apache.org/licenses/LICENSE-2.0.txt) is not transitive clearance.

[Dependency admission/upgrade policy](../engineering/15-dependencies-and-supply-chain.md) owns provenance, notices, advisory/SBOM gaps and replacement costs. Fonts, Unicode data and corpora require their own review. Historical Ghostty/xterm/Zig are experiment-only unless a later explicit shipping decision says otherwise; there is no automatic second-engine fallback.

## ADR-A9 — Two frontends, one authority

**Historical title/anchor retained; two-frontends requirement superseded by E5.** The current invariant is one authoritative harness/terminal model across agent adapters, selected emulator and optional diagnostics. Focus/selection do not grant writer ownership. Clients cannot read private persistence, access managed PTYs or dispatch replies. Retain historical DF fixtures and Electron tests; future qualification chooses applicable client profiles after H0 instead of requiring a retired client.

## ADR-A10 — Small workspaces and measured engineering gates

[Repository practices](../engineering/14-repository-and-engineering-practices.md) owns layout, schemas, typed errors/logging and release versions; [CI](../engineering/16-test-and-ci-strategy.md) owns lanes. The linked repository inventory now includes the owned finite `crates/terminal` core/worker and real Linux primitives. MK1 is integrated and delivered at its finite boundary; full harness/terminal qualification remains unearned. No microcrate hierarchy, generic workflow framework or public plugin ecosystem is a prerequisite for N1.

## Coherent stopping point

Use [H0–H2 current proposals](../engine/delivery-and-validation.md#current-harness-milestones) and the retained [N/NG history](../engine/delivery-and-validation.md#ordered-milestones), the [current risk/open-decision register](../engine/delivery-and-validation.md#risk-register-and-remaining-decisions) and [G6 budget owner](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets). PP/P adoption plans are historical. The delivered MK1 dual-client slice is historical evidence; the proposed harness gates govern new coordination, scale and emulator work. No implementation or performance result is implied by this consolidation.
