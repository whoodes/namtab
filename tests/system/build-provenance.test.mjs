import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

test("Codeberg prefetch preserves the declared package and rejects corrupt cached bytes", () => {
  const inputs = JSON.parse(
    readFileSync("tests/ea1/build-inputs.json", "utf8"),
  );
  assert.match(
    inputs.url,
    /^https:\/\/codeberg\.org\/vancluever\/translate-c\/archive\/[a-f0-9]{40}\.tar\.gz$/u,
  );
  assert.match(inputs.archive_sha256, /^[a-f0-9]{64}$/u);
  assert.match(inputs.zig_hash, /^translate_c-0\.0\.0-/u);
  mkdirSync(".artifacts/bootstrap-tests", { recursive: true });
  const folder = mkdtempSync(".artifacts/bootstrap-tests/run-");
  const cache = folder + "/cached-source.tar.gz";
  writeFileSync(cache, "corrupt cache, not downloaded or executed");
  const script = `import importlib.util,json,hashlib
from pathlib import Path
spec=importlib.util.spec_from_file_location('assessment_bootstrap','scripts/assessment/bootstrap.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
pin=json.loads(Path('tests/ea1/build-inputs.json').read_text())
cache=Path(${JSON.stringify(cache)})
assert m.acquire(pin['url'],hashlib.sha256(cache.read_bytes()).hexdigest(),cache)==cache.read_bytes()
m.acquire(pin['url'],pin['archive_sha256'],cache)
`;
  const r = spawnSync("python3", ["-B", "-c", script], {
    encoding: "utf8",
    timeout: 5000,
  });
  assert.equal(r.status, 1, r.stderr);
  assert.match(r.stderr, /Cached digest mismatch/u);
  assert.equal(
    readFileSync(cache, "utf8"),
    "corrupt cache, not downloaded or executed",
  );
});
