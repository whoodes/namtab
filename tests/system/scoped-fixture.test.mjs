import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("CLI evidence retains failure for missing or changed owned bytes", () => {
  const result = spawnSync(
    "python3",
    ["-B", "tests/mvp/test_scoped_fixture.py"],
    {
      encoding: "utf8",
      timeout: 10000,
    },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 4 tests/u);
});
