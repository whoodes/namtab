# Owned agent-native engine: scope and initial design

[Design index](../index.md) · [Ownership decision](../adr/0004-owned-agent-native-engine.md) · [Source ledger](research.md) · [Delivery and traceability](delivery-and-validation.md)

Decision date and research retrieval date: **2026-10-15**. **Chosen by the owner:** build namtab's agent-native engine. **Accepted for design:** the boundaries below. **Pending implementation and measurement:** every new engine capability. This campaign changes the design, not executable code, dependencies, licenses or the historical gate results.

## Executive decision

Own the session/event model, terminal compatibility semantics, incremental parser state, logical input policy, observations and complete-state format in Rust. Build native operations around identified work and explicit evidence; provide a PTY compatibility layer for unmodified shells and interactive programs. A third-party legacy emulator will not determine native state behind an adapter. Reuse OS facilities, Unicode algorithms/data, UI libraries and storage primitives where their contracts fit.

The engine is broader than a VT emulator but narrower than an operating system or autonomous coding platform. Its pure compatibility core interprets bytes; the surrounding session service coordinates execution, evidence and authority. Neither the core nor an application-output marker can grant permission. Native callers can request a managed process directly without manufacturing a shell command or reverse-engineering its screen. Interactive legacy programs remain useful, with uncertainty disclosed.

This direction is justified by the desired native contract and long-term ownership. Three failed aliases or an incomplete serializer do not by themselves establish that a full engine is economically necessary. [E1–E3](../adr/0003-legacy-input-public-policy.md) and reports 18–23 retain the earlier adoption rationale and failures. **Original X4 and libghostty EA1 remain FAIL.** Their regression inputs are useful; neither old candidate test becomes the new product definition. No EA2, SDK patch, dependency replacement or production engine is delivered here.

## Read the design

The [design guide](../index.md#reading-order) owns reading order and the [authoritative document map](../index.md#documentation-ownership). Follow native operations → compatibility core → runtime/recovery for implementation contracts, and shared UX → host profile → frontend integration for human behavior. [Delivery](delivery-and-validation.md) is the only current roadmap/risk/open-decision register; [research](research.md) is the current source ledger. Earlier adoption mechanisms and their recommendations are historical.

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
| Presentation contracts, accessibility mapping, selection provenance | Ratatui/Crossterm; Electron/browser text, fonts/shaping, OS accessibility | Frontends cache presentation, never parse inner VT or choose canonical widths |
| Journal/capture policy, migrations and integrity validation | SQLite and filesystem durability primitives | Storage transactions do not make PTY delivery transactional |

No new dependency is selected or installed by this document. Apache-2.0 remains a project-license recommendation, not adoption. Standards, Unicode data, reused code, fonts and test corpora each need provenance/license review before incorporation.

## Useful slice versus v1 versus vision

**First working slice (N1):** one local managed pipe attempt and one interactive shell session; a small owned UTF-8/VT profile; native operation/receipt/observe APIs; human takeover; one useful terminal viewport in each frontend; memory-only output; reconnect to a surviving owner. Demonstrate one agent-native workflow and a zero-agent shell without requiring broad compatibility or durable engine recovery first. All unsupported features are visible and unadvertised.

**v1 target (N2–N4):** bounded Linux modern-text compatibility, tested Bash/readline + less + Vim profiles, negotiated keyboard/paste/mouse/focus, both useful frontends, task/artifact review, bounded parallelism and fairness, authenticated local IPC, explicit runtime profiles, opt-in same-version complete checkpoints/tails and tested unknown outcomes. No shipping claim until the corresponding gates pass. v1 engineering admission is not conditioned on passing an old engine's adoption gate.

**Later:** additional shells/TUIs and SSH helper transport, richer native application adapters, more graphics/text profiles, cross-version migration and broader OS support. macOS/Windows require native lifecycle and rendering qualification; no Linux-equivalence assumption. A legacy `ssh` process is available earlier as an opaque local PTY program.

Non-goals: universal VT/ECMA conformance, an invented shell/OS/libc, universal TUI semantic extraction, a model provider or inference scheduler, arbitrary native plugins, deterministic execution replay, restoring dead processes from terminal snapshots, and hostile-same-UID protection without an actual OS boundary. Full bidi terminal editing, legacy character encodings and graphics protocols are not v1 promises. Excellent ordinary terminal behavior is still a product requirement for the **published** profile.

## Current engineering gate

The completed campaign chose ownership; later authorized N0 work develops reviewed contracts, experimental public schema/DTO artifacts and independent fixture data. The [NG0 review packet](delivery-and-validation.md#ng0-review-packet-draft) owns current input revisions, pending merges and owner decisions. These static artifacts are not an owned engine or NG0 acceptance. N1 needs separate authorization and a bounded acceptance tuple; no broad emulator rewrite, all-xterm port, legacy authority or crate-per-subsystem follows automatically. [Delivery](delivery-and-validation.md#ordered-milestones) remains the single implementation sequence and failure-criteria owner.
