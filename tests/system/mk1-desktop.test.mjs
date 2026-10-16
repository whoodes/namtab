import test from "node:test";
import assert from "node:assert/strict";
import mk1 from "../../apps/desktop/mk1/shared.js";

const screen = () => ({
  cols: 8,
  rows: 2,
  lines: ["<b>£</b>", "ready"],
  cursor_col: 2,
  cursor_row: 1,
  cursor_visible: true,
  revision: 9,
  mode_revision: 3,
  alternate: false,
  input_known: true,
  diagnostics: [],
});
const snapshot = () => ({
  version: mk1.VERSION,
  epoch: "daemon-1",
  endpoint: "human-1",
  endpoints: [{ id: "human-1", role: "human", connected: true }],
  sessions: [
    {
      id: "shell-1",
      name: "shell",
      owner: "human-1",
      generation: 2,
      process: "running",
      exit_code: null,
      screen: screen(),
    },
  ],
  attempts: [],
  artifacts: [],
  receipts: [],
  degraded: null,
});
const key = (value, extra = {}) => ({
  key: value,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  isComposing: false,
  ...extra,
});

test("MK1 desktop emits the strict nested wire envelope and limits IPC fields", () => {
  assert.deepEqual(mk1.requestEnvelope(1, { op: "observe" }), {
    version: "namtab.mk1/1",
    id: 1,
    operation: { op: "observe" },
  });
  for (const id of [0, -1, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1])
    assert.throws(() => mk1.requestEnvelope(id, { op: "observe" }));
  assert.equal(mk1.validateCommandShape({ op: "state" }), true);
  assert.equal(
    mk1.validateCommandShape({ op: "state", raw: "not allowed" }),
    false,
  );
  assert.equal(
    mk1.validateCommandShape({ op: "raw_bytes", bytes: "x" }),
    false,
  );
  assert.equal(
    mk1.validateCommandShape({
      op: "take_control",
      session: "shell-1",
      force: false,
    }),
    true,
  );
  assert.equal(
    mk1.validateCommandShape({ op: "take_control", session: "shell-1" }),
    false,
  );
  assert.equal(
    mk1.validateIntent({ kind: "text", text: "ok", raw: "extra" }),
    false,
  );
});

test("MK1 desktop refuses Linux socket paths before an opaque transport failure", () => {
  assert.equal(mk1.validSocketPath("/" + "a".repeat(106)), true);
  assert.equal(mk1.validSocketPath("/" + "a".repeat(107)), false);
  assert.equal(mk1.validSocketPath("/" + "£".repeat(53)), true);
  assert.equal(mk1.validSocketPath("/" + "£".repeat(54)), false);
  for (const value of [null, undefined, "relative.sock", "/a\0b"])
    assert.equal(mk1.validSocketPath(value), false);
});

test("MK1 desktop argv validation matches the bounded managed-process admission", () => {
  assert.equal(mk1.validArgv(["/usr/bin/printf", "hello\\n"]), true);
  assert.equal(mk1.validArgv(Array(32).fill("x")), true);
  for (const value of [
    [],
    Array(33).fill("x"),
    [""],
    ["x", 1],
    ["x", "a\0b"],
    ["x", "a".repeat(4097)],
  ])
    assert.equal(mk1.validArgv(value), false);
});

test("MK1 desktop keeps layout-consumed text distinct from modified keys", () => {
  assert.deepEqual(mk1.normalizeKey(key("£", { shiftKey: true })), {
    intent: { kind: "text", text: "£" },
  });
  assert.deepEqual(mk1.normalizeKey(key("c", { ctrlKey: true })), {
    intent: { kind: "key", key: "c", ctrl: true, alt: false, shift: false },
  });
  assert.deepEqual(
    mk1.normalizeKey(key("ArrowUp", { altKey: true, shiftKey: true })),
    { intent: { kind: "key", key: "Up", ctrl: false, alt: true, shift: true } },
  );
  assert.deepEqual(mk1.normalizeKey(key("Tab", { shiftKey: true })), {
    intent: { kind: "key", key: "Tab", ctrl: false, alt: false, shift: true },
  });
  assert.deepEqual(
    mk1.normalizeKey(key("@", { ctrlKey: true, altKey: true, altGraph: true })),
    { intent: { kind: "text", text: "@" } },
  );
  for (const value of [
    "Enter",
    "Escape",
    "Backspace",
    "Home",
    "End",
    "Insert",
    "Delete",
    "PageUp",
    "PageDown",
    "F1",
    "F12",
  ])
    assert.equal(mk1.normalizeKey(key(value)).intent.key, value);
});

