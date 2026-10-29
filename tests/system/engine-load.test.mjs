import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("engine-load evidence refuses missing, stale, starved and over-budget results", () => {
  const r = spawnSync("python3", ["-B", "tests/mvp/test_load_metrics.py"], {
    encoding: "utf8",
    timeout: 10000,
  });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stderr, /Ran 5 tests/u);
});
