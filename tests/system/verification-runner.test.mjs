import test from "node:test";
import assert from "node:assert/strict";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

// Execute the real runner with disposable command stubs: no native artifact or
// dependency setup is needed to test selection, reports and failure propagation.
function verify(args, failure = "", extraEnv = {}) {
  mkdirSync(".artifacts", { recursive: true });
  const root = resolve(mkdtempSync(".artifacts/verification-runner-"));
  for (const dir of ["scripts", "tests/system", "bin", ".venv-docs/bin"])
    mkdirSync(root + "/" + dir, { recursive: true });
  for (const file of ["verify.mjs", "source-identity.mjs"])
    copyFileSync("scripts/" + file, root + "/scripts/" + file);
  writeFileSync(root + "/.node-version", process.versions.node + "\n");
  for (const name of ["z.test.mjs", "a.test.mjs", "helper.mjs"])
    writeFileSync(root + "/tests/system/" + name, "");
  const stub = `#!${process.execPath}
const { appendFileSync, existsSync, writeFileSync } = require("node:fs");
const { basename } = require("node:path");
const bin = basename(process.argv[1]);
const args = process.argv.slice(2);
appendFileSync("commands.jsonl", JSON.stringify({ bin, args }) + "\\n");
if (bin === "git") {
  if (args[0] === "rev-parse") console.log("a".repeat(40));
  else if (args[0] === "diff" && existsSync("changed-source")) console.log("changed");
  process.exit(0);
}
if (process.env.VERIFY_TEST_FAILURE === "source" && args[0] === "test") writeFileSync("changed-source", "fixture");
console.log("stub " + bin);
if ((process.env.VERIFY_TEST_FAILURE === "check" && bin === "cargo" && args[0] === "test") ||
    (process.env.VERIFY_TEST_FAILURE === "coordination" && args.includes("tests/mk1/agents.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "journey" && args.includes("tests/mk1/e2e.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "h0" && bin === "namtab-h0-probe") ||
    (process.env.VERIFY_TEST_FAILURE === "comparison" && args.includes("tests/h0/compare-model.mjs")) ||
    (process.env.VERIFY_TEST_FAILURE === "mcp" && args.includes("tests/mvp/adapter.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "cancel" && args.includes("tests/mvp/attempt_control.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "scoped" && args.includes("tests/mvp/scoped_observation.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "flow" && args.includes("tests/h0/flow.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "version" && bin === "rustc")) process.exitCode = 1;
`;
  for (const name of ["cargo", "rustc", "node", "git", "python3"])
    writeFileSync(root + "/bin/" + name, stub, { mode: 0o755 });
  writeFileSync(root + "/.venv-docs/bin/python", stub, { mode: 0o755 });
  const target =
    resolve(root, extraEnv.CARGO_TARGET_DIR || "target") + "/debug";
  mkdirSync(target, { recursive: true });
  writeFileSync(target + "/namtab-h0-probe", stub, { mode: 0o755 });
  const result = spawnSync(
    process.execPath,
    [root + "/scripts/verify.mjs", ...args],
    {
      cwd: root,
      env: {
        ...process.env,
        PATH: root + "/bin:" + process.env.PATH,
        VERIFY_TEST_FAILURE: failure,
        ...extraEnv,
      },
      encoding: "utf8",
      timeout: 15000,
    },
  );
  const reports = root + "/.artifacts/verification";
  const runs = readdirSync(reports);
  assert.equal(runs.length, 1);
  return {
    result,
    report: JSON.parse(readFileSync(reports + "/" + runs[0] + "/checks.json")),
    commands: readFileSync(root + "/commands.jsonl", "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line)),
  };
}

test("both verification scopes retain Cargo coverage and all discovered system tests", () => {
  for (const sourceOnly of [false, true]) {
    const { result, report, commands } = verify(
      sourceOnly ? ["--source-foundation"] : [],
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(
      report.scope,
      sourceOnly
        ? "source-foundation: excludes historical native worker only"
        : "full-workspace",
    );
    assert.deepEqual(
      report.excluded_packages,
      sourceOnly ? ["project-x-engine"] : [],
    );
    for (const name of ["rust-clippy", "rust-contracts"]) {
      const args = report.checks.find((c) => c.name === name).command;
      assert.ok(args.includes("--workspace"));
      assert.ok(args.includes("--locked"));
      assert.equal(args.includes("--exclude"), sourceOnly);
      if (sourceOnly)
        assert.equal(args[args.indexOf("--exclude") + 1], "project-x-engine");
    }
    const files = ["tests/system/a.test.mjs", "tests/system/z.test.mjs"];
    assert.deepEqual(report.system_test_files, files);
    assert.deepEqual(
      report.checks.find((c) => c.name === "synthetic-contracts").command,
      ["node", "--test", ...files],
    );
    const launch = report.checks.findIndex(
      (c) => c.name === "mk1-launch-build",
    );
    const journey = report.checks.findIndex(
      (c) => c.name === "mk1-real-journeys",
    );
    const agents = report.checks.findIndex(
      (c) => c.name === "mk1-two-agent-journeys",
    );
    assert.ok(launch >= 0 && journey > launch && agents > journey);
    assert.ok(report.checks[agents].command.includes("tests/mk1/agents.py"));
    assert.deepEqual(report.checks[launch].command, [
      "python3",
      "-B",
      "scripts/mk1.py",
      "build",
    ]);
    assert.ok(report.checks[journey].command.includes("tests/mk1/e2e.py"));
    assert.ok(report.checks.every((c) => c.verdict === "PASS"));
    assert.equal(Object.hasOwn(report, "essential_gate"), false);
    assert.equal(
      commands.some((c) => c.args.includes("scripts/run-x4.mjs")),
      false,
    );
    assert.doesNotMatch(result.stdout, /Historical X4 gate:/u);
    assert.match(report.limits, /recorded FAIL verdicts/u);
  }
});

test("foundation check or tool version failure still fails verification and retains a report", () => {
  for (const failure of ["check", "version"]) {
    const { result, report } = verify(["--source-foundation"], failure);
    assert.equal(result.status, 1, result.stderr);
    if (failure === "check")
      assert.equal(
        report.checks.find((c) => c.name === "rust-contracts").verdict,
        "FAIL",
      );
    else {
      assert.ok(report.checks.every((c) => c.verdict === "PASS"));
      assert.equal(
        report.versions.find((c) => c.command[0] === "rustc").exit,
        1,
      );
    }
  }
});

test("a failed real MK1 journey fails the active lane without erasing other checks", () => {
  const { result, report } = verify(["--source-foundation"], "journey");
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    report.checks.find((c) => c.name === "mk1-real-journeys").verdict,
    "FAIL",
  );
  assert.equal(
    report.checks.find((c) => c.name === "strict-mkdocs").verdict,
    "PASS",
  );
});

test("a failed two-agent coordination journey fails the active lane", () => {
  const { result, report } = verify(["--source-foundation"], "coordination");
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    report.checks.find((c) => c.name === "mk1-two-agent-journeys").verdict,
    "FAIL",
  );
  assert.equal(
    report.checks.find((c) => c.name === "strict-mkdocs").verdict,
    "PASS",
  );
});

