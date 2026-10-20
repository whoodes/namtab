import { spawnSync } from "node:child_process";
import { readFile, writeFile, mkdir, readdir } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import os from "node:os";
import { resolve } from "node:path";
const root = new URL("../", import.meta.url).pathname;
const version = (
  await readFile(new URL("../.node-version", import.meta.url), "utf8")
).trim();
if (process.version !== "v" + version)
  throw new Error("Use pinned project Node through make verify");
const sourceOnly = process.argv[2] === "--source-foundation";
if (process.argv.length > (sourceOnly ? 3 : 2))
  throw new Error("Usage: verify.mjs [--source-foundation]");
const scopeArgs = sourceOnly ? ["--exclude", "project-x-engine"] : [];
console.log(
  sourceOnly
    ? "Scope: source foundation; historical native worker excluded (not full workspace)."
    : "Scope: full workspace including historical native worker.",
);
const env = {
  ...process.env,
  CARGO_HOME: root + ".cache/cargo",
  PIP_CACHE_DIR: root + ".cache/pip",
  NPM_CONFIG_USERCONFIG: "/dev/null",
  NPM_CONFIG_GLOBALCONFIG: "/dev/null",
  PROPTEST_RNG_SEED: "424242",
  PROPTEST_CASES: "128",
  XDG_CACHE_HOME: root + ".cache/xdg",
  XDG_DATA_HOME: root + ".cache/xdg-data",
  XDG_STATE_HOME: root + ".cache/xdg-state",
};
const command = (bin, args) => {
  const start = Date.now();
  const r = spawnSync(bin, args, {
    cwd: root,
    env,
    encoding: "utf8",
    timeout: 120000,
    maxBuffer: 4 * 1024 * 1024,
  });
  return {
    command: [bin, ...args],
    exit: r.status,
    error: r.error?.code ?? null,
    elapsed_ms: Date.now() - start,
    stdout: r.stdout ?? "",
    stderr: r.stderr ?? "",
  };
};
const versions = [
  ["cargo", ["--version"]],
  ["rustc", ["--version"]],
  ["node", ["--version"]],
  ["node", [".tools/pnpm/package/bin/pnpm.cjs", "--version"]],
  [".venv-docs/bin/python", ["--version"]],
  ["git", ["--version"]],
].map(([b, a]) => command(b, a));
const systemTests = (await readdir(root + "tests/system"))
  .filter((name) => name.endsWith(".test.mjs"))
  .sort()
  .map((name) => "tests/system/" + name);
