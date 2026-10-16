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
function verify(args, failure = "") {
  mkdirSync(".artifacts", { recursive: true });
  const root = resolve(mkdtempSync(".artifacts/verification-runner-"));
  for (const dir of ["scripts", "tests/system", "bin", ".venv-docs/bin"])
    mkdirSync(root + "/" + dir, { recursive: true });
  copyFileSync("scripts/verify.mjs", root + "/scripts/verify.mjs");
  writeFileSync(root + "/.node-version", process.versions.node + "\n");
  for (const name of ["z.test.mjs", "a.test.mjs", "helper.mjs"])
    writeFileSync(root + "/tests/system/" + name, "");
  const stub = `#!${process.execPath}
const { appendFileSync } = require("node:fs");
const { basename } = require("node:path");
const bin = basename(process.argv[1]);
const args = process.argv.slice(2);
appendFileSync("commands.jsonl", JSON.stringify({ bin, args }) + "\\n");
console.log("stub " + bin);
if ((process.env.VERIFY_TEST_FAILURE === "check" && bin === "cargo" && args[0] === "test") ||
    (process.env.VERIFY_TEST_FAILURE === "coordination" && args.includes("tests/mk1/agents.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "journey" && args.includes("tests/mk1/e2e.py")) ||
    (process.env.VERIFY_TEST_FAILURE === "version" && bin === "rustc")) process.exitCode = 1;
`;
  for (const name of ["cargo", "rustc", "node", "git", "python3"])
    writeFileSync(root + "/bin/" + name, stub, { mode: 0o755 });
  writeFileSync(root + "/.venv-docs/bin/python", stub, { mode: 0o755 });
  const result = spawnSync(
    process.execPath,
    [root + "/scripts/verify.mjs", ...args],
    {
      cwd: root,
      env: {
        ...process.env,
        PATH: root + "/bin:" + process.env.PATH,
        VERIFY_TEST_FAILURE: failure,
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
