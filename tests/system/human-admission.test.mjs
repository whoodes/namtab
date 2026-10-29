import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("admission evidence rejects unrelated fences and incomplete populations", () => {
  const r = spawnSync("python3", ["-B", "tests/mvp/test_human_admission.py"], {
    encoding: "utf8",
    timeout: 10000,
  });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stderr, /Ran 4 tests/u);
});