if (systemTests.length === 0) throw new Error("No system test files found");
const binaryDirectory = resolve(
  root,
  env.CARGO_TARGET_DIR || "target",
  "debug",
);
const realRun = root + ".artifacts/mk1-ci-" + randomUUID();
const steps = [
  [
    "repository-local-links",
    ".venv-docs/bin/python",
    ["-B", "scripts/check-doc-links.py"],
  ],
  [
    "js-format",
    "node",
    [
      ".tools/pnpm/package/bin/pnpm.cjs",
      "exec",
      "prettier",
      "--check",
      "scripts",
      "apps",
      "packages",
      "tests/system",
      "tests/mk1/outer-screen.mjs",
      "tests/h0",
      "tests/fuzz",
      "tests/assessment",
      "tests/ea1",
      "package.json",
      "tsconfig.json",
      "mkdocs.yml",
      "pnpm-workspace.yaml",
      ".github",
    ],
  ],
  [
    "frozen-offline-install",
    "node",
    [
      ".tools/pnpm/package/bin/pnpm.cjs",
      "install",
      "--frozen-lockfile",
      "--offline",
      "--ignore-scripts",
    ],
  ],
  ["rust-format", "cargo", ["fmt", "--all", "--", "--check"]],
  [
    "rust-clippy",
    "cargo",
    [
      "clippy",
      "--workspace",
      "--all-targets",
      ...scopeArgs,
      "--locked",
      "--",
      "-D",
      "warnings",
    ],
  ],
  [
    "rust-contracts",
    "cargo",
    ["test", "--workspace", ...scopeArgs, "--locked"],
  ],
  ["mk1-launch-build", "python3", ["-B", "scripts/mk1.py", "build"]],
  [
    "mk1-real-journeys",
    "python3",
    [
      "-B",
      "tests/mk1/e2e.py",
      "--bin-dir",
      binaryDirectory,
      "--terminfo",
      root + ".artifacts/terminfo",
      "--artifacts",
      realRun,
    ],
  ],
  [
    "mk1-two-agent-journeys",
    "python3",
    [
      "-B",
      "tests/mk1/agents.py",
      "--bin-dir",
      binaryDirectory,
      "--terminfo",
      root + ".artifacts/terminfo",
      "--artifacts",
      realRun + "-agents",
    ],
  ],
  [
    "h0-real-single-write-handoff",
    binaryDirectory + "/namtab-h0-probe",
    [
      "--artifacts",
      realRun + "-h0",
      "--worker",
      binaryDirectory + "/namtab-terminal-worker",
      "--pty-helper",
      binaryDirectory + "/namtab-pty-helper",
    ],
  ],
  [
    "h0-model-subset-comparison",
    "node",
    ["tests/h0/compare-model.mjs", realRun + "-h0/report.json"],
  ],
  [
    "h0-producer-consumer",
    "python3",
    [
      "-B",
      "tests/h0/flow.py",
      "--bin-dir",
      binaryDirectory,
      "--artifacts",
      realRun + "-flow",
    ],
  ],
  ["bindings-drift", "node", ["scripts/generate-bindings.mjs", "--check"]],
  ["typescript-build", "node", [".tools/pnpm/package/bin/pnpm.cjs", "build"]],
  ["dependency-boundaries", "node", ["scripts/check-boundaries.mjs"]],
  ["fixture-integrity", "node", ["scripts/check-fixtures.mjs"]],
  ["synthetic-contracts", "node", ["--test", ...systemTests]],
  ["seeded-partitions", "node", ["tests/fuzz/chunk-corpus.mjs"]],
  [
    "offline-console",
    "cargo",
    ["run", "--quiet", "--locked", "-p", "project-x-console", "--", "diagnose"],
  ],
  [
    "strict-mkdocs",
    ".venv-docs/bin/python",
    ["-m", "mkdocs", "build", "--strict"],
  ],
];
const checks = [];
for (const [name, bin, args] of steps) {
  const r = command(bin, args);
  const verdict = r.exit === 0 ? "PASS" : "FAIL";
  checks.push({ name, verdict, ...r });
  console.log(name + ": " + verdict);
}
const folder = new URL(
  "../.artifacts/verification/" + randomUUID() + "/",
  import.meta.url,
);
await mkdir(folder, { recursive: true });
const report = {
  date: new Date().toISOString(),
  scope: sourceOnly
    ? "source-foundation: excludes historical native worker only"
    : "full-workspace",
  excluded_packages: sourceOnly ? ["project-x-engine"] : [],
  environment: {
    platform: os.platform(),
    release: os.release(),
    arch: os.arch(),
    cpu_count: os.cpus().length,
    memory_bytes: os.totalmem(),
    os_release: await readFile("/etc/os-release", "utf8"),
    display:
      "Includes a headless real Linux PTY/host-TUI journey; no Electron display, IME, screen-reader or human visual qualification in this runner.",
  },
  versions,
  property_seed: env.PROPTEST_RNG_SEED,
  property_cases: env.PROPTEST_CASES,
  checks,
  system_test_files: systemTests,
  limits:
    (sourceOnly
      ? "Source-only selection excludes the historical native worker from Cargo Clippy/tests; not a full-workspace pass. "
      : "") +
    "Foundation PASS is not terminal-engine certification. Historical X4/EA1 qualification is separate; retained regressions do not change their recorded FAIL verdicts. No performance budgets measured.",
};
await writeFile(
  new URL("checks.json", folder),
  JSON.stringify(report, null, 2) + "\n",
  { flag: "wx" },
);
console.log(
  "Foundation verification report: " + new URL("checks.json", folder).pathname,
);
if (
  checks.some((c) => c.verdict === "FAIL") ||
  versions.some((c) => c.exit !== 0)
)
  process.exitCode = 1;
