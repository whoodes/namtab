import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, readdir, realpath } from "node:fs/promises";
import { resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

const corpusDirectory = "tests/fixtures/native-conformance";
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const same = (a, b, message) => assert.deepEqual(a, b, message);
const requiredCoverage = [
  "historical",
  "continuation",
  "hidden-pair",
  "wrap",
  "saved-attributes",
  "utf8",
  "eof",
  "malformed",
  "strings",
  "wide-exclusion",
  "queries",
  "reply-authority",
  "reset",
  "both-buffers",
  "resize",
  "crop-no-reflow",
  "input-modes",
];
const limits = {
  rows: [2, 200],
  columns: [2, 500],
  cells_per_grid: 100000,
  esc_intermediates: 2,
  esc_total_bytes: 4,
  csi_fields: 64,
  csi_digits: 5,
  csi_value: 65535,
  csi_intermediates: 2,
  csi_total_bytes: 512,
  utf8_prefix_bytes: 3,
  retained_string_payload_bytes: 0,
  history_rows: 1000,
  output_event_bytes: 512,
  reply_bytes: 32,
  pending_replies: 128,
  pending_reply_bytes: 4096,
  work_items_per_yield: 4096,
  input_content_bytes: 8192,
  input_batch_events: 64,
};

// JSON.parse checks grammar first. This second lexical pass detects duplicate
// object keys, including escaped spellings, before an overwritten value is trusted.
export function parseUniqueJson(bytes, label = "JSON") {
  const source = bytes.toString();
  const value = JSON.parse(source);
  const tokens = source.match(/"(?:\\.|[^"\\])*"|[{}\[\]:,]/gs) ?? [];
  const stack = [];
  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i];
    if (token === "{") stack.push(new Set());
    else if (token === "[") stack.push(null);
    else if (token === "}" || token === "]") stack.pop();
    else if (token.startsWith('"') && tokens[i + 1] === ":") {
      const key = JSON.parse(token);
      const keys = stack.at(-1);
      assert(keys && !keys.has(key), `${label}: duplicate JSON key ${key}`);
      keys.add(key);
    }
  }
  return value;
}

