import test from "node:test";
import assert from "node:assert/strict";
import { fixtureFailureSummary } from "../../scripts/fixture-diagnostics.mjs";

test("fixture errors are allowlisted, bounded and escaped without raw output", () => {
  const result = fixtureFailureSummary(
    "ignored raw prefix\n" +
      JSON.stringify({
        verdict: "FAIL",
        screen: "DO_NOT_ECHO",
        errors: {
          execution_error: {
            type: "AssertionError",
            message: "\n\u001b[31m" + "x".repeat(2000),
          },
          unknown_error: { type: "DO_NOT_ECHO", message: "DO_NOT_ECHO" },
        },
      }) +
      "\n",
  );
  assert.ok(result);
  assert.ok(!result.includes("DO_NOT_ECHO"));
  assert.ok(!result.includes("\n") && !result.includes("\u001b"));
  const errors = JSON.parse(result);
  assert.deepEqual(Object.keys(errors), ["execution_error"]);
  assert.equal(errors.execution_error.message.length, 512);
});

test("unrecognized, successful, malformed and oversized summaries are not echoed", () => {
  for (const value of [
    "",
    "not JSON",
    "null",
    JSON.stringify({
      verdict: "PASS",
      errors: { execution_error: { type: "x", message: "x" } },
    }),
    JSON.stringify({ verdict: "FAIL", errors: { screen: "raw" } }),
    JSON.stringify({
      verdict: "FAIL",
      errors: { execution_error: { type: 1, message: "x" } },
    }),
    "x".repeat(8193),
  ])
    assert.equal(fixtureFailureSummary(value), null);
});
