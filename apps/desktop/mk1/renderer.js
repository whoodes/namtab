"use strict";
(() => {
  const api = window.mk1Desktop;
  const protocol = window.Mk1;
  const element = (id) => document.getElementById(id);
  let state = {
    connected: false,
    snapshot: null,
    selectedSession: null,
    viewEpoch: 0,
    focused: false,
  };
  let screenKey = null;
  let artifactKey = null;
  let taskCounter = 1;
  let focusAfterTake = false;
  function text(id, value) {
    const next = protocol.safeText(value);
    if (element(id).textContent !== next) element(id).textContent = next;
  }
  function notice(message) {
    text("notice", message);
  }
  async function send(command) {
    const result = await api.dispatch(command);
    if (!result.ok) notice(result.error);
    return result;
  }
  function bind(id, event, callback) {
    element(id).addEventListener(event, callback);
  }
  function session() {
    return state.snapshot?.sessions.find(
      (value) => value.id === state.selectedSession,
    );
  }
  function attempt() {
    return state.snapshot?.attempts.find(
      (value) => value.id === element("attempts").value,
    );
  }
  function artifact() {
    return state.snapshot?.artifacts.find(
      (value) => value.id === element("artifacts").value,
    );
  }
  function options(id, values, label, selected) {
    const select = element(id);
    const wanted = selected ?? select.value;
    const signature = JSON.stringify(
      values.map((value) => [value.id, label(value)]),
    );
    if (select.dataset.signature !== signature) {
      select.replaceChildren(
        ...values.map((value) => {
          const option = document.createElement("option");
          option.value = value.id;
          option.textContent = protocol.safeText(label(value));
          return option;
        }),
      );
      select.dataset.signature = signature;
    }
    if (values.some((value) => value.id === wanted)) select.value = wanted;
  }
  function renderScreen(value) {
    const key = value
      ? `${value.id}/${value.screen.revision}/${value.screen.mode_revision}`
      : "empty";
    if (key === screenKey) return;
    screenKey = key;
    if (!value) {
      text(
        "screen",
        "No session observed. Create a clean Bash session to begin.",
      );
      return;
    }
    const projection = protocol.projectScreen(value.screen);
    const fragment = document.createDocumentFragment();
    projection.lines.forEach((line, row) => {
      if (projection.cursor?.row === row) {
        const col = projection.cursor.col;
        fragment.append(document.createTextNode(line.slice(0, col)));
        const cursor = document.createElement("span");
        cursor.className = "cursor";
        cursor.textContent = line[col] || " ";
        fragment.append(cursor);
        fragment.append(document.createTextNode(line.slice(col + 1)));
      } else fragment.append(document.createTextNode(line));
      if (row + 1 < projection.rows)
        fragment.append(document.createTextNode("\n"));
    });
    element("screen").replaceChildren(fragment);
  }
  function renderDetails() {
    const managed = attempt();
    text(
      "task-status",
      managed
        ? `${state.connected ? "" : "last observed "}${managed.state} · exit ${managed.exit_code ?? "not observed"}${managed.truncated ? " · output truncated" : ""}`
        : "No attempt",
    );
    text("stdout", managed?.stdout || "");
    text("stderr", managed?.stderr || "");
    element("observe-artifact").disabled = !state.connected || !managed;
    const file = artifact();
    const key = file ? `${file.id}/${file.revision}` : null;
    if (artifactKey !== key) {
      artifactKey = key;
      element("reviewed").checked = false;
    }
    text(
      "artifact-status",
      file
        ? `${file.path} · revision ${file.revision} · ${file.stale ? "STALE: prior review does not cover current revision" : file.validated_revision === file.revision ? "current revision marked reviewed" : "unreviewed"}`
        : "No artifact observed",
    );
    element("validate").disabled =
      !state.connected || !file || !element("reviewed").checked;
  }
  function render(next) {
    state = next;
    text(
      "connection",
      `${next.status}${next.connected ? ` · queued ${next.queued}/64` : ""}`,
    );
    text(
      "endpoint",
      next.endpoint
        ? `Endpoint ${next.endpoint} · epoch ${next.epoch || "pending"}`
        : "No endpoint",
    );
    notice(next.notice);
    element("reconnect").disabled =
      next.connected || next.status === "Connecting…";
    element("new-endpoint").disabled =
      next.connected || next.status === "Connecting…";
    element("degraded").hidden = !next.snapshot?.degraded;
    text("degraded", next.snapshot?.degraded || "");
    options(
      "sessions",
      next.snapshot?.sessions || [],
      (value) => `${value.name} · ${value.id}`,
      next.selectedSession,
    );
    options(
      "attempts",
      next.snapshot?.attempts || [],
      (value) => `${value.task} · ${value.id} · ${value.state}`,
    );
    options(
      "artifacts",
      next.snapshot?.artifacts || [],
      (value) => `${value.path} · ${value.stale ? "stale" : "current"}`,
    );
    const value = session();
    const owner = value?.owner === next.endpoint;
    const canInput =
      next.connected &&
      owner &&
      value?.process === "running" &&
      value.screen.input_known &&
      !next.snapshot?.degraded;
    text(
      "owner",
      value
        ? `Input owner${next.connected ? "" : " (last observed)"}: ${value.owner || "none"}${owner ? " (this endpoint)" : ""} · generation ${value.generation}`
        : "No selected session",
    );
    text(
      "process",
      value
        ? `${next.connected ? "" : "last observed "}${value.process} · exit ${value.exit_code ?? "not observed"}`
        : "No session",
    );
    text(
      "focus-status",
      !canInput
        ? "Input frozen"
        : next.focused
          ? "Sending to this session"
          : "Focus to send keys",
    );
    element("keyboard").disabled = !canInput;
    if (focusAfterTake && canInput) {
      focusAfterTake = false;
      element("keyboard").focus();
    }
    element("take").disabled = !next.connected || !value || owner;
    element("force").disabled = !next.connected || !value || owner;
    element("release").disabled = !next.connected || !owner;
    element("resize").disabled = !next.connected || !owner;
    element("run").disabled = !next.connected;
    element("create-form").querySelector("button").disabled =
      !next.connected || (next.snapshot?.sessions.length || 0) >= 2;
    text(
      "screen-status",
      value
        ? `${next.connected ? "" : "LAST OBSERVED · "}${value.screen.cols}×${value.screen.rows} · screen revision ${value.screen.revision} · mode revision ${value.screen.mode_revision} · ${value.screen.alternate ? "alternate" : "primary"} screen · input ${value.screen.input_known ? "known" : "UNKNOWN / refused"}${value.screen.diagnostics.length ? " · " + value.screen.diagnostics.join("; ") : ""}`
        : "No canonical observation",
    );
    renderScreen(value);
    renderDetails();
    element("receipts").replaceChildren(
      ...(next.snapshot?.receipts || [])
        .slice(-24)
        .reverse()
        .map((receipt) => {
          const row = document.createElement("div");
          row.className = "receipt";
          row.textContent = protocol.safeText(
            `#${receipt.id} ${receipt.operation} · ${receipt.state} · ${receipt.delivered_bytes} bytes · ${receipt.detail}`,
          );
          return row;
        }),
    );
  }
  bind("reconnect", "click", () => send({ op: "connect" }));
  bind("new-endpoint", "click", () => send({ op: "new_endpoint" }));
  bind("sessions", "change", (event) =>
    send({ op: "select", session: event.target.value }),
  );
  for (const [id, op, force] of [
    ["take", "take_control", false],
    ["force", "take_control", true],
    ["release", "release_control", false],
  ]) {
    bind(id, "click", async () => {
      const result = await send({
        op,
        session: state.selectedSession,
        ...(op === "take_control" ? { force } : {}),
      });
      if (result.ok && op === "take_control") {
        focusAfterTake = true;
        if (!element("keyboard").disabled) {
          focusAfterTake = false;
          element("keyboard").focus();
        }
      }
    });
  }
  bind("create-form", "submit", (event) => {
    event.preventDefault();
    send({ op: "create_session", name: element("session-name").value });
  });
  bind("resize-form", "submit", (event) => {
    event.preventDefault();
    send({
      op: "resize",
      session: state.selectedSession,
      cols: Number(element("cols").value),
      rows: Number(element("rows").value),
    });
  });
  bind("keyboard", "focus", () => send({ op: "focus", focused: true }));
  bind("keyboard", "blur", () => send({ op: "focus", focused: false }));
  function sendIntent(intent) {
    const fence = protocol.captureFence(
      state.snapshot,
      state.selectedSession,
      state.viewEpoch,
    );
    if (!fence || !state.focused) {
      notice("Wait for terminal focus to be acknowledged; nothing was sent.");
      return;
    }
    send({ op: "input", fence, intent });
  }
  bind("keyboard", "keydown", (event) => {
    const result = protocol.normalizeKey({
      key: event.key,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
      isComposing: event.isComposing,
      altGraph: event.getModifierState("AltGraph"),
    });
    if (result.native) return;
    event.preventDefault();
    if (result.error) notice(result.error);
    else sendIntent(result.intent);
  });
  bind("keyboard", "paste", (event) => {
    event.preventDefault();
    const result = protocol.normalizePaste(
      event.clipboardData?.getData("text/plain") || "",
    );
    if (result.error) notice(result.error);
    else sendIntent(result.intent);
  });
  bind("keyboard", "beforeinput", (event) => {
    event.preventDefault();
    if (event.inputType !== "insertFromPaste")
      notice(
        "Composition, drop and virtual-keyboard text input are not supported in MK1; nothing was sent.",
      );
  });
  bind("keyboard", "compositionstart", (event) => {
    event.preventDefault();
    notice("IME composition is unsupported in MK1; nothing was sent.");
  });
  bind("keyboard", "input", () => {
    element("keyboard").value = "";
  });
  bind("run-form", "submit", async (event) => {
    event.preventDefault();
    let argv;
    try {
      argv = JSON.parse(element("argv").value);
    } catch {
      notice(
        'argv must be valid JSON, for example ["/usr/bin/printf", "hello\\n"].',
      );
      return;
    }
    const result = await send({
      op: "run",
      task: element("task-name").value,
      argv,
    });
    if (result.ok) element("task-name").value = `desktop-task-${++taskCounter}`;
  });
  bind("attempts", "change", renderDetails);
  bind("artifacts", "change", renderDetails);
  bind("reviewed", "change", renderDetails);
  bind("artifact-form", "submit", (event) => {
    event.preventDefault();
    const value = attempt();
    if (value)
      send({
        op: "artifact",
        task: value.task,
        path: element("artifact-path").value,
      });
  });
  bind("validate", "click", () => {
    const value = artifact();
    if (value && element("reviewed").checked)
      send({
        op: "validate_artifact",
        artifact: value.id,
        revision: value.revision,
      });
  });
  bind("receipt-form", "submit", (event) => {
    event.preventDefault();
    send({ op: "receipt", request_id: Number(element("receipt-id").value) });
  });
  api.subscribe(render);
  send({ op: "state" });
})();