test("MK1 desktop leaves native copy/paste/meta shortcuts and rejects composition", () => {
  for (const event of [
    key("v", { ctrlKey: true }),
    key("c", { ctrlKey: true, shiftKey: true }),
    key("Insert", { shiftKey: true }),
    key("q", { metaKey: true }),
    key("Control"),
  ])
    assert.equal(mk1.normalizeKey(event).native, true);
  for (const event of [
    key("a", { isComposing: true }),
    key("Dead"),
    key("Process"),
    key("Unidentified"),
    key("界"),
    key("😀"),
    key("\u00ad"),
    key("a\u0301"),
  ])
    assert.ok(mk1.normalizeKey(event).error);
});

test("MK1 desktop preserves supported physical keypad identity for canonical encoding", () => {
  const codes = [
    ...Array.from({ length: 10 }, (_, i) => String(i)),
    "Enter",
    "Add",
    "Subtract",
    "Multiply",
    "Divide",
    "Decimal",
    "Comma",
    "Equal",
    "Tab",
    "Space",
  ];
  for (const suffix of codes) {
    const intent = mk1.normalizeKey(
      key("Enter", {
        code: `Numpad${suffix}`,
        location: 3,
        ctrlKey: true,
        shiftKey: true,
      }),
    ).intent;
    assert.deepEqual(intent, {
      kind: "key",
      key: `Keypad${suffix}`,
      ctrl: true,
      alt: false,
      shift: true,
    });
    assert.equal(mk1.validateIntent(intent), true);
    const envelope = JSON.parse(
      JSON.stringify(mk1.requestEnvelope(5, { op: "input", intent })),
    );
    assert.equal(envelope.operation.intent.key, `Keypad${suffix}`);
  }
  assert.ok(
    mk1.normalizeKey(key("Enter", { location: 3, code: "UnrecognizedNumpad" }))
      .error,
  );
  assert.equal(
    mk1.normalizeKey(key("Enter", { location: 0, code: "Enter" })).intent.key,
    "Enter",
  );
});

test("MK1 UI prefix returns keyboard focus without leaking unknown or timed-out keys", () => {
  const ctrlG = key("g", { ctrlKey: true });
  const armed = mk1.routeUiKey(ctrlG, null, 10);
  assert.deepEqual(armed, { kind: "prefix", prefixDeadline: 1510 });
  assert.deepEqual(mk1.routeUiKey(key("Escape"), armed.prefixDeadline, 20), {
    kind: "leave",
    prefixDeadline: null,
  });
  const literal = mk1.routeUiKey(ctrlG, armed.prefixDeadline, 20);
  assert.deepEqual(literal.intent, {
    kind: "key",
    key: "g",
    ctrl: true,
    alt: false,
    shift: false,
  });
  for (const event of [
    key("x"),
    key("v", { ctrlKey: true }),
    key("ArrowUp"),
    key("a", { isComposing: true }),
  ]) {
    const consumed = mk1.routeUiKey(event, armed.prefixDeadline, 20);
    assert.equal(consumed.kind, "consume");
    assert.equal(consumed.prefixDeadline, null);
    assert.equal(consumed.intent, undefined);
  }
  const expired = mk1.routeUiKey(key("x"), armed.prefixDeadline, 1510);
  assert.equal(expired.kind, "consume");
  assert.equal(expired.intent, undefined);
  assert.deepEqual(mk1.routeUiKey(key("Control"), armed.prefixDeadline, 20), {
    kind: "consume",
    prefixDeadline: 1510,
  });
  assert.equal(
    mk1.routeUiKey(key("g", { ctrlKey: true, shiftKey: true }), null, 10).kind,
    "normal",
  );
  assert.equal(mk1.routeUiPaste("safe", armed.prefixDeadline).consumed, true);
  assert.equal(
    mk1.routeUiPaste("safe", armed.prefixDeadline).intent,
    undefined,
  );
  assert.deepEqual(mk1.routeUiPaste("safe", null), {
    intent: { kind: "paste", text: "safe" },
  });
});

