// Synthetic existing frontend normalization only; no window, raw terminal or TUI navigation.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { normalize } from "../../apps/desktop/dist/input.js";
const consoleIntents = JSON.parse(
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--locked",
      "-p",
      "project-x-console",
      "--",
      "input-fixture",
    ],
    { encoding: "utf8" },
  ),
);
const desktopIntents = [
  ["ArrowUp", false, false],
  ["Tab", true, false],
  ["F12", false, true],
].map(([key, shiftKey, ctrlKey]) =>
  normalize({
    key,
    shiftKey,
    ctrlKey,
    altKey: false,
    metaKey: false,
    isComposing: false,
    repeat: false,
    location: 0,
  }),
);
assert.deepEqual(consoleIntents, desktopIntents);
await mkdir(".artifacts/ea1", { recursive: true });
await writeFile(
  ".artifacts/ea1/shared-inputs.json",
  JSON.stringify({ consoleIntents, desktopIntents }, null, 2) + "\n",
);
console.log(
  "Three synthetic console/desktop key intents agree; native encoding asserted separately.",
);
