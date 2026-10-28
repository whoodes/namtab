# Baseline decision register

[Architecture](index.md) · [Document owners](../index.md#documentation-ownership)

## Executive decision summary

This register preserves A1–A10 citation IDs. It is a concise index of decisions, not a second specification. [Actual engine/harness architecture](index.md) owns the current structure; E4/E5/E6 own direction. Proposed production mechanisms below remain unimplemented unless explicitly identified as current. Versions come from locks/toolchain/evidence, not recommendation prose.

Current main: owned Rust finite core/worker, local broker/PTY/managed execution, public client/MCP adapter, native X11 feedback window, diagnostic TUI and bounded ENG1/NAT1 controls. Experimental2PTY/2attempt/four-live-connection limits are not measured fleet capacity. No full H0/NG1 or real-model mediation qualification.

## ADR-A1 — Frontend and language boundary

Rust owns the engine and native harness implementation. E5 retires Electron; its app/plumbing are removed. The X11/winit/softbuffer/fontdue feasibility tuple is implemented; final toolkit/rendering/accessibility remains open. Host TUI is diagnostic, not a second mandatory product. Retained TS/pnpm is reference/schema/test infrastructure. [E6](../adr/0006-engine-and-native-harness-workstreams.md), [native study](../ux/h0-interface-decision.md).

## ADR-A2 — One interpretation authority, replaceable presentation

E4 selects owned parser/state/input/replies. Clients render projections and submit admitted intents; no client parser or direct managed-PTY authority. X4/EA1 are historical failed alternatives, not product criteria or fallback permission. [Core owner](../engine/compatibility-and-core.md), [frontend owner](../engine/frontend-contracts.md).

## ADR-A3 — Supervision and actual process guarantees

The broker/platform owns direct process handles and dispatch. Exact-attempt cancellation distinguishes request, observed child exit and unknown; it does not contain descendants or reverse effects. Proposed independent keepers are not current runtime. PTY output merges stdout/stderr; managed argv preserves stream attribution. [Attempt control](../engine/attempt-control.md), [runtime owner](../engine/runtime-persistence-security.md), [execution](03-execution-terminal-and-remote.md).

## ADR-A4 — Linux and eventual desktop ports

Ubuntu22.04 x86_64 is the exercised VM/source-build tuple. Prior Ubuntu24.04/26.04 shipping qualification targets and eventual Windows/macOS remain proposals, not support claims. Linux PTY tests and X11 VM journeys do not qualify another OS/display/backend. [Platform matrix](../engineering/16-test-and-ci-strategy.md#headless-desktop-and-os-matrix).

## ADR-A5 — Enforced execution without building a container platform

Rootless Podman/offline restricted profiles remain design recommendations. Current supervised execution is developer-host execution, not a sandbox. Worktrees separate edits, not runtime authority. Any future restricted claim requires negative filesystem/network/process tests and explicit admission. [Security/profile owner](04-isolation-security-and-approvals.md#filesystem-and-runtime-profiles).

## ADR-A6 — Capture and retention defaults

The current MK1 capture-off bounded metadata journal is not the proposed managed storage/retention service below. These defaults remain **proposals**, retained here because storage/evidence contracts cite them; no new capture, expiry, storage quota or credential integration is implemented by this document.

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

Current private experimental MK1/1, stdio MCP adapter0 and attempt-control0 are distinct from proposed N0/native schemas and a future public1.0 API. Negotiate advertised capability plus authority/context, validate bindings and refuse stale/unknown without automatic replay. Built-in/narrow adapters precede any separately admitted plugin API; no stable ABI promised. Portable configuration/migration details are design, not an installed configuration wizard. [Native contract](../engine/native-contract.md), [extension owner](05-architecture-storage-and-extensions.md#versioned-apis-and-plugins).

## ADR-A8 — License and dependency maintenance

Apache-2.0 was adopted for owner-original material on2026-10-20. Upstream assets retain their terms; no blanket binary redistribution clearance. Preserve locks, provenance, full notices and release-specific dependency review. [Legal scope](../licensing.md), [dependency owner](../engineering/15-dependencies-and-supply-chain.md).

## ADR-A9 — Two frontends, one authority

The old two-product-frontend/Electron commitment is superseded, while its authority lesson remains: native harness and optional diagnostic clients share one engine contract. The host TUI has outer decoding/TERM/SSH/paste limitations, not parity or independent policy. [Host diagnostic profile](../ux/host-terminal-profile.md), [human UX design](../ux/11-human-experience-spec.md).

## ADR-A10 — Small workspaces and measured engineering gates

Cargo owns seven active Rust crates plus the excluded historical worker; pnpm owns tooling/reference packages. No speculative plugin/microservice split. Active-source CI and independent reviews concern their declared scope; workload-bound p50/p95/p99 and all-process resource measurements need admitted populations/hardware. Rust alone is not a speed result. [Repository owner](../engineering/14-repository-and-engineering-practices.md), [tests/CI](../engineering/16-test-and-ci-strategy.md), [G6 proposals](../engineering/07-validation-roadmap-and-risks.md#performance-and-resource-budgets).

## Coherent stopping point

Use the [roadmap](../engine/delivery-and-validation.md) and [H0 residual criteria](../engine/h0-residual-criteria.md) for unresolved owner decisions and qualification. A merged bounded increment does not accept the broader gate. The [user guide](../users/index.md) is the source of launch/lifecycle instructions; the [agent guide](../agents/index.md) routes contributors to canonical rules. No approval follows from this register.
