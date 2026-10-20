import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import test from "node:test";

// Exercise the real scanner after a normal checkout leaves ignored app output.
// Remove only the disposable directory this test creates, never a user's cache.
test("boundary checks preserve manifestless retired output and reject unknown or duplicate packages", () => {
  const directory = mkdtempSync("apps/.retired-electron-test-");
  mkdirSync(directory + "/dist");
  mkdirSync(directory + "/src");
  const sentinel = directory + "/dist/owned-test-sentinel.txt";
  writeFileSync(sentinel, "preserve ignored output");
  const check = () =>
    spawnSync(process.execPath, ["scripts/check-boundaries.mjs"], {
      encoding: "utf8",
    });
  try {
    let result = check();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(readFileSync(sentinel, "utf8"), "preserve ignored output");
    writeFileSync(
      directory + "/package.json",
      JSON.stringify({ name: "@project-x/retired-electron" }),
    );
    result = check();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Unreviewed TS package/u);
    writeFileSync(
      directory + "/package.json",
      readFileSync("packages/protocol/package.json"),
    );
    result = check();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Duplicate TS package/u);
  } finally {
    rmSync(directory, { recursive: true });
  }
});

test("historical pnpm evidence allows only the exact local retired importer removal", () => {
  const script = `import hashlib,json,sys,tarfile
from pathlib import Path
sys.path.insert(0,'tests/ea1')
from lock_policy import DESKTOP_IMPORTER,verify_pnpm_lock
manifest=json.loads(next(Path('evidence/archive').glob('pre-ea1-*.manifest.json')).read_text())
with tarfile.open(manifest['archive']) as archive:
    original=archive.extractfile('pnpm-lock.yaml').read()
expected=manifest['lockfiles']['pnpm-lock.yaml']
current=Path('pnpm-lock.yaml').read_bytes()
assert verify_pnpm_lock(original,original,expected)['delta']=='historical lock unchanged'
assert verify_pnpm_lock(original,current,expected)['delta'].startswith('only retired apps/desktop')
assert current==original.replace(DESKTOP_IMPORTER,b'',1)
for old,new in [(original+b'changed',current),
                (original,current.replace(b'6.0.0',b'6.0.1')),
                (original,current.replace(b'autoInstallPeers: true',b'autoInstallPeers: false')),
                (original,current.replace(b'  apps/engine-host:',b'  apps/renamed-host:')),
                (original,current+DESKTOP_IMPORTER),
                (original,current+b'\\n')]:
    try: verify_pnpm_lock(old,new,expected)
    except ValueError: pass
    else: raise AssertionError('unexpected lock change accepted')
print('2 positive and 6 negative lock-policy cases passed; no native artifact executed')
`;
  const result = spawnSync("python3", ["-B", "-c", script], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /2 positive and 6 negative/u);
});
