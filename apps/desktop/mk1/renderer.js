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
    inputPaused: true,
  };
  let screenKey = null;
  let artifactKey = null;
  let taskCounter = 1;
  let selectedTask = null;
  let selectedAttempt = null;
  let focusAfterTake = false;
  let focusRequest = 0;
  let acknowledgedFocusEpoch = null;
  let prefixDeadline = null;
  let prefixTimer = null;
  let localInputPaused = true;
  let localPauseReason =
    "Inspect the selected session, then Take input to arm typing.";
  function refuseInput(message) {
    localInputPaused = true;
    localPauseReason = message;
    acknowledgedFocusEpoch = null;
    clearPrefix();
    element("keyboard").disabled = true;
    notice(
      message + " Input paused: inspect the session and Take input again.",
    );
    send({ op: "pause_input", reason: message });
    return { ok: false, error: message };
  }
  function clearPrefix() {
    prefixDeadline = null;
    clearTimeout(prefixTimer);
    prefixTimer = null;
  }
  async function acknowledgeFocus(focused) {
    const request = ++focusRequest;
    acknowledgedFocusEpoch = null;
    clearPrefix();
    const response = await send({ op: "focus", focused });
    if (
      request === focusRequest &&
      focused &&
      response.ok &&
      response.result?.focused &&
      document.hasFocus() &&
      document.activeElement === element("keyboard")
    )
      acknowledgedFocusEpoch = response.result.viewEpoch;
  }
  function text(id, value) {
    const next = protocol.safeText(value);
    if (element(id).textContent !== next) element(id).textContent = next;
  }
  function notice(message, urgent = true) {
    text("notice", message);
    element("attention").hidden = !urgent;
    text("attention", urgent ? message : "");
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
  function task() {
    return state.snapshot?.tasks?.find(
      (value) => value.task === element("tasks").value,
    );
  }
  function showActivity(kind) {
    const shell = kind === "shell";
    element("shell-panel").hidden = !shell;
    element("task-panel").hidden = shell;
    element("show-shell").setAttribute("aria-pressed", String(shell));
    element("show-task").setAttribute("aria-pressed", String(!shell));
  }
  function renderTask() {
    const selected = task();
    const supported = state.snapshot?.capabilities?.includes("shared_tasks_v1");
    const empty = !!supported && (state.snapshot.tasks?.length || 0) === 0;
    const all = !!supported && !empty && !selected;
    const definition = element("define-task-tools");
    const host = element(
      empty ? "empty-task-composer" : "task-definition-sidebar",
    );
    if (definition.parentElement !== host) {
      host.append(definition);
      definition.open = empty;
    }
    element("activity").hidden =
      empty &&
      !state.snapshot.sessions.length &&
      !state.snapshot.attempts.length;
    text(
      "task-heading",
      selected?.task ||
        (all
          ? "All attempts / unregistered"
          : supported
            ? "Define the first shared task"
            : "Shared tasks unavailable"),
    );
    text(
      "task-goal",
      selected?.goal ||
        (all
          ? "Showing all retained attempts. Unregistered labels do not bind an attempt to a task contract."
          : supported
            ? "Share an immutable goal and expected artifacts with connected typed clients. No model is started by this app."
            : "This daemon has no shared-task capability. Existing shell and attempt activity remain available."),
    );
    text(
      "task-expectations",
      selected
        ? selected.expected_artifacts.length
          ? "Declared expected artifacts: " +
            selected.expected_artifacts.join(", ") +
            ". File existence and review are separate observations."
          : "No expected artifact paths declared."
        : "",
    );
    text(
      "task-revision",
      selected ? selected.revision : "No contract observed",
    );
    const attempts = protocol.attemptsForTask(
      state.snapshot?.attempts || [],
      selected,
    );
    options(
      "attempts",
      attempts,
      (value) => `${value.task} · ${value.id} · ${value.state}`,
      selectedAttempt,
    );
    if (attempts.some((value) => value.id === selectedAttempt))
      selectedAttempt = null;
    text(
      "run-target",
      selected
        ? `Launch is bound to selected task “${selected.task}” and its exact immutable revision.`
        : "No shared task selected. The command will use the unregistered label below.",
    );
    element("fallback-task-label").hidden = !!selected;
    const used = state.snapshot?.attempts.length;
    text(
      "attempt-budget",
      used === undefined
        ? "Attempt capacity unobserved"
        : `${used}/2 retained attempts used for this daemon lifetime. ${used >= 2 ? "Capacity reached, including completed attempts; no further managed launch can be admitted." : "Completed attempts also occupy this bounded prototype capacity."}`,
    );
    element("run").disabled = !state.connected || used >= 2;
    element("define-task").disabled =
      !state.connected ||
      !supported ||
      (state.snapshot?.tasks?.length || 0) >= 4;
    element("command-label").hidden = element("command-mode").value !== "shell";
    element("argv-label").hidden = element("command-mode").value !== "argv";
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
    text(
      "stdout",
      managed?.stdout ||
        (managed
          ? "No standard output observed."
          : "No attempt observed for this selection."),
    );
    const origin = managed?.origin;
    const originRole = state.snapshot?.endpoints.find(
      (value) => value.id === origin?.endpoint,
    )?.role;
    text(
      "attempt-origin",
      managed
        ? origin
          ? `Launch endpoint ${origin.endpoint}${originRole ? " (declared " + originRole + ")" : ""} · request ${origin.request_id} · ${managed.task_revision ? "bound task revision " + managed.task_revision : "unregistered task label; no contract binding"}`
          : "Legacy attempt: launch origin and task binding unknown."
        : "No launch origin observed",
    );
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
    if (next.viewEpoch !== state.viewEpoch || !next.connected) {
      clearPrefix();
    }
    acknowledgedFocusEpoch = protocol.reconcileFocusAcknowledgement(
      acknowledgedFocusEpoch,
      next.viewEpoch,
      next.connected,
    );
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
    const declaredAgents = next.snapshot?.endpoints.filter(
      (endpoint) => endpoint.role === "agent",
    );
    text(
      "agent-status",
      declaredAgents
        ? `${next.connected ? "" : "Last observed · "}Declared agent endpoints: ${declaredAgents.filter((endpoint) => endpoint.connected).length} connected / ${declaredAgents.length} retained`
        : "Declared agent endpoints: unobserved",
    );
    element("agent-connections").replaceChildren(
      ...(declaredAgents || []).map((endpoint) => {
        const row = document.createElement("div");
        row.textContent = protocol.safeText(
          `${endpoint.id} · declared agent · ${endpoint.connected ? "connected" : "disconnected"}${next.connected ? "" : " (last observed)"}`,
        );
        return row;
      }),
    );
    notice(next.notice, next.noticeKind === "warning");
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
      "tasks",
      [
        { id: "", label: "All attempts / unregistered" },
        ...(next.snapshot?.tasks || []).map((value) => ({
          ...value,
          id: value.task,
          label: value.task,
        })),
      ],
      (value) => value.label,
      selectedTask,
    );
    if (
      selectedTask &&
      next.snapshot?.tasks?.some((value) => value.task === selectedTask)
    )
      selectedTask = null;
    renderTask();
    options(
      "artifacts",
      next.snapshot?.artifacts || [],
      (value) => `${value.path} · ${value.stale ? "stale" : "current"}`,
    );
    const value = session();
    const owner = value?.owner === next.endpoint;
    if (value && next.inputPaused && next.inputPauseRevision > 0) {
      element("attention").hidden = false;
      text("attention", "Typing paused. " + next.inputPauseReason);
    }
    const canInput =
      next.connected &&
      owner &&
      value?.process === "running" &&
      value.screen.input_known &&
      !next.snapshot?.degraded &&
      !next.inputPaused &&
      !localInputPaused;
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
        : next.focused && acknowledgedFocusEpoch === next.viewEpoch
          ? "Sending to this session"
          : document.activeElement === element("keyboard") &&
              document.hasFocus()
            ? "Waiting for focus acknowledgement"
            : "Focus to send keys",
    );
    element("keyboard").disabled = !canInput;
    element("input-pause").hidden = !next.inputPaused && !localInputPaused;
    text(
      "input-pause",
      "Typing paused. " +
        (next.inputPauseReason ||
          localPauseReason ||
          "Inspect the session, then Take input to rearm."),
    );
    if (focusAfterTake && canInput) {
      focusAfterTake = false;
      element("keyboard").focus();
    }
    element("take").disabled =
      !next.connected ||
      !value ||
      (owner && !next.inputPaused && !localInputPaused);
    text(
      "take",
      owner && (next.inputPaused || localInputPaused)
        ? "Inspect / Rearm input"
        : "Take input",
    );
    element("force").disabled = !next.connected || !value || owner;
    element("release").disabled = !next.connected || !owner;
    element("resize").disabled = !next.connected || !owner;

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
  bind("show-task", "click", () => showActivity("task"));
  bind("show-shell", "click", () => showActivity("shell"));
  bind("open-shell", "click", () => showActivity("shell"));
  bind("tasks", "change", () => {
    renderTask();
    renderDetails();
  });
  bind("command-mode", "change", renderTask);
  bind("define-task-form", "submit", async (event) => {
    event.preventDefault();
    const paths = element("expected-artifacts").value.trim()
      ? element("expected-artifacts").value.split(/\r?\n/)
      : [];
    const result = await send({
      op: "define_task",
      task: element("contract-name").value,
      goal: element("contract-goal").value,
      expected_artifacts: paths,
    });
    const reference = result.result?.result;
    if (result.ok && reference?.kind === "task_defined") {
      selectedTask = reference.task;
      render(state);
      element("define-task-tools").open = false;
    }
  });
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
        if (!result.result?.rearmed) return;
        localInputPaused = false;
        localPauseReason = null;
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
    showActivity("shell");
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
  bind("keyboard", "focus", () => acknowledgeFocus(true));
  bind("keyboard", "blur", () => acknowledgeFocus(false));
  window.addEventListener("blur", () => acknowledgeFocus(false));
  window.addEventListener("focus", () => {
    if (
      document.activeElement === element("keyboard") &&
      !element("keyboard").disabled
    )
      acknowledgeFocus(true);
  });
  bind("terminal-scroll", "click", () => {
    if (!element("keyboard").disabled && !window.getSelection()?.toString())
      element("keyboard").focus();
  });
  async function sendIntent(intent) {
    if (state.inputPaused || localInputPaused)
      return {
        ok: false,
        error: "Input is paused until explicit inspection and Take input.",
      };
    const fence = protocol.captureFence(
      state.snapshot,
      state.selectedSession,
      state.viewEpoch,
    );
    if (
      !fence ||
      !protocol.focusReady(
        acknowledgedFocusEpoch,
        state.viewEpoch,
        state.focused,
        document.hasFocus(),
        document.activeElement === element("keyboard"),
      )
    ) {
      return refuseInput(
        "Application input was refused because focus or the selected session is not acknowledged.",
      );
    }
    const result = await send({ op: "input", fence, intent });
    if (!result.ok || result.result?.state !== "applied") {
      localInputPaused = true;
      localPauseReason =
        "Application input was refused or delivery is uncertain. Inspect the session and receipt.";
      element("keyboard").disabled = true;
    }
    return result;
  }
  bind("keyboard", "keydown", (event) => {
    const description = {
      key: event.key,
      code: event.code,
      location: event.location,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
      isComposing: event.isComposing,
      altGraph: event.getModifierState("AltGraph"),
    };
    const route = protocol.routeUiKey(
      description,
      prefixDeadline,
      performance.now(),
    );
    if (route.kind !== "normal") {
      event.preventDefault();
      if (route.kind === "prefix") {
        clearPrefix();
        prefixDeadline = route.prefixDeadline;
        notice(
          "UI prefix: Escape returns to controls; Ctrl+G sends literal Ctrl+G. Other next keys are discarded.",
        );
        prefixTimer = setTimeout(() => {
          clearPrefix();
          refuseInput("UI prefix timed out; nothing was sent.");
        }, 1500);
      } else if (route.kind === "leave") {
        clearPrefix();
        element("keyboard").blur();
        element("sessions").focus();
        notice(
          "Keyboard focus returned to session controls; no application bytes sent.",
        );
      } else if (route.kind === "literal") {
        clearPrefix();
        sendIntent(route.intent);
      } else {
        prefixDeadline = route.prefixDeadline;
        if (prefixDeadline === null) clearPrefix();
        if (route.notice) refuseInput(route.notice);
      }
      return;
    }
    const result = protocol.normalizeKey(description);
    if (result.native) return;
    event.preventDefault();
    if (result.error) refuseInput(result.error);
    else sendIntent(result.intent);
  });
  bind("keyboard", "paste", (event) => {
    event.preventDefault();
    const result = protocol.routeUiPaste(
      event.clipboardData?.getData("text/plain") || "",
      prefixDeadline,
    );
    clearPrefix();
    if (result.consumed) refuseInput(result.notice);
    else if (result.error) refuseInput(result.error);
    else sendIntent(result.intent);
  });
  bind("keyboard", "beforeinput", (event) => {
    event.preventDefault();
    if (event.inputType !== "insertFromPaste")
      refuseInput(
        "Composition, drop and virtual-keyboard text input are not supported in MK1; nothing was sent.",
      );
  });
  bind("keyboard", "compositionstart", (event) => {
    event.preventDefault();
    refuseInput("IME composition is unsupported in MK1; nothing was sent.");
  });
  bind("keyboard", "input", () => {
    element("keyboard").value = "";
  });
  bind("run-form", "keydown", (event) => {
    if (
      event.key === "Enter" &&
      event.target.tagName !== "TEXTAREA" &&
      event.target.tagName !== "BUTTON"
    )
      event.preventDefault();
  });
  bind("run-form", "submit", async (event) => {
    event.preventDefault();
    if (event.submitter !== element("run")) {
      notice("Use the explicit Run command button to execute.");
      return;
    }
    let argv;
    if (element("command-mode").value === "shell") {
      argv = protocol.shellCommandArgv(element("command").value);
      if (!argv) {
        notice(
          "Enter a nonblank local shell command within the bounded argv limit.",
        );
        return;
      }
    } else {
      try {
        argv = JSON.parse(element("argv").value);
      } catch {
        notice(
          'Literal argv must be a JSON array, for example ["/usr/bin/printf", "hello\\n"].',
        );
        return;
      }
    }
    const contract = task();
    const result = await send({
      op: "run",
      task: contract?.task || element("task-name").value,
      argv,
      task_revision: contract?.revision || null,
    });
    const reference = result.result?.result;
    if (result.ok) {
      if (reference?.kind === "attempt_started")
        selectedAttempt = reference.attempt;
      if (!contract)
        element("task-name").value = `desktop-task-${++taskCounter}`;
      render(state);
      showActivity("task");
    }
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
