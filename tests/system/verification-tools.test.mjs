import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, rmdirSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";

test("raw X4 wrapper preserves the original report and still exits nonzero", () => {
  const original = readFileSync("evidence/engine-continuation.json");
  const result = spawnSync(
    process.execPath,
    ["scripts/run-x4.mjs", "--require-pass"],
    { encoding: "utf8", timeout: 10000 },
  );
  assert.equal(result.status, 1, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.gate, "X4");
  assert.equal(report.verdict, "FAIL");
  assert.deepEqual(readFileSync("evidence/engine-continuation.json"), original);
});

test("EA1 outputs are confined to new-run namespaces, never historical folders", () => {
  const run = (destination) =>
    spawnSync("python3", ["-B", "-c", "import evidence_dir"], {
      env: {
        ...process.env,
        PYTHONPATH: "tests/ea1",
        EA1_EVIDENCE_DIR: destination,
      },
      encoding: "utf8",
      timeout: 5000,
    });
  for (const destination of [
    "evidence",
    "evidence/ea1",
    "evidence/engine-assessment",
    "evidence/ea1-input-investigation/verification",
    "evidence/ea1-runs",
    "../outside-project",
  ]) {
    const result = run(destination);
    assert.notEqual(result.status, 0, destination);
    assert.match(result.stderr, /ValueError/u);
  }
  const destination = "evidence/ea1-runs/test-" + randomUUID();
  const result = run(destination);
  assert.equal(result.status, 0, result.stderr);
  rmdirSync(destination); // Only the empty directory created by this test.
});
