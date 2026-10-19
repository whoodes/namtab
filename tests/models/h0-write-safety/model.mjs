// Test-only finite transition model. No IO, clocks, workers or production imports.
import { isDeepStrictEqual } from "node:util";

export const BOUNDS = Object.freeze({
  sequences: 4,
  calls: 3,
  bytes: 3,
  generations: 8,
});
const actors = ["RA", "RB"];
const faultNames = ["none", "skip-run", "skip-assignment", "skip-suffix"];
const copy = (x) => structuredClone(x);
const ok = (value) => ({ ok: true, value });
const no = (reason) => ({ ok: false, reason });
const keyOf = (r) => `${r.namespace}/${r.epoch}/${r.sequence}`;

export function initial(fault = "none") {
  if (!faultNames.includes(fault)) throw new Error("unknown test fault");
  return {
    fault,
    now: 0,
    epoch: 1,
    nextOperation: 1,
    runs: Object.fromEntries(
      actors.map((a) => [
        a,
        {
          open: true,
          control: 1,
          capability: 1,
          permitted: true,
          context: "C1",
          connected: true,
        },
      ]),
    ),
    assignment: {
      holder: null,
      generation: 1,
      lifecycle: 1,
      attempt: "A1",
      task: "T1",
      context: "C1",
    },
    writer: { holder: "RA", generation: 1 },
    session: { guard: 1, known: true, tainted: false },
    namespaces: {
      NA: { owner: "RA", consumed: 0 },
      NB: { owner: "RB", consumed: 0 },
    },
    operations: {},
    bindings: {},
    active: null,
  };
}

function binding(s, run) {
  return {
    run,
    control: s.runs[run].control,
    capability: s.runs[run].capability,
    assignment: s.assignment.generation,
    attempt: s.assignment.attempt,
    task: s.assignment.task,
    context: s.assignment.context,
    writer: s.writer.generation,
    epoch: s.epoch,
    guard: s.session.guard,
  };
}

// Test helper assumes an issued namespace/authenticated principal and canonical
// frozen synthetic encoding. This is not a wire decoder or an encoder.
export function intent(s, run = "RA", sequence = 1, bytes = "abc") {
  if (!actors.includes(run)) throw new Error("unknown run");
  return {
    namespace: run === "RA" ? "NA" : "NB",
    epoch: s.epoch,
    sequence,
    deadline: 5,
    bytes,
    binding: binding(s, run),
  };
}

function eligible(s, op) {
  if (
    !s.session.known ||
    op.epoch !== s.epoch ||
    op.guard !== s.session.guard ||
    s.now >= op.deadline
  )
    return false;
  if (op.reply) return true; // Independent canonical lane, not agent authority.
  const b = op.binding,
    r = s.runs[b.run],
    a = s.assignment;
  if (b.epoch !== s.epoch) return false;
  if (s.fault === "skip-suffix" && op.calls > 0) return true;
  if (
    s.fault !== "skip-run" &&
    (!r?.open ||
      !r.permitted ||
      b.control !== r.control ||
      b.capability !== r.capability)
  )
    return false;
  if (
    s.fault !== "skip-assignment" &&
    (a.holder !== b.run ||
      b.assignment !== a.generation ||
      b.attempt !== a.attempt ||
      b.task !== a.task ||
      b.context !== a.context ||
      r.context !== a.context)
  )
    return false;
  return s.writer.holder === b.run && s.writer.generation === b.writer;
}

function close(s, op, unknown = false) {
  op.phase = "closed";
  op.delivery = unknown
    ? "unknown"
    : op.known === op.bytes.length && op.known > 0
      ? "full"
      : op.known > 0
        ? "partial"
        : "zero";
  if (unknown || (op.known > 0 && op.known < op.bytes.length))
    s.session.tainted = true;
  if (s.active === op.id && !op.flight) s.active = null;
}
function fence(s) {
  for (const op of Object.values(s.operations)) {
    if (op.phase === "closed" || (op.retained && eligible(s, op))) continue;
    if (op.flight) {
      op.phase = "closing";
      op.delivery = "unknown";
    } else close(s, op);
  }
}
const advance = (object, name) => {
  if (object[name] >= BOUNDS.generations) return false;
  object[name] += 1;
  return true;
};

function shape(r) {
  return (
    r &&
    Object.keys(r).sort().join() ===
      "binding,bytes,deadline,epoch,namespace,sequence" &&
    typeof r.bytes === "string" &&
    /^[a-z]{0,3}$/.test(r.bytes) &&
    Number.isInteger(r.sequence) &&
    r.sequence >= 1 &&
    r.sequence <= BOUNDS.sequences &&
    Number.isInteger(r.deadline) &&
    r.deadline > 0 &&
    r.deadline <= 5 &&
    Number.isInteger(r.epoch) &&
    r.binding &&
    Object.keys(r.binding).sort().join() ===
      "assignment,attempt,capability,context,control,epoch,guard,run,task,writer" &&
    ["control", "capability", "assignment", "writer", "epoch", "guard"].every(
      (k) =>
        Number.isInteger(r.binding[k]) &&
        r.binding[k] >= 1 &&
        r.binding[k] <= BOUNDS.generations,
    ) &&
    ["attempt", "task", "context"].every(
      (k) => typeof r.binding[k] === "string" && r.binding[k].length > 0,
    )
  );
}

