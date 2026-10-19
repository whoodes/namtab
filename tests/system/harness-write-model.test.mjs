import test from "node:test";
import assert from "node:assert/strict";
import {
  initial,
  intent,
  step,
  recoveryAction,
  BOUNDS,
} from "../models/h0-write-safety/model.mjs";
import { audit } from "../models/h0-write-safety/invariants.mjs";

function rig(fault = "none") {
  let state = initial(fault);
  const history = [];
  return {
    get state() {
      return structuredClone(state);
    },
    do(action, check = true) {
      const before = state,
        t = step(before, action);
      if (check) audit(before, t);
      history.push(t);
      state = t.state;
      return t.result;
    },
    get history() {
      return history;
    },
  };
}
const claim = (to = "RA", lifecycle = 1, generation = 1) => ({
  type: "claim",
  to,
  lifecycle,
  generation,
  attempt: "A1",
  context: "C1",
});
const transfer = {
  type: "transfer",
  from: "RA",
  to: "RB",
  lifecycle: 2,
  generation: 2,
  attempt: "A1",
  context: "C1",
};
function assigned(fault) {
  const r = rig(fault);
  assert.ok(r.do(claim()).ok);
  return r;
}
function submit(r, request = intent(r.state)) {
  const result = r.do({ type: "submit", actor: request.binding.run, request });
  assert.equal(result.value.admission, "admitted");
  return result.value.operation;
}
function call(r, id = "O1") {
  return r.do({ type: "authorize-write", operation: id });
}
function finish(r, count, { id = "O1", terminal = false } = {}) {
  return r.do({
    type: "write-result",
    operation: id,
    call: r.state.operations[id].flight?.id,
    count,
    terminal,
  });
}

// Goldens assert observable decisions, not a second call to model eligibility.
test("exclusive claim CAS, compatible transfer, release and stale attempt pins", () => {
  const r = assigned();
  assert.equal(r.do(claim("RB")).reason, "stale-assignment");
  const old = intent(r.state);
  assert.ok(r.do(transfer).ok);
  assert.equal(r.state.assignment.attempt, "A1");
  assert.deepEqual(r.do({ type: "submit", actor: "RA", request: old }).value, {
    admission: "rejected",
    reason: "stale-or-denied",
  });
  assert.ok(r.do({ type: "takeover", to: "RB" }).ok);
  const fresh = intent(r.state, "RB");
  const id = submit(r, fresh);
  assert.ok(call(r, id).ok);
  finish(r, 3, { id });
  assert.equal(r.state.operations[id].origin.run, "RB");
  assert.ok(r.do({ type: "replace-attempt", lifecycle: 3 }).ok);
  assert.equal(r.state.assignment.attempt, "A2");
  const stale = { ...intent(r.state, "RB", 2), binding: { ...fresh.binding } };
  assert.equal(
    r.do({ type: "submit", actor: "RB", request: stale }).value.admission,
    "rejected",
  );
  assert.ok(
    r.do({
      type: "release",
      from: "RB",
      lifecycle: 4,
      generation: 4,
      attempt: "A2",
      context: "C1",
    }).ok,
  );
  assert.equal(r.state.assignment.holder, null);
  assert.equal(r.state.operations[id].delivery, "full");
});

test("issued-key decisions dedupe, conflict and missing binding never dispatch again", () => {
  const r = assigned();
  const request = intent(r.state);
  const id = submit(r, request);
  const original = structuredClone(request);
  assert.equal(
    r.do({ type: "submit", actor: "RA", request }).value.operation,
    id,
  );
  assert.equal(Object.keys(r.state.operations).length, 1);
  const changed = { ...request, bytes: "xyz" };
  assert.equal(
    r.do({ type: "submit", actor: "RA", request: changed }).reason,
    "binding-conflict",
  );
  assert.ok(call(r).ok);
  finish(r, 3);
  assert.equal(
    r.do({ type: "submit", actor: "RA", request }).value.operation,
    id,
  );
  assert.equal(call(r).reason, "not-dispatchable");
  r.do({ type: "forget-binding", key: "NA/1/1" });
  assert.equal(
    r.do({ type: "submit", actor: "RA", request: original }).reason,
    "binding-unavailable",
  );
  assert.equal(r.state.namespaces.NA.consumed, 1);
  assert.equal(r.do({ type: "query", key: "NA/1/1" }).value.delivery, "full");
});

