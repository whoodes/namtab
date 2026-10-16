// Invoke the immutable X4 oracle without its old CLI's committed-report write.
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { run } from "../tests/system/engine-probe.mjs";

const result = await run();
const folder = new URL("../.artifacts/x4-runs/", import.meta.url);
await mkdir(folder, { recursive: true });
const report = new URL(randomUUID() + ".json", folder);
await writeFile(report, JSON.stringify(result, null, 2) + "\n", {
  flag: "wx",
});
console.log(JSON.stringify(result, null, 2));
console.error("Historical X4 evidence: " + report.pathname);
if (process.argv.includes("--require-pass") && result.verdict === "FAIL")
  process.exitCode = 1;
