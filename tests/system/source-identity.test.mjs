import test from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { sourceIdentity } from "../../scripts/source-identity.mjs";

test("source identity binds clean revision, unstaged bytes and index without dumping content", () => {
  mkdirSync(".artifacts", { recursive: true });
  const root = resolve(mkdtempSync(".artifacts/source-id-"));
  const git = (...args) => execFileSync("git", args, { cwd: root });
  git("init", "--quiet");
  writeFileSync(root + "/fixture", "baseline");
  git("add", "fixture");
  git(
    "-c",
    "user.name=Test Fixture",
    "-c",
    "user.email=fixture@example.invalid",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
    "--quiet",
    "-m",
    "Create synthetic fixture",
  );
  const before = sourceIdentity(root);
  assert.equal(before.dirty, false);
  writeFileSync(root + "/fixture", "changed-only-synthetic-data");
  const dirty = sourceIdentity(root);
  assert.equal(dirty.dirty, true);
  assert.equal(dirty.head, before.head);
  assert.notEqual(dirty.tracked_diff_sha256, before.tracked_diff_sha256);
  assert.ok(!JSON.stringify(dirty).includes("changed-only-synthetic-data"));
  git("add", "fixture");
  assert.notEqual(sourceIdentity(root).index_sha256, dirty.index_sha256);
});
