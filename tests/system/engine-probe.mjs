import { createRequire } from "node:module";
import { readFile, writeFile } from "node:fs/promises";
const require = createRequire(
  new URL("../../packages/terminal-adapter/package.json", import.meta.url),
);
const { Terminal } = require("@xterm/headless");
const { SerializeAddon } = require("@xterm/addon-serialize");
export const create = () =>
  new Terminal({ cols: 12, rows: 5, scrollback: 20, allowProposedApi: true });
export const feed = (t, bytes) =>
  new Promise((resolve) => t.write(bytes, resolve));
export function view(t) {
  const b = t.buffer.active;
  return {
    cols: t.cols,
    rows: t.rows,
    cursor: [b.cursorX, b.cursorY],
    baseY: b.baseY,
    type: b.type,
    lines: Array.from({ length: t.rows }, (_, i) =>
      b.getLine(b.baseY + i)?.translateToString(false),
    ),
    modes: { ...t.modes },
  };
}
export const cases = JSON.parse(
  await readFile(
    new URL("../fixtures/terminal/continuations.json", import.meta.url),
    "utf8",
  ),
);
export async function continuation(c) {
  const canonical = create(),
    replica = create(),
    addon = new SerializeAddon();
  try {
    canonical.loadAddon(addon);
    await feed(canonical, c.before);
    const serialized = addon.serialize();
    await feed(replica, serialized);
    const atAttach =
      JSON.stringify(view(canonical)) === JSON.stringify(view(replica));
    await feed(canonical, c.after);
    await feed(replica, c.after);
    const a = view(canonical),
      b = view(replica);
    return {
      name: c.name,
      before: JSON.stringify(c.before),
      after: JSON.stringify(c.after),
      atAttach,
      afterContinuation: JSON.stringify(a) === JSON.stringify(b),
      canonical: a,
      replica: b,
    };
  } finally {
    canonical.dispose();
    replica.dispose();
  }
}
export async function run() {
  const results = [];
  for (const c of cases) results.push(await continuation(c));
  const complete = results.filter((x) => x.name !== "partial-csi");
  return {
    gate: "X4",
    verdict: complete.every((x) => x.afterContinuation)
      ? "PASS_MINIMAL"
      : "FAIL",
    scope:
      "Necessary-condition headless→serialize→headless continuation, same common parser family; not a desktop/DOM test. Partial CSI is separately an unsafe-boundary case.",
    results,
  };
}
if (process.argv[1]?.endsWith("engine-probe.mjs")) {
  const result = await run();
  await writeFile(
    new URL("../../evidence/engine-continuation.json", import.meta.url),
    JSON.stringify(result, null, 2) + "\n",
  );
  console.log(JSON.stringify(result, null, 2));
  if (process.argv.includes("--require-pass") && result.verdict === "FAIL")
    process.exitCode = 1;
}