function safePath(path) {
  assert(
    typeof path === "string" && /^[A-Za-z0-9._/-]+$/.test(path),
    "unsafe fixture path",
  );
  assert(
    !path.startsWith("/") &&
      path.split("/").every((p) => p && p !== "." && p !== ".."),
    "unsafe fixture path",
  );
  return path;
}
function unique(values, label) {
  assert(Array.isArray(values), `${label}: expected array`);
  assert.equal(new Set(values).size, values.length, `${label}: duplicate`);
}
function hex(value, label, max = 512) {
  assert(
    typeof value === "string" && /^(?:[0-9a-f]{2})*$/.test(value),
    `${label}: canonical hex`,
  );
  assert(value.length / 2 <= max, `${label}: byte cap`);
  return value.length / 2;
}
function position(value, rows, columns, label) {
  assert(
    Array.isArray(value) && value.length === 2 && value.every(Number.isInteger),
    `${label}: position`,
  );
  assert(
    value[0] >= 1 && value[0] <= rows && value[1] >= 1 && value[1] <= columns,
    `${label}: outside geometry`,
  );
}
function attributes(value) {
  same(Object.keys(value).sort(), [
    "background",
    "bold",
    "foreground",
    "inverse",
    "underline",
  ]);
  for (const key of ["foreground", "background"])
    assert(
      value[key] === "default" ||
        (Number.isInteger(value[key]) && value[key] >= 0 && value[key] <= 7),
      "attribute color",
    );
  for (const key of ["bold", "underline", "inverse"])
    assert.equal(typeof value[key], "boolean");
}
function save(value, rows, columns) {
  position(value.cursor, rows, columns, "save cursor");
  attributes(value.attributes);
  for (const key of ["pending_wrap", "origin", "autowrap"])
    assert.equal(typeof value[key], "boolean");
  if (value.pending_wrap)
    assert.equal(value.cursor[1], columns, "saved pending wrap column");
}
function modes(value) {
  const booleanModes = [
    "origin",
    "autowrap",
    "cursor_visible",
    "application_cursor",
    "application_keypad",
    "bracketed_paste",
    "focus_reporting",
  ];
  same(
    Object.keys(value).sort(),
    [...booleanModes, "cursor_shape", "backarrow"].sort(),
  );
  for (const key of booleanModes) assert.equal(typeof value[key], "boolean");
  assert.equal(value.cursor_shape, "steady-block");
  assert(["BS", "DEL"].includes(value.backarrow));
}
function state(value) {
  const { rows, columns } = value.geometry;
  assert(
    Number.isInteger(rows) &&
      rows >= 2 &&
      rows <= 200 &&
      Number.isInteger(columns) &&
      columns >= 2 &&
      columns <= 500,
    "state geometry",
  );
  assert(rows * columns <= 100000, "cell cap");
  assert(
    ["primary", "alternate"].includes(value.active_buffer),
    "active buffer",
  );
  same(Object.keys(value.grids).sort(), ["alternate", "primary"]);
  for (const grid of Object.values(value.grids)) {
    assert.equal(grid.rows.length, rows, "grid row count");
    for (const row of grid.rows) {
      assert.equal([...row].length, columns, "grid column count");
      for (const scalar of row) {
        const cp = scalar.codePointAt(0);
        assert(
          (cp >= 0x20 && cp <= 0x7e) ||
            (cp >= 0xa0 && cp <= 0xff && cp !== 0xad) ||
            cp === 0xfffd,
          "unadvertised scalar in golden cells",
        );
      }
    }
    attributes(grid.default_cell_attributes);
    unique(
      grid.cell_attribute_overrides.map((o) => JSON.stringify(o.at)),
      "cell attributes",
    );
    for (const override of grid.cell_attribute_overrides) {
      position(override.at, rows, columns, "cell attributes");
      attributes(override.attributes);
    }
    position(grid.cursor, rows, columns, "grid cursor");
    assert.equal(typeof grid.pending_wrap, "boolean");
    if (grid.pending_wrap)
      assert.equal(grid.cursor[1], columns, "pending wrap column");
    assert(
      grid.region.length === 2 &&
        grid.region.every(Number.isInteger) &&
        grid.region[0] >= 1 &&
        grid.region[0] < grid.region[1] &&
        grid.region[1] <= rows,
      "region bounds",
    );
    unique(grid.tabs, "tabs");
    assert(
      grid.tabs.every(
        (n, i, a) =>
          Number.isInteger(n) &&
          n >= 1 &&
          n <= columns &&
          (i === 0 || n > a[i - 1]),
      ),
      "tab bounds/order",
    );
    unique(grid.soft_wrap_rows, "soft wraps");
    assert(
      grid.soft_wrap_rows.every(
        (r) => Number.isInteger(r) && r >= 1 && r < rows,
      ),
      "soft wrap rows",
    );
    save(grid.save, rows, columns);
  }
  attributes(value.current_attributes);
  modes(value.modes);
  if (value.return_slot !== null) {
    assert.equal(value.active_buffer, "alternate");
    save(value.return_slot, rows, columns);
  }
  const parser = value.parser;
  assert(
    [
      "ground",
      "esc-collect",
      "esc-ignore",
      "csi-parameters",
      "csi-ignore",
      "string-discard",
    ].includes(parser.state),
    "parser state",
  );
  const prefixBytes = hex(parser.utf8_prefix_hex, "UTF-8 prefix", 3);
  if (prefixBytes) assert.equal(parser.state, "ground", "UTF-8 only in ground");
  if (parser.state === "csi-parameters") {
    assert(
      parser.fields.length <= 64 && parser.total_bytes <= 512,
      "CSI retained bounds",
    );
    for (const field of parser.fields)
      assert(
        /^\d{1,5}$/.test(field.digits) &&
          Number(field.digits) === field.value &&
          field.value <= 65535,
        "CSI field bounds",
      );
    hex(parser.intermediates_hex, "CSI intermediates", 2);
  }
  if (parser.state === "string-discard") {
    assert(
      ["OSC", "DCS", "APC", "PM", "SOS"].includes(parser.kind),
      "string kind",
    );
    assert.equal(parser.retained_payload_bytes, 0, "string retention cap");
    assert.equal(typeof parser.pending_escape, "boolean");
  }
  assert(
    Array.isArray(value.primary_history) &&
      value.primary_history.length <= 1000,
    "history cap",
  );
  assert.equal(typeof value.stream_closed, "boolean");
  assert.equal(value.interpretation, "known");
}

