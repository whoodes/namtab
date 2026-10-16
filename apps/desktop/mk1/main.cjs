"use strict";
const { app, BrowserWindow, ipcMain, session } = require("electron");
const path = require("node:path");
const fs = require("node:fs");
const { pathToFileURL } = require("node:url");
const { Connection } = require("./connection.cjs");
const {
  string,
  integer,
  validateIntent,
  validateCommandShape,
  validSocketPath,
  validArgv,
  inputFenceValid,
} = require("./shared.js");

const projectRoot = path.resolve(__dirname, "../../..");
const stateRoot = path.join(
  projectRoot,
  ".artifacts",
  "mk1-desktop",
  `${Date.now()}-${process.pid}`,
);
for (const name of [
  "XDG_CONFIG_HOME",
  "XDG_CACHE_HOME",
  "XDG_DATA_HOME",
  "TMPDIR",
]) {
  const directory = path.join(stateRoot, name);
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  process.env[name] = directory;
}
for (const name of [
  "appData",
  "userData",
  "sessionData",
  "crashDumps",
  "temp",
  "logs",
]) {
  const directory = path.join(stateRoot, name);
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  app.setPath(name, directory);
}
app.enableSandbox();
const socketIndex = process.argv.indexOf("--socket");
const socketPath =
  socketIndex >= 0 ? process.argv[socketIndex + 1] : process.env.NAMTAB_SOCKET;
if (!validSocketPath(socketPath)) {
  process.stderr.write(
    "MK1 desktop requires --socket /absolute/public.sock or NAMTAB_SOCKET, at most 107 UTF-8 bytes on Linux and without NUL. Use a short public endpoint path.\n",
  );
  app.exit(1);
} else {
  app
    .whenReady()
    .then(() => createWindow(socketPath))
    .catch(() => {
      process.stderr.write("MK1 desktop startup failed.\n");
      app.exit(1);
    });
}
app.on("window-all-closed", () => app.quit());