// All non-submit events model already-authorized ordered control/keeper facts.
// They are NOT callable control APIs, authentication or actual syscalls.
export function step(before, action) {
  const s = copy(before),
    event = copy(action);
  let result = no("invalid-event");
  const a = s.assignment,
    run = s.runs[action.run];
  switch (action.type) {
    case "claim":
    case "transfer":
    case "release": {
      const claim = action.type === "claim",
        release = action.type === "release";
      if (
        action.lifecycle !== a.lifecycle ||
        action.generation !== a.generation ||
        action.attempt !== a.attempt ||
        action.context !== a.context ||
        (claim ? a.holder !== null : action.from !== a.holder)
      ) {
        result = no("stale-assignment");
        break;
      }
      const target = s.runs[action.to];
      if (
        !release &&
        (!target?.open || !target.permitted || target.context !== a.context)
      ) {
        result = no("target-refused");
        break;
      }
      if (
        a.generation >= BOUNDS.generations ||
        a.lifecycle >= BOUNDS.generations
      ) {
        result = no("exhausted");
        break;
      }
      a.holder = release ? null : action.to;
      a.generation++;
      a.lifecycle++;
      fence(s);
      result = ok("assignment-transition");
      break;
    }
    case "replace-attempt":
      if (action.lifecycle !== a.lifecycle || a.attempt !== "A1") {
        result = no("stale-attempt");
        break;
      }
      if (
        a.generation >= BOUNDS.generations ||
        a.lifecycle >= BOUNDS.generations
      ) {
        result = no("exhausted");
        break;
      }
      a.generation++;
      a.lifecycle++;
      a.attempt = "A2";
      a.task = "T2";
      fence(s);
      result = ok("attempt-replaced");
      break;
    case "close-run":
    case "revoke-capability":
      if (!run?.open) {
        result = no("closed-run");
        break;
      }
      if (
        !advance(run, action.type === "close-run" ? "control" : "capability")
      ) {
        result = no("exhausted");
        break;
      }
      if (action.type === "close-run") run.open = false;
      else run.permitted = false;
      fence(s);
      result = ok("run-fenced");
      break;
    case "disconnect":
    case "reconnect":
      if (!run) break;
      if (
        action.type === "disconnect" &&
        s.writer.holder === action.run &&
        !advance(s.writer, "generation")
      ) {
        result = no("exhausted");
        break;
      }
      run.connected = action.type === "reconnect";
      fence(s);
      result = ok("connection-only");
      break;
    case "takeover":
      if (
        ![...actors, "H"].includes(action.to) ||
        !advance(s.writer, "generation")
      )
        break;
      s.writer.holder = action.to;
      fence(s);
      result = ok("writer-transition");
      break;
    case "session-fence":
      if (!advance(s.session, "guard")) {
        result = no("exhausted");
        break;
      }
      fence(s);
      result = ok("session-fenced");
      break;
    case "epoch-change":
      if (!advance(s, "epoch")) {
        result = no("exhausted");
        break;
      }
      fence(s);
      result = ok("epoch-fenced");
      break;
    case "tick":
      if (s.now >= 6) {
        result = no("exhausted");
        break;
      }
      s.now++;
      fence(s);
      result = ok("clock-step");
      break;
    case "submit": {
      const r = action.request,
        ns = s.namespaces[r?.namespace];
      if (
        !shape(r) ||
        !ns ||
        ns.owner !== action.actor ||
        r.binding.run !== action.actor ||
        !s.runs[action.actor]?.connected
      ) {
        result = no("preflight");
        break;
      }
      if (r.epoch !== s.epoch || s.now >= 5) {
        result = no("preflight");
        break;
      }
      const key = keyOf(r);
      // Historical lookup never creates a new effect, even after run closure.
      if (r.sequence <= ns.consumed && r.epoch === 1) {
        const old = s.bindings[key];
        if (!old) result = no("binding-unavailable");
        else if (!isDeepStrictEqual(old.request, r))
          result = no("binding-conflict");
        else result = ok(old.decision);
        break;
      }
      if (
        r.epoch !== s.epoch ||
        r.epoch !== 1 ||
        s.now >= 5 ||
        s.now >= r.deadline ||
        r.sequence !== ns.consumed + 1
      ) {
        result = no("preflight");
        break;
      }
      ns.consumed++;
      const op = {
        id: `O${s.nextOperation}`,
        key,
        binding: copy(r.binding),
        origin: copy(r.binding),
        bytes: r.bytes,
        deadline: r.deadline,
        epoch: r.epoch,
        guard: r.binding.guard,
        phase: "queued",
        delivery: "pending",
        known: 0,
        calls: 0,
        flight: null,
        retained: true,
      };
      if (!eligible(s, op)) {
        const decision = { admission: "rejected", reason: "stale-or-denied" };
        s.bindings[key] = { request: copy(r), decision };
        result = ok(decision);
        break;
      }
      s.nextOperation++;
      s.operations[op.id] = op;
      const decision = { admission: "admitted", operation: op.id };
      s.bindings[key] = { request: copy(r), decision };
      result = ok(decision);
      break;
    }
    case "query": {
      const entry = s.bindings[action.key];
      const op = Object.values(s.operations).find((x) => x.key === action.key);
      // Assume current authorized read; this is not an auth check by label.
      result = !op
        ? entry
          ? ok(entry.decision)
          : no("unavailable-unknown")
        : !op.retained
          ? no("unavailable-unknown")
          : ok({
              id: op.id,
              phase: op.phase,
              delivery: op.delivery,
              known: op.known,
            });
      break;
    }
    case "forget-receipt": {
      const op = s.operations[action.operation];
      if (!op) break;
      op.retained = false;
      delete s.bindings[op.key];
      fence(s);
      result = ok("receipt-unavailable");
      break;
    }
    case "forget-binding":
      delete s.bindings[action.key];
      result = ok("binding-forgotten");
      break;
    case "queue-reply": {
      // Trusted canonical event, fixed allowlisted bytes; no client payload API.
      if (!["Q1", "Q2"].includes(action.query)) break;
      const id = `reply:${action.query}`;
      if (s.operations[id]) {
        result = ok(id);
        break;
      }
      s.operations[id] = {
        id,
        reply: true,
        origin: action.query,
        bytes: "r",
        deadline: 5,
        epoch: s.epoch,
        guard: s.session.guard,
        phase: "queued",
        delivery: "pending",
        known: 0,
        calls: 0,
        flight: null,
        retained: true,
      };
      result = ok(id);
      break;
    }
    case "authorize-write": {
      const op = s.operations[action.operation];
      if (!op || !["queued", "suffix"].includes(op.phase)) {
        result = no("not-dispatchable");
        break;
      }
      if (s.active && s.active !== op.id) {
        result = no("queue-busy");
        break;
      }
      if (s.session.tainted) {
        result = no("continuation-required");
        break;
      }
      if (!op.retained || !eligible(s, op) || op.calls >= BOUNDS.calls) {
        close(s, op);
        result = no("fenced-or-bounded");
        break;
      }
      if (op.bytes.length === 0) {
        close(s, op);
        result = ok("empty-zero");
        break;
      }
      op.calls++;
      op.flight = {
        id: `${op.id}:${op.calls}`,
        start: op.known,
        length: op.bytes.length - op.known,
      };
      op.phase = "in-flight";
      s.active = op.id;
      result = ok(copy(op.flight));
      event.authorized = { operation: op.id, ...copy(op.flight) };
      break;
    }
    case "lose-ack": {
      const op = s.operations[action.operation];
      if (!op?.flight) break;
      op.phase = "closing";
      op.delivery = "unknown";
      result = ok("reconcile-only");
      break;
    }
    case "write-result": {
      const op = s.operations[action.operation];
      if (!op?.flight || action.call !== op.flight.id) {
        result = no("no-matching-call");
        break;
      }
      if (
        action.count !== "unknown" &&
        (!Number.isInteger(action.count) ||
          action.count < 0 ||
          action.count > op.flight.length)
      )
        break;
      const closed = op.phase === "closing";
      if (action.count === "unknown") {
        op.flight = null;
        close(s, op, true);
        result = ok("unknown-call-ceased");
        break;
      }
      op.known += action.count;
      op.flight = null;
      if (closed || action.terminal || op.known === op.bytes.length)
        close(s, op);
      else {
        op.phase = "suffix";
        op.delivery = "pending";
      }
      result = ok("result-recorded");
      break;
    }
    case "continue-after-abandon":
      if (
        s.active ||
        !s.session.known ||
        action.fresh !== true ||
        action.toleratesRisk !== true
      ) {
        result = no("unsafe-continuation");
        break;
      }
      if (!advance(s.session, "guard")) {
        result = no("exhausted");
        break;
      }
      s.session.tainted = false;
      fence(s); // Fresh continuation cannot revive a queued old-context tail.
      result = ok("explicit-continuation");
      break;
  }
  return { state: s, event, result };
}

export function recoveryAction(receipt) {
  return !receipt?.ok || receipt.value?.delivery === "unknown"
    ? "query"
    : "inspect";
}