// Enumerates transport schedules only, never terminal actions or expected state.
export function* shortPartitions(value) {
  const length = hex(value, "partition input", 8);
  if (!length) {
    yield [];
    return;
  }
  for (let mask = 0; mask < 2 ** (length - 1); mask++) {
    const chunks = [];
    let start = 0;
    for (let cut = 1; cut < length; cut++) {
      if (mask & (1 << (cut - 1))) {
        chunks.push(value.slice(start * 2, cut * 2));
        start = cut;
      }
    }
    chunks.push(value.slice(start * 2));
    yield chunks;
  }
}

export function validateCorpus(manifest, goldens, bounds, historicalInputs) {
  same(manifest.format, "n0-conformance-manifest-1");
  for (const doc of [manifest, goldens, bounds])
    same(doc.profile, "px-dev-text-0");
  same(manifest.historical_selection_verdicts, { X4: "FAIL", EA1: "FAIL" });
  for (const field of ["new_data", "upstream", "capture"])
    assert(
      typeof manifest.licensing[field] === "string" &&
        manifest.licensing[field].length > 0,
      "missing corpus provenance/license",
    );
  const sourceIds = new Set(manifest.source_records.map((s) => s.id));
  for (const c of [...goldens.state_cases, ...goldens.input_cases]) {
    assert(c.source && c.source.kind, "missing case provenance");
    if (c.source.source_record)
      assert(sourceIds.has(c.source.source_record), "unknown source record");
  }
  same(manifest.reference_disagreement.allowed_classes, [
    "standard-requirement",
    "deliberate-profile-difference",
    "implementation-bug",
    "reference-bug",
    "unresolved",
  ]);
  for (const d of manifest.reference_disagreement.cases) {
    assert(
      manifest.reference_disagreement.allowed_classes.includes(
        d.classification,
      ),
      "unclassified reference disagreement",
    );
    if (d.source_record)
      assert(sourceIds.has(d.source_record), "unknown disagreement source");
  }
  for (const key of [
    "state_conformance",
    "continuation_conformance",
    "key_reply_conformance",
    "NG1",
  ])
    same(manifest.status[key], "NOT RUN");
  same(manifest.status.NG0, "NOT ACCEPTED");
  same(manifest.partition_policy, {
    exhaustive_up_to_bytes: 8,
    every_single_cut: true,
    nonempty_contiguous_chunks_only: true,
    preserve_keeper_events: true,
    empty_chunks:
      "not output events; zero/end cuts are harness boundaries, never EOF",
  });
  same(bounds.limits, limits, "finite profile caps changed");
  same(manifest.future_requirements, ["V07", "V08", "V09", "V10", "V11"]);
  const cases = goldens.state_cases;
  unique(
    cases.map((c) => c.id),
    "state cases",
  );
  const byId = new Map(cases.map((c) => [c.id, c]));
  let partitionCount = 0;
  for (const c of cases) {
    unique(
      c.events.map((e) => e.id),
      "events",
    );
    unique(
      c.checkpoints.map((p) => p.id),
      "checkpoints",
    );
    assert(c.checkpoints.length > 0, "missing state checkpoints");
    same(
      c.initial_geometry,
      c.checkpoints[0].state.geometry,
      "initial geometry cut",
    );
    for (const e of c.events) {
      assert(["output", "resize", "output_eof"].includes(e.kind), "event kind");
      if (e.kind === "output") {
        const n = hex(e.hex, "output");
        assert(n > 0, "empty keeper event");
        same(
          e.transport.single_cut_offsets,
          Array.from({ length: n + 1 }, (_, i) => i),
          "missing byte cuts",
        );
        same(
          e.transport.exhaustive_partitions,
          n <= 8,
          "short partition requirement",
        );
        assert(e.transport.sample_chunks_hex.length > 0, "missing chunks");
        for (const chunk of e.transport.sample_chunks_hex)
          assert(hex(chunk, "chunk") > 0, "empty transport chunk");
        same(
          e.transport.sample_chunks_hex.join(""),
          e.hex,
          "chunk bytes differ from event",
        );
        if (n <= 8)
          for (const chunks of shortPartitions(e.hex)) {
            same(chunks.join(""), e.hex);
            partitionCount++;
          }
      } else if (e.kind === "resize")
        assert(
          Number.isInteger(e.rows) &&
            e.rows >= 2 &&
            e.rows <= 200 &&
            Number.isInteger(e.columns) &&
            e.columns >= 2 &&
            e.columns <= 500,
          "resize bounds",
        );
    }
    let prior = -1;
    for (const p of c.checkpoints) {
      const cut = c.events.findIndex((e) => e.id === p.after_event);
      assert(cut >= prior && cut >= 0, "checkpoint event order");
      prior = cut;
      state(p.state);
      const ids = [];
      for (const reply of p.proposed_replies) {
        const eventIndex = c.events.findIndex((e) => e.id === reply.event_id);
        assert(
          eventIndex >= 0 &&
            eventIndex <= cut &&
            c.events[eventIndex].kind === "output",
          "reply event cut",
        );
        assert(
          Number.isInteger(reply.ordinal) &&
            reply.ordinal >= 1 &&
            reply.ordinal <= 128,
          "reply ordinal",
        );
        ids.push(`${reply.event_id}:${reply.ordinal}`);
        hex(reply.hex, "reply", 32);
        assert(
          /^[\x20-\x7e\x1b]*$/.test(
            Buffer.from(reply.hex, "hex").toString("latin1"),
          ),
          "reply ASCII",
        );
      }
      unique(ids, "reply identities");
    }
    assert.equal(
      c.checkpoints.at(-1).after_event,
      c.events.at(-1).id,
      "missing final oracle",
    );
  }
  same(
    Object.keys(manifest.coverage).sort(),
    [...requiredCoverage].sort(),
    "coverage categories",
  );
  for (const category of requiredCoverage) {
    const expected = cases
      .filter((c) => c.coverage.includes(category))
      .map((c) => c.id);
    assert(expected.length > 0, `missing coverage: ${category}`);
    same(
      manifest.coverage[category],
      expected,
      `coverage mismatch: ${category}`,
    );
  }
  same(
    cases
      .filter((c) => c.source.kind === "historical-input-derivation")
      .map((c) => c.source.case_id)
      .sort(),
    historicalInputs.map((c) => c.name).sort(),
    "all original X4 inputs required",
  );
  for (const original of historicalInputs) {
    const c = byId.get(`historical-${original.name}`);
    same(
      c.events.map((e) => e.hex),
      [
        Buffer.from(original.before).toString("hex"),
        Buffer.from(original.after).toString("hex"),
      ],
      "historical input bytes changed",
    );
    same(
      c.checkpoints[0].state.geometry,
      { rows: 5, columns: 12 },
      "historical geometry",
    );
  }
  assert(
    goldens.visually_equal_pairs.length >= 2,
    "missing hidden-state pairs",
  );
  for (const pair of goldens.visually_equal_pairs) {
    const a = byId.get(pair.a),
      b = byId.get(pair.b);
    assert(a && b, "missing paired trace");
    const at = (c, id) => c.checkpoints.find((p) => p.id === id).state;
    const sa = at(a, pair.at_checkpoint),
      sb = at(b, pair.at_checkpoint);
    const visible = (s) => {
      const g = s.grids[s.active_buffer];
      return {
        rows: g.rows,
        attributes: g.cell_attribute_overrides,
        cursor: g.cursor,
      };
    };
    same(visible(sa), visible(sb), "pair must have equal visible state");
    const field = (s) => pair.hidden_path.split(".").reduce((v, k) => v[k], s);
    assert.notDeepEqual(
      field(sa),
      field(sb),
      "pair must differ in hidden state",
    );
    same(a.events.at(-1).hex, b.events.at(-1).hex, "paired continuation bytes");
    assert.notDeepEqual(
      visible(at(a, pair.after_checkpoint)),
      visible(at(b, pair.after_checkpoint)),
      "continuation must distinguish visible effects",
    );
  }
  unique(
    goldens.input_cases.map((c) => c.id),
    "input cases",
  );
  for (const c of goldens.input_cases) {
    assert(c.intent && c.modes, "input mode/intent cut");
    modes(c.modes);
    hex(c.expected.hex, "input bytes", 8192 + 12);
    assert(
      c.expected.error === null ||
        [
          "InvalidRequest",
          "UnsupportedCapability",
          "ResourceExhausted",
        ].includes(c.expected.error),
      "input error family",
    );
    if (c.expected.error)
      same(c.expected.hex, "", "refusal must write zero bytes");
  }
  for (const id of [
    "alias-Ctrl-I",
    "alias-Ctrl-M",
    "alias-Ctrl-[",
    "wide-text",
    "combining-text",
    "paste-crlf",
    "focus-in",
    "up-application",
    "kp1-application",
    "release",
    "enhancement",
  ])
    assert(
      goldens.input_cases.some((c) => c.id === id),
      `missing input coverage: ${id}`,
    );
  same(
    bounds.recipes.map((r) => r.id),
    ["csi-total-at-bound", "csi-total-over-bound", "long-discard"],
    "missing bounds coverage",
  );
  for (const r of bounds.recipes) {
    const recipe = r.recipe;
    assert(
      Number.isSafeInteger(recipe.repeat_count) && recipe.repeat_count >= 0,
      "repeat bound",
    );
    const n =
      hex(recipe.prefix_hex, "recipe prefix") +
      hex(recipe.repeat_hex, "recipe repeat") * recipe.repeat_count +
      hex(recipe.suffix_hex, "recipe suffix");
    same(r.expanded_bytes, n, "recipe arithmetic");
    assert(n <= 131072, "recipe materialization cap");
    same(r.keeper_event_max_bytes, 512, "keeper event cap");
    assert(Math.ceil(n / 512) <= 257, "recipe event cap");
    position(r.initial_cursor, 3, 8, "bound initial cursor");
    position(r.expected_cursor, 3, 8, "bound cursor");
    position(r.expected_x_at, 3, 8, "bound cell");
  }
  same(bounds.recipes[0].sequence_bytes, 512);
  same(bounds.recipes[1].sequence_bytes, 513);
  for (const r of bounds.recipes.slice(0, 2))
    same(
      r.sequence_bytes,
      r.expanded_bytes - 1,
      "sequence final counted before X",
    );
  same(
    bounds.input_bounds.map((b) => b.id),
    [
      "text-at-cap",
      "text-over-cap",
      "geometry-at-cap",
      "geometry-over-cap",
      "geometry-arithmetic",
    ],
    "input/geometry bounds coverage",
  );
  for (const [index, count, encoded, error] of [
    [0, 8192, 8192, null],
    [1, 8193, 0, "ResourceExhausted"],
  ]) {
    const b = bounds.input_bounds[index];
    same(
      [b.kind, b.repeat_hex, b.repeat_count, b.expected_encoded_bytes, b.error],
      ["CommitText", "61", count, encoded, error],
      "input byte cap oracle",
    );
  }
  same(
    [
      bounds.input_bounds[2].rows,
      bounds.input_bounds[2].columns,
      bounds.input_bounds[2].error,
    ],
    [200, 500, null],
    "geometry exact bound",
  );
  for (const b of bounds.input_bounds.slice(3)) {
    same(b.error, "InvalidRequest");
    same(b.expected_mutation, false);
  }
  same(
    bounds.recipes[2].expected_retained_payload_bytes,
    0,
    "long string retention",
  );
  same(
    bounds.future_execution,
    {
      status: "NOT RUN",
      seed: 1300711,
      long_partition_samples: 64,
      max_materialized_bytes: 131072,
      max_events_per_recipe: 257,
      time_budget_seconds_per_recipe: 10,
      note: "A future implementation runner must enforce these bounds and measure retained state/work; static recipe arithmetic is not runtime resource evidence.",
    },
    "future bounded-run requirements",
  );
  return {
    state_cases: cases.length,
    input_cases: goldens.input_cases.length,
    transport_partitions_checked: partitionCount,
  };
}

