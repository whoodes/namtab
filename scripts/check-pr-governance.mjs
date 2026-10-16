import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateCommitMessage } from "./commit-message.mjs";

export function associatedIssue(body, repository) {
  const line = (body ?? "").trim().split(/\r?\n/u)[0];
  const match =
    /^Associated issue: https:\/\/github\.com\/([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)\/issues\/([1-9][0-9]*)$/u.exec(
      line,
    );
  if (!match || match[1].toLowerCase() !== repository.toLowerCase())
    throw new Error(
      "First nonempty PR-body line must be: Associated issue: https://github.com/" +
        repository +
        "/issues/NUMBER (an issue in this repository; no closing keyword required).",
    );
  const number = Number(match[2]);
  if (!Number.isSafeInteger(number))
    throw new Error("Issue number is out of range.");
  return number;
}

export async function checkPullRequest({
  repository,
  number,
  expectedHead,
  read,
}) {
  if (
    !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u.test(repository) ||
    !Number.isSafeInteger(number) ||
    number < 1
  )
    throw new Error("Invalid repository or pull request identity.");
  const prefix = "repos/" + repository;
  const pr = await read(prefix + "/pulls/" + number);
  if (pr.head.sha !== expectedHead)
    throw new Error("Candidate HEAD changed; rerun for the current SHA.");
  const issueNumber = associatedIssue(pr.body, repository);
  const issue = await read(prefix + "/issues/" + issueNumber);
  if (
    issue.pull_request ||
    issue.number !== issueNumber ||
    issue.repository_url?.toLowerCase() !==
      ("https://api.github.com/" + prefix).toLowerCase()
  )
    throw new Error(
      "Association must resolve to an actual issue in this repository, not a PR.",
    );
  if (!Number.isSafeInteger(pr.commits) || pr.commits < 1 || pr.commits > 250)
    throw new Error(
      "Cannot verify the complete candidate range (GitHub PR commits API limit: 250).",
    );
  const commits = [];
  for (let page = 1; page <= 3; page++) {
    const batch = await read(
      prefix + "/pulls/" + number + "/commits?per_page=100&page=" + page,
    );
    commits.push(...batch);
    if (batch.length < 100) break;
  }
  if (
    commits.length !== pr.commits ||
    new Set(commits.map((c) => c.sha)).size !== commits.length ||
    !commits.some((c) => c.sha === expectedHead)
  )
    throw new Error(
      "Candidate commit range is incomplete/stale; rerun rather than silently skip messages.",
    );
  const errors = commits.flatMap((c) =>
    validateCommitMessage(c.commit.message, { mode: "ci" }).map(
      (e) => c.sha.slice(0, 12) + ": " + e,
    ),
  );
  if (errors.length) throw new Error(errors.join("\n"));
  return {
    issue: issue.html_url,
    head: expectedHead,
    candidateCommits: commits.length,
  };
}

if (
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
) {
  try {
    const event = JSON.parse(
      readFileSync(process.env.GITHUB_EVENT_PATH, "utf8"),
    );
    const result = await checkPullRequest({
      repository: process.env.GITHUB_REPOSITORY,
      number: event.pull_request.number,
      expectedHead: event.pull_request.head.sha,
      read: async (endpoint) =>
        JSON.parse(
          execFileSync("gh", ["api", endpoint, "--method", "GET"], {
            encoding: "utf8",
            timeout: 30000,
            maxBuffer: 4 * 1024 * 1024,
          }),
        ),
    });
    console.log(
      "PR association and candidate commit formatting PASS: " +
        JSON.stringify(result),
    );
  } catch (error) {
    console.error("PR governance rejected:\n" + error.message);
    process.exitCode = 1;
  }
}