test("preflight does not consume; stale valid decisions do; old epoch queries only", () => {
  const r = assigned();
  const request = intent(r.state);
  for (const patch of [
    { sequence: 2 },
    { namespace: "unknown" },
    { extra: true },
    { binding: {} },
    { bytes: "a".repeat(4) },
    { binding: { ...request.binding, epoch: 2 } },
  ]) {
    const t = r.do({
      type: "submit",
      actor: "RA",
      request: { ...request, ...patch },
    });
    // A well-shaped stale epoch binding is a consumed subject rejection.
    if (patch.binding?.epoch === 2) assert.equal(t.value.admission, "rejected");
    else {
      assert.equal(t.reason, "preflight");
      assert.equal(r.state.namespaces.NA.consumed, 0);
    }
  }
  assert.equal(r.state.namespaces.NA.consumed, 1);
  const current = intent(r.state, "RA", 2);
  submit(r, current);
  r.do({ type: "epoch-change" });
  assert.equal(
    r.do({ type: "submit", actor: "RA", request: current }).reason,
    "preflight",
  );
  assert.equal(r.do({ type: "query", key: "NA/1/2" }).value.delivery, "zero");
});

test("fences before first call prove zero; each suffix independently checks every modeled fence", () => {
  const controls = [
    transfer,
    { type: "close-run", run: "RA" },
    { type: "revoke-capability", run: "RA" },
    { type: "takeover", to: "H" },
    { type: "replace-attempt", lifecycle: 2 },
    { type: "session-fence" },
    { type: "epoch-change" },
    { type: "disconnect", run: "RA" },
  ];
  for (const control of controls)
    for (const partial of [false, true]) {
      const r = assigned();
      const id = submit(r);
      if (partial) {
        call(r);
        finish(r, 1);
      }
      assert.ok(r.do(control).ok);
      assert.equal(call(r).ok, false);
      assert.equal(
        r.state.operations[id].delivery,
        partial ? "partial" : "zero",
      );
      assert.equal(r.state.operations[id].known, partial ? 1 : 0);
      assert.equal(r.state.operations[id].origin.run, "RA");
    }
});

test("in-flight takeover is not rollback; late results refine original receipt without a suffix", () => {
  for (const count of [0, 1, 3, "unknown"]) {
    const r = assigned();
    submit(r);
    call(r);
    r.do({ type: "takeover", to: "H" });
    assert.equal(r.state.operations.O1.phase, "closing");
    assert.equal(r.state.operations.O1.delivery, "unknown");
    r.do({ type: "queue-reply", query: "Q1" });
    assert.equal(call(r, "reply:Q1").reason, "queue-busy");
    const callId = r.state.operations.O1.flight.id;
    finish(r, count);
    assert.equal(
      r.state.operations.O1.delivery,
      count === "unknown"
        ? "unknown"
        : count === 0
          ? "zero"
          : count === 3
            ? "full"
            : "partial",
    );
    assert.equal(call(r).ok, false);
    assert.equal(
      r.do({ type: "write-result", operation: "O1", call: callId, count: 3 })
        .reason,
      "no-matching-call",
    );
    assert.equal(r.state.operations.O1.calls, 1);
  }
});

test("partial suffix retains frozen offset and queue; replies are independent but cannot interleave", () => {
  const r = assigned();
  submit(r);
  call(r);
  finish(r, 1);
  r.do({ type: "queue-reply", query: "Q1" });
  assert.equal(call(r, "reply:Q1").reason, "queue-busy");
  assert.deepEqual(call(r).value, { id: "O1:2", start: 1, length: 2 });
  finish(r, 2);
  assert.equal(r.state.operations.O1.delivery, "full");
  r.do({ type: "close-run", run: "RA" });
  assert.ok(call(r, "reply:Q1").ok);
  finish(r, 1, { id: "reply:Q1" });
  assert.equal(r.do({ type: "queue-reply", query: "Q1" }).value, "reply:Q1");
  assert.equal(call(r, "reply:Q1").ok, false);
  assert.equal(Object.keys(r.state.operations).length, 2);
});

