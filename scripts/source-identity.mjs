// Bind a check record to tracked source/index without retaining raw diffs.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
export function sourceIdentity(root) {
  const git = (...args) =>
    execFileSync("git", args, { cwd: root, maxBuffer: 32 * 1024 * 1024 });
  const revision = (...args) => {
    const value = git(...args)
      .toString()
      .trim();
    if (!/^[a-f0-9]{40}$/u.test(value))
      throw new Error("Invalid Git source revision");
    return value;
  };
  const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
  const status = git(
    "status",
    "--porcelain=v1",
    "-z",
    "--untracked-files=normal",
  );
  return {
    head: revision("rev-parse", "HEAD"),
    tree: revision("rev-parse", "HEAD^{tree}"),
    dirty: status.length !== 0,
    status_sha256: digest(status),
    index_sha256: digest(git("ls-files", "--stage", "-z")),
    tracked_diff_sha256: digest(
      git("diff", "--no-ext-diff", "--no-textconv", "--binary", "HEAD", "--"),
    ),
    limits:
      "Head/tree plus index, status and tracked worktree diff hashes. Ignored output and untracked file contents are excluded. Not an atomic filesystem snapshot; transient edits restored between observations may escape detection. A dirty check is not a clean-commit attestation.",
  };
}
