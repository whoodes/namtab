// Compare ONLY a deliberately projected first-write/transfer subset. This is
// neither a bisimulation nor a model of the real PTY's parser/concurrency.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { initial, intent, step } from "../models/h0-write-safety/model.mjs";
import { audit } from "../models/h0-write-safety/invariants.mjs";

export function compare(report) {
  assert.equal(report.profile, "h0-pty-once/0+px-dev-text-0");
  const records = report.receipts;
  assert.ok(Array.isArray(records));
  const fullA = records.find(
    (r) => r.origin.run === "run-0" && r.sequence === 1,
  );
  const staleA = records.find(
    (r) => r.origin.run === "run-0" && r.sequence === 3,
  );
  const fullB = records.find(
    (r) => r.origin.run === "run-1" && r.sequence === 1,
  );
  assert.ok(fullA && staleA && fullB);
  for (const r of [fullA, staleA, fullB]) {
    assert.equal(r.origin.context, report.context_sha256);
    assert.equal(r.origin.task, report.task_sha256);
    assert.equal(r.origin.attempt, "attempt-0");
  }
  assert.equal(fullA.origin.epoch, staleA.origin.epoch);
  assert.equal(fullA.origin.epoch, fullB.origin.epoch);
  assert.notEqual(fullA.origin.principal, fullB.origin.principal);
  assert.notEqual(fullA.origin.namespace, fullB.origin.namespace);
  assert.equal(fullA.origin.assignment, staleA.origin.assignment);
  assert.ok(fullB.origin.assignment > staleA.origin.assignment);
  let state = initial();
  const apply = (event) => {
    const transition = step(state, event);
    audit(state, transition);
    state = transition.state;
    assert.ok(transition.result.ok, JSON.stringify(event));
    return transition.result.value;
  };
  apply({
    type: "claim",
    to: "RA",
    lifecycle: 1,
    generation: 1,
    attempt: "A1",
    context: "C1",
  });
  const submit = (actor, sequence) =>
    apply({
      type: "submit",
      actor,
      request: intent(state, actor, sequence, "abc"),
    }).operation;
  const finish = (id) => {
    apply({ type: "authorize-write", operation: id });
    apply({
      type: "write-result",
      operation: id,
      call: state.operations[id].flight.id,
      count: 3,
      terminal: true,
    });
  };
  const a = submit("RA", 1);
  finish(a);
  const stale = submit("RA", 2);
  apply({
    type: "transfer",
    from: "RA",
    to: "RB",
    lifecycle: 2,
    generation: 2,
    attempt: "A1",
    context: "C1",
  });
  apply({ type: "takeover", to: "RB" });
  // A model refusal still needs an audit; it must not invent a call.
  const denied = step(state, { type: "authorize-write", operation: stale });
  audit(state, denied);
  assert.equal(denied.result.ok, false);
  state = denied.state;
  const b = submit("RB", 1);
  finish(b);
  for (const [real, model] of [
    [fullA, state.operations[a]],
    [staleA, state.operations[stale]],
    [fullB, state.operations[b]],
  ]) {
    assert.equal(real.delivery, model.delivery);
    assert.equal(real.calls, model.calls);
    if (real.delivery === "full") assert.equal(real.accepted, real.requested);
    else {
      assert.equal(real.accepted, 0);
      assert.equal(real.calls, 0);
    }
    assert.equal(
      real.origin.run,
      model.origin.run === "RA" ? "run-0" : "run-1",
    );
  }
  return "PASS: 3 real receipts vs audited first-call/transfer projection; no suffix/rearm/OS-state equivalence";
}
if (process.argv[1]?.endsWith("compare-model.mjs") && process.argv[2]) {
  const report = JSON.parse(readFileSync(process.argv[2], "utf8"));
  console.log(compare(report));
  for (const change of [
    (r) => {
      r.receipts.find(
        (x) => x.origin.run === "run-0" && x.sequence === 3,
      ).calls = 1;
    },
    (r) => {
      r.receipts.find(
        (x) => x.origin.run === "run-1" && x.sequence === 1,
      ).origin.context = "wrong";
    },
    (r) => {
      r.receipts[0].accepted = 0;
    },
  ]) {
    const corrupted = structuredClone(report);
    change(corrupted);
    assert.throws(
      () => compare(corrupted),
      "comparison missed intentional corruption",
    );
  }
  console.log(
    "PASS: stale-call, cross-context and false-full corruption sensitivity",
  );
}