function createWindow(publicSocketPath) {
  const connection = new Connection(publicSocketPath);
  let snapshot = null;
  let selectedSession = null;
  let focused = false;
  let viewEpoch = 0;
  let status = "Disconnected";
  let notice = "Capture is off. Input and observations remain in RAM.";
  const localUrl = pathToFileURL(path.join(__dirname, "index.html")).href;
  const isolatedSession = session.fromPartition(`mk1-${process.pid}`, {
    cache: false,
  });
  isolatedSession.setPermissionRequestHandler(
    (_contents, _permission, callback) => callback(false),
  );
  isolatedSession.setPermissionCheckHandler(() => false);
  isolatedSession.webRequest.onBeforeRequest((details, callback) =>
    callback({
      cancel: !details.url.startsWith(pathToFileURL(__dirname + path.sep).href),
    }),
  );
  const window = new BrowserWindow({
    title: "Namtab MK1",
    width: 1160,
    height: 900,
    minWidth: 700,
    minHeight: 600,
    show: false,
    backgroundColor: "#17191c",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      session: isolatedSession,
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      webSecurity: true,
      devTools: false,
      v8CacheOptions: "none",
    },
  });
  window.removeMenu();
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());
  window.webContents.on("will-attach-webview", (event) =>
    event.preventDefault(),
  );
  function publish() {
    if (!window.isDestroyed() && !window.webContents.isDestroyed())
      window.webContents.send("mk1:state", {
        connected: connection.ready,
        endpoint: connection.endpoint,
        epoch: connection.epoch,
        snapshot,
        selectedSession,
        focused,
        viewEpoch,
        status,
        notice,
        queued: connection.queue.length + (connection.pending ? 1 : 0),
      });
  }
  function fenceFocus() {
    focused = false;
    viewEpoch++;
    connection.clearQueuedInputs();
    publish();
  }
  function selected() {
    return snapshot?.sessions.find((value) => value.id === selectedSession);
  }
  function guardSession(sessionId, generation) {
    return () =>
      connection.ready &&
      selectedSession === sessionId &&
      selected()?.generation === generation &&
      selected()?.owner === connection.endpoint;
  }
  function checkedString(value, limit, label) {
    if (
      !string(value, limit) ||
      !value.trim() ||
      /[\u0000-\u001f\u007f]/.test(value)
    )
      throw new Error(`Invalid ${label}`);
    return value;
  }
  async function dispatch(command) {
    if (!validateCommandShape(command)) throw new Error("Malformed UI command");
    if (command.op === "state") {
      publish();
      return;
    }
    if (command.op === "connect" || command.op === "new_endpoint") {
      if (connection.socket) throw new Error("Already connected or connecting");
      fenceFocus();
      if (command.op === "new_endpoint") {
        snapshot = null;
        selectedSession = null;
        notice =
          "A new endpoint cannot restore old owner state or settle unknown input delivery.";
      }
      connection.connect(command.op === "new_endpoint");
      return;
    }
    if (command.op === "select") {
      if (!snapshot?.sessions.some((value) => value.id === command.session))
        throw new Error("Unknown session");
      selectedSession = command.session;
      fenceFocus();
      return;
    }
    if (command.op === "focus") {
      if (typeof command.focused !== "boolean")
        throw new Error("Invalid focus state");
      focused = command.focused && window.isFocused();
      viewEpoch++;
      connection.clearQueuedInputs();
      publish();
      return { viewEpoch, focused };
    }
    if (!connection.ready || !snapshot)
      throw new Error("Input frozen until connected and observed");
    let fields;
    let guard = null;
    if (command.op === "input") {
      if (
        !validateIntent(command.intent) ||
        !inputFenceValid(
          command.fence,
          snapshot,
          selectedSession,
          viewEpoch,
          focused,
        )
      )
        throw new Error(
          "Input refused: focus, owner, selection, mode or repertoire is stale/unsupported. Observe and take input explicitly.",
        );
      const fence = { ...command.fence };
      fields = {
        op: "input",
        session: fence.session,
        generation: fence.generation,
        revision: fence.revision,
        mode_revision: fence.mode_revision,
        policy: "human_live",
        intent: command.intent,
      };
      guard = () =>
        inputFenceValid(fence, snapshot, selectedSession, viewEpoch, focused);
    } else if (
      ["take_control", "release_control", "resize"].includes(command.op)
    ) {
      const value = selected();
      if (!value || command.session !== selectedSession)
        throw new Error("Select the intended session first");
      fields = {
        op: command.op,
        session: value.id,
        generation: value.generation,
      };
      guard = () =>
        selectedSession === value.id &&
        selected()?.generation === value.generation;
      if (command.op === "take_control") {
        if (typeof command.force !== "boolean")
          throw new Error("Invalid takeover flag");
        fields.force = command.force;
      } else {
        guard = guardSession(value.id, value.generation);
        if (command.op === "resize") {
          if (!integer(command.cols, 1, 160) || !integer(command.rows, 1, 48))
            throw new Error("Geometry must be 1–160 columns by 1–48 rows");
          fields.cols = command.cols;
          fields.rows = command.rows;
        }
      }
    } else if (command.op === "create_session")
      fields = {
        op: command.op,
        name: checkedString(command.name, 128, "session name"),
      };
    else if (command.op === "run") {
      if (!validArgv(command.argv))
        throw new Error(
          "argv must be a JSON array of 1–32 strings without NUL; executable must be nonempty",
        );
      fields = {
        op: "run",
        task: checkedString(command.task, 128, "task label"),
        argv: command.argv,
      };
    } else if (command.op === "artifact")
      fields = {
        op: "artifact",
        task: checkedString(command.task, 128, "task label"),
        path: checkedString(command.path, 4096, "artifact path"),
      };
    else if (command.op === "validate_artifact")
      fields = {
        op: command.op,
        artifact: checkedString(command.artifact, 256, "artifact identity"),
        revision: checkedString(command.revision, 256, "artifact revision"),
      };
    else if (command.op === "receipt") {
      if (!integer(command.request_id, 1))
        throw new Error("Receipt ID must be a positive safe integer");
      fields = { op: "receipt", request_id: command.request_id };
    } else throw new Error("Unsupported UI operation");
    const response = await connection.request(fields, guard);
    if (!response.ok)
      throw new Error(
        response.error || "Backend refused operation; inspect its receipt",
      );
    return response.receipt
      ? { id: response.receipt.id, state: response.receipt.state }
      : null;
  }
  ipcMain.handle("mk1:command", async (event, command) => {
    if (
      event.sender !== window.webContents ||
      event.senderFrame !== window.webContents.mainFrame ||
      event.senderFrame.url !== localUrl
    )
      return { ok: false, error: "Untrusted UI sender" };
    try {
      const result = await dispatch(command);
      publish();
      return { ok: true, result };
    } catch (error) {
      notice = String(error.message || "Operation refused");
      publish();
      return { ok: false, error: notice };
    }
  });
  connection.on("status", (value) => {
    status = value;
    publish();
  });
  connection.on("disconnected", fenceFocus);
  connection.on("response", (response) => {
    if (response.snapshot) {
      if (
        response.snapshot.epoch !== response.epoch ||
        (connection.endpoint &&
          response.snapshot.endpoint !== connection.endpoint)
      ) {
        connection.disconnect(
          "Snapshot endpoint/epoch mismatch; input frozen.",
        );
        return;
      }
      snapshot = response.snapshot;
      if (!snapshot.sessions.some((value) => value.id === selectedSession)) {
        selectedSession = snapshot.sessions[0]?.id || null;
        fenceFocus();
      }
    }
    if (response.receipt)
      notice = `Receipt ${response.receipt.id}: ${response.receipt.state}. ${response.receipt.detail}`;
    if (!response.ok) notice = response.error || "Backend refused request";
    publish();
  });
  connection.on("ready", publish);
  const poll = setInterval(() => {
    if (
      connection.ready &&
      !connection.pending &&
      connection.queue.length === 0
    )
      connection.request({ op: "observe" }).catch(() => {});
  }, 100);
  window.on("blur", fenceFocus);
  window.on("closed", () => {
    clearInterval(poll);
    connection.disconnect();
    ipcMain.removeHandler("mk1:command");
  });
  window.webContents.on("did-finish-load", () => {
    publish();
    connection.connect();
  });
  window.once("ready-to-show", () => window.show());
  window.loadFile(path.join(__dirname, "index.html"));
  return window;
}
