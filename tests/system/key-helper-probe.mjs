// Research-only extraction of one pinned upstream private helper. Never imported by production.
import ts from "typescript";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import vm from "node:vm";
const require = createRequire(
  new URL("../../packages/terminal-adapter/package.json", import.meta.url),
);
const base = require
  .resolve("@xterm/xterm/package.json")
  .replace(/package.json$/u, "");
const map = JSON.parse(await readFile(base + "lib/xterm.js.map", "utf8"));
const source = (name) =>
  map.sourcesContent[map.sources.findIndex((s) => s.endsWith("./src/" + name))];
const keyboard = source("common/input/Keyboard.ts");
const types = await readFile(
  new URL("../../.cache/xterm-source/src/common/Types.ts", import.meta.url),
  "utf8",
);
const enumeration = types.match(
  /export const enum KeyboardResultType\s*\{[^}]+\}/su,
)?.[0];
if (!enumeration || !keyboard) throw new Error("Pinned source shape changed");
const combined =
  enumeration +
  "\n" +
  source("common/data/EscapeSequences.ts") +
  "\n" +
  keyboard.replace(/^import .*;\r?\n/gmu, "");
const js = ts.transpileModule(combined, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.CommonJS,
  },
}).outputText;
const context = { exports: {} };
vm.runInNewContext(js, context, { timeout: 1000 });
export const evaluate = context.exports.evaluateKeyboardEvent;
export function event(key, keyCode, options = {}) {
  return {
    key,
    keyCode,
    code: key,
    location: 0,
    shiftKey: false,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    ...options,
  };
}
