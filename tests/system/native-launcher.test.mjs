import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("native launcher preserves existing namespaces and refuses unsafe setup", () => {
  const result = spawnSync("python3", ["-B", "tests/mvp/test_launcher.py"], {
    encoding: "utf8",
    timeout: 15000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 10 tests/u);
});

test("synthetic-only model test gate refuses wider programs and authorities", () => {
  const result = spawnSync("python3", ["-B", "tests/mvp/test_model_gate.py"], {
    encoding: "utf8",
    timeout: 15000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 4 tests/u);
});