test("MK1 UI input waits for the exact focus acknowledgement and active keyboard", () => {
  assert.equal(mk1.focusReady(4, 4, true, true, true), true);
  assert.equal(mk1.focusReady(null, 4, true, true, true), false);
  assert.equal(mk1.focusReady(3, 4, true, true, true), false);
  for (const values of [
    [false, true, true],
    [true, false, true],
    [true, true, false],
  ])
    assert.equal(mk1.focusReady(4, 4, ...values), false);
});

test("MK1 desktop refuses controls, unsupported repertoire and oversized paste before IPC", () => {
  for (const value of [
    "\0",
    "\x1b[2J",
    "\x7f",
    "\u0085",
    "界",
    "\u00ad",
    "a\u0301",
    "😀",
  ]) {
    assert.equal(mk1.textAllowed(value), false);
    assert.ok(mk1.normalizePaste(value).error);
  }
  assert.equal(mk1.textAllowed("a\n"), false);
  assert.deepEqual(mk1.normalizePaste("a\r\n\tb"), {
    intent: { kind: "paste", text: "a\r\n\tb" },
  });
  assert.equal(mk1.textAllowed("£".repeat(4096), true), true);
  assert.equal(mk1.textAllowed("£".repeat(4096) + "a", true), false);
  assert.equal(
    mk1.validateIntent({
      kind: "key",
      key: "Up",
      ctrl: true,
      alt: false,
      shift: false,
    }),
    true,
  );
  assert.equal(
    mk1.validateIntent({
      kind: "key",
      key: "Up",
      ctrl: "false",
      alt: false,
      shift: false,
    }),
    false,
  );
});

test("MK1 human-live fences allow ordinary echo but refuse stale targets and modes", () => {
  const observed = snapshot();
  const fence = mk1.captureFence(observed, "shell-1", 4);
  const valid = (
    current = observed,
    selected = "shell-1",
    epoch = 4,
    focused = true,
  ) => !!mk1.inputFenceValid(fence, current, selected, epoch, focused);
  assert.equal(valid(), true);
  observed.sessions[0].screen.revision++;
  assert.equal(
    valid(),
    true,
    "ordinary screen echo does not stale human-live typing",
  );
  for (const mutation of [
    (value) => value.sessions[0].generation++,
    (value) => value.sessions[0].screen.mode_revision++,
    (value) => {
      value.sessions[0].owner = "agent-2";
    },
    (value) => {
      value.sessions[0].screen.input_known = false;
    },
    (value) => {
      value.sessions[0].process = "exited";
    },
    (value) => {
      value.epoch = "daemon-2";
    },
  ]) {
    const changed = structuredClone(observed);
    mutation(changed);
    assert.equal(valid(changed), false);
  }
  assert.equal(valid(observed, "shell-2"), false);
  assert.equal(
    valid(observed, "shell-1", 5),
    false,
    "blur/refocus changes the local epoch",
  );
  assert.equal(valid(observed, "shell-1", 4, false), false);
  assert.equal(valid({ ...observed, degraded: "journal unavailable" }), false);
  assert.equal(
    mk1.inputFenceValid(
      { ...fence, revision: undefined },
      observed,
      "shell-1",
      4,
      true,
    ),
    false,
  );
});

test("MK1 screen projection is immutable text with a bounded separate cursor", () => {
  const original = screen();
  const projected = mk1.projectScreen(original);
  assert.equal(projected.lines[0], "<b>£</b>");
  assert.equal(projected.lines[1], "ready   ");
  assert.deepEqual(projected.cursor, { col: 2, row: 1 });
  original.lines[1] = "changed";
  assert.equal(projected.lines[1], "ready   ");
  assert.throws(() => projected.lines.push("changed"), TypeError);
  assert.equal(mk1.safeText("\x1b[2J<script>\u202e"), "⟦1b⟧[2J<script>⟦202e⟧");
  assert.throws(
    () => mk1.projectScreen({ ...screen(), cursor_col: 8 }),
    /Malformed/,
  );
  assert.throws(
    () => mk1.projectScreen({ ...screen(), lines: ["\x1b[2J"] }),
    /Malformed/,
  );
});

