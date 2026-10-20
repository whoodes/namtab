// Compare ONLY a deliberately projected first-write/transfer subset. This is
// neither a bisimulation nor a model of the real PTY's parser/concurrency.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { isAbsolute } from "node:path";
import { initial, intent, step } from "../models/h0-write-safety/model.mjs";
import { audit } from "../models/h0-write-safety/invariants.mjs";

export function compare(report) {
  assert.equal(report.profile, "h0-pty-once/0+px-dev-text-0");
  const digest = (s) => createHash("sha256").update(s).digest("hex");
  assert.equal(
    report.context_sha256,
    digest(JSON.stringify(report.provenance)),
  );
  assert.equal(
    report.task_sha256,
    digest(
      "h0-pty-once/0: two scripted clients; one context/attempt; handoff; no automatic retry",
    ),
  );
  const p = report.provenance;
  assert.equal(p.version, "namtab.h0-write/0");
  assert.equal(p.profile, report.profile);
  assert.equal(p.capture, "off; explicit synthetic fixture output only");
  assert.equal(p.program, "/bin/bash");
  assert.deepEqual(p.argv, ["--noprofile", "--norc"]);
  assert.deepEqual(p.geometry, [80, 24]);
  assert.ok(
    typeof p.cwd === "string" &&
      isAbsolute(p.cwd) &&
      p.cwd.endsWith("/workspace"),
  );
  assert.deepEqual(p.environment, [
    ["PATH", "/usr/bin:/bin"],
    ["LC_ALL", "C.UTF-8"],
    ["TERM", "dumb"],
    ["HOME", "/nonexistent"],
    ["HISTFILE", "/dev/null"],
    ["INPUTRC", "/dev/null"],
    ["BASH_ENV", "/dev/null"],
    ["ENV", "/dev/null"],
    ["PS1", "h0$ "],
  ]);
  for (const field of ["worker_sha256", "pty_helper_sha256", "program_sha256"])
    assert.match(p[field], /^[0-9a-f]{64}$/u);
  for (const direction of ["request", "response"]) {
    assert.equal(
      p[`${direction}_schema_sha256`],
      digest(
        readFileSync(
          new URL(
            `../../schemas/experimental/h0-write-${direction}.schema.json`,
            import.meta.url,
          ),
        ),
      ),
    );
  }
  assert.ok(
    Number.isInteger(report.output_bytes) &&
      report.output_bytes > 0 &&
      report.output_bytes <= 65536,
  );
  assert.equal(report.replies, 1);
  assert.equal(report.reply_evidence.length, 1);
  assert.equal(report.reply_evidence[0].bytes_hex, "1b5b306e");
  assert.equal(report.reply_evidence[0].delivery, "full");
  assert.equal(report.reply_evidence[0].accepted, 4);
  assert.equal(
    report.reply_evidence[0].source_revision,
    report.reply_evidence[0].output_event + 1,
  );
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
  for (const [r, index, generation] of [
    [fullA, 0, 1],
    [staleA, 0, 1],
    [fullB, 1, 2],
  ]) {
    const o = r.origin;
    assert.equal(o.context, report.context_sha256);
    assert.equal(o.task, report.task_sha256);
    assert.equal(o.attempt, "attempt-0");
    assert.equal(o.session, "session-0");
    assert.equal(o.incarnation, 1);
    assert.equal(o.profile, report.profile);
    assert.equal(o.cols, 80);
    assert.equal(o.rows, 24);
    assert.equal(o.principal, `principal-${index}`);
    assert.equal(o.namespace, `namespace-${index}`);
    assert.equal(o.connection, 1);
    assert.equal(o.control, 1);
    assert.equal(o.capability, 1);
    assert.equal(o.assignment, generation);
    assert.equal(o.writer, generation);
    assert.equal(o.guard, 1);
    assert.match(o.epoch, /^[0-9a-f]{32}$/u);
    for (const field of ["revision", "mode_revision"])
      assert.ok(
        Number.isInteger(o[field]) && o[field] > 0 && o[field] <= 65535,
      );
  }
  assert.equal(fullA.origin.epoch, staleA.origin.epoch);
  assert.equal(fullA.origin.epoch, fullB.origin.epoch);
  assert.equal(fullA.requested, Buffer.byteLength("printf 'H0_A_DONE\\n'"));
  assert.equal(fullB.requested, Buffer.byteLength("printf 'H0_B_DONE\\n'"));
  assert.equal(staleA.requested, Buffer.byteLength("SHOULD_NEVER_BE_WRITTEN"));
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
      r.receipts[0].requested = 0;
      r.receipts[0].accepted = 0;
    },
    (r) => {
      r.receipts.find(
        (x) => x.origin.run === "run-1" && x.sequence === 1,
      ).origin.writer = 1;
    },
    (r) => {
      r.receipts.find(
        (x) => x.origin.run === "run-1" && x.sequence === 1,
      ).origin.session = "session-other";
    },
    (r) => {
      r.provenance.version = "namtab.mk1/1";
      r.provenance.capture = "on";
      r.context_sha256 = createHash("sha256")
        .update(JSON.stringify(r.provenance))
        .digest("hex");
      for (const receipt of r.receipts)
        receipt.origin.context = r.context_sha256;
    },
    (r) => {
      r.provenance.capture = "unreviewed";
    },
    (r) => {
      r.reply_evidence[0].bytes_hex = "1b5b316e";
    },
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
    "PASS: manifest, exact-reply, stale-call, cross-context and false-full corruption sensitivity",
  );
}
