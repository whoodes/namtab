# AI-first harness and owned terminal engine

[Design index](../index.md) · [Ownership decision](../adr/0004-owned-agent-native-engine.md) · [Source ledger](research.md) · [Delivery and traceability](delivery-and-validation.md)

**Direction updated 2026-10-19; research ledger retrieved 2026-10-15.** E5 owns current scope. MK1 is implemented at its recorded finite boundary; proposed harness scale, model integration and broader qualification remain pending. This alignment changes documentation/planning only.

## Executive decision

[ADR E5](../adr/0005-ai-first-harness-direction.md) records the user-chosen **AI-first execution/coordination harness around a custom owned terminal engine**, with optimized control intended for dozens of agents. Electron is out of active product scope. The emulator's presentation form and rendering stack are unresolved; the host TUI remains a diagnostic/attach tool, not a mandatory second product frontend. Ordinary Linux shell/CLI/TUI compatibility and excellent human intervention remain explicit goals.

MK1 is **delivered** at `ff15a18b81c78ebcc539d521ec6ca00ebcd14b69` ([PR44](https://github.com/whoodes/namtab/pull/44)): finite Rust core/worker, real PTY/managed execution, typed receipts, shared immutable task declarations, exact attempt origins and bounded clients. Its four live clients/two lifetime PTYs/two managed attempts and two scripted-agent proof do not establish dozens-agent scale or model integration. Full NG1 was not earned. Historical X4/EA1 remain FAIL.

Own native work/control and terminal semantics; reuse reviewed OS/text/font/rendering primitives where justified. The pure core has no process/grant/dispatch authority. The broker controls admission and evidence; keepers own process/PTY facts; presentation consumes canonical state. Task readiness, resource admission, process exit, artifact validation and acceptance remain distinct. No opaque-shell command inference, replayed unknown effect or model-status claim creates authority.

## Read the design

The [design guide](../index.md#reading-order) owns reading order and the [authoritative document map](../index.md#documentation-ownership). Follow native operations → compatibility core → runtime/recovery for implementation contracts, and human UX → presentation contract; prior host profile is diagnostic reference for human behavior. [Delivery](delivery-and-validation.md) is the only current roadmap/risk/open-decision register; [research](research.md) is the current source ledger. Earlier adoption mechanisms and their recommendations are historical.

## What is native about it

An agent works with a task, attempt, execution context, operation ID and evidence cursor. It can launch a bounded `argv` operation, wait asynchronously for a particular process observation, inspect distinct pipe streams, register an authorized artifact revision and request human attention without interpreting an ANSI prompt. A terminal surface is attached when interactive compatibility is needed. Machine facts, cooperative application statements and model inferences have different types and trust.

Agents and people share identities, leases, observations and receipts, not implicit “current pane” state. A session may have several tasks over time; a task may use several sessions or processes. The system must not assign an exit code to a typed shell line just because a prompt reappeared. A native operation's receipt also cannot prove that all external effects succeeded or stopped.

Notifications, approvals, event APIs and control sockets already exist in Warp, cmux and tmux [research S19](research.md). The product hypothesis is that a coherent native work/evidence/control contract plus useful compatibility improves supervision. That advantage requires a comparative vertical-slice test; it is not established novelty or a performance result.

## Ownership and reuse

| Own | Reuse / evaluate | Boundary |
| --- | --- | --- |
| Agent/session events, receipts, provenance and authority rules | Existing domain/protocol foundation; versioned JSON/schema tooling | One policy owner; adapters cannot mint grants |
| VT lexer continuation, semantic state and profile dispatch | Standards, published state-machine techniques and independent conformance cases | A parser library is eligible only if all continuation/resource state is publicly representable; no hidden emulator authority |
| Input policies and typed keyboard capabilities | OS/host event facts; protocol specifications | Do not infer lost physical keys, consumed modifiers or IME facts |
| Cell/grapheme model, snapshots, deltas and recovery envelope | Unicode data/segmentation/width primitives, reviewed codecs/hashes | Freeze versions and tailoring; library-private state is not checkpointable by assumption |
| Process/session lifecycle policy and keeper protocol | Linux PTYs, termios, process groups, pidfds, cgroups; reviewed Rust OS wrappers | Kernel processes, shell semantics and runtime isolation remain separate |
| Presentation contracts, accessibility mapping, selection provenance | Existing Ratatui diagnostics; future presentation/text/font/accessibility stack pending E5 decision | Frontends cache presentation, never parse inner VT or choose canonical widths |
| Journal/capture policy, migrations and integrity validation | SQLite and filesystem durability primitives | Storage transactions do not make PTY delivery transactional |

No new dependency is selected or installed by this document. Apache-2.0 remains a project-license recommendation, not adoption. Standards, Unicode data, reused code, fonts and test corpora each need provenance/license review before incorporation.

## Useful slice versus v1 versus vision

**Historical first-slice plan (N1):** managed pipe and interactive shell execution, a finite owned terminal profile, typed receipts, takeover, live-owner reconnect and minimal paired viewports. MK1 delivered a bounded subset, not full NG1. This retains the old rationale; [H0–H2](delivery-and-validation.md#current-harness-milestones) replace it as the current plan. No second product frontend follows from the historical slice.

**v1 target (N2–N4):** bounded Linux modern-text compatibility, tested Bash/readline + less + Vim profiles, negotiated keyboard/paste/mouse/focus, the selected emulator and optional diagnostics, task/artifact review, bounded parallelism and fairness, authenticated local IPC, explicit runtime profiles, opt-in same-version complete checkpoints/tails and tested unknown outcomes. No shipping claim until the corresponding gates pass. v1 engineering admission is not conditioned on passing an old engine's adoption gate.

**Later:** additional shells/TUIs and SSH helper transport, richer native application adapters, more graphics/text profiles, cross-version migration and broader OS support. macOS/Windows require native lifecycle and rendering qualification; no Linux-equivalence assumption. A legacy `ssh` process is available earlier as an opaque local PTY program.

Non-goals: universal VT/ECMA conformance, an invented shell/OS/libc, universal TUI semantic extraction, a model provider or inference scheduler, arbitrary native plugins, deterministic execution replay, restoring dead processes from terminal snapshots, and hostile-same-UID protection without an actual OS boundary. Full bidi terminal editing, legacy character encodings and graphics protocols are not v1 promises. Excellent ordinary terminal behavior is still a product requirement for the **published** profile.

## Current engineering gate

PR53 merged approved external-agent-first responsibility and native-window feasibility before toolkit selection. Current authorization is one bounded remaining H0 prerequisite **draft PR** and independent review; no further merge or runtime implementation. [H0–H2](delivery-and-validation.md#current-harness-milestones) own proposed successor acceptance. H0 resolves contracts/workload and interface/adapters; H1 proves scheduling, targeted observations and coordination; H2 proves admitted real-agent and emulator integration. Exact counts, numerical targets, providers and UI framework require review and owner decisions.

Preserve the [MK1 run/limits](../engineering/mk1-quickstart.md), source/tests and historical native pins. New measurements must distinguish connected versus runnable agents, managed jobs, PTYs and provider/model cost; include p50/p95/p99 control/observation/input/paint with workload/hardware and all-process bounds. No existing benchmark earns the new gate.
