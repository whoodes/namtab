import test from "node:test";
import assert from "node:assert/strict";
import { outerScreen } from "../mk1/outer-screen.mjs";
import { spawnSync } from "node:child_process";

test("controlled Ctrl-C prompt oracle rejects old rows and accepts ECHOCTL", () => {
  const result = spawnSync("python3", ["-B", "tests/mk1/test_prompt.py"], {
    encoding: "utf8",
    timeout: 10000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 5 tests/u);
});

test("outer host paint observation reconstructs cursor deltas instead of grepping byte history", async () => {
  const paint = Buffer.from("sepXrate-streams\r\x1b[3Ca");
  assert.equal(paint.includes(Buffer.from("separate-streams")), false);
  const frame = await outerScreen(paint);
  assert.equal(frame[0], "separate-streams");
  assert.equal(frame.length, 38);
});
test("outer observer handles alternate screen and finite geometry without sending replies", async () => {
  const paint = Buffer.from("old\x1b[?1049hnew\x1b[6n\x1b[?1049l");
  assert.equal((await outerScreen(paint))[0], "old");
  await assert.rejects(outerScreen(Buffer.alloc(2_097_153)));
  await assert.rejects(outerScreen(Buffer.alloc(0), 161, 38));
});