test("real journey binaries follow Cargo's native target directory", () => {
  const { result, report } = verify(["--source-foundation"], "", {
    CARGO_TARGET_DIR: ".artifacts/isolated-target",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(
    report.checks.find((c) => c.name === "h0-real-single-write-handoff")
      .command[0],
    /\/\.artifacts\/isolated-target\/debug\/namtab-h0-probe$/u,
  );
  for (const name of ["mk1-real-journeys", "mk1-two-agent-journeys"]) {
    const command = report.checks.find((c) => c.name === name).command;
    assert.match(
      command[command.indexOf("--bin-dir") + 1],
      /\/\.artifacts\/isolated-target\/debug$/u,
    );
  }
});

test("headless runtime and independent comparison failures fail the active lane", () => {
  for (const [failure, name] of [
    ["h0", "h0-real-single-write-handoff"],
    ["comparison", "h0-model-subset-comparison"],
    ["flow", "h0-producer-consumer"],
  ]) {
    const { result, report } = verify(["--source-foundation"], failure);
    assert.equal(result.status, 1, result.stderr);
    assert.equal(report.checks.find((c) => c.name === name).verdict, "FAIL");
    assert.equal(
      report.checks.find((c) => c.name === "strict-mkdocs").verdict,
      "PASS",
    );
  }
});

test("MCP lifecycle failure fails the active lane without skipping other checks", () => {
  const { result, report } = verify(["--source-foundation"], "mcp");
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    report.checks.find((c) => c.name === "mvp-mcp-lifecycle").verdict,
    "FAIL",
  );
  assert.equal(
    report.checks.find((c) => c.name === "strict-mkdocs").verdict,
    "PASS",
  );
});

test("source changes during checks fail verification without concealing passing steps", () => {
  const { result, report } = verify(["--source-foundation"], "source");
  assert.equal(result.status, 1, result.stderr);
  assert.ok(report.checks.every((c) => c.verdict === "PASS"));
  assert.equal(report.source_identity.verdict, "FAIL");
  assert.notEqual(
    report.source_identity.before.tracked_diff_sha256,
    report.source_identity.after.tracked_diff_sha256,
  );
});

test("failed exact-attempt control journey fails the aggregate without hiding other checks", () => {
  const { result, report } = verify(["--source-foundation"], "cancel");
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    report.checks.find((c) => c.name === "mvp-attempt-control").verdict,
    "FAIL",
  );
  assert.equal(
    report.checks.find((c) => c.name === "strict-mkdocs").verdict,
    "PASS",
  );
});

test("failed scoped observation journey fails the aggregate without hiding other checks", () => {
  const { result, report } = verify(["--source-foundation"], "scoped");
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    report.checks.find((c) => c.name === "mvp-scoped-observations").verdict,
    "FAIL",
  );
  assert.equal(
    report.checks.find((c) => c.name === "strict-mkdocs").verdict,
    "PASS",
  );
});