test("MK1 display accepts the core replacement marker but never accepts it as input", () => {
  const replacement = { ...screen(), lines: ["safe \ufffd", ""] };
  assert.equal(mk1.validateScreen(replacement), true);
  assert.equal(mk1.projectScreen(replacement).lines[0], "safe \ufffd  ");
  assert.equal(mk1.textAllowed("\ufffd"), false);
  assert.equal(mk1.validateIntent({ kind: "text", text: "\ufffd" }), false);
  assert.ok(mk1.normalizeKey(key("\ufffd")).error);
  assert.ok(mk1.normalizePaste("\ufffd").error);
  for (const value of ["\u202e", "\x1b", "界", "☃"])
    assert.equal(mk1.validateScreen({ ...screen(), lines: [value] }), false);
});

test("MK1 draining PTY sessions remain observable with known exit but reject input", () => {
  const observed = snapshot();
  observed.sessions[0].process = "draining";
  observed.sessions[0].exit_code = 7;
  assert.equal(mk1.validateSnapshot(observed), true);
  const fence = mk1.captureFence(observed, "shell-1", 4);
  assert.equal(mk1.inputFenceValid(fence, observed, "shell-1", 4, true), false);
  assert.equal(mk1.projectScreen(observed.sessions[0].screen).lines.length, 2);
});

test("MK1 response checks reject malformed canonical data before renderer delivery", () => {
  const valid = {
    id: 7,
    ok: true,
    error: null,
    epoch: "daemon-1",
    endpoint: null,
    resume: null,
    receipt: null,
    snapshot: snapshot(),
  };
  assert.equal(mk1.validateResponse(valid, 7), true);
  assert.equal(mk1.validateResponse(valid, 8), false);
  for (const changed of [
    { ...valid, epoch: null },
    { ...valid, snapshot: { ...snapshot(), version: "other" } },
    {
      ...valid,
      snapshot: {
        ...snapshot(),
        sessions: Array(3).fill(snapshot().sessions[0]),
      },
    },
    {
      ...valid,
      snapshot: {
        ...snapshot(),
        sessions: [
          {
            ...snapshot().sessions[0],
            generation: Number.MAX_SAFE_INTEGER + 1,
          },
        ],
      },
    },
    {
      ...valid,
      receipt: {
        endpoint: "human-1",
        id: 1,
        operation: "input",
        state: "completed",
        detail: "",
        delivered_bytes: 1,
      },
    },
  ])
    assert.equal(mk1.validateResponse(changed, 7), false);
  const draining = snapshot();
  draining.attempts.push({
    id: "attempt-1",
    task: "task-1",
    state: "draining",
    exit_code: 0,
    stdout: "real stream",
    stderr: "",
    truncated: false,
  });
  assert.equal(mk1.validateSnapshot(draining), true);
});

test("MK1 endpoint observations are declared roles and bounded connection facts", () => {
  const observed = snapshot();
  observed.endpoints.push({ id: "agent-1", role: "agent", connected: false });
  assert.equal(mk1.validateSnapshot(observed), true);
  for (const endpoints of [
    [{ id: "agent-1", role: "thinking", connected: true }],
    [{ id: "agent-1", role: "agent", connected: "true" }],
    [
      { id: "same", role: "human", connected: true },
      { id: "same", role: "agent", connected: false },
    ],
    Array.from({ length: 17 }, (_, i) => ({
      id: `e${i}`,
      role: "agent",
      connected: false,
    })),
    Array.from({ length: 5 }, (_, i) => ({
      id: `e${i}`,
      role: "agent",
      connected: true,
    })),
  ])
    assert.equal(mk1.validateSnapshot({ ...observed, endpoints }), false);
  assert.equal(
    mk1.validateSnapshot({
      ...observed,
      endpoints: Array.from({ length: 16 }, (_, i) => ({
        id: `e${i}`,
        role: "agent",
        connected: i < 4,
      })),
    }),
    true,
  );
});
