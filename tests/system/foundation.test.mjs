import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { CanonicalProbe } from "../../packages/terminal-adapter/dist/node.js";
import { encodeText } from "../../packages/terminal-adapter/dist/index.js";
import { normalize } from "../../apps/desktop/dist/input.js";
import { continuation, cases, create, feed, view } from "./engine-probe.mjs";
import { evaluate, event } from "./key-helper-probe.mjs";
import validate from "../../packages/protocol/src/validate.generated.mjs";
import {
  isDecimalU64,
  validateParsedRequest,
} from "../../packages/protocol/dist/index.js";
const bytes = (s) => new TextEncoder().encode(s);

test("shared wire shape and bounded decimal semantics", async () => {
  const fixture = JSON.parse(
    await readFile("tests/fixtures/input/request.json", "utf8"),
  );
  assert.equal(validate(fixture), true);
  assert.equal(validateParsedRequest(fixture), true);
  assert.equal(validateParsedRequest({ ...fixture, version: 2 }), false);
  assert.equal(
    validateParsedRequest({
      ...fixture,
      context: { ...fixture.context, prefix: "00" },
    }),
    false,
  );
  assert.equal(validate({ ...fixture, grant: true }), false);
  for (const value of ["00", "-1", "18446744073709551616"])
    assert.equal(isDecimalU64(value), false);
  assert.equal(isDecimalU64("18446744073709551615"), true);
  // TS validator consumes parsed values only: duplicate-key wire decoding is not implemented here.
});
test("synthetic Crossterm and desktop key intents agree", () => {
  const rust = JSON.parse(
    execFileSync(
      "cargo",
      [
        "run",
        "--quiet",
        "--locked",
        "-p",
        "project-x-console",
        "--",
        "input-fixture",
      ],
      {
        encoding: "utf8",
        env: { ...process.env, CARGO_HOME: process.cwd() + "/.cache/cargo" },
      },
    ),
  );
  const dom = [
    ["ArrowUp", false, false],
    ["Tab", true, false],
    ["F12", false, true],
  ].map(([key, shiftKey, ctrlKey]) =>
    normalize({
      key,
      shiftKey,
      ctrlKey,
      altKey: false,
      metaKey: false,
      isComposing: false,
      repeat: false,
      location: 0,
    }),
  );
  assert.deepEqual(rust, dom);
  assert.equal(
    normalize({
      key: "a",
      shiftKey: false,
      ctrlKey: false,
      altKey: false,
      metaKey: false,
      isComposing: true,
      repeat: false,
      location: 0,
    }),
    undefined,
  );
});
test("pinned upstream helper handles cursor modes, modifiers, Tab and function keys", () => {
  assert.equal(
    evaluate(event("ArrowUp", 38), false, false, false).key,
    "\x1b[A",
  );
  assert.equal(
    evaluate(event("ArrowUp", 38), true, false, false).key,
    "\x1bOA",
  );
  assert.equal(
    evaluate(event("ArrowUp", 38, { ctrlKey: true }), true, false, false).key,
    "\x1b[1;5A",
  );
  assert.equal(
    evaluate(event("Tab", 9, { shiftKey: true }), false, false, false).key,
    "\x1b[Z",
  );
  assert.equal(
    evaluate(event("F12", 123), false, false, false).key,
    "\x1b[24~",
  );
});
test("helper counterexample: keypad Enter has no application-keypad input", () => {
  // No applicationKeypadMode argument exists. Location/code cannot alter this branch.
  assert.equal(
    evaluate(
      event("Enter", 13, { location: 3, code: "NumpadEnter" }),
      false,
      false,
      false,
    ).key,
    "\r",
  );
  // Required VT application-keypad Enter would be ESC O M; this helper cannot provide it.
});
test("canonical feed-only observations, no UI needed, detached immutable snapshots", async () => {
  const c = new CanonicalProbe(12, 5);
  try {
    await c.feed(bytes("a"));
    const first = c.observe();
    await c.feed(bytes("\x1b[?1h\x1b[?66h\x1b[?2004hb"));
    const second = c.observe();
    assert.equal(first.lines[0][1].text, "");
    assert.equal(second.lines[0][1].text, "b");
    assert.equal(second.modes.applicationCursorKeysMode, true);
    assert.equal(second.modes.applicationKeypadMode, true);
    assert.equal(second.modes.bracketedPasteMode, true);
    assert.throws(() => {
      first.lines[0][0].text = "forged";
    });
    c.resize(10, 4);
    assert.equal(c.observe().prefix, 3);
  } finally {
    c.dispose();
  }
});
test("two identical query events produce two canonical replies; replicas have no dispatch lane", async () => {
  const canonical = new CanonicalProbe(12, 5),
    replicaA = new CanonicalProbe(12, 5),
    replicaB = new CanonicalProbe(12, 5);
  try {
    const query = bytes("\x1b[6n\x1b[6n");
    await Promise.all(
      [canonical, replicaA, replicaB].map((c) => c.feed(query)),
    );
    const replies = canonical.drainReplies();
    assert.equal(replies.length, 2);
    assert.deepEqual(
      replies.map((r) => r.bytes),
      ["\x1b[1;1R", "\x1b[1;1R"],
    );
    assert.equal(replicaA.drainReplies().length, 2);
    assert.equal(replicaB.drainReplies().length, 2);
    // Only canonical replies are selected by harness. This is not IPC/origin or browser certification.
  } finally {
    canonical.dispose();
    replicaA.dispose();
    replicaB.dispose();
  }
});
test("all byte split positions preserve UTF-8/query/control interpretation", async () => {
  const input = bytes("A界e\u0301\x1b[31mZ\x1b[0m\x1b[6n");
  const baseline = create();
  await feed(baseline, input);
  const expected = view(baseline);
  baseline.dispose();
  for (let i = 0; i <= input.length; i++) {
    const t = create();
    try {
      await feed(t, input.subarray(0, i));
      await feed(t, input.subarray(i));
      assert.deepEqual(view(t), expected);
    } finally {
      t.dispose();
    }
  }
});
test("paste mode encoding is separate from key events and embedded boundaries reject", () => {
  const modes = {
    applicationCursorKeysMode: false,
    applicationKeypadMode: false,
    bracketedPasteMode: true,
  };
  assert.equal(
    new TextDecoder().decode(
      encodeText({ kind: "paste", text: "a\nb" }, modes),
    ),
    "\x1b[200~a\nb\x1b[201~",
  );
  assert.throws(() => encodeText({ kind: "paste", text: "\x1b[201~" }, modes));
  assert.throws(
    () =>
      encodeText(
        { kind: "key", key: "Up", ctrl: false, alt: false, shift: false },
        modes,
      ),
    /UnsupportedCapability/u,
  );
});
test("bounded per-feed and reply traffic surface unknown rather than complete", async () => {
  const c = new CanonicalProbe();
  try {
    await assert.rejects(c.feed(new Uint8Array(16385)));
    await c.feed(bytes("\x1b[6n".repeat(65)));
    assert.throws(() => c.observe(), /UnknownState/u);
  } finally {
    c.dispose();
  }
});
test("X4 regression: equal screens do not establish safe continuation", async () => {
  for (const name of ["saved-cursor", "scroll-region", "custom-tabs"]) {
    const r = await continuation(cases.find((c) => c.name === name));
    assert.equal(r.atAttach, true);
    assert.equal(r.afterContinuation, false);
  }
});

test("Unicode text rejects unpaired surrogates instead of lossy replacement", async () => {
  const fixture = JSON.parse(
    await readFile("tests/fixtures/input/request.json", "utf8"),
  );
  const modes = {
    applicationCursorKeysMode: false,
    applicationKeypadMode: false,
    bracketedPasteMode: false,
  };
  for (const text of ["\ud800", "\udfff"]) {
    assert.equal(
      validateParsedRequest({ ...fixture, intent: { kind: "text", text } }),
      false,
    );
    assert.throws(
      () => encodeText({ kind: "text", text }, modes),
      /unpaired surrogate/u,
    );
  }
  assert.equal(
    new TextDecoder().decode(encodeText({ kind: "text", text: "界😀" }, modes)),
    "界😀",
  );
});

test("accepted parser bytes are detached from mutable caller storage", async () => {
  const c = new CanonicalProbe(12, 5);
  try {
    const payload = bytes("a");
    const done = c.feed(payload);
    payload.fill(98);
    await done;
    assert.equal(c.observe().lines[0][0].text, "a");
  } finally {
    c.dispose();
  }
});
