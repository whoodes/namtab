import test from "node:test";
import assert from "node:assert/strict";
import {
  readFileSync,
  rmdirSync,
  existsSync,
  mkdirSync,
  symlinkSync,
} from "node:fs";
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

test("EA1 command recorder validates namespace before dispatch and outer reports", () => {
  const id = randomUUID();
  const destination = "evidence/ea1-runs/recorder-" + id;
  const sentinel = ".artifacts/ea1-recorder-" + id;
  const reports = destination + "/commands";
  const invoke = (run) =>
    spawnSync(
      "python3",
      [
        "-B",
        "scripts/record-command.py",
        "--ea1",
        "--evidence-dir",
        reports,
        "--name",
        "guard-probe",
        "--",
        "python3",
        "-c",
        "from pathlib import Path; Path(" +
          JSON.stringify(sentinel) +
          ").write_text('dispatched')",
      ],
      {
        env: { ...process.env, EA1_EVIDENCE_DIR: run },
        encoding: "utf8",
        timeout: 10000,
      },
    );
  mkdirSync(".artifacts", { recursive: true });
  const invalid = invoke("evidence/ea1");
  assert.notEqual(invalid.status, 0);
  assert.match(invalid.stderr, /Historical reports are immutable/u);
  assert.equal(
    existsSync(sentinel),
    false,
    "invalid namespace must not launch child",
  );
  assert.equal(
    existsSync(reports),
    false,
    "invalid namespace must not create command reports",
  );
  const valid = invoke(destination);
  assert.equal(valid.status, 0, valid.stderr);
  assert.equal(readFileSync(sentinel, "utf8"), "dispatched");
  assert.equal(
    JSON.parse(readFileSync(reports + "/guard-probe-1.json")).verdict,
    "PASS",
  );
  const dryRun = spawnSync(
    "make",
    ["-n", "ea1-build", "EA1_EVIDENCE_DIR=evidence/ea1"],
    { encoding: "utf8" },
  );
  assert.equal(dryRun.status, 0, dryRun.stderr);
  assert.match(
    dryRun.stdout,
    /record-command\.py --ea1 --evidence-dir "evidence\/ea1\/commands"/u,
  );
});

test("EA1 recorder refuses commands-directory redirects before dispatch", () => {
  const id = randomUUID();
  const run = "evidence/ea1-runs/redirect-" + id;
  const other = "evidence/ea1-runs/redirect-target-" + id;
  mkdirSync(run, { recursive: true });
  mkdirSync(other, { recursive: true });
  symlinkSync("../redirect-target-" + id, run + "/commands", "dir");
  const result = spawnSync(
    "python3",
    [
      "-B",
      "scripts/record-command.py",
      "--ea1",
      "--evidence-dir",
      run + "/commands",
      "--name",
      "guard-probe",
      "--",
      "python3",
      "-c",
      "print('CHILD_DISPATCHED')",
    ],
    {
      env: { ...process.env, EA1_EVIDENCE_DIR: run },
      encoding: "utf8",
      timeout: 10000,
    },
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /without redirects/u);
  assert.doesNotMatch(result.stdout, /CHILD_DISPATCHED/u);
  assert.equal(existsSync(other + "/guard-probe-1.json"), false);
});

test("verification refuses unknown modes before running any checks", () => {
  const result = spawnSync(
    process.execPath,
    ["scripts/verify.mjs", "--skip-native-silently"],
    { encoding: "utf8", timeout: 5000 },
  );
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /Usage: verify.mjs/u);
  assert.doesNotMatch(result.stdout, /repository-local-links|Scope:/u);
});