test("lost ACK holds queue and query-only recovery; abandoned prefix needs explicit fresh continuation", () => {
  const r = assigned();
  const req = intent(r.state);
  submit(r, req);
  call(r);
  r.do({ type: "lose-ack", operation: "O1" });
  const unknown = r.do({ type: "query", key: "NA/1/1" });
  assert.equal(recoveryAction(unknown), "query");
  assert.equal(
    recoveryAction({ ok: false, reason: "unavailable-unknown" }),
    "query",
  );
  assert.equal(
    r.do({ type: "submit", actor: "RA", request: req }).value.operation,
    "O1",
  );
  assert.equal(call(r).ok, false);
  r.do({ type: "queue-reply", query: "Q1" });
  assert.equal(call(r, "reply:Q1").reason, "queue-busy");
  assert.equal(
    r.do({ type: "continue-after-abandon", fresh: true, toleratesRisk: true })
      .ok,
    false,
  );
  finish(r, 1);
  assert.equal(r.state.operations.O1.delivery, "partial");
  assert.equal(call(r, "reply:Q1").reason, "continuation-required");
  for (const fields of [{}, { fresh: true }, { toleratesRisk: true }])
    assert.equal(r.do({ type: "continue-after-abandon", ...fields }).ok, false);
  r.do({ type: "continue-after-abandon", fresh: true, toleratesRisk: true });
  assert.equal(call(r, "reply:Q1").ok, false); // Old context cannot revive.
  r.do({ type: "queue-reply", query: "Q2" });
  assert.ok(call(r, "reply:Q2").ok);
  // A late duplicate does not alter the prefix or relabel it as a reply.
  assert.equal(
    r.do({ type: "write-result", operation: "O1", call: "O1:1", count: 3 }).ok,
    false,
  );
  assert.equal(r.state.operations.O1.known, 1);
});

test("deadline equality, no-progress bounded retry, empty input and reconnect are explicit", () => {
  const r = assigned();
  submit(r);
  call(r);
  finish(r, 0);
  for (let n = 1; n < BOUNDS.calls; n++) {
    call(r);
    finish(r, 0);
  }
  assert.equal(call(r).reason, "fenced-or-bounded");
  assert.equal(r.state.operations.O1.delivery, "zero");
  const e = assigned();
  const empty = submit(e, intent(e.state, "RA", 1, ""));
  assert.equal(call(e, empty).value, "empty-zero");
  assert.equal(e.state.operations[empty].calls, 0);
  const d = assigned();
  submit(d);
  for (let i = 0; i < 5; i++) d.do({ type: "tick" });
  assert.equal(d.state.operations.O1.delivery, "zero");
  assert.equal(call(d).ok, false);
  const c = assigned();
  const old = intent(c.state);
  submit(c, old);
  c.do({ type: "disconnect", run: "RA" });
  c.do({ type: "reconnect", run: "RA" });
  assert.equal(c.state.assignment.holder, "RA");
  assert.equal(call(c).ok, false);
  assert.equal(c.state.namespaces.NA.consumed, 1);
});

test("namespace ownership and valid sequence1 for another principal do not cross-attribute", () => {
  const r = assigned();
  const ra = intent(r.state);
  assert.equal(
    r.do({ type: "submit", actor: "RB", request: ra }).reason,
    "preflight",
  );
  assert.equal(r.state.namespaces.NA.consumed, 0);
  const first = submit(r, ra);
  call(r, first);
  finish(r, 3, { id: first });
  r.do(transfer);
  r.do({ type: "takeover", to: "RB" });
  const second = submit(r, intent(r.state, "RB", 1));
  assert.equal(r.state.namespaces.NA.consumed, 1);
  assert.equal(r.state.namespaces.NB.consumed, 1);
  assert.notEqual(first, second);
  assert.equal(r.state.operations[first].origin.run, "RA");
  assert.equal(r.state.operations[second].origin.run, "RB");
});

test("independent obligation audit kills run, assignment and suffix-check omissions", () => {
  for (const [fault, control] of [
    ["skip-run", { type: "close-run", run: "RA" }],
    ["skip-assignment", transfer],
    ["skip-suffix", { type: "takeover", to: "H" }],
  ]) {
    const r = assigned(fault);
    submit(r);
    call(r);
    finish(r, 1);
    r.do(control);
    assert.throws(
      () => call(r),
      /closed\/revoked|wrong assignment|wrong writer/,
    );
  }
  const r = assigned();
  submit(r);
  const before = r.state;
  const forged = step(before, { type: "authorize-write", operation: "O1" });
  forged.event.authorized.start = 1;
  assert.throws(
    () => audit(before, forged),
    /event\/state call token mismatch/,
  );
});

