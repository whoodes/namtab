// Test-only finite design oracle. No production imports or external effects.
// Hand-authored reported traces can be internally consistent yet never occur in
// the real runtime; passing these checks is not runtime or model qualification.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const fixture = JSON.parse(
  readFileSync(
    new URL("../fixtures/harness-h0/coordination.json", import.meta.url),
  ),
);
const clone = () => structuredClone(fixture);
const decimal = (value) => {
  assert.match(value, /^(0|[1-9][0-9]{0,19})$/);
  const n = BigInt(value);
  assert.ok(n <= 18446744073709551615n);
  return n;
};

function checkAttribution(example) {
  assert.ok(example.launches.length <= 4 && example.streams.length <= 16);
  const operations = new Set();
  const executions = new Map();
  const keys = new Set();
  for (const launch of example.launches) {
    for (const key of [
      "run",
      "requestNamespace",
      "operation",
      "attempt",
      "execution",
      "taskRevision",
      "contextRevision",
    ])
      assert.equal(typeof launch[key], "string");
    assert.ok(decimal(launch.sequence) > 0n);
    const key = JSON.stringify([launch.requestNamespace, launch.sequence]);
    assert.ok(!keys.has(key), "one issued key cannot create two launches");
    assert.ok(
      !operations.has(launch.operation),
      "operation identity collision",
    );
    assert.ok(
      !executions.has(launch.execution),
      "execution identity collision",
    );
    keys.add(key);
    operations.add(launch.operation);
    executions.set(launch.execution, launch);
  }
  const ends = new Map();
  for (const event of example.streams) {
    const launch = executions.get(event.execution);
    assert.ok(launch, "unknown execution cannot borrow the latest launch");
    assert.equal(event.attempt, launch.attempt, "cross-attributed stream");
    assert.ok(["stdout", "stderr"].includes(event.stream));
    const stream = JSON.stringify([event.execution, event.stream]);
    const previous = ends.get(stream) ?? 0n;
    const start = decimal(event.offset);
    assert.ok(start >= previous, "overlapping/backward stream range");
    assert.equal(
      event.gap,
      start > previous,
      "missing range must be disclosed",
    );
    ends.set(stream, start + decimal(event.length));
    assert.ok(ends.get(stream) <= 18446744073709551615n);
  }
  return ends;
}

function checkDependency(evidence) {
  assert.equal(evidence.selectedArtifact, evidence.validationArtifact);
  assert.equal(evidence.selectedContext, evidence.validationContext);
  assert.equal(evidence.validationState, "passed");
  assert.equal(
    evidence.factSource,
    "validator",
    "agent report is not validation",
  );
  assert.equal(evidence.quality, evidence.requiredQuality);
}

function checkWriter(trace) {
  let generation = 1n;
  let owner = "RA";
  for (const event of trace) {
    const observed = decimal(event.generation);
    if (event.kind === "takeoverAcknowledged") {
      assert.ok(observed > generation, "generation cannot wrap/reopen");
      generation = observed;
      owner = event.owner;
    } else {
      assert.equal(event.kind, "input");
      if (observed !== generation || event.owner !== owner) {
        assert.equal(event.disposition, "refused");
        assert.equal(event.effect, "zero");
      } else {
        assert.ok(["admitted", "refused"].includes(event.disposition));
        if (event.disposition === "refused") assert.equal(event.effect, "zero");
      }
    }
  }
  // Deliberately no process-stop result: this is a queued first-write subset,
  // not an in-flight/partial OS-delivery model or opaque-application oracle.
}

test("distinct issued namespaces can use sequence1 with independent origins", () => {
  assert.equal(fixture.profile, "harness-contract-review/0");
  assert.equal(fixture.launches[0].sequence, fixture.launches[1].sequence);
  const ends = checkAttribution(fixture);
  assert.equal(ends.get('["E1","stdout"]'), 5n);
  assert.equal(ends.get('["E2","stderr"]'), 4n);
  const collision = clone();
  collision.launches[1].requestNamespace = "req-A";
  assert.throws(() => checkAttribution(collision), /two launches/);
});

