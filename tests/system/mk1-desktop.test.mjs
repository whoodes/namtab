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

test("MK1 desktop leaves native paste/meta shortcuts and rejects composition", () => {
  for (const event of [
    key("v", { ctrlKey: true }),
    key("Insert", { shiftKey: true }),
    key("q", { metaKey: true }),
    key("Control"),
  ])
    assert.equal(mk1.normalizeKey(event).native, true);
  assert.deepEqual(
    mk1.normalizeKey(key("C", { ctrlKey: true, shiftKey: true })),
    { intent: { kind: "key", key: "C", ctrl: true, alt: false, shift: true } },
  );
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

test("MK1 focus acknowledgement survives either IPC acknowledgement/state ordering", () => {
  let acknowledgement = 5;
  acknowledgement = mk1.reconcileFocusAcknowledgement(acknowledgement, 4, true);
  assert.equal(
    acknowledgement,
    5,
    "a late prior state does not erase a future acknowledged epoch",
  );
  assert.equal(mk1.focusReady(acknowledgement, 4, true, true, true), false);
  acknowledgement = mk1.reconcileFocusAcknowledgement(acknowledgement, 5, true);
  assert.equal(mk1.focusReady(acknowledgement, 5, true, true, true), true);
  assert.equal(
    mk1.reconcileFocusAcknowledgement(null, 5, true),
    null,
    "state alone never grants input focus",
  );
  assert.equal(
    mk1.reconcileFocusAcknowledgement(5, 6, true),
    null,
    "a newer fence invalidates the prior acknowledgement",
  );
  assert.equal(
    mk1.reconcileFocusAcknowledgement(5, 5, false),
    null,
    "disconnect always clears acknowledgement",
  );
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

// An adversarial local protocol peer tests transport ordering, not application
// semantics. Real namtabd queue/refusal evidence is recorded separately.
async function refusalPeer(t, state) {
  const net = await import("node:net");
  const fs = await import("node:fs/promises");
  const os = await import("node:os");
  const path = await import("node:path");
  const { once } = await import("node:events");
  const { Connection } = await import("../../apps/desktop/mk1/connection.cjs");
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), "mk1-input-"));
  const socketPath = path.join(directory, "socket");
  const operations = [];
  const sockets = new Set();
  const server = net.createServer((socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
    let buffer = "";
    socket.on("data", (data) => {
      buffer += data.toString("utf8");
      for (;;) {
        const newline = buffer.indexOf("\n");
        if (newline < 0) break;
        const request = JSON.parse(buffer.slice(0, newline));
        buffer = buffer.slice(newline + 1);
        const operation = request.operation;
        operations.push(operation.op);
        const input = operation.op === "input";
        const response = {
          id: request.id,
          ok: !input,
          error: input ? "Injected input refusal" : null,
          epoch: "fault-peer-epoch",
          endpoint: "human-1",
          resume: operation.op === "hello" ? "test-only-token" : null,
          snapshot: null,
          receipt: input
            ? {
                endpoint: "human-1",
                id: request.id,
                operation: "input",
                state,
                detail: "Injected disposition",
                delivered_bytes: state === "partial" ? 1 : 0,
              }
            : null,
        };
        socket.write(JSON.stringify(response) + "\n");
      }
    });
  });
  server.listen(socketPath);
  await once(server, "listening");
  const connection = new Connection(socketPath);
  t.after(async () => {
    connection.disconnect();
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve) => server.close(resolve));
    await fs.rm(directory, { recursive: true, force: true });
  });
  const ready = once(connection, "ready");
  connection.connect();
  await ready;
  return { connection, operations, once };
}

for (const disposition of ["rejected", "partial", "unknown"]) {
  test(`MK1 transport synchronously pauses after ${disposition} input and drops the queued command suffix`, async (t) => {
    const { connection, operations, once } = await refusalPeer(t, disposition);
    assert.equal(connection.rearmInput(connection.inputPauseRevision), true);
    const priorPause = connection.inputPauseRevision;
    const first = connection.request({
      op: "input",
      intent: { kind: "text", text: "e" },
    });
    const suffix = ["x", "i", "t", "\r"].map((text) =>
      connection.request({ op: "input", intent: { kind: "text", text } }),
    );
    const queuedMutation = connection.request({
      op: "run",
      task: "queued",
      argv: ["/bin/true"],
    });
    const results = await Promise.allSettled([
      first,
      ...suffix,
      queuedMutation,
    ]);
    assert.equal(results[0].value.receipt.state, disposition);
    assert.equal(
      results.slice(1).every((result) => result.status === "rejected"),
      true,
    );
    assert.equal(
      operations.filter((operation) => operation === "input").length,
      1,
    );
    assert.equal(
      operations.includes("run"),
      false,
      "a queued mutation is not sent after refusal",
    );
    assert.equal(connection.inputPaused, true);
    assert.equal(connection.queue.length, 0);
    assert.equal(connection.rearmInput(priorPause), false);
    await connection.request({ op: "observe" });
    connection.clearQueuedInputs();
    await assert.rejects(connection.request({ op: "input" }), /Inspect/i);
    assert.equal(
      connection.inputPaused,
      true,
      "observe and focus reconciliation cannot rearm",
    );
    connection.disconnect("Test explicit reconnect");
    const ready = once(connection, "ready");
    connection.connect();
    await ready;
    await assert.rejects(connection.request({ op: "input" }), /Inspect/i);
    assert.equal(
      operations.filter((operation) => operation === "input").length,
      1,
    );
    assert.equal(
      connection.rearmInput(connection.inputPauseRevision),
      true,
      "only explicit revision-matched rearm clears pause",
    );
  });
}

