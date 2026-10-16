"use strict";
const net = require("node:net");
const { EventEmitter } = require("node:events");
const {
  MAX_FRAME,
  MAX_QUEUE,
  validateResponse,
  requestEnvelope,
} = require("./shared.js");

class Connection extends EventEmitter {
  constructor(socketPath) {
    super();
    this.socketPath = socketPath;
    this.nextId = 1;
    this.resume = null;
    this.endpoint = null;
    this.epoch = null;
    this.queue = [];
    this.pending = null;
    this.socket = null;
    this.ready = false;
    this.buffer = Buffer.alloc(0);
  }
  connect(fresh = false) {
    if (this.socket) return;
    if (fresh) {
      this.resume = null;
      this.endpoint = null;
      this.epoch = null;
      this.nextId = 1;
    }
    this.buffer = Buffer.alloc(0);
    this.emit("status", "Connecting…");
    const socket = net.createConnection({ path: this.socketPath });
    this.socket = socket;
    socket.once("connect", () => {
      this.request(
        {
          op: "hello",
          label: "desktop human",
          role: "human",
          resume: this.resume,
        },
        null,
        true,
      )
        .then((response) => {
          if (!response.ok || !response.endpoint || !response.resume) {
            this.disconnect(
              response.error ||
                "Hello refused; choose explicit reconnect or new endpoint.",
            );
            return;
          }
          if (this.epoch && response.epoch !== this.epoch) {
            this.disconnect(
              "Daemon epoch changed. Prior owner/input disposition is unknown; explicitly start a new endpoint.",
            );
            return;
          }
          this.resume = response.resume;
          this.endpoint = response.endpoint;
          this.epoch = response.epoch;
          this.ready = true;
          this.emit("status", "Connected");
          this.emit("ready");
        })
        .catch(() => {});
    });
    socket.on("data", (data) => this.receive(data));
    socket.on("error", (error) =>
      this.disconnect(
        `Disconnected: ${error.code || "socket error"}. Input frozen; mutations are never retried.`,
      ),
    );
    socket.on("close", () => {
      if (this.socket === socket)
        this.disconnect(
          "Disconnected. Input frozen; mutation disposition may be unknown. Reconnect explicitly.",
        );
    });
  }
  request(fields, guard = null, hello = false) {
    if (!this.socket || (!this.ready && !hello))
      return Promise.reject(new Error("Not connected"));
    if (this.queue.length + (this.pending ? 1 : 0) >= MAX_QUEUE)
      return Promise.reject(
        new Error(
          "Input queue full (64); nothing added. Wait and observe before continuing.",
        ),
      );
    return new Promise((resolve, reject) => {
      this.queue.push({ fields, guard, resolve, reject });
      this.pump();
    });
  }
  pump() {
    if (!this.socket || this.pending || this.queue.length === 0) return;
    const item = this.queue.shift();
    if (item.guard && !item.guard()) {
      item.reject(
        new Error(
          "Queued operation refused: focus, selection, ownership or mode changed.",
        ),
      );
      this.pump();
      return;
    }
    if (
      !Number.isSafeInteger(this.nextId) ||
      this.nextId > Number.MAX_SAFE_INTEGER
    ) {
      item.reject(
        new Error("Request ID namespace exhausted; new endpoint required."),
      );
      return;
    }
    const id = this.nextId++;
    const frame = Buffer.from(
      JSON.stringify(requestEnvelope(id, item.fields)) + "\n",
    );
    if (frame.length > MAX_FRAME) {
      item.reject(new Error("Request exceeds 1 MiB frame limit"));
      this.pump();
      return;
    }
    this.pending = {
      ...item,
      id,
      timer: setTimeout(
        () =>
          this.disconnect(
            "Response timeout. Input frozen; delivery may be unknown. No mutation was retried.",
          ),
        10000,
      ),
    };
    this.socket.write(frame, (error) => {
      if (error)
        this.disconnect(
          "Socket write failed; delivery may be partial or unknown. No mutation was retried.",
        );
    });
  }
  receive(data) {
    this.buffer = Buffer.concat([this.buffer, data]);
    while (this.socket) {
      const newline = this.buffer.indexOf(10);
      if (newline < 0) {
        if (this.buffer.length >= MAX_FRAME)
          this.disconnect("Response frame exceeds 1 MiB; input frozen.");
        return;
      }
      if (newline + 1 > MAX_FRAME) {
        this.disconnect("Response frame exceeds 1 MiB; input frozen.");
        return;
      }
      const frame = this.buffer.subarray(0, newline);
      this.buffer = this.buffer.subarray(newline + 1);
      let response;
      try {
        response = JSON.parse(
          new TextDecoder("utf-8", { fatal: true }).decode(frame),
        );
      } catch {
        this.disconnect("Invalid response encoding or JSON; input frozen.");
        return;
      }
      if (!this.pending || !validateResponse(response, this.pending.id)) {
        this.disconnect("Malformed or unsolicited response; input frozen.");
        return;
      }
      if (this.epoch && response.epoch !== this.epoch) {
        this.disconnect(
          "Daemon epoch changed. Old mutations were not retried; explicitly start a new endpoint.",
        );
        return;
      }
      const pending = this.pending;
      clearTimeout(pending.timer);
      this.pending = null;
      this.emit("response", response);
      pending.resolve(response);
      this.pump();
    }
  }
  clearQueuedInputs() {
    this.queue = this.queue.filter((item) => {
      if (
        !["input", "take_control", "release_control", "resize"].includes(
          item.fields.op,
        )
      )
        return true;
      item.reject(
        new Error(
          "Queued operation cleared after focus, selection or connection changed.",
        ),
      );
      return false;
    });
  }
  disconnect(reason = "Window closed") {
    const socket = this.socket;
    this.socket = null;
    this.ready = false;
    this.buffer = Buffer.alloc(0);
    if (this.pending) {
      clearTimeout(this.pending.timer);
      this.pending.reject(new Error(reason));
      this.pending = null;
    }
    for (const item of this.queue.splice(0)) item.reject(new Error(reason));
    if (socket) socket.destroy();
    this.emit("status", reason);
    this.emit("disconnected");
  }
}
module.exports = { Connection };
