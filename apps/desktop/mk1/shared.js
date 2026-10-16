/* Pure MK1 presentation/input checks; no terminal parsing or byte encoding. */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.Mk1 = Object.freeze(api);
})(typeof globalThis === "object" ? globalThis : this, function () {
  "use strict";
  const VERSION = "namtab.mk1/1";
  const MAX_FRAME = 1024 * 1024;
  const MAX_QUEUE = 64;
  const keypadCodes = Object.freeze(
    Object.fromEntries([
      ...Array.from({ length: 10 }, (_, index) => [
        `Numpad${index}`,
        `Keypad${index}`,
      ]),
      ...[
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
      ].map((name) => [`Numpad${name}`, `Keypad${name}`]),
    ]),
  );
  const namedKeys = new Set([
    "Enter",
    "Tab",
    "Escape",
    "Backspace",
    "Up",
    "Down",
    "Left",
    "Right",
    "Home",
    "End",
    "Insert",
    "Delete",
    "PageUp",
    "PageDown",
    ...Array.from({ length: 12 }, (_, i) => `F${i + 1}`),
    ...Object.values(keypadCodes),
  ]);
  const domKeys = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };
  function record(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
  }
  function integer(value, minimum = 0, maximum = Number.MAX_SAFE_INTEGER) {
    return Number.isSafeInteger(value) && value >= minimum && value <= maximum;
  }
  function string(value, maximum = 4096) {
    return typeof value === "string" && value.length <= maximum;
  }
  function exactKeys(value, keys) {
    return (
      record(value) &&
      Object.keys(value).length === keys.length &&
      keys.every((key) => Object.hasOwn(value, key))
    );
  }
  function requestEnvelope(id, operation) {
    if (!integer(id, 1) || !record(operation) || !string(operation.op, 32))
      throw new Error("Invalid request envelope");
    return { version: VERSION, id, operation };
  }
  function validSocketPath(value) {
    return (
      string(value, 107) &&
      value.startsWith("/") &&
      !value.includes("\0") &&
      new TextEncoder().encode(value).length <= 107
    );
  }
  function validateCommandShape(command) {
    const fields = {
      state: [],
      connect: [],
      new_endpoint: [],
      select: ["session"],
      focus: ["focused"],
      input: ["fence", "intent"],
      create_session: ["name"],
      take_control: ["session", "force"],
      release_control: ["session"],
      resize: ["session", "cols", "rows"],
      run: ["task", "argv"],
      artifact: ["task", "path"],
      validate_artifact: ["artifact", "revision"],
      receipt: ["request_id"],
    };
    return (
      record(command) &&
      Object.hasOwn(fields, command.op) &&
      exactKeys(command, ["op", ...fields[command.op]])
    );
  }
  function scalarAllowed(character) {
    const code = character.codePointAt(0);
    return (
      (code >= 0x20 && code <= 0x7e) ||
      (code >= 0xa0 && code <= 0xff && code !== 0xad)
    );
  }
  function textAllowed(value, paste = false) {
    return (
      string(value, 8192) &&
      value.length > 0 &&
      new TextEncoder().encode(value).length <= 8192 &&
      Array.from(value).every(
        (character) =>
          scalarAllowed(character) || (paste && "\t\r\n".includes(character)),
      )
    );
  }
  function validateIntent(intent) {
    if (!record(intent)) return false;
    if (intent.kind === "text" || intent.kind === "paste")
      return (
        exactKeys(intent, ["kind", "text"]) &&
        textAllowed(intent.text, intent.kind === "paste")
      );
    return (
      intent.kind === "key" &&
      exactKeys(intent, ["kind", "key", "ctrl", "alt", "shift"]) &&
      string(intent.key, 32) &&
      (namedKeys.has(intent.key) ||
        (Array.from(intent.key).length === 1 && scalarAllowed(intent.key))) &&
      [intent.ctrl, intent.alt, intent.shift].every(
        (value) => typeof value === "boolean",
      )
    );
  }
  function normalizeKey(event) {
    if (
      event.metaKey ||
      (event.ctrlKey &&
        !event.altKey &&
        (event.key.toLowerCase() === "v" ||
          (event.shiftKey && event.key.toLowerCase() === "c"))) ||
      (event.shiftKey &&
        event.key === "Insert" &&
        !event.ctrlKey &&
        !event.altKey)
    )
      return { native: true };
    if (
      event.isComposing ||
      ["Dead", "Process", "Unidentified"].includes(event.key)
    )
      return {
        error:
          "IME and composed keyboard input are outside MK1. Use supported width-one text.",
      };
    if (
      [
        "Shift",
        "Control",
        "Alt",
        "AltGraph",
        "Meta",
        "CapsLock",
        "NumLock",
      ].includes(event.key)
    )
      return { native: true };
    if (event.location === 3 && !Object.hasOwn(keypadCodes, event.code))
      return { error: "Unsupported physical keypad code; nothing was sent." };
    const key =
      event.location === 3
        ? keypadCodes[event.code]
        : domKeys[event.key] || event.key;
    if (namedKeys.has(key))
      return {
        intent: {
          kind: "key",
          key,
          ctrl: !!event.ctrlKey,
          alt: !!event.altKey,
          shift: !!event.shiftKey,
        },
      };
    if (Array.from(key).length === 1 && scalarAllowed(key)) {
      if ((!event.ctrlKey && !event.altKey) || event.altGraph)
        return { intent: { kind: "text", text: key } };
      return {
        intent: {
          kind: "key",
          key,
          ctrl: !!event.ctrlKey,
          alt: !!event.altKey,
          shift: !!event.shiftKey,
        },
      };
    }
    return {
      error: "Unsupported MK1 key or text repertoire; nothing was sent.",
    };
  }
  function normalizePaste(text) {
    if (!textAllowed(text, true))
      return {
        error:
          "Paste refused: use at most 8 KiB of width-one ASCII/Latin-1 (no soft hyphen), with TAB/CR/LF only.",
      };
    return { intent: { kind: "paste", text } };
  }
  function validateScreen(screen) {
    return (
      record(screen) &&
      integer(screen.cols, 1, 160) &&
      integer(screen.rows, 1, 48) &&
      Array.isArray(screen.lines) &&
      screen.lines.length <= screen.rows &&
      screen.lines.every(
        (line) =>
          string(line, screen.cols) && Array.from(line).every(scalarAllowed),
      ) &&
      integer(screen.cursor_col, 0, screen.cols - 1) &&
      integer(screen.cursor_row, 0, screen.rows - 1) &&
      integer(screen.revision) &&
      integer(screen.mode_revision) &&
      [screen.cursor_visible, screen.alternate, screen.input_known].every(
        (value) => typeof value === "boolean",
      ) &&
      Array.isArray(screen.diagnostics) &&
      screen.diagnostics.length <= 64 &&
      screen.diagnostics.every((value) => string(value, 4096))
    );
  }
  function validateReceipt(receipt) {
    return (
      record(receipt) &&
      string(receipt.endpoint, 256) &&
      integer(receipt.id, 1) &&
      string(receipt.operation, 64) &&
      ["rejected", "reserved", "applied", "partial", "unknown"].includes(
        receipt.state,
      ) &&
      string(receipt.detail, 8192) &&
      integer(receipt.delivered_bytes)
    );
  }
  function validateSnapshot(snapshot) {
    return (
      record(snapshot) &&
      snapshot.version === VERSION &&
      string(snapshot.epoch, 256) &&
      string(snapshot.endpoint, 256) &&
      Array.isArray(snapshot.sessions) &&
      snapshot.sessions.length <= 2 &&
      snapshot.sessions.every(
        (session) =>
          record(session) &&
          string(session.id, 256) &&
          string(session.name, 4096) &&
          (session.owner === null || string(session.owner, 256)) &&
          integer(session.generation) &&
          ["running", "exited", "unknown"].includes(session.process) &&
          (session.exit_code === null ||
            integer(session.exit_code, -2147483648, 2147483647)) &&
          validateScreen(session.screen),
      ) &&
      Array.isArray(snapshot.attempts) &&
      snapshot.attempts.length <= 2 &&
      snapshot.attempts.every(
        (attempt) =>
          record(attempt) &&
          string(attempt.id, 256) &&
          string(attempt.task, 4096) &&
          ["running", "draining", "exited", "unknown"].includes(
            attempt.state,
          ) &&
          (attempt.exit_code === null ||
            integer(attempt.exit_code, -2147483648, 2147483647)) &&
          string(attempt.stdout, MAX_FRAME) &&
          string(attempt.stderr, MAX_FRAME) &&
          typeof attempt.truncated === "boolean",
      ) &&
      Array.isArray(snapshot.artifacts) &&
      snapshot.artifacts.length <= 128 &&
      snapshot.artifacts.every(
        (artifact) =>
          record(artifact) &&
          string(artifact.id, 256) &&
          string(artifact.task, 4096) &&
          string(artifact.path, 4096) &&
          string(artifact.revision, 256) &&
          (artifact.validated_revision === null ||
            string(artifact.validated_revision, 256)) &&
          typeof artifact.stale === "boolean",
      ) &&
      Array.isArray(snapshot.receipts) &&
      snapshot.receipts.length <= 512 &&
      snapshot.receipts.every(validateReceipt) &&
      (snapshot.degraded === null || string(snapshot.degraded, 8192))
    );
  }
  function validateResponse(response, expectedId) {
    return (
      record(response) &&
      response.id === expectedId &&
      typeof response.ok === "boolean" &&
      (response.error === null || string(response.error, 8192)) &&
      string(response.epoch, 256) &&
      (response.endpoint === null || string(response.endpoint, 256)) &&
      (response.resume === null || string(response.resume, 4096)) &&
      (response.receipt === null || validateReceipt(response.receipt)) &&
      (response.snapshot === null || validateSnapshot(response.snapshot))
    );
  }
  function captureFence(snapshot, sessionId, viewEpoch) {
    const session = snapshot?.sessions.find((value) => value.id === sessionId);
    if (!session) return null;
    return {
      session: session.id,
      generation: session.generation,
      revision: session.screen.revision,
      mode_revision: session.screen.mode_revision,
      epoch: snapshot.epoch,
      view_epoch: viewEpoch,
    };
  }
  function inputFenceValid(
    fence,
    snapshot,
    selectedSession,
    viewEpoch,
    focused,
  ) {
    const session = snapshot?.sessions.find(
      (value) => value.id === selectedSession,
    );
    return (
      focused &&
      snapshot?.degraded === null &&
      exactKeys(fence, [
        "session",
        "generation",
        "revision",
        "mode_revision",
        "epoch",
        "view_epoch",
      ]) &&
      integer(fence.revision) &&
      integer(fence.generation) &&
      integer(fence.mode_revision) &&
      integer(fence.view_epoch) &&
      session &&
      fence.session === selectedSession &&
      fence.epoch === snapshot.epoch &&
      fence.view_epoch === viewEpoch &&
      session.owner === snapshot.endpoint &&
      session.process === "running" &&
      session.screen.input_known &&
      fence.generation === session.generation &&
      fence.mode_revision === session.screen.mode_revision
    );
  }
  function projectScreen(screen) {
    if (!validateScreen(screen)) throw new Error("Malformed canonical screen");
    return Object.freeze({
      cols: screen.cols,
      rows: screen.rows,
      lines: Object.freeze(
        Array.from({ length: screen.rows }, (_, index) =>
          (screen.lines[index] || "").padEnd(screen.cols, " "),
        ),
      ),
      cursor: screen.cursor_visible
        ? Object.freeze({ col: screen.cursor_col, row: screen.cursor_row })
        : null,
      revision: screen.revision,
      modeRevision: screen.mode_revision,
    });
  }
  function safeText(value) {
    return String(value).replace(
      /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f\u0080-\u009f\u202a-\u202e\u2066-\u2069]/g,
      (character) => `⟦${character.codePointAt(0).toString(16)}⟧`,
    );
  }
  return {
    VERSION,
    MAX_FRAME,
    MAX_QUEUE,
    record,
    integer,
    string,
    requestEnvelope,
    validSocketPath,
    validateCommandShape,
    textAllowed,
    validateIntent,
    normalizeKey,
    normalizePaste,
    validateScreen,
    validateSnapshot,
    validateResponse,
    captureFence,
    inputFenceValid,
    projectScreen,
    safeText,
  };
});
