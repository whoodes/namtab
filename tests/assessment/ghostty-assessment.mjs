import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
const root = new URL("../../", import.meta.url);
const fixtures = JSON.parse(
  await readFile(
    new URL("../fixtures/terminal/continuations.json", import.meta.url),
    "utf8",
  ),
);
const oracle = JSON.parse(
  await readFile(new URL("./expectations.json", import.meta.url), "utf8"),
).cases;
const executable = new URL(".artifacts/engine-assessment/ghostty-proof", root)
  .pathname;
const run = (args) =>
  JSON.parse(
    execFileSync(executable, args, {
      encoding: "utf8",
      timeout: 10000,
      maxBuffer: 1024 * 1024,
    }),
  );
function normalized(v) {
  return {
    ...v,
    lines: v.cells.map((row) =>
      row
        .map((cell) => String.fromCodePoint(cell[0] || 32))
        .join("")
        .trimEnd(),
    ),
  };
}
function expected(name, v) {
  const o = oracle[name];
  v = normalized(v);
  for (const key of ["lines", "cursor", "buffer"])
    assert.deepEqual(v[key], o[key], name + " independent " + key);
  if (o.foregroundPaletteAtOrigin !== undefined)
    assert.equal(v.cells[0][0][1], o.foregroundPaletteAtOrigin);
}
const results = [];
for (const c of fixtures) {
  const before = Buffer.from(c.before, "utf8"),
    after = Buffer.from(c.after, "utf8");
  const r = run([before.toString("hex"), after.toString("hex")]);
  expected(c.name, r.canonicalAfter);
  expected(c.name, r.restoredAfter);
  assert.deepEqual(r.before, r.restoredBefore, c.name + " initial state");
  assert.deepEqual(
    r.canonicalAfter,
    r.restoredAfter,
    c.name + " continued state",
  );
  assert.deepEqual(
    r.resizedCanonical,
    r.resizedRestored,
    c.name + " resize and further output",
  );
  for (const key of [
    "truncatedDecodeResult",
    "corruptedDecodeResult",
    "wrongVersionDecodeResult",
  ])
    assert.notEqual(r[key], 0, key);
  if (c.name === "partial-csi")
    assert.notEqual(
      r.noTrackingEncodeResult,
      0,
      "partial parser state cannot snapshot without prior tracking",
    );
  let partitions = 0;
  for (let split = 0; split <= before.length; split++) {
    const splitResult = run([
      before.toString("hex"),
      after.toString("hex"),
      String(split),
    ]);
    assert.deepEqual(splitResult.canonicalAfter, r.canonicalAfter);
    assert.deepEqual(splitResult.restoredAfter, r.canonicalAfter);
    partitions++;
  }
  assert.throws(() =>
    expected(c.name, { ...r.restoredAfter, cursor: [99, 99] }),
  );
  results.push({
    name: c.name,
    verdict: "PASS_BOUNDED",
    partitions,
    oracleMutationRejected: true,
    ...r,
  });
}
const extras = [
  { name: "partial-utf8", beforeHex: "e795", afterHex: "8c" },
  {
    name: "combining-and-wide",
    beforeHex: Buffer.from("e\u0301界").toString("hex"),
    afterHex: "58",
  },
  {
    name: "pending-wrap",
    beforeHex: Buffer.from("abcdefghijkl").toString("hex"),
    afterHex: "58",
  },
  {
    name: "both-screens",
    beforeHex: Buffer.from("normal\x1b7\x1b[?1049h\x1b[2;3HALT").toString(
      "hex",
    ),
    afterHex: Buffer.from("\x1b[?1049l\x1b8Z").toString("hex"),
  },
  {
    name: "partial-osc",
    beforeHex: Buffer.from("\x1b]0;synthetic").toString("hex"),
    afterHex: Buffer.from(" title\x07X").toString("hex"),
  },
];
const extended = [];
for (const c of extras) {
  const r = run([c.beforeHex, c.afterHex]);
  assert.deepEqual(r.before, r.restoredBefore);
  assert.deepEqual(r.canonicalAfter, r.restoredAfter);
  assert.deepEqual(r.resizedCanonical, r.resizedRestored);
  if (c.name === "partial-utf8") {
    assert.equal(r.canonicalAfter.cells[0][0][0], 30028);
    assert.equal(r.canonicalAfter.cells[0][0][3], 1);
    assert.notEqual(r.noTrackingEncodeResult, 0);
  }
  if (c.name === "combining-and-wide") {
    assert.deepEqual(r.canonicalAfter.cells[0][0][4], [101, 769]);
    assert.equal(r.canonicalAfter.cells[0][1][3], 1);
  }
  extended.push({ ...c, verdict: "PASS_BOUNDED", ...r });
}
const controls = run(["--controls"]);
for (const [key, value] of Object.entries({
  upNormal: "1b5b41",
  keypadNormal: "0d",
  upApplication: "1b4f41",
  keypadApplicationDefault1035: "0d",
  keypadApplication: "1b4f4d",
  restoredKeypadApplication: "1b4f4d",
  restoredUpApplication: "1b4f41",
  ctrlUp: "1b5b313b3541",
  shiftTab: "1b5b5a",
  f12: "1b5b32347e",
  bracketedPaste: "1b5b3230307e610a621b5b3230317e",
  focusEncoding: "1b5b49",
  canonicalReplies: "1b5b313b31521b5b313b3152",
  sourceReplyCallbacksDuringDecode: 0,
  inheritedSourceReplyCallbacks: 0,
  restoredNewReplies: "1b5b313b31521b5b313b3152",
}))
  assert.equal(controls[key], value, key);
assert.notEqual(controls.overLimitContinuationEncodeResult, 0);
const report = {
  candidate: "B official libghostty-vt",
  commit: "22fa134a07f5749edd9e92c6bc17d8aaf61843be",
  verdict: "PASS_BOUNDED_C_API",
  fixtureSha256: createHash("sha256")
    .update(
      await readFile(
        new URL("../fixtures/terminal/continuations.json", import.meta.url),
      ),
    )
    .digest("hex"),
  results,
  extended,
  controls,
  limitations:
    "Same-build native in-memory snapshot round trips and public grid/key APIs. C observation omits some attributes, palette overrides, links, mouse and non-ASCII full-string oracles. No GUI/host TUI renderer, Rust FFI, portable cross-version durability, actual engine/process crash or full conformance demonstrated.",
};
await writeFile(
  new URL(
    "../../evidence/engine-assessment/ghostty-assessment.json",
    import.meta.url,
  ),
  JSON.stringify(report, null, 2) + "\n",
);
console.log(
  JSON.stringify(
    {
      candidate: report.candidate,
      verdict: report.verdict,
      sharedCases: results.map((r) => ({
        name: r.name,
        snapshotBytes: r.snapshotBytes,
        partitions: r.partitions,
      })),
      extendedCases: extended.map((r) => ({
        name: r.name,
        verdict: r.verdict,
      })),
      controls,
    },
    null,
    2,
  ),
);
