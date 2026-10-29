# Bounded fresh local-control admission

[Engine architecture](../architecture/index.md) · [Issue84](https://github.com/whoodes/namtab/issues/84) · [Parent ENG4](bounded-load.md)

**ENG5 draft, stacked on unmerged ENG4/PR83.** Main does not yet have this policy. It changes transport admission, not execution capacities or the `namtab.mk1/1` operation shape. Discover `human_admission_v0` through the existing capability list. The observation/profile versions,2lifetimePTY/2lifetimeattempt/15ordinary+1lifecycle identities and all input/receipt fences remain unchanged.

## Resource and trust boundary

There are **four dispatching connections plus one auxiliary accepted transport** shared by classification or retirement. This is an explicit change from the old four-accepted-transport limit: at most five accepted public socket objects, never five dispatchers. Reserve a permit **before accept**; the kernel listen backlog and duplicated file descriptors are separate. No capacity/fleet/fairness or authentication qualification follows.

Only at saturation, the auxiliary candidate must supply its first newline-terminated frame within an **absolute250ms** and **1024bytes including newline**. Normal below-capacity framing remains1MiB/300s; response writes retain2s timeout. Slow trickles do not renew the deadline. Buffered bytes after a valid Hello stay with that reader and are processed once after binding. Oversize/malformed/timeout/EOF closes the candidate; valid unsupported candidates receive refusal. State acquisition adds at most a further250ms requested wait budget (OS scheduling may overshoot). The acceptor is serial during classification/promotion/error response, so this is bounded best effort, not a priority scheduler.

The Unix socket still checks effective UID and mode0700directory/0600socket. `Human` and restricted `LifecycleHello` are **same-UID declarations, not authenticated humans**. A same-UID agent can claim Human and exhaust protected slots. Admission neither grants writer ownership nor approves tasks/artifacts; ordinary role/epoch/generation/origin checks still apply. Hostile same-UID denial of service and direct OS bypass remain outside this profile.

## Atomic handoff and refusal

1. Validate candidate Hello, role/resume/exclusive binding, identity capacity and entropy before retirement. Fresh nonce acquisition occurs outside registry locks. Before acquiring the admission registry, broker State acquisition has its own absolute250ms budget with1ms polling. Expiry refuses without eviction. No registry lock is held during that wait; it is pre-dispatch lock acquisition, not operation retry.
2. Under one admission mutex, first recheck whether a dispatcher slot became free during classification/State acquisition; use it without eviction. Otherwise choose the oldest accepted **eligible** transport: unbound idle, or declared Agent idle/in one of `Observe`, `ObserveScoped`, `Receipt`. Protect initial binding, known Human/Lifecycle, and every other operation through its response write. Request entry and selection use the same mutex.
3. Mark the victim retired, shut down its socket, promote the candidate and bind its endpoint. Future buffered requests on the victim cannot enter dispatch, even if already read. An already dispatched read may finish; retirement does not cancel past effects or application processes.
4. Keep the auxiliary permit with the retiring handler until its read/write handles drop and **one** broker disconnect completes. Do not eagerly disconnect then repeat it later: that can clear a newly resumed identity. No admission mutex is held while waiting for disconnect State. Only then accept another auxiliary candidate.

Agent candidates at saturation refuse `control_admission_requires_human_or_lifecycle_hello`. Valid control candidates may refuse `control_admission_no_eligible_transport`, `control_admission_state_busy`, invalid/already-connected resume, exhausted identity budget or entropy failure. Invalid candidates do not evict or allocate identity. If all four transports are protected or retirement cannot finish, fresh control is **not guaranteed**. Unknown/partial operations are never retried by this mechanism. The victim's client must explicitly reconnect with its original in-memory resume capability; no request/identity/origin relabeling or input replay.

## Validation and limits

`make human-admission-test` builds the same-worktree binaries and runs `namtab.human-admission/0`: four declared Agent transports, one PTY, two distinct managed jobs, noisy output, an unread consumer with a queued mutation, fresh Human takeover with an exact real append effect, zero-byte stale generation refusal, explicit identity resume, concurrent arrivals, all-protected refusal and bounded malformed/trickled cleanup. Synthetic evidence is explicit test capture despite broker capture-off. No models/GUI or user sessions are involved.

Private Rust tests independently exercise reservation-before-accept, mutation/binding/Human protection, retired-buffer dispatch refusal, broker-busy/invalid/duplicate/exhausted candidates, exact-once cleanup, full-frame bounds, suffix preservation and absolute trickle deadline. The actual process test records **all** concurrent outcomes and sampled deduplicated socket inodes. Samples are not instantaneous peak proof. Finite tests do not prove OS scheduling, hostile-user isolation or universal response latency.

The parent [engine-load/0](https://github.com/whoodes/namtab/pull/83) candidate/evidence stays unchanged. This child uses **engine-load/1** because its post-measurement admission probe now expects a fresh control handoff while refusing a fifth Agent. Numerical service/control/RSS gates and measured workload remain unchanged. Keep the parent failure/gap visible; changing the policy does not retroactively qualify its result.

The first real development journey refused fresh control during ordinary output because an immediate State try-lock collided with a short publication lock. That failure is retained. Bounded lock acquisition without the registry addresses this cause; held-State expiry/no-eviction and contended-then-released tests retain the explicit refusal boundary. No mutation or failed admission is automatically replayed.

Review found and repaired a free-slot-during-Hello race and damaged-evidence false passes. The candidate now rechecks dispatch capacity atomically; tests require the exact stale-writer rejection for an otherwise valid Agent observed input, both expected attempt IDs, and valid correlated concurrent outcomes. Malformed responses/timeouts cannot be counted as benign refusals. Client-close failure injection verifies owned process cleanup still runs. Original review reports and failed aggregate remain retained.
