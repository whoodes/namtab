import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("MCP evidence rejects fallback, binding corruption and unrequested bodies", () => {
  const result = spawnSync(
    "python3",
    ["-B", "tests/mvp/test_mcp_scoped_fixture.py"],
    {
      encoding: "utf8",
      timeout: 10000,
    },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 5 tests/u);
});
