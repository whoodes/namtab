import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

test("native launch helper follows relative/absolute Cargo target directories", () => {
  for (const target of [
    ".artifacts/my-target",
    resolve(".artifacts/absolute-target"),
  ]) {
    const r = spawnSync(
      "python3",
      [
        "-B",
        "-c",
        'import runpy; print(runpy.run_path("scripts/mk1.py")["BIN"])',
      ],
      { encoding: "utf8", env: { ...process.env, CARGO_TARGET_DIR: target } },
    );
    assert.equal(r.status, 0, r.stderr);
    assert.equal(r.stdout.trim(), resolve(target, "debug"));
  }
});
test("launch refuses cross-target execution before invoking a compiler or binary", () => {
  const r = spawnSync("python3", ["-B", "scripts/mk1.py", "build"], {
    encoding: "utf8",
    env: { ...process.env, CARGO_BUILD_TARGET: "unsupported-test-target" },
  });
  assert.equal(r.status, 1);
  assert.match(r.stderr, /Cross-target launch is unsupported/u);
});
