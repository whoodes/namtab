# Engine and native harness architecture

[Use namtab](../users/index.md) · [Work on namtab](../agents/index.md) · [Detailed document owners](../index.md#documentation-ownership)

This page describes the **implemented main baseline after PR74/75, ENG2 PR79 and ENG3 PR81**. The full native-contract/core/recovery specifications describe a larger design; they are not claims that every operation or guarantee exists. [E4](../adr/0004-owned-agent-native-engine.md), [E5](../adr/0005-ai-first-harness-direction.md) and [E6](../adr/0006-engine-and-native-harness-workstreams.md) own the direction and responsibility split. Two first-class workstreams do not require separate repositories or duplicated state.

```text
native harness / diagnostic TUI       external agent / CLI
          |                                   |
          +---- public client / MK1/1 ops -----+  MCP stdio adapter
                              |
                     service / broker
            admission, receipts, attribution, ownership
                /             |              \
       platform handles   terminal worker   metadata journal
       PTYs/pipes/Child    parser/state/
                          input/replies

program output -> platform -> broker -> worker -> broker -> observations
terminal input intent -> broker -> worker encoding -> broker -> platform -> program
task/run/cancel -> broker admission/receipts -> task/attempt/process owner
worker reply  -> broker's restricted reply route -> platform -> program
              (clients/worker never own managed PTY handles)
```

## Engine

The product engine includes work/control/execution, not just a VT library. Rust `protocol/domain/platform/service/client/terminal` modules share explicit boundaries:

| Owner | Actual responsibility | Important limit |
| --- | --- | --- |
| Owned core and private worker | Incremental finite byte/Unicode/VT state, canonical cells/modes, logical-key encoding and generated replies | Bounded implemented subset; no second frontend parser, full VT/reflow/portable snapshot guarantee |
| Broker | Managed literal argv with separate stdout/stderr, PTY compatibility, exact attempt origins/task revisions, admission and receipts | A PTY merges stdout/stderr; persistent shell command boundaries remain opaque |
| Broker/platform | One writer generation per PTY; pre-write checks; uniquely owned process handles | Fences future Namtab writes, not hostile same-UID OS writes or already-completed effects |
| Attempt monitor | `namtab.attempt-control/0` / `attempt_cancel_v0`, exact epoch+attempt+immutable launch origin, requested versus observed exit | No PID supplied by the caller; no all-descendant/all-side-effect termination promise |
| Journal/lifecycle | Bounded capture-off receipt metadata, explicit epoch-bound restricted status/stop and live-owner reconnect | Not SQLite projections, durable output, engine-crash/process resurrection or deterministic execution replay |

The broker is the single dispatcher for permitted application input and worker-generated terminal replies. Reply authority is restricted and not an agent grant. Canonical state belongs to the core; clients consume projections. A managed pipe command is exactly attributable to its attempt; a Bash prompt or terminal marker cannot prove a command completed or a task succeeded. [Implemented core limits](https://github.com/whoodes/namtab/blob/main/crates/terminal/README.md), [exact control contract](../engine/attempt-control.md), [execution design](03-execution-terminal-and-remote.md).

The prototype admits two lifetime PTYs, two lifetime managed attempts and four simultaneous public connections. Retained identities are bounded separately (15 ordinary plus one lifecycle). These are admission limits, not calibrated capacity or dozens-agent throughput. Frame/queue/output/receipt bounds and versions stay in their [implementation admission records](../engineering/external-agent-mvp.md#admitted-experimental-tuple), not a duplicated limit table here.

## Native harness

Rust `crates/console` owns fleet navigation, selected task/attempt/terminal detail, rendering, keyboard/focus state, action confirmation and visible uncertainty. It consumes the public client; it does not inspect the journal or grant authority, create an alternate parser, kill a PID or dispatch to a PTY directly.

- Work selection has a stable identity and differs from terminal input ownership. Enter opens a terminal paused. Take/release/resize target the highlighted epoch/session/generation, never a hidden last-opened context.
- Cancellation freezes one attempt and immutable origin, consumes confirmation once, and dismisses on selection/view/focus/resize/stale/disconnect. The connector rechecks its local fence after observation; the broker is final authority. A focus change cannot recall an already-dispatched effect.
- Missing/misbound receipts fence apparent success. Requested cancellation, observed child exit, stale/last-observed state, failed and unknown are distinct. A model's thinking/progress is not inferred from a process label or output stream.
- The current experimental native tuple is X11/winit/softbuffer/fontdue. This is a feasibility implementation, not final toolkit, shaping, IME or accessibility approval. The host TUI remains diagnostic/attach tooling; Electron is retired/removed.

The [fleet interaction contract](../ux/fleet-control.md) owns keyboard/fencing semantics and evidence. [Host diagnostic constraints](../ux/host-terminal-profile.md) own outer terminal limits. [Frontend design](../engine/frontend-contracts.md) owns future selection/accessibility/rendering contracts.

## Shared contracts and control flow

1. A client negotiates/observes the live broker epoch and capabilities. A fresh connection identity is distinct from live-owner resume and from actual model identity.
2. The client constructs one typed action with exact material context. Engine admission checks identities, owner/generation and observation/mode revisions where required; it reserves a bound receipt before effects.
3. The authoritative worker/platform boundary performs the admitted operation. A partial write or lost/uncertain result is not automatically retried, repaired by a suffix or moved to a different target.
4. Receipt/result reconciliation and subsequent observations tell the client what is known. Delivery, process exit, output drain, artifact revision and task acceptance are different facts.

MCP is an external-agent adapter over this path, not an engine-owned model loop. Only operations actually sent through Namtab are mediated; an agent's own shell/tools/files/model calls bypass it unless separately admitted. Current MCP tools expose task/run/observation/terminal/artifact/receipt operations; native targeted cancellation uses the public client contract rather than implying a new MCP tool. [Adapter details and retained live-test blocker](../engineering/external-agent-mvp.md).

## Evidence versus remaining design

Merged ENG1/NAT1 have bounded real-process/headless/native evidence, independent reviews and exact-revision checks. The Ctrl-C fixture false-negative, initial native focus/setup failures and X4/EA1 failures remain recorded; fixing an oracle is not a runtime claim. The [evidence and validation catalogs](../engineering/16-test-and-ci-strategy.md) specify what each check actually proves.

Not implemented/qualified: dozens-agent scheduling/capacity, authenticated model principals, sealed general artifact causality, production isolation, portable complete-state recovery, full compatibility, final renderer/a11y/IME and real-model Namtab mediation. [H0 residual gates](../engine/h0-residual-criteria.md) and [roadmap](../engine/delivery-and-validation.md) own those decisions. Proposed keepers, SQLite and richer schemas must not be described as this prototype's runtime.

[Scoped observations](../engine/scoped-observations.md) owns merged ENG2's public read projections and the merged ENG3 MCP consumer. The latter uses one full discovery then exact session/attempt or freshness-preserving work export, never a partial full snapshot. Native/diagnostic polling and capacity stay unchanged. MCP consumes public contracts and applies export filtering; the broker owns artifact freshness, attribution and canonical state.

The [ENG4 bounded-load draft](../engine/bounded-load.md) adds test/evidence only around those public contracts. It separates preconnected human control from new-human admission at full connection capacity and sampled RSS from enforced allocation; no new scheduler/parser/harness policy follows.

[ENG5 admission draft](../engine/human-admission.md) changes only bounded same-UID transport handoff; main/PR83 do not yet implement it. Four dispatch slots remain. [PTY command visibility](../engine/pty-command-visibility.md) is a separate proposed cooperative shell boundary, not authoritative command attribution.
