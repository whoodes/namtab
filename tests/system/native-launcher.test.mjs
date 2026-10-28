import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

test("native launcher preserves existing namespaces and refuses unsafe setup", () => {
  const result = spawnSync("python3", ["-B", "tests/mvp/test_launcher.py"], {
    encoding: "utf8",
    timeout: 15000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 14 tests/u);
});

test("synthetic-only model test gate refuses wider programs and authorities", () => {
  const result = spawnSync("python3", ["-B", "tests/mvp/test_model_gate.py"], {
    encoding: "utf8",
    timeout: 15000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 4 tests/u);
});

test("fixture child cleanup handles EOF, bounded escalation and initialization failure", () => {
  const result = spawnSync(
    "python3",
    ["-B", "tests/mvp/test_owned_process.py"],
    {
      encoding: "utf8",
      timeout: 15000,
    },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /Ran 8 tests/u);
});

test("native fixture root is its source worktree rather than common Git storage", () => {
  const result = spawnSync(
    "python3",
    [
      "-B",
      "-c",
      'import sys; from pathlib import Path; from types import SimpleNamespace; sys.path[:0]=["tests/mk1"]; from e2e import Journey; import tempfile; root=Path.cwd(); tmp=tempfile.mkdtemp(prefix="worktree-root-", dir=root/".artifacts"); j=Journey(SimpleNamespace(bin_dir=root/"scripts",terminfo=root/"tests",artifacts=Path(tmp)/"evidence")); assert j.root==root; assert j.runtime.is_relative_to(root/".artifacts")',
    ],
    { encoding: "utf8", timeout: 15000 },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
});
