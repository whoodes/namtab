import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
const headless = createRequire(
  new URL("../packages/terminal-adapter/package.json", import.meta.url),
);
const desktop = createRequire(
  new URL("../packages/terminal-adapter/package.json", import.meta.url),
);
const commit = "f447274f430fd22513f6adbf9862d19524471c04";
const checks = [];
async function inspect(require, name, version) {
  const installed = JSON.parse(
    await readFile(require.resolve(name + "/package.json"), "utf8"),
  );
  const url = "https://registry.npmjs.org/" + name + "/" + version;
  const response = await fetch(url);
  if (!response.ok) throw new Error("Registry " + response.status);
  const registry = await response.json();
  const tar = await fetch(registry.dist.tarball);
  if (!tar.ok) throw new Error("Tarball " + tar.status);
  const bytes = Buffer.from(await tar.arrayBuffer());
  if (
    "sha512-" + createHash("sha512").update(bytes).digest("base64") !==
    registry.dist.integrity
  )
    throw new Error("Integrity " + name);
  if (
    installed.commit !== commit ||
    registry.commit !== commit ||
    registry.gitHead !== commit
  )
    throw new Error("Source family " + name);
  const installedRoot = require
    .resolve(name + "/package.json")
    .replace(/package.json$/u, "");
  await mkdir(".cache/xterm-artifacts", { recursive: true });
  const archive =
    ".cache/xterm-artifacts/" +
    name.replaceAll("@", "").replaceAll("/", "-") +
    "-" +
    version +
    ".tgz";
  await writeFile(archive, bytes);
  const installedComparison = JSON.parse(
    execFileSync(
      "python3",
      ["-B", "scripts/verify-installed-tarball.py", archive, installedRoot],
      { encoding: "utf8" },
    ),
  );
  checks.push({
    installedComparison,
    name,
    version,
    commit,
    registry: url,
    tarball: registry.dist.tarball,
    integrity: registry.dist.integrity,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    installedMain: installed.main,
    license: installed.license,
  });
  return require.resolve(name + "/package.json").replace(/package.json$/u, "");
}
const h = await inspect(headless, "@xterm/headless", "6.0.0");
const b = await inspect(desktop, "@xterm/xterm", "6.0.0");
await inspect(headless, "@xterm/addon-serialize", "0.14.0");
const maps = [
  JSON.parse(await readFile(h + "lib-headless/xterm-headless.js.map", "utf8")),
  JSON.parse(await readFile(b + "lib/xterm.js.map", "utf8")),
];
const file = "src/common/InputHandler.ts";
const source = await readFile(
  new URL("../.cache/xterm-source/" + file, import.meta.url),
  "utf8",
);
const matched = maps.map((m) => {
  const i = m.sources.findIndex((s) => s.endsWith("./" + file));
  return i >= 0 && m.sourcesContent[i] === source;
});
if (!matched.every(Boolean))
  throw new Error("InputHandler embedded-source mismatch");
const keyboard = await readFile(
  new URL(
    "../.cache/xterm-source/src/common/input/Keyboard.ts",
    import.meta.url,
  ),
  "utf8",
);
const ki = maps[1].sources.findIndex((s) =>
  s.endsWith("./src/common/input/Keyboard.ts"),
);
if (maps[1].sourcesContent[ki] !== keyboard)
  throw new Error("Keyboard source mismatch");
const common = new Map(
  maps[0].sources
    .map((s, i) => [s.replace(/^.*?\.\//u, ""), maps[0].sourcesContent[i]])
    .filter(([s]) => s.startsWith("src/common/")),
);
let commonFilesCompared = 0;
for (let i = 0; i < maps[1].sources.length; i++) {
  const path = maps[1].sources[i].replace(/^.*?\.\//u, "");
  if (common.has(path)) {
    if (common.get(path) !== maps[1].sourcesContent[i])
      throw new Error("Common source mismatch: " + path);
    commonFilesCompared++;
  }
}
if (commonFilesCompared < 30)
  throw new Error("Insufficient shared-source coverage");
const report = {
  command: "make provenance",
  checkedAt: new Date().toISOString(),
  node: process.version,
  gate: "X1",
  verdict: "PASS_BOUNDED",
  commit,
  checks,
  sourceMapChecks: {
    commonFilesCompared,
    inputHandlerInBothMatchesPinnedSource: matched,
    browserKeyboardMatchesPinnedSource: true,
  },
  limits:
    "Registry integrity, every packaged installed file, all shared common source-map files and selected pinned upstream source checks, not independent reproducible build or signature verification. Optional addons not admitted.",
};
await writeFile(
  new URL("../evidence/xterm-artifacts.json", import.meta.url),
  JSON.stringify(report, null, 2) + "\n",
);
console.log(
  "X1 PASS_BOUNDED: registry tarball integrity, embedded commit and common InputHandler source match; browser Keyboard matches pinned source.",
);
