// Independent obligation audit: imports no model helper or guard. Also challenged
// with deliberately broken model variants and hand-forged transition evidence.
import assert from "node:assert/strict";

export function audit(before, transition) {
  const { state: after, event } = transition;
  const callsBefore = Object.values(before.operations).reduce(
    (n, o) => n + o.calls,
    0,
  );
  const callsAfter = Object.values(after.operations).reduce(
    (n, o) => n + o.calls,
    0,
  );
  assert.equal(
    callsAfter - callsBefore,
    event.authorized ? 1 : 0,
    "unrecorded/duplicate call",
  );
  for (const [id, op] of Object.entries(after.operations)) {
    const previous = before.operations[id];
    assert.equal(
      op.calls - (previous?.calls ?? 0),
      event.authorized?.operation === id ? 1 : 0,
      "call counter attributed to another operation",
    );
    if (!previous?.flight && op.flight)
      assert.equal(event.authorized?.operation, id, "unlogged flight");
  }
  if (event.authorized) {
    const { operation, ...token } = event.authorized;
    assert.equal(
      event.type,
      "authorize-write",
      "call outside write authorization",
    );
    assert.equal(event.operation, operation, "action/event operation mismatch");
    assert.equal(
      after.active,
      operation,
      "authorized operation differs from queue owner",
    );
    assert.deepEqual(
      after.operations[operation]?.flight,
      token,
      "event/state call token mismatch",
    );
    assert.equal(
      after.operations[operation]?.phase,
      "in-flight",
      "authorized call not in flight",
    );
    assert.equal(
      transition.result.ok,
      true,
      "authorized call reported refusal",
    );
    assert.deepEqual(
      transition.result.value,
      token,
      "returned/event call token mismatch",
    );
  }
  for (const [id, old] of Object.entries(before.operations)) {
    const next = after.operations[id];
    assert.ok(next, "operation forgotten");
    assert.deepEqual(next.origin, old.origin, "origin relabeled");
    if (!old.reply)
      assert.deepEqual(next.binding, old.binding, "intent binding changed");
    if (next.known > old.known) {
      assert.equal(event.type, "write-result", "prefix without result");
      assert.equal(event.operation, id, "cross-attributed result");
      assert.equal(event.call, old.flight?.id, "result for wrong call");
      assert.equal(
        next.known - old.known,
        event.count,
        "invented delivery extent",
      );
    }
    if (old.flight && next.flight)
      assert.deepEqual(next.flight, old.flight, "in-flight token changed");
    if (old.flight && !next.flight) {
      assert.equal(
        event.type,
        "write-result",
        "call disappeared without result",
      );
      assert.equal(event.operation, id, "call completed by another operation");
      assert.equal(
        event.call,
        old.flight.id,
        "call completed by another token",
      );
    }
    if (old.delivery === "unknown" && old.phase === "closed")
      assert.equal(next.delivery, "unknown", "closed unknown became certainty");
    if (old.flight && event.operation === id && event.type === "lose-ack") {
      assert.equal(next.phase, "closing", "lost ACK reopened dispatch");
      assert.equal(next.delivery, "unknown", "lost ACK became certainty");
      assert.deepEqual(
        next.flight,
        old.flight,
        "lost ACK released unresolved call",
      );
    }
    if (
      old.flight &&
      event.operation === id &&
      event.type === "write-result" &&
      event.call === old.flight.id
    ) {
      const validCount =
        event.count === "unknown" ||
        (Number.isInteger(event.count) &&
          event.count >= 0 &&
          event.count <= old.flight.length);
      const validTerminal =
        event.terminal === undefined || typeof event.terminal === "boolean";
      if (validCount && validTerminal) {
        assert.equal(transition.result.ok, true, "valid result lost");
        assert.equal(next.flight, null, "result did not retire call token");
        if (event.count === "unknown") {
          assert.equal(
            next.delivery,
            "unknown",
            "unknown result became certainty",
          );
          assert.equal(
            next.known,
            old.known,
            "unknown invented a proven prefix",
          );
          assert.equal(next.phase, "closed", "unknown result reopened suffix");
        } else {
          assert.equal(
            next.known,
            old.known + event.count,
            "result extent differs",
          );
          if (
            old.phase === "closing" ||
            event.terminal === true ||
            next.known === old.bytes.length
          ) {
            const expected =
              next.known === old.bytes.length && next.known > 0
                ? "full"
                : next.known > 0
                  ? "partial"
                  : "zero";
            assert.equal(
              next.phase,
              "closed",
              "terminal result reopened suffix",
            );
            assert.equal(next.delivery, expected, "known result mislabeled");
          } else {
            assert.equal(
              next.phase,
              "suffix",
              "nonterminal result lost suffix",
            );
            assert.equal(
              next.delivery,
              "pending",
              "pending result became certainty",
            );
          }
        }
      } else {
        assert.equal(transition.result.ok, false, "malformed result accepted");
        assert.deepEqual(next, old, "malformed result changed operation");
      }
    }
    assert.equal(next.bytes, old.bytes, "frozen payload changed");
    assert.ok(
      next.known >= old.known && next.known <= next.bytes.length,
      "prefix regressed/overflow",
    );
    if (old.phase === "closed")
      assert.equal(next.calls, old.calls, "closed effect reopened");
  }
  assert.ok(
    Object.values(after.operations).filter((o) => o.flight).length <= 1,
    "interleaved unresolved calls",
  );
  for (const o of Object.values(after.operations)) {
    if (o.flight)
      assert.equal(
        after.active,
        o.id,
        "in-flight operation lost queue ownership",
      );
    if (["partial", "zero", "full"].includes(o.delivery)) {
      assert.equal(o.phase, "closed", "terminal delivery before closure");
      assert.ok(!o.flight, "certainty with unresolved call");
    }
    if (o.delivery === "partial")
      assert.ok(o.known > 0 && o.known < o.bytes.length && !o.flight);
    if (o.delivery === "zero") assert.equal(o.known, 0);
    if (o.delivery === "full") assert.equal(o.known, o.bytes.length);
  }
  if (!event.authorized) return;
  const op = before.operations[event.authorized.operation];
  assert.ok(
    op && ["queued", "suffix"].includes(op.phase),
    "call from invalid phase",
  );
  assert.ok(
    !before.active || before.active === op.id,
    "another byte lane owns queue",
  );
  assert.ok(
    !before.session.tainted && before.session.known && op.retained,
    "unsafe continuation",
  );
  assert.ok(before.now < op.deadline, "expired dispatch");
  assert.equal(op.epoch, before.epoch, "old authority epoch");
  assert.equal(op.guard, before.session.guard, "stale session context");
  assert.equal(
    event.authorized.start,
    op.known,
    "suffix skipped/repeated bytes",
  );
  assert.equal(
    event.authorized.length,
    op.bytes.length - op.known,
    "suffix re-encoded",
  );
  if (op.reply) return; // Canonical reply does not borrow an agent writer/run.
  const b = op.binding,
    r = before.runs[b.run],
    a = before.assignment;
  assert.ok(r?.open && r.permitted, "closed/revoked run dispatched");
  assert.equal(b.control, r.control, "stale run control");
  assert.equal(b.capability, r.capability, "stale capability");
  assert.equal(b.epoch, before.epoch, "stale binding epoch");
  assert.equal(a.holder, b.run, "wrong assignment holder");
  assert.equal(b.assignment, a.generation, "stale assignment");
  assert.equal(b.attempt, a.attempt, "retired attempt");
  assert.equal(b.task, a.task, "repinned task");
  assert.equal(b.context, a.context, "repinned context");
  assert.equal(r.context, a.context, "run context differs");
  assert.equal(before.writer.holder, b.run, "wrong writer");
  assert.equal(b.writer, before.writer.generation, "stale writer");
}
