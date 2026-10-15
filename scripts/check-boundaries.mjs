import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--locked", "--no-deps", "--format-version", "1"],
    {
      encoding: "utf8",
      env: { ...process.env, CARGO_HOME: process.cwd() + "/.cache/cargo" },
    },
  ),
);
const allowed = {
  protocol: [],
  engine: ["protocol"],
  domain: [],
  platform: ["domain"],
  service: ["protocol", "domain", "platform"],
  client: ["protocol"],
  console: ["client"],
};
const rust = new Map();
for (const p of metadata.packages) {
  const name = p.name.replace("project-x-", "");
  assert.ok(name in allowed, name);
  const edges = p.dependencies
    .filter((d) => d.name.startsWith("project-x-"))
    .map((d) => d.name.replace("project-x-", ""));
  assert.deepEqual(
    edges.sort(),
    allowed[name].toSorted(),
    p.name + " dependency boundary",
  );
  rust.set(name, edges);
}
function acyclic(graph) {
  const active = new Set(),
    done = new Set();
  function visit(n) {
    assert.ok(!active.has(n), "Cycle " + n);
    if (done.has(n)) return;
    active.add(n);
    for (const d of graph.get(n) ?? []) visit(d);
    active.delete(n);
    done.add(n);
  }
  for (const n of graph.keys()) visit(n);
}
acyclic(rust);
for (const p of metadata.packages) {
  if (p.name === "project-x-engine")
    assert.ok(
      p.targets.every((t) => !t.kind.includes("lib")),
      "Engine must stay binary-only",
    );
  if (["project-x-client", "project-x-console"].includes(p.name)) {
    const contents = await readFile(
      p.manifest_path.replace("Cargo.toml", "src/lib.rs"),
      "utf8",
    );
    assert.ok(
      !/internal_engine/u.test(contents),
      "Client imports private engine namespace",
    );
  }
}
const js = new Map();
const permitted = {
  "@project-x/desktop": ["@project-x/protocol", "@project-x/terminal-adapter"],
  "@project-x/engine-host": ["@project-x/terminal-adapter"],
  "@project-x/protocol": [],
  "@project-x/terminal-adapter": [],
};
for (const root of ["apps", "packages"])
  for (const dir of await readdir(root)) {
    const p = JSON.parse(
      await readFile(root + "/" + dir + "/package.json", "utf8"),
    );
    const dependencies = Object.keys(p.dependencies ?? {});
    const local = dependencies.filter((x) => x.startsWith("@project-x/"));
    assert.deepEqual(
      local.sort(),
      permitted[p.name].toSorted(),
      p.name + " imports",
    );
    js.set(p.name, local);
    if (dir !== "terminal-adapter")
      assert.ok(
        !dependencies.some((x) => x.startsWith("@xterm/")),
        "xterm dependency outside adapter",
      );
    for (const file of await readdir(root + "/" + dir + "/src")) {
      if (!file.endsWith(".ts") || file === "generated.ts") continue;
      const content = await readFile(root + "/" + dir + "/src/" + file, "utf8");
      assert.ok(!/\._core\b/u.test(content), "Private xterm graph traversal");
      if (dir === "desktop")
        assert.ok(
          !/['"](?:node:|@project-x\/terminal-adapter\/node|.*crates\/|.*schemas\/private)/u.test(
            content,
          ),
          "Renderer capability boundary",
        );
      if (dir !== "terminal-adapter")
        assert.ok(!/['"]@xterm\//u.test(content), "Direct engine import");
    }
  }
acyclic(js);
assert.equal(metadata.packages.length, 7);
assert.equal(js.size, 4);
console.log(
  "Boundary checks passed: seven Rust crates (engine is binary-only EA1), four TS packages, allowed edges acyclic. Static checks do not enforce runtime authority.",
);
