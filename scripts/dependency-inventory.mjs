// Metadata inventory, deliberately not called an SBOM or legal/advisory clearance.
import { readFile, readdir, writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
const hash = (s) => createHash("sha256").update(s).digest("hex");
const lock = await readFile("Cargo.lock", "utf8");
const registry = new Map();
for (const folder of await readdir(".cache/cargo/registry/src")) {
  for (const crate of await readdir(".cache/cargo/registry/src/" + folder)) {
    const path =
      ".cache/cargo/registry/src/" + folder + "/" + crate + "/Cargo.toml";
    const text = await readFile(path, "utf8");
    const pkg = text.match(/\[package\]([\s\S]*?)(?=\n\[|$)/u)?.[1] ?? "";
    registry.set(crate, {
      license: pkg.match(/^license\s*=\s*"([^"]+)"/mu)?.[1] ?? null,
      license_file: pkg.match(/^license-file\s*=\s*"([^"]+)"/mu)?.[1] ?? null,
    });
  }
}
const cargo = [];
for (const block of lock.split("[[package]]").slice(1)) {
  const get = (key) =>
    block.match(new RegExp("^" + key + ' = "([^"]+)"', "mu"))?.[1] ?? null;
  const name = get("name"),
    version = get("version");
  cargo.push({
    name,
    version,
    source: get("source"),
    checksum: get("checksum"),
    ...registry.get(name + "-" + version),
  });
}
const js = [];
for (const entry of await readdir("node_modules/.pnpm", {
  withFileTypes: true,
})) {
  if (!entry.isDirectory()) continue;
  const dir = entry.name;
  if (dir === "node_modules" || dir.startsWith(".")) continue;
  const root = "node_modules/.pnpm/" + dir + "/node_modules";
  for (const first of await readdir(root)) {
    const candidates = first.startsWith("@")
      ? (await readdir(root + "/" + first)).map((n) => first + "/" + n)
      : [first];
    for (const name of candidates) {
      const path = root + "/" + name + "/package.json";
      try {
        const p = JSON.parse(await readFile(path, "utf8"));
        if (!js.some((x) => x.name === p.name && x.version === p.version))
          js.push({
            name: p.name,
            version: p.version,
            license: p.license ?? null,
            has_install_scripts: Boolean(
              p.scripts?.preinstall ||
                p.scripts?.install ||
                p.scripts?.postinstall,
            ),
          });
      } catch (e) {
        if (e.code !== "ENOENT") throw e;
      }
    }
  }
}
const locks = {};
for (const file of ["Cargo.lock", "pnpm-lock.yaml", "requirements-docs.txt"])
  locks[file] = hash(await readFile(file));
const result = {
  description:
    "Lock-selected Cargo entries and installed registry metadata; unfetched target licenses unknown. Cargo manifest license extraction handles simple scalar declarations only. No legal clearance, advisory scan, release SBOM or build attestation.",
  locks,
  cargo,
  js: js.sort((a, b) => a.name.localeCompare(b.name)),
  pnpmPolicy: {
    ignoreScripts: true,
    minimumReleaseAgeMinutes: 10080,
    workspaceProtocol: true,
  },
  scriptsExecuted:
    "Cargo build.rs/proc macros as part of compilation; no JS dependency lifecycle scripts allowed",
};
await writeFile(
  "evidence/dependencies.json",
  JSON.stringify(result, null, 2) + "\n",
);
console.log(
  `Inventoried ${cargo.length} Cargo lock entries and ${js.length} installed JS packages; lock SHA256 recorded.`,
);