test("bounded graph explores interleavings and nondeterministic result choices", (t) => {
  const start = assigned();
  submit(start);
  const frontier = [{ state: start.state, trace: [] }];
  const seen = new Set();
  let edges = 0,
    calls = 0,
    partials = 0,
    unknowns = 0;
  for (let index = 0; index < frontier.length; index++) {
    const { state, trace } = frontier[index];
    if (trace.length === 6) continue;
    const flight = state.operations.O1.flight;
    const choices = [
      { type: "authorize-write", operation: "O1" },
      { type: "close-run", run: "RA" },
      transfer,
      { type: "takeover", to: "H" },
      { type: "revoke-capability", run: "RA" },
      ...(flight
        ? [...new Set([0, 1, flight.length, "unknown"])].map((count) => ({
            type: "write-result",
            operation: "O1",
            call: flight.id,
            count,
          }))
        : []),
      ...(flight ? [{ type: "lose-ack", operation: "O1" }] : []),
    ];
    for (const event of choices) {
      const next = step(state, event);
      edges++;
      try {
        audit(state, next);
      } catch (error) {
        error.message += `\ntrace=${JSON.stringify([...trace, event])}`;
        throw error;
      }
      calls += Number(!!next.event.authorized);
      partials += Number(next.state.operations.O1.delivery === "partial");
      unknowns += Number(next.state.operations.O1.delivery === "unknown");
      const key = JSON.stringify([trace.length + 1, next.state]);
      if (!seen.has(key)) {
        seen.add(key);
        frontier.push({ state: next.state, trace: [...trace, event] });
      }
      assert.ok(
        frontier.length < 20000,
        "exploration budget exceeded: fail, never silently truncate",
      );
    }
  }
  assert.ok(calls > 1 && partials > 0 && unknowns > 0 && frontier.length > 100);
  t.diagnostic(
    JSON.stringify({
      depth: 6,
      depthIndexedStatesIncludingInitial: frontier.length,
      transitions: edges,
      authorizedEdges: calls,
      partialEdges: partials,
      unknownEdges: unknowns,
      limit: 20000,
      property: "safety only, finite depth/state assumptions",
    }),
  );
});

test("missing receipt never certifies no effect; consumed identity cannot be reused", () => {
  for (const inFlight of [false, true]) {
    const r = assigned();
    const request = intent(r.state);
    submit(r, request);
    if (inFlight) call(r);
    r.do({ type: "forget-receipt", operation: "O1" });
    assert.equal(
      r.do({ type: "query", key: "NA/1/1" }).reason,
      "unavailable-unknown",
    );
    assert.equal(
      r.do({ type: "submit", actor: "RA", request }).reason,
      "binding-unavailable",
    );
    assert.equal(r.state.namespaces.NA.consumed, 1);
    assert.equal(call(r).ok, false);
    if (inFlight) {
      assert.equal(r.state.operations.O1.delivery, "unknown");
      finish(r, 1);
      assert.equal(
        r.do({ type: "query", key: "NA/1/1" }).reason,
        "unavailable-unknown",
      );
    }
  }
});

test("new keys are distinct intents, not server-side semantic dedupe or safe automatic retries", () => {
  const r = assigned();
  submit(r);
  call(r);
  r.do({ type: "lose-ack", operation: "O1" });
  assert.equal(recoveryAction(r.do({ type: "query", key: "NA/1/1" })), "query");
  const second = submit(r, intent(r.state, "RA", 2));
  assert.notEqual(second, "O1");
  assert.equal(call(r, second).reason, "queue-busy");
  finish(r, "unknown");
  assert.equal(call(r, second).reason, "continuation-required");
  r.do({ type: "continue-after-abandon", fresh: true, toleratesRisk: true });
  assert.equal(call(r, second).ok, false); // Explicit re-arm invalidates old intents.
  assert.equal(r.state.operations.O1.delivery, "unknown");
  const fresh = submit(r, intent(r.state, "RA", 3));
  assert.ok(call(r, fresh).ok); // A newly granted action does not erase old uncertainty.
});

test("forged evidence cannot change a binding, invent certainty or attribute another call's result", () => {
  const r = assigned();
  submit(r);
  call(r);
  const before = r.state;
  const result = step(before, {
    type: "write-result",
    operation: "O1",
    call: "O1:1",
    count: 1,
  });
  const bindingChange = structuredClone(result);
  bindingChange.state.operations.O1.binding.control++;
  assert.throws(() => audit(before, bindingChange), /intent binding changed/);
  const crossResult = structuredClone(result);
  crossResult.event.operation = "O2";
  assert.throws(() => audit(before, crossResult), /cross-attributed result/);
  const certainty = step(before, { type: "lose-ack", operation: "O1" });
  certainty.state.operations.O1.delivery = "zero";
  assert.throws(() => audit(before, certainty), /lost ACK became certainty/);
  for (const count of [-1, 4, 0.5, null]) {
    const bad = step(before, {
      type: "write-result",
      operation: "O1",
      call: "O1:1",
      count,
    });
    assert.equal(bad.result.ok, false);
    assert.deepEqual(bad.state, before);
  }
});

