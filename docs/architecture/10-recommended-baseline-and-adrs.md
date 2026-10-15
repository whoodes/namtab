# Baseline decision register

[Index / document owners](../index.md#documentation-ownership) · [ADR E4](../adr/0004-owned-agent-native-engine.md) · [Current milestones](../engine/delivery-and-validation.md#ordered-milestones)

**Current as consolidated 2026-10-15.** This register records defaults and rationale, not a second architecture specification. Accepted-for-design and recommended choices remain unimplemented or measurement-gated. Historical A identifiers remain stable for citations; adoption-specific choices are superseded by E4. Actual artifact versions are recorded in locks and dated evidence, not implied by a recommended technology.

## Executive decision summary

| Area | Decision / status | Contract owner and unresolved condition |
| --- | --- | --- |
| Native engine | Project-owned Rust work/control model and compatibility core; owner-chosen | [E4](../adr/0004-owned-agent-native-engine.md), [scope](../engine/index.md); NG0–NG4 unearned |
| Initial clients | Ratatui/Crossterm host TUI and Electron/TS/React desktop; recommended | [UX](../ux/11-human-experience-spec.md), [host profile](../ux/13-dual-frontend-and-host-tui.md); both required, real accessibility/rendering unqualified |
| Canonical state/input | One owned interpreter and reply source, semantic/cell presentation | [Core](../engine/compatibility-and-core.md), [frontend seam](../engine/frontend-contracts.md); no frontend parser or PTY access |
| Runtime/lifetime | Rust service, independent keepers, multiplexed owned terminal worker | [Runtime](../engine/runtime-persistence-security.md); process boundaries do not establish a sandbox |
| Native execution | Typed managed pipes plus explicit PTY compatibility | [Native contract](../engine/native-contract.md), [execution](03-execution-terminal-and-remote.md); no shell-prompt success oracle |
| Platform/distribution | Ubuntu 24.04 x86_64 build baseline, 24.04/26.04 qualification; later native ports | A4 below; current Ubuntu 22.04 VM is not certification |
| Restricted execution | Rootless Podman offline profile; recommended | [Security profile](04-isolation-security-and-approvals.md#filesystem-and-runtime-profiles); negative host tests before any restricted claim |
| Capture/retention | Memory-only ordinary output; explicit capture classes | A6 below; capture consent is not consent to model export |
| Storage | SQLite journal/projections + policy-scoped objects; accepted for design | [Domain storage](05-architecture-storage-and-extensions.md#storage-and-durability), [terminal recovery](../engine/runtime-persistence-security.md#complete-snapshots-and-journal-cuts) |
| IPC/extensions | Bounded local authenticated JSON; built-in then out-of-process adapters | [Runtime IPC](../engine/runtime-persistence-security.md#ipc-configuration-and-extension-scope), [extension lifecycle](05-architecture-storage-and-extensions.md#versioned-apis-and-plugins); private versioned interfaces, no stable plugin ABI |
| Repository | Cargo + pnpm, current seven Rust/four TS packages; one future terminal package | [Repository target/actual](../engineering/14-repository-and-engineering-practices.md); no package migration yet |
| License | Apache-2.0 recommended only | A8 below; owner adoption and release-specific dependency clearance pending |

## ADR-A1 — Frontend and language boundary

Rust owns service/domain/OS/CLI/TUI and the owned core; TypeScript owns Electron UI. The two clients serve complementary host/desktop needs without duplicating authority. Electron is recommended for rich review/accessibility and a controlled browser runtime, not proven lower memory or faster rendering. Historical alternatives/rationale remain in [technology comparisons](../research/06-technology-distribution-and-compatibility.md#desktop-and-terminal-alternatives); [dependencies](../engineering/15-dependencies-and-supply-chain.md#rust-tui-choice) owns the Rust TUI comparison. Current renderer selection is [DOM first, gated acceleration](../engine/frontend-contracts.md#text-rendering-selection-and-accessibility).

## ADR-A2 — One interpretation authority, replaceable presentation

E4 replaces adopted emulator semantics with owned parser/state/input. The [core contract](../engine/compatibility-and-core.md) owns modes, replies and complete semantic state; the [native contract](../engine/native-contract.md) owns admission/receipts. Both frontends render projections. X4/EA1 remain historical failures, not new-engine criteria or implicit fallback permission.

## ADR-A3 — Supervision and actual process guarantees

[Runtime/recovery](../engine/runtime-persistence-security.md) owns process boundaries and failure cuts; [execution](03-execution-terminal-and-remote.md#process-and-pty-ownership) owns process-group/PTY guarantees. A surviving keeper is not a surviving interpreter, and neither can resurrect a dead process. Session/task takeover fences future mediated actions; it does not stop all side effects. Native adapters report attributed facts; opaque agent CLIs remain ordinary programs.

## ADR-A4 — Linux and eventual desktop ports

**Recommended; qualification pending N2/N3.** Build against Ubuntu 24.04 LTS userland for x86_64. Certify maintained 24.04 and 26.04 installations; for Electron, Wayland is primary and X11 a tested compatibility path. Host TUI profiles are independently certified and do not require a desktop display server. This is a product support target, not a claim about this VM or every Ubuntu flavor. Native package first; no silent startup-file changes or required account. Exact dependency/kernel patch floors are fixed from the prototype build and security inventory, rather than invented here. Canonical's lifecycle page documents both LTS lines. [Ubuntu lifecycle](https://ubuntu.com/about/release-cycle)

Package aligned core/CLI/TUI (including the qualified native engine) and desktop artifacts under one product tuple; both frontends are initial release scope. Package Linux user-level supervisor/keeper lifecycle explicitly; background service starts on demand, not automatically at login. Closing a desktop view or detaching a TUI releases its endpoint leases and unsent input while processes can continue; desktop last-window quit offers keep-running versus explicit termination. TUI detach restores outer terminal modes on normal exit; a killed client may need manual host recovery. Logout stops local owned work by default, with any unattended/linger mode an informed opt-in. Update only after compatible runners are retained or work is intentionally drained. Do not inherit a service-manager kill policy that contradicts advertised GUI persistence.

Later: macOS Apple Silicon first, Windows 11 x64 first, each with an OS version still supported by the selected Electron release. macOS needs its own PTY/session/credential and sandbox/VM decisions. Windows uses ConPTY and Job Objects where applicable; Ctrl-C, descendants, resize, paths, ACLs and IPC require native contracts. WSL is a separate Linux remote/runtime target, not equivalent to native Windows process control. Restricted Linux containers on macOS/Windows require an explicit Linux VM/remote host; do not relabel them native host confinement. [Microsoft ConPTY](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session), [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

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

[Shared human UX](../ux/11-human-experience-spec.md), [host limits/parity](../ux/13-dual-frontend-and-host-tui.md) and [frame integration](../engine/frontend-contracts.md) own the presentation contracts. Focus/selection are not input ownership. Neither client reads private persistence or reaches managed PTY/keeper/worker endpoints. DF01–DF08 tests their common semantics with honest physical/accessibility differences.

## ADR-A10 — Small workspaces and measured engineering gates

[Repository practices](../engineering/14-repository-and-engineering-practices.md) owns layout, schemas, typed errors/logging and release versions; [CI](../engineering/16-test-and-ci-strategy.md) owns lanes. Current code is seven Rust/four TS packages, including historical experiments; proposed `crates/terminal` is absent. No microcrate hierarchy, generic workflow framework or public plugin ecosystem is a prerequisite for N1.

## Coherent stopping point

Use [N0–N5](../engine/delivery-and-validation.md#ordered-milestones), the [current risk/open-decision register](../engine/delivery-and-validation.md#risk-register-and-remaining-decisions) and [G6 budget owner](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets). PP/P adoption plans are historical. A useful narrow dual-client slice precedes full v1 compatibility, resource, isolation and durable-recovery qualification. No implementation or performance result is implied by this consolidation.
