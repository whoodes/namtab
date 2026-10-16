import test from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { validateCommitMessage } from "../../scripts/commit-message.mjs";
import {
  associatedIssue,
  checkPullRequest,
} from "../../scripts/check-pr-governance.mjs";

test("ordinary, multiline, Unicode and CRLF commit messages need no prefix/body filler", () => {
  for (const message of [
    "Fix shared input bounds",
    "Keep λ and 界 intact\n\nExplain the non-obvious boundary.\n\nValidation: make test\n",
    "Keep input attribution exact\r\n\r\nRationale and validation.\r\n",
    "Merge branch 'topic' into audit/local-correctness-20261016\n\nMerge context.",
    'Revert "Keep input attribution exact"\n\nThis reverts commit abc123.',
  ])
    assert.deepEqual(validateCommitMessage(message), []);
  for (const message of [
    "",
    "   ",
    "Bad\ttitle",
    "x".repeat(73),
    "Title\nBody without separation",
    "Title\n\nHidden\u0000control",
  ])
    assert.ok(validateCommitMessage(message).length, JSON.stringify(message));
});

test("local autosquash workflow is deliberate; unfinished candidates fail CI", () => {
  for (const prefix of ["fixup!", "squash!", "amend!"]) {
    const message = prefix + " Keep input attribution exact";
    assert.deepEqual(validateCommitMessage(message, { mode: "hook" }), []);
    assert.ok(
      validateCommitMessage(message, { mode: "ci" }).some((e) =>
        e.includes("temporary"),
      ),
    );
  }
  assert.ok(validateCommitMessage("fixup! ", { mode: "hook" }).length);
  assert.ok(validateCommitMessage("fixup!", { mode: "hook" }).length);
});

test("versioned executable hook works noninteractively without creating commits", () => {
  assert.ok(statSync(".githooks/commit-msg").mode & 0o111);
  mkdirSync(".artifacts/hook-tests", { recursive: true });
  const dir = mkdtempSync(".artifacts/hook-tests/run-");
  const file = dir + "/message";
  for (const [message, expected] of [
    ["Keep Unicode 界 intact\r\n\r\nContext.\r\n# Git editor comment\r\n", 0],
    ["Title\nMissing blank separator\n", 1],
    ["fixup! Keep input attribution exact\n", 0],
    ["Merge branch 'topic' into audit/local-correctness-20261016\n", 0],
    ["fixup!\n", 1],
    [
      'Revert "Keep input attribution exact"\n\nThis reverts commit abc123.\n',
      0,
    ],
  ]) {
    writeFileSync(file, message);
    const r = spawnSync(".githooks/commit-msg", [file], {
      encoding: "utf8",
      timeout: 10000,
    });
    assert.equal(r.status, expected, r.stderr);
    if (expected) assert.match(r.stderr, /blank line|target title/u);
  }
  writeFileSync(file, Buffer.from([0x46, 0x69, 0x78, 0x20, 0xff]));
  const malformed = spawnSync(".githooks/commit-msg", [file], {
    encoding: "utf8",
  });
  assert.equal(malformed.status, 1);
  assert.match(malformed.stderr, /encoded data|encoding/u);
});

test("association is an explicit same-repository issue URL, not prose/code/example text", () => {
  assert.equal(
    associatedIssue(
      "\nAssociated issue: https://github.com/whoodes/namtab/issues/25\n\nRelated: #1",
      "whoodes/namtab",
    ),
    25,
  );
  for (const body of [
    "See #25",
    "Closes #1",
    "```\nAssociated issue: https://github.com/whoodes/namtab/issues/25\n```",
    "<!-- Associated issue: https://github.com/whoodes/namtab/issues/25 -->",
    "Associated issue: https://github.com/other/repo/issues/25",
    "Associated issue: https://github.com/whoodes/namtab/pull/25",
    "Associated issue: https://example.invalid/whoodes/namtab/issues/25",
  ])
    assert.throws(
      () => associatedIssue(body, "whoodes/namtab"),
      /First nonempty/u,
    );
});

const fixture = (overrides = {}) => {
  const calls = [];
  const pr = {
    body: "Associated issue: https://github.com/whoodes/namtab/issues/25",
    head: { sha: "candidate" },
    commits: 1,
    ...overrides.pr,
  };
  const issue = {
    number: 25,
    repository_url: "https://api.github.com/repos/whoodes/namtab",
    html_url: "https://github.com/whoodes/namtab/issues/25",
    ...overrides.issue,
  };
  return {
    calls,
    read: async (endpoint) => {
      calls.push(endpoint);
      if (endpoint === "repos/whoodes/namtab/pulls/1") return pr;
      if (endpoint === "repos/whoodes/namtab/issues/25") {
        if (overrides.missingIssue)
          throw new Error("HTTP 404: issue absent/inaccessible");
        return issue;
      }
      if (
        endpoint === "repos/whoodes/namtab/pulls/1/commits?per_page=100&page=1"
      )
        return (
          overrides.commits ?? [
            {
              sha: "candidate",
              commit: {
                message: "Keep candidate checks exact\n\nValidation: make test",
              },
            },
          ]
        );
      throw new Error("Unexpected API path " + endpoint);
    },
  };
};

test("live association and only PR-introduced commits are checked, including edited bodies", async () => {
  const f = fixture();
  const result = await checkPullRequest({
    repository: "whoodes/namtab",
    number: 1,
    expectedHead: "candidate",
    read: f.read,
  });
  assert.equal(result.candidateCommits, 1);
  assert.deepEqual(f.calls, [
    "repos/whoodes/namtab/pulls/1",
    "repos/whoodes/namtab/issues/25",
    "repos/whoodes/namtab/pulls/1/commits?per_page=100&page=1",
  ]);
  for (const overrides of [
    { pr: { body: "Edited away the association" } },
    { pr: { head: { sha: "newer" } } },
    { issue: { pull_request: {} } },
    { missingIssue: true },
    { pr: { commits: 2 } },
    { pr: { commits: 251 } },
    {
      commits: [
        {
          sha: "candidate",
          commit: { message: "fixup! Keep candidate checks exact" },
        },
      ],
    },
  ]) {
    const bad = fixture(overrides);
    await assert.rejects(
      checkPullRequest({
        repository: "whoodes/namtab",
        number: 1,
        expectedHead: "candidate",
        read: bad.read,
      }),
    );
  }
});

test("candidate pagination covers every introduced commit and rejects duplicates", async () => {
  const commits = Array.from({ length: 101 }, (_, i) => ({
    sha: i === 100 ? "candidate" : "commit-" + i,
    commit: { message: "Keep commit boundaries complete" },
  }));
  for (const duplicate of [false, true]) {
    const f = fixture({ pr: { commits: 101 } });
    const read = async (endpoint) => {
      if (endpoint.endsWith("commits?per_page=100&page=1"))
        return commits.slice(0, 100);
      if (endpoint.endsWith("commits?per_page=100&page=2"))
        return duplicate ? [commits[0]] : commits.slice(100);
      return f.read(endpoint);
    };
    const check = checkPullRequest({
      repository: "whoodes/namtab",
      number: 1,
      expectedHead: "candidate",
      read,
    });
    if (duplicate) await assert.rejects(check, /incomplete\/stale/u);
    else assert.equal((await check).candidateCommits, 101);
  }
});
