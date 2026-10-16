import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import Ajv from "ajv";

const root = "tests/fixtures/native-public/";
const read = (name) => JSON.parse(readFileSync(root + name, "utf8"));
const { examples, view_commands: viewCommands } = read("examples.json");
const schema = JSON.parse(
  readFileSync("schemas/public/native-experimental-n0.schema.json", "utf8"),
);
const viewSchema = JSON.parse(
  readFileSync(
    "schemas/public/native-experimental-n0-view.schema.json",
    "utf8",
  ),
);
const ajv = new Ajv({ strict: true });
const validate = ajv.compile(schema);
const validateView = ajv.compile(viewSchema);

// Apply review-data edits only. This is not a public decoder or service validator.
function variant(item) {
  const value = structuredClone(examples[item.base]);
  assert.ok(value, "Unknown fixture base: " + item.base);
  for (const change of item.changes) {
    const path = [...change.path];
    const last = path.pop();
    let parent = value;
    for (const key of path) {
      assert.ok(Object.hasOwn(parent, key), "Unknown fixture edit path");
      parent = parent[key];
    }
    if (change.remove) {
      assert.ok(Object.hasOwn(parent, last), "Unknown fixture removal");
      delete parent[last];
    } else parent[last] = change.value;
  }
  return value;
}

test("N0 public examples fit the separate strict static schemas", () => {
  for (const [name, value] of Object.entries(examples))
    assert.ok(validate(value), name + ": " + ajv.errorsText(validate.errors));
  for (const command of viewCommands) {
    assert.ok(validateView(command), ajv.errorsText(validateView.errors));
    const input = structuredClone(examples["input-key"]);
    input.body.request.intent = command;
    assert.equal(validate(input), false, "UI command entered app input plane");
  }
});

test("subscription control shapes route concurrent and zero-credit examples", () => {
  const first = examples.subscribe.body;
  const zero = examples["subscribe-zero-credit"].body;
  assert.notEqual(first.correlation_id, zero.correlation_id);
  assert.deepEqual(first.session, zero.session);
  assert.deepEqual(first.start, zero.start);
  assert.deepEqual(zero.credit, { pages: 0, bytes: 0 });
  const opened = examples.opened.body;
  const openedZero = examples["opened-zero-credit"].body;
  for (const [request, response] of [
    [zero, openedZero],
    [first, opened],
  ]) {
    assert.equal(response.correlation_id, request.correlation_id);
    assert.deepEqual(response.session, request.session);
    assert.deepEqual(response.projection, request.start.projection);
  }
  assert.notEqual(opened.subscription_id, openedZero.subscription_id);
  for (const name of ["grant-credit-zero", "cancel-zero", "closed-zero"])
    assert.equal(
      examples[name].body.subscription_id,
      openedZero.subscription_id,
    );
  const refused = examples["subscribe-refused"];
  assert.equal(
    refused.body.correlation_id,
    examples["subscribe-denied"].body.correlation_id,
  );
  assert.equal(Object.hasOwn(refused.body, "subscription_id"), false);
  const unavailable = examples.unavailable.body;
  const openedHistory = examples["opened-history"].body;
  assert.equal(unavailable.subscription_id, openedHistory.subscription_id);
  assert.deepEqual(unavailable.session, openedHistory.session);
  assert.deepEqual(unavailable.projection, openedHistory.projection);
});

test("opening/refusal/active-unavailable schemas require explicit routing", () => {
  for (const [name, fields] of [
    ["opened", ["correlation_id", "subscription_id", "session", "projection"]],
    [
      "opened-zero-credit",
      ["correlation_id", "subscription_id", "session", "projection"],
    ],
    ["subscribe-refused", ["correlation_id"]],
    ["unavailable", ["subscription_id"]],
  ]) {
    const original = examples[name];
    assert.ok(validate(original), name + " positive shape failed");
    for (const field of fields) {
      const missing = structuredClone(original);
      delete missing.body[field];
      assert.equal(
        validate(missing),
        false,
        name + " accepted missing " + field,
      );
    }
  }
  const refused = structuredClone(examples["subscribe-refused"]);
  refused.body.subscription_id = "unallocated-subscription";
  assert.equal(
    validate(refused),
    false,
    "refusal allocated an active stream ID",
  );
});

test("both-client matrix executes shape expectations only", () => {
  const ids = new Set();
  for (const item of read("intent-cases.json").cases) {
    assert.ok(!ids.has(item.id), "Duplicate case: " + item.id);
    ids.add(item.id);
    assert.deepEqual(item.clients, ["host_tui", "electron"]);
    assert.equal(
      validate(variant(item)),
      item.expected_shape,
      item.id + ": " + ajv.errorsText(validate.errors),
    );
  }
});

test("empty tagged variants preserve positive shapes and reject extra fields", () => {
  const cases = new Map(
    read("intent-cases.json").cases.map((item) => [item.id, item]),
  );
  for (const id of [
    "active-grid-extra-field",
    "default-color-extra-field",
    "direct-session-extra-field",
  ]) {
    const item = cases.get(id);
    const valid = structuredClone(examples[item.base]);
    if (id === "direct-session-extra-field")
      valid.body.request.context.route = { kind: "direct_session" };
    assert.ok(validate(valid), id + " positive shape failed");
    assert.equal(validate(variant(item)), false, id + " accepted extra fields");
  }
});

test("unknown facts are absent; known false remains a typed fact", () => {
  const original = examples["input-key"].body.request.intent.facts;
  for (const name of [
    "physical_code",
    "layout_id",
    "consumed_modifiers",
    "composing",
    "phase",
  ]) {
    assert.equal(Object.hasOwn(original, name), false);
    const value = structuredClone(examples["input-key"]);
    value.body.request.intent.facts[name] = null;
    assert.equal(validate(value), false, name + " accepted explicit null");
  }
  const known = structuredClone(examples["input-key"]);
  known.body.request.intent.facts.composing = false;
  assert.ok(validate(known));
});

test("schema acceptance does not claim u64, UTF-8 bytes, freshness or assembly", () => {
  const cases = new Map(
    read("intent-cases.json").cases.map((item) => [item.id, item]),
  );
  for (const id of [
    "u64-overflow",
    "text-byte-overflow",
    "wrong-focus",
    "stale-observation",
    "delta-wrong-base",
    "processed-ahead-of-source",
    "incomplete-grid-row",
  ]) {
    const item = cases.get(id);
    assert.ok(validate(variant(item)), id + " is intentionally schema-valid");
    assert.match(item.semantic_expectation, /^(refuse_|resync_required)/u);
  }
  const tooLarge = variant(cases.get("text-byte-overflow"));
  assert.equal(Buffer.byteLength(tooLarge.body.request.intent.text), 8194);
  assert.ok(
    BigInt(variant(cases.get("u64-overflow")).body.request.key.request_seq) >
      18446744073709551615n,
  );
});

test("native public local manifest binds every payload and review references", () => {
  const manifest = read("manifest.json");
  assert.deepEqual(manifest.files.map((item) => item.path).sort(), [
    "examples.json",
    "intent-cases.json",
    "subscription-cases.json",
  ]);
  for (const item of manifest.files) {
    assert.equal(
      createHash("sha256")
        .update(readFileSync(root + item.path))
        .digest("hex"),
      item.sha256,
      item.path,
    );
  }
  for (const item of read("subscription-cases.json").cases)
    for (const name of item.examples) assert.ok(Object.hasOwn(examples, name));
});