test("per-stream attribution survives cross-stream reorder; gaps stay explicit", () => {
  const reordered = clone();
  reordered.streams = [
    fixture.streams[1],
    fixture.streams[2],
    fixture.streams[0],
    fixture.streams[3],
  ];
  assert.deepEqual(checkAttribution(reordered), checkAttribution(fixture));
  const wrong = clone();
  wrong.streams[1].attempt = "A1";
  assert.throws(() => checkAttribution(wrong), /cross-attributed/);
  const missing = clone();
  missing.streams[3].offset = "9";
  assert.throws(() => checkAttribution(missing), /disclosed/);
  missing.streams[3].gap = true;
  checkAttribution(missing);
});

test("exact dependency evidence cannot follow a changed selection or agent claim", () => {
  checkDependency(fixture.dependency);
  for (const patch of [
    { selectedArtifact: "R2" },
    { selectedContext: "CA@2" },
    { factSource: "agent" },
    { validationState: "unknown" },
    { requiredQuality: "sealed" },
  ])
    assert.throws(() => checkDependency({ ...fixture.dependency, ...patch }));
  // An unrelated producer's new latest revision does not change pinned R1.
  checkDependency({ ...fixture.dependency, latestProducerArtifact: "R2" });
});

test("acknowledged takeover refuses queued old-writer effects without stopping work", () => {
  checkWriter(fixture.writer);
  const stale = clone();
  stale.writer[1].disposition = "admitted";
  stale.writer[1].effect = "full";
  assert.throws(() => checkWriter(stale.writer));
  const disguised = clone();
  disguised.writer[1].generation = "2";
  disguised.writer[1].disposition = "admitted";
  disguised.writer[1].effect = "full";
  assert.throws(() => checkWriter(disguised.writer));
  assert.equal(fixture.unknown.processState, "running");
});

test("unknown effect requires reconciliation, not automatic new-key redispatch", () => {
  const check = (receipt) => {
    assert.equal(receipt.delivery, "unknown");
    assert.equal(receipt.automaticNextAction, "query");
  };
  check(fixture.unknown);
  for (const automaticNextAction of ["retry-same-key", "retry-new-key"])
    assert.throws(() => check({ ...fixture.unknown, automaticNextAction }));
});

test("population accounting does not equate clients, runs, processes and models", () => {
  const check = (populations, records) => {
    for (const [count, rows] of [
      ["connected", "connections"],
      ["runs", "runs"],
      ["commands", "executions"],
      ["ptys", "sessions"],
      ["mediatedModelRequests", "mediatedCalls"],
    ])
      assert.equal(populations[count], new Set(records[rows]).size);
    assert.equal(populations.unmediatedModelRequests, "unknown");
  };
  const { populations, populationRecords } = fixture;
  check(populations, populationRecords);
  assert.throws(() =>
    check(
      { ...populations, commands: populations.connected },
      populationRecords,
    ),
  );
  assert.notEqual(populations.connected, populations.commands);
  assert.notEqual(populations.connected, populations.ptys);
  assert.equal(populations.mediatedModelRequests, 0);
  assert.equal(populations.unmediatedModelRequests, "unknown");
  // Disconnected-but-continuing runs are legal; C is not a run-capacity claim.
  check(
    { ...populations, connected: 1 },
    { ...populationRecords, connections: ["EH"] },
  );
});

test("all original residual criteria have one explicit reviewed disposition row", () => {
  const text = readFileSync(
    new URL("../../docs/engine/h0-residual-criteria.md", import.meta.url),
    "utf8",
  );
  const actual = [...text.matchAll(/^\| (\d+\.C\d+) \|/gm)].map((m) => m[1]);
  const expected = Object.entries(fixture.originalCriterionCounts).flatMap(
    ([issue, count]) =>
      Array.from({ length: count }, (_, i) => `${issue}.C${i + 1}`),
  );
  assert.equal(actual.length, 27);
  assert.equal(new Set(actual).size, 27);
  assert.deepEqual(actual.sort(), expected.sort());
});
