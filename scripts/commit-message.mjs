// Shared objective policy. Rationale, grammar and logical completeness need review.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function validateCommitMessage(message, { mode = "ci" } = {}) {
  const errors = [];
  const text = message.replaceAll("\r\n", "\n");
  const lines = text.split("\n");
  const title = lines[0];
  if (!title.trim()) errors.push("Use a nonempty commit title.");
  if (title !== title.trim())
    errors.push("Remove leading/trailing whitespace from the title.");
  if (/[\p{Cc}\p{Zl}\p{Zp}]/u.test(title))
    errors.push("Keep the title on one line without control characters.");
  if (/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/u.test(text))
    errors.push(
      "Remove control characters; LF, CRLF and body tabs are supported.",
    );
  const autosquash = /^(?:fixup|squash|amend)!(?: (.*))?$/u.exec(title);
  if (autosquash && mode === "ci")
    errors.push(
      "Resolve temporary fixup!/squash!/amend! commits before merge; history changes still need authorization.",
    );
  const generated = /^Merge /u.test(title) || /^Revert ".+"$/u.test(title);
  const measured = autosquash ? (autosquash[1] ?? "") : title;
  if (autosquash && !measured.trim())
    errors.push("An autosquash message needs its target title.");
  if (Array.from(measured).length > (generated ? 120 : 72))
    errors.push(
      `Shorten the title to ${generated ? 120 : 72} Unicode code points or fewer; move detail into the body.`,
    );
  if (lines.slice(1).some((line) => line.trim()) && lines[1]?.trim())
    errors.push("Separate the title and body with a blank line.");
  return errors;
}

if (
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
) {
  try {
    if (process.argv.length !== 3)
      throw new Error("Usage: node scripts/commit-message.mjs COMMIT_MSG_FILE");
    // A hook cannot infer per-invocation --cleanup flags. Validate the literal
    // file, including comment-like text, rather than hide text Git may retain.
    const raw = new TextDecoder("utf-8", {
      fatal: true,
      ignoreBOM: true,
    }).decode(readFileSync(process.argv[2]));
    const errors = validateCommitMessage(raw, { mode: "hook" });
    if (errors.length) throw new Error(errors.join("\n"));
  } catch (error) {
    console.error("Commit message rejected:\n" + error.message);
    process.exitCode = 1;
  }
}