export async function checkNativeFixtures(root = process.cwd()) {
  const base = await realpath(root);
  async function bytes(path) {
    const actual = await realpath(resolve(base, safePath(path)));
    assert(actual.startsWith(base + sep), "fixture symlink escapes root");
    const data = await readFile(actual);
    assert(data.length <= 2 * 1024 * 1024, "fixture file byte cap");
    return data;
  }
  const json = async (path) => parseUniqueJson(await bytes(path), path);
  const directories = (
    await readdir(resolve(base, "tests/fixtures"), { withFileTypes: true })
  ).filter((e) => e.isDirectory() && e.name.startsWith("native-"));
  for (const name of [
    "native-contract",
    "native-receipts",
    "native-state",
    "native-profile",
    "native-provenance",
    "native-conformance",
  ])
    assert(
      directories.some((d) => d.name === name),
      `missing native fixture directory: ${name}`,
    );
  let verified = 0;
  for (const directory of directories) {
    const path = `tests/fixtures/${directory.name}`;
    const names = (await readdir(resolve(base, path)))
      .filter((n) => n.endsWith(".json"))
      .sort();
    for (const name of names) await json(`${path}/${safePath(name)}`);
    if (directory.name === "native-profile") {
      same(names, ["finite-profile-cases.json"]);
      continue;
    }
    assert(names.includes("manifest.json"), "native manifest missing");
    const m = await json(`${path}/manifest.json`);
    unique(
      m.files.map((f) => f.path),
      "manifest paths",
    );
    same(
      m.files.map((f) => f.path).sort(),
      names.filter((n) => n !== "manifest.json"),
      "unlisted native JSON payload",
    );
    for (const f of m.files) {
      safePath(f.path);
      assert(/^[0-9a-f]{64}$/.test(f.sha256), "manifest digest shape");
      same(
        digest(await bytes(`${path}/${f.path}`)),
        f.sha256,
        `fixture drift: ${path}/${f.path}`,
      );
      verified++;
    }
  }
  const manifest = await json(`${corpusDirectory}/manifest.json`);
  unique(
    manifest.source_records.map((s) => s.id),
    "source IDs",
  );
  unique(
    manifest.source_records.map((s) => s.path),
    "source paths",
  );
  for (const source of manifest.source_records) {
    assert(
      source.role && source.licensing,
      "source provenance/license missing",
    );
    same(
      digest(await bytes(source.path)),
      source.sha256,
      `source drift: ${source.id}`,
    );
  }
  for (const id of [
    "original-x4-inputs",
    "original-shared-manifest",
    "original-x4-oracle",
    "original-x4-result",
    "ea1-continuation-results",
    "ea1-alias-results",
    "ea1-selection",
    "n0-profile-review",
    "n0-state-continuations",
    "n0-state-transitions",
    "n0-state-bounds",
  ])
    assert(
      manifest.source_records.some((s) => s.id === id),
      `missing source pin: ${id}`,
    );
  const result = validateCorpus(
    manifest,
    await json(`${corpusDirectory}/goldens.json`),
    await json(`${corpusDirectory}/bounds.json`),
    await json("tests/fixtures/terminal/continuations.json"),
  );
  return {
    ...result,
    local_payload_digests_verified: verified,
    static_oracle_data: "PASS",
    engine_state_continuation_key_reply_conformance: "NOT RUN",
  };
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  console.log(JSON.stringify(await checkNativeFixtures()));
