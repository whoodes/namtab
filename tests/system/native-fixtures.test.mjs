import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  cp,
  mkdtemp,
  mkdir,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  checkNativeFixtures,
  parseUniqueJson,
  shortPartitions,
} from "../../scripts/check-native-fixtures.mjs";

const corpus = "tests/fixtures/native-conformance";
const hash = (data) => createHash("sha256").update(data).digest("hex");
const readJson = async (root, path) =>
  JSON.parse(await readFile(join(root, path), "utf8"));
const writeJson = async (root, path, data) =>
  writeFile(join(root, path), JSON.stringify(data, null, 2) + "\n");
async function sandbox(run) {
  const root = await mkdtemp(join(tmpdir(), "native-fixtures-"));
  try {
    await cp("tests/fixtures", join(root, "tests/fixtures"), {
      recursive: true,
    });
    const manifest = await readJson(root, `${corpus}/manifest.json`);
    for (const source of manifest.source_records) {
      if (source.path.startsWith("tests/fixtures/")) continue;
      await mkdir(dirname(join(root, source.path)), { recursive: true });
      await cp(source.path, join(root, source.path));
    }
    await run(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}
async function changePayload(root, file, mutate) {
  const path = `${corpus}/${file}`;
  const value = await readJson(root, path);
  mutate(value);
  await writeJson(root, path, value);
  // Update integrity deliberately: subsequent rejection must come from the
  // independent shape/coverage/budget checks, not merely stale file hashes.
  const manifest = await readJson(root, `${corpus}/manifest.json`);
  manifest.files.find((f) => f.path === file).sha256 = hash(
    await readFile(join(root, path)),
  );
  await writeJson(root, `${corpus}/manifest.json`, manifest);
}

test("native corpus and earlier native manifests receive static checks only", async () => {
  const result = await checkNativeFixtures();
  assert.equal(result.state_cases, 26);
  assert.equal(result.input_cases, 24);
  assert.equal(result.transport_partitions_checked, 879);
  assert(result.local_payload_digests_verified >= 20);
  assert.equal(result.static_oracle_data, "PASS");
  assert.equal(
    result.engine_state_continuation_key_reply_conformance,
    "NOT RUN",
  );
});

test("merged ownership fixtures cannot disappear as an entire category", async () =>
  sandbox(async (root) => {
    await rm(join(root, "tests/fixtures/native-ownership"), {
      recursive: true,
    });
    await assert.rejects(
      checkNativeFixtures(root),
      /missing native fixture directory: native-ownership/,
    );
  }));

test("short transport enumeration contains every contiguous partition", () => {
  assert.deepEqual([...shortPartitions("01")], [["01"]]);
  assert.deepEqual(
    [...shortPartitions("010203")],
    [["010203"], ["01", "0203"], ["0102", "03"], ["01", "02", "03"]],
  );
  assert.equal([...shortPartitions("0001020304050607")].length, 128);
  assert.throws(() => [...shortPartitions("000102030405060708")], /byte cap/);
});

test("JSON duplicate keys cannot overwrite review expectations", () => {
  assert.throws(
    () => parseUniqueJson('{"a":1,"\\u0061":2}'),
    /duplicate JSON key a/,
  );
  assert.throws(
    () => parseUniqueJson('{"nested":[{"a":1,"a":2}]}'),
    /duplicate JSON key a/,
  );
  assert.deepEqual(parseUniqueJson('{"a":{"x":1},"b":{"x":2},"text":"a: b"}'), {
    a: { x: 1 },
    b: { x: 2 },
    text: "a: b",
  });
});

test("changed expected cells fail integrity", async () =>
  sandbox(async (root) => {
    const path = `${corpus}/goldens.json`,
      value = await readJson(root, path);
    value.state_cases[0].checkpoints.at(-1).state.grids.primary.rows[0] =
      "WRONG       ";
    await writeJson(root, path, value);
    await assert.rejects(checkNativeFixtures(root), /fixture drift/);
  }));

test("earlier identity, receipt and state payloads cannot drift silently", async () => {
  for (const path of [
    "native-contract/valid-event-cuts.json",
    "native-receipts/valid-lifecycle.json",
    "native-state/continuations.json",
    "native-provenance/admission.json",
  ]) {
    await sandbox(async (root) => {
      await writeFile(join(root, "tests/fixtures", path), "{}\n");
      await assert.rejects(checkNativeFixtures(root), /fixture drift/);
    });
  }
});

for (const [name, mutate, error] of [
  [
    "initial geometry metadata changed",
    (g) => {
      g.state_cases[0].initial_geometry.rows = 6;
    },
    /initial geometry cut/,
  ],
  [
    "input mode value invalid",
    (g) => {
      g.input_cases[0].modes.application_cursor = "yes";
    },
    /string.*boolean/s,
  ],
  [
    "continuation has no distinguishing visible effect",
    (g) => {
      g.state_cases
        .find((c) => c.id === "saved-attributes-a")
        .checkpoints.at(-1).state.grids.primary.cell_attribute_overrides = [];
    },
    /distinguish visible effects/,
  ],
  [
    "out-of-range cursor",
    (g) => {
      g.state_cases[0].checkpoints[0].state.grids.primary.cursor = [6, 1];
    },
    /outside geometry/,
  ],
  [
    "missing row",
    (g) => {
      g.state_cases[0].checkpoints[0].state.grids.alternate.rows.pop();
    },
    /row count/,
  ],
  [
    "wide cell introduced into excluded profile",
    (g) => {
      g.state_cases[0].checkpoints[0].state.grids.primary.rows[0] =
        "界ello       ";
    },
    /unadvertised scalar/,
  ],
  [
    "missing byte cut",
    (g) => {
      g.state_cases[0].events[0].transport.single_cut_offsets.splice(1, 1);
    },
    /missing byte cuts/,
  ],
  [
    "short exhaustive partitions disabled",
    (g) => {
      g.state_cases[0].events[0].transport.exhaustive_partitions = false;
    },
    /short partition requirement/,
  ],
  [
    "transport bytes tampered",
    (g) => {
      g.state_cases[0].events[0].transport.sample_chunks_hex = ["00"];
    },
    /chunk bytes differ/,
  ],
  [
    "historical bytes changed with matching chunks",
    (g) => {
      const e = g.state_cases[0].events[0];
      e.hex = "6a656c6c6f";
      e.transport.sample_chunks_hex = [e.hex];
    },
    /historical input bytes changed/,
  ],
  [
    "missing wide exclusion coverage",
    (g) => {
      g.state_cases = g.state_cases.filter(
        (c) => c.id !== "wide-combining-replacement",
      );
    },
    /coverage mismatch|missing coverage/,
  ],
  [
    "hidden difference erased",
    (g) => {
      g.state_cases.find(
        (c) => c.id === "pending-wrap-a",
      ).checkpoints[0].state.grids.primary.pending_wrap = false;
    },
    /differ in hidden state/,
  ],
  [
    "duplicate reply identity",
    (g) => {
      g.state_cases.find(
        (c) => c.id === "identical-queries",
      ).checkpoints[0].proposed_replies[1].ordinal = 1;
    },
    /reply identities: duplicate/,
  ],
  [
    "refused input emits bytes",
    (g) => {
      g.input_cases.find((c) => c.id === "wide-text").expected.hex = "41";
    },
    /refusal must write zero/,
  ],
])
  test(`rehashed corpus still rejects ${name}`, async () =>
    sandbox(async (root) => {
      await changePayload(root, "goldens.json", mutate);
      await assert.rejects(checkNativeFixtures(root), error);
    }));

for (const [name, mutate, error] of [
  [
    "larger CSI cap",
    (b) => {
      b.limits.csi_total_bytes = 513;
    },
    /finite profile caps changed/,
  ],
  [
    "string retention enabled",
    (b) => {
      b.limits.retained_string_payload_bytes = 65536;
    },
    /finite profile caps changed/,
  ],
  [
    "unbounded recipe expansion",
    (b) => {
      b.recipes[2].recipe.repeat_count = 200000;
      b.recipes[2].expanded_bytes = 200005;
    },
    /materialization cap/,
  ],
  [
    "oversized keeper event",
    (b) => {
      b.recipes[0].keeper_event_max_bytes = 513;
    },
    /keeper event cap/,
  ],
])
  test(`rehashed bounds reject ${name}`, async () =>
    sandbox(async (root) => {
      await changePayload(root, "bounds.json", mutate);
      await assert.rejects(checkNativeFixtures(root), error);
    }));

test("duplicate, unsafe and unlisted native payload paths are rejected", async () => {
  for (const kind of ["duplicate", "unsafe", "unlisted"])
    await sandbox(async (root) => {
      const path = `${corpus}/manifest.json`,
        manifest = await readJson(root, path);
      if (kind === "duplicate") manifest.files.push(manifest.files[0]);
      if (kind === "unsafe")
        manifest.source_records[0].path = "../outside.json";
      if (kind === "unlisted")
        await writeFile(join(root, corpus, "extra.json"), "{}\n");
      await writeJson(root, path, manifest);
      await assert.rejects(
        checkNativeFixtures(root),
        /duplicate|unsafe fixture path|unlisted native/,
      );
    });
});

test("native source pins reject original result drift and missing provenance", async () => {
  await sandbox(async (root) => {
    await writeFile(join(root, "evidence/engine-continuation.json"), "{}\n");
    await assert.rejects(
      checkNativeFixtures(root),
      /source drift: original-x4-result/,
    );
  });
  await sandbox(async (root) => {
    const path = `${corpus}/manifest.json`,
      manifest = await readJson(root, path);
    manifest.source_records = manifest.source_records.filter(
      (s) => s.id !== "original-x4-oracle",
    );
    await writeJson(root, path, manifest);
    await assert.rejects(
      checkNativeFixtures(root),
      /missing source pin: original-x4-oracle/,
    );
  });
});

test("fixture symlinks cannot read outside the checkout", async () =>
  sandbox(async (root) => {
    const path = join(root, corpus, "goldens.json");
    await rm(path);
    await symlink(join(process.cwd(), corpus, "goldens.json"), path);
    await assert.rejects(checkNativeFixtures(root), /symlink escapes root/);
  }));

test("coverage removal fails even after updating hashes and index", async () =>
  sandbox(async (root) => {
    await changePayload(root, "goldens.json", (g) => {
      g.state_cases = g.state_cases.filter(
        (c) => c.id !== "wide-combining-replacement",
      );
    });
    const path = `${corpus}/manifest.json`,
      manifest = await readJson(root, path);
    for (const category of Object.keys(manifest.coverage))
      manifest.coverage[category] = manifest.coverage[category].filter(
        (id) => id !== "wide-combining-replacement",
      );
    await writeJson(root, path, manifest);
    await assert.rejects(
      checkNativeFixtures(root),
      /missing coverage: wide-exclusion/,
    );
  }));

test("missing earlier native directory cannot silently remove integrity coverage", async () =>
  sandbox(async (root) => {
    await rm(join(root, "tests/fixtures/native-contract"), { recursive: true });
    await assert.rejects(
      checkNativeFixtures(root),
      /missing native fixture directory: native-contract/,
    );
  }));

test("existing native payload duplicate keys are rejected before digest checks", async () =>
  sandbox(async (root) => {
    await writeFile(
      join(root, "tests/fixtures/native-receipts/valid-lifecycle.json"),
      '{"a":1,"a":2}',
    );
    await assert.rejects(checkNativeFixtures(root), /duplicate JSON key a/);
  }));