test("MK1 local command fallback preserves explicit Bash shell intent without interpreting a goal", () => {
  const command = "printf '%s\\n' hello > result.txt\nprintf second";
  assert.deepEqual(mk1.shellCommandArgv(command), [
    "/bin/bash",
    "--noprofile",
    "--norc",
    "-c",
    command,
  ]);
  for (const value of [
    null,
    "",
    " \n ",
    "echo\0bad",
    "x".repeat(4097),
    "😀".repeat(3000),
  ])
    assert.equal(mk1.shellCommandArgv(value), null);
  assert.equal(
    mk1.validateCommandShape({
      op: "run",
      task: "task",
      argv: ["/bin/true"],
      task_revision: null,
    }),
    true,
  );
  assert.equal(
    mk1.validateCommandShape({ op: "run", task: "task", argv: ["/bin/true"] }),
    false,
  );
});

test("MK1 immutable task definitions validate bounded instructions and declared normal paths", () => {
  assert.equal(
    mk1.validTaskDefinition("human task: 1", "Produce a reviewed result", [
      ".artifacts/result.txt",
    ]),
    true,
  );
  assert.equal(mk1.validTaskDefinition("task", "x".repeat(512), []), true);
  for (const args of [
    ["task", " ", []],
    ["task", "line\nbreak", []],
    ["task", "é".repeat(257), []],
    ["界", "goal", []],
    ["x".repeat(81), "goal", []],
    ["task", "goal", ["a", "b", "c"]],
    ...["/absolute", "../outside", "a/./b", "a//b", "a\\b", "a/", ""].map(
      (value) => ["task", "goal", [value]],
    ),
  ])
    assert.equal(mk1.validTaskDefinition(...args), false);
});

test("MK1 shared task activity exposes exact launch origin and receipt references while legacy stays unknown", () => {
  const revision = "sha256:" + "a".repeat(64);
  const value = snapshot();
  value.capabilities = [
    "shared_tasks_v1",
    "attempt_origin_v1",
    "receipt_results_v1",
  ];
  value.tasks = [
    {
      task: "task 1",
      revision,
      goal: "Create a local result",
      expected_artifacts: ["result.txt"],
    },
  ];
  value.attempts = [
    {
      id: "a1",
      task: "task 1",
      task_revision: revision,
      origin: { endpoint: "human-1", request_id: 8 },
      state: "exited",
      exit_code: 0,
      stdout: "done",
      stderr: "",
      truncated: false,
    },
  ];
  assert.equal(mk1.validateSnapshot(value), true);
  const response = {
    id: 9,
    ok: true,
    error: null,
    epoch: value.epoch,
    endpoint: null,
    resume: null,
    snapshot: value,
    receipt: {
      endpoint: "human-1",
      id: 9,
      operation: "run",
      state: "applied",
      delivered_bytes: 0,
      detail: "started",
      result: { kind: "attempt_started", attempt: "a1" },
    },
  };
  assert.equal(mk1.validateResponse(response, 9), true);
  response.receipt.result = { kind: "task_defined", task: "task 1", revision };
  assert.equal(mk1.validateResponse(response, 9), true);
  response.receipt.result = { kind: "attempt_started", task: "task 1" };
  assert.equal(mk1.validateResponse(response, 9), false);
  value.attempts[0].origin.request_id = 0;
  assert.equal(mk1.validateSnapshot(value), false);
  value.attempts[0].origin = null;
  value.attempts[0].task_revision = null;
  assert.equal(
    mk1.validateSnapshot(value),
    true,
    "unknown launch origin is not inferred from task label",
  );
  value.tasks[0].revision = "not-a-revision";
  assert.equal(mk1.validateSnapshot(value), false);
  assert.equal(
    mk1.validateSnapshot(snapshot()),
    true,
    "legacy daemon observations remain usable",
  );
});

test("MK1 All attempts keeps unregistered activity accessible after defining a same-label contract", () => {
  const revision = "sha256:" + "a".repeat(64);
  const contract = { task: "shared-label", revision };
  const attempts = [
    { id: "unregistered", task: "shared-label", task_revision: null },
    { id: "bound", task: "shared-label", task_revision: revision },
    {
      id: "other-revision",
      task: "shared-label",
      task_revision: "sha256:" + "b".repeat(64),
    },
    { id: "other-label", task: "other", task_revision: null },
  ];
  const before = structuredClone(attempts);
  assert.deepEqual(
    mk1.attemptsForTask(attempts, undefined).map((value) => value.id),
    attempts.map((value) => value.id),
  );
  assert.deepEqual(
    mk1.attemptsForTask(attempts, contract).map((value) => value.id),
    ["bound"],
  );
  assert.deepEqual(
    mk1.attemptsForTask(attempts, undefined).map((value) => value.id),
    ["unregistered", "bound", "other-revision", "other-label"],
  );
  assert.deepEqual(attempts, before);
});