test("closed runs do not reopen on reconnect; finite sequence/generation bounds never wrap", () => {
  const r = assigned();
  r.do({ type: "close-run", run: "RA" });
  r.do({ type: "disconnect", run: "RA" });
  r.do({ type: "reconnect", run: "RA" });
  assert.equal(r.state.runs.RA.open, false);
  assert.equal(
    r.do({ type: "submit", actor: "RA", request: intent(r.state) }).value
      .admission,
    "rejected",
  );
  const q = assigned();
  for (let n = 1; n <= 4; n++) {
    const id = submit(q, intent(q.state, "RA", n, ""));
    call(q, id);
  }
  assert.equal(
    q.do({ type: "submit", actor: "RA", request: intent(q.state, "RA", 5) })
      .reason,
    "preflight",
  );
  assert.equal(q.state.namespaces.NA.consumed, 4);
  for (let n = 1; n < 8; n++) assert.ok(q.do({ type: "takeover", to: "H" }).ok);
  const exhausted = q.state;
  assert.equal(q.do({ type: "takeover", to: "RA" }).ok, false);
  assert.deepEqual(q.state, exhausted);
});

test("finite request alphabets reject oversized/unknown references without retaining or consuming", () => {
  const r = assigned();
  const base = intent(r.state);
  for (const field of ["attempt", "task", "context", "run"])
    for (const value of ["unknown", "x".repeat(8192)]) {
      const request = { ...base, binding: { ...base.binding, [field]: value } };
      assert.equal(
        r.do({ type: "submit", actor: "RA", request }).reason,
        "preflight",
      );
      assert.equal(r.state.namespaces.NA.consumed, 0);
      assert.deepEqual(r.state.bindings, {});
    }
  for (const patch of [{ namespace: ["NA"] }, { epoch: 9 }])
    assert.equal(
      r.do({ type: "submit", actor: "RA", request: { ...base, ...patch } })
        .reason,
      "preflight",
    );
  assert.equal(submit(r, base), "O1");
});

test("auditor rejects event/state token mismatch and cross-operation authorization", () => {
  const r = assigned();
  submit(r);
  submit(r, intent(r.state, "RA", 2));
  const before = r.state;
  const good = step(before, { type: "authorize-write", operation: "O1" });
  audit(before, good);
  const wrongOffset = structuredClone(good);
  wrongOffset.state.operations.O1.flight.start = 1;
  wrongOffset.state.operations.O1.flight.length = 2;
  assert.throws(
    () => audit(before, wrongOffset),
    /event\/state call token mismatch/,
  );
  const other = step(before, { type: "authorize-write", operation: "O2" });
  audit(before, other);
  other.event.authorized.operation = "O1";
  other.event.authorized.id = "O1:1";
  assert.throws(
    () => audit(before, other),
    /another operation|operation mismatch/,
  );
  const wrongReturn = structuredClone(good);
  wrongReturn.result.value.start = 1;
  assert.throws(
    () => audit(before, wrongReturn),
    /returned\/event call token mismatch/,
  );
});

test("unknown syscall evidence cannot be forged into zero or partial certainty", () => {
  for (const prefix of [0, 1]) {
    const r = assigned();
    submit(r);
    call(r);
    if (prefix) {
      finish(r, 1);
      call(r);
    }
    const before = r.state;
    const unknown = step(before, {
      type: "write-result",
      operation: "O1",
      call: before.operations.O1.flight.id,
      count: "unknown",
    });
    audit(before, unknown);
    unknown.state.operations.O1.delivery = prefix ? "partial" : "zero";
    assert.throws(
      () => audit(before, unknown),
      /unknown result became certainty/,
    );
  }
});

test("no-call or refused results cannot invent a proven full prefix", () => {
  const r = assigned();
  submit(r);
  const before = r.state;
  const refused = step(before, {
    type: "write-result",
    operation: "O1",
    count: 3,
  });
  assert.equal(refused.result.ok, false);
  audit(before, refused);
  for (const pretendSuccess of [false, true]) {
    const forged = structuredClone(refused);
    forged.state.operations.O1.known = 3;
    forged.state.operations.O1.phase = "closed";
    forged.state.operations.O1.delivery = "full";
    if (pretendSuccess) forged.result = { ok: true, value: "result-recorded" };
    assert.throws(
      () => audit(before, forged),
      pretendSuccess
        ? /delivery without authorized call/
        : /refused write result changed state/,
    );
  }
});
