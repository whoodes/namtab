import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const root = resolve(import.meta.dirname, "../..");

test("first-party license is the unmodified standard Apache-2.0 text", () => {
  assert.equal(
    createHash("sha256")
      .update(readFileSync(resolve(root, "LICENSE")))
      .digest("hex"),
    "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
    "Keep the canonical license intact; scope belongs in the licensing document",
  );
});

test("all first-party Cargo and npm packages declare their own license", () => {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      [
        "metadata",
        "--locked",
        "--offline",
        "--no-deps",
        "--format-version",
        "1",
      ],
      {
        cwd: root,
        env: { ...process.env, CARGO_HOME: resolve(root, ".cache/cargo") },
        encoding: "utf8",
      },
    ),
  );
  assert.equal(metadata.workspace_members.length, 8);
  for (const pkg of metadata.packages) {
    assert.equal(pkg.license, "Apache-2.0", pkg.name);
    assert.deepEqual(pkg.publish, [], "Keep registry publication disabled");
  }
  const manifests = execFileSync(
    "git",
    ["ls-files", "-z", "--", "*package.json"],
    {
      cwd: root,
      encoding: "utf8",
    },
  )
    .split("\0")
    .filter(Boolean);
  assert.equal(manifests.length, 6);
  for (const file of manifests) {
    const pkg = JSON.parse(readFileSync(resolve(root, file), "utf8"));
    assert.equal(pkg.license, "Apache-2.0", file);
    assert.equal(pkg.private, true, "Do not enable npm publication: " + file);
  }
});
