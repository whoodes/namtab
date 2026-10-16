#!/usr/bin/env python3
"""Real Linux MK1 journeys. No compiler, mock broker or terminal-state oracle.

All commands and captured streams are synthetic and local. Input is sent once;
only observations are polled. Resume capabilities remain in RAM and are never
included in the result. This headless PTY run is not a desktop/visual claim.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import socket
import struct
import subprocess
import sys
import termios
import threading
import time
import traceback
import uuid


VERSION = "namtab.mk1/1"
FRAME_CAP = 1_048_576
OUTPUT_CAP = 2_097_152
TIMEOUT = 8.0


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1_048_576), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_metadata(path):
    def command(*args):
        result = subprocess.run(["git", "-C", str(path), *args], capture_output=True, text=True, check=True)
        return result.stdout.strip()
    return {"head": command("rev-parse", "HEAD"), "dirty_paths": command("status", "--short", "--untracked-files=no").splitlines()}


class Connection:
    """One endpoint, ordered requests, explicit close/resume, no input retries."""

    def __init__(self, path, label, role, resume=None, next_id=1):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(4.0)
        self.socket.connect(str(path))
        self.reader = self.socket.makefile("rb")
        self.next_id = next_id
        self.epoch = None
        response = self.call({"op": "hello", "label": label, "role": role, "resume": resume})
        require(response["ok"], "hello/resume refused: " + str(response.get("error")))
        self.endpoint = response["endpoint"]
        self.resume = response["resume"]  # Deliberately RAM-only, never logged.
        self.epoch = response["epoch"]

    def call(self, operation, request_id=None):
        if request_id is None:
            request_id = self.next_id
            self.next_id += 1
        body = json.dumps({"version": VERSION, "id": request_id, "operation": operation}, separators=(",", ":")).encode() + b"\n"
        require(len(body) <= FRAME_CAP, "test request exceeded admitted frame cap")
        self.socket.sendall(body)
        line = self.reader.readline(FRAME_CAP + 1)
        require(line and len(line) <= FRAME_CAP and line.endswith(b"\n"), "closed/oversized broker response")
        response = json.loads(line)
        require(response["id"] == request_id, "response correlation mismatch")
        require(self.epoch is None or response["epoch"] == self.epoch, "broker epoch changed; no replay allowed")
        return response

    def observe(self):
        response = self.call({"op": "observe"})
        require(response["ok"], "observation refused")
        return response["snapshot"]

    def close(self):
        try:
            self.socket.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        self.reader.close()
        self.socket.close()


class Tui:
    def __init__(self, command, environment, output_path):
        self.master, slave = pty.openpty()
        import fcntl
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 38, 110, 0, 0))
        # A fresh exec child gets its own session. The frontend uses the slave
        # descriptors directly; broker-owned application PTYs are separate.
        self.process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=environment, start_new_session=True, close_fds=True)
        os.close(slave)
        self.output_path = output_path
        self.output = bytearray()
        self.truncated = False
        self.closed = False
        self.lock = threading.Lock()
        self.thread = threading.Thread(target=self.drain, daemon=True)
        self.thread.start()

    def drain(self):
        with self.output_path.open("xb") as log:
            while not self.closed:
                try:
                    if not select.select([self.master], [], [], 0.1)[0]:
                        continue
                    data = os.read(self.master, 8192)
                    if not data:
                        return
                except OSError:
                    return
                with self.lock:
                    available = max(0, OUTPUT_CAP - len(self.output))
                    accepted = data[:available]
                    self.output.extend(accepted)
                    self.truncated |= len(accepted) != len(data)
                log.write(accepted)

    def send(self, data):
        require(self.process.poll() is None, "TUI exited before requested input")
        require(os.write(self.master, data) == len(data), "partial host-PTY write; input not retried")

    def prefix(self, key):
        self.send(b"\x07" + key)

    def text_after(self, offset=0):
        with self.lock:
            return bytes(self.output[offset:])

    def mark(self):
        with self.lock:
            return len(self.output)

    def stop(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=2)
        self.closed = True
        self.thread.join(timeout=1)
        os.close(self.master)


class Journey:
    def __init__(self, args):
        self.root = Path(subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], text=True).strip()).parent
        self.bin_dir = args.bin_dir.resolve(strict=True)
        self.terminfo = args.terminfo.resolve(strict=True)
        self.artifacts = args.artifacts.resolve()
        require(self.artifacts.is_relative_to(self.root / ".artifacts"), "artifacts must be inside this repository's ignored .artifacts")
        self.artifacts.mkdir(parents=True, exist_ok=False)
        (self.artifacts / "harness.py").write_bytes(Path(__file__).read_bytes())
        self.runtime = self.root / ".artifacts" / ("mk1-s-" + uuid.uuid4().hex[:10])
        self.runtime.mkdir(mode=0o700)
        self.workspace = self.artifacts / "workspace"
        self.workspace.mkdir(mode=0o700)
        self.socket_path = self.runtime / "s"
        require(len(os.fsencode(self.socket_path)) < 107, "project-local socket path exceeds Linux bound")
        self.process = None
        self.tui = None
        self.observer = None
        self.agent = None
        self.owned_pidfds = {}
        self.results = []
        self.record = {
            "contract": VERSION,
            "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "source": git_metadata(Path.cwd()),
            "harness_sha256": sha256(Path(__file__)),
            "invocation": sys.argv,
            "environment": {
                "python": sys.version,
                "os": os.uname().sysname,
                "kernel": os.uname().release,
                "architecture": os.uname().machine,
            },
            "binary_source_observation": git_metadata(self.bin_dir),
            "source_binding_limit": "Hashes identify exercised binaries; Git metadata alone is not an attestation of compiler inputs.",
            "caps": {
                "sessions": 2, "managed_attempts": 2, "endpoint_identities_used_max": 4,
                "public_frame": FRAME_CAP, "tui_log_bytes": OUTPUT_CAP,
                "scenario_timeout_seconds": TIMEOUT,
            },
            "limitations": [
                "Headless real TUI/PTY journey; not actual Electron GUI, visual fidelity, accessibility or human-feel acceptance",
                "Finite MK1 profile, not NG1/production compatibility or performance qualification",
                "Capture-off broker; synthetic harness logs are explicit test capture",
                "No descendant containment, restart recovery or deterministic execution replay claim",
            ],
            "human_commands": [],
            "checks": self.results,
        }

    def wait(self, predicate, description, timeout=TIMEOUT):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            value = predicate()
            if value:
                return value
            time.sleep(0.025)
        raise AssertionError("timed out: " + description)

    def snapshot(self):
        return self.observer.observe()

    def session(self, session_id):
        return next(s for s in self.snapshot()["sessions"] if s["id"] == session_id)

    def wait_snapshot(self, predicate, description):
        def observed():
            snapshot = self.snapshot()
            return snapshot if predicate(snapshot) else None
        return self.wait(observed, description)

    def wait_session(self, session_id, predicate, description):
        def observed():
            session = self.session(session_id)
            return session if predicate(session) else None
        return self.wait(observed, description)

    def mutate(self, client, operation):
        # Explicit fresh observation is part of every new mutation journey.
        client.observe()
        return client.call(operation)

    def passed(self, name, **facts):
        self.results.append({"name": name, "status": "pass", **facts})

    def descendants(self):
        """Retain pidfds only for verified descendants of our still-owned daemon."""
        if self.process is None or self.process.poll() is not None:
            return
        pending = [self.process.pid]
        visited = set()
        while pending:
            parent = pending.pop()
            if parent in visited:
                continue
            visited.add(parent)
            try:
                tasks = list(Path(f"/proc/{parent}/task").iterdir())
            except FileNotFoundError:
                continue
            values = set()
            # Linux lists children under the spawning thread, not necessarily
            # the main thread. The broker launches processes from RPC threads.
            for task in tasks:
                try:
                    values.update((task / "children").read_text().split())
                except FileNotFoundError:
                    continue
            for value in values:
                pid = int(value)
                if pid in self.owned_pidfds:
                    previous = self.owned_pidfds[pid]
                    if not select.select([previous], [], [], 0)[0]:
                        pending.append(pid)
                        continue
                descriptor = None
                try:
                    descriptor = os.pidfd_open(pid)
                    stat = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
                    if int(stat[1]) != parent:
                        os.close(descriptor)
                        continue
                    previous = self.owned_pidfds.get(pid)
                    if previous is not None:
                        os.close(previous)
                    self.owned_pidfds[pid] = descriptor
                    pending.append(pid)
                except (FileNotFoundError, ProcessLookupError):
                    if descriptor is not None:
                        os.close(descriptor)
                    continue

    def start(self):
        require(hasattr(os, "pidfd_open") and hasattr(signal, "pidfd_send_signal"), "Linux pidfd cleanup APIs required; do not fall back to arbitrary PID signals")
        names = ["namtabd", "namtab-terminal-worker", "namtab-pty-helper", "namtab-tui", "namtab-agent"]
        self.record["binaries"] = {name: {"path": str(self.bin_dir / name), "sha256": sha256(self.bin_dir / name)} for name in names}
        command = [str(self.bin_dir / "namtabd"), "--socket", str(self.socket_path), "--workspace", str(self.workspace), "--worker", str(self.bin_dir / "namtab-terminal-worker"), "--pty-helper", str(self.bin_dir / "namtab-pty-helper"), "--terminfo", str(self.terminfo)]
        self.record["daemon_command"] = command
        environment = {"PATH": "/usr/bin:/bin", "HOME": str(self.workspace), "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TERM": "xterm-256color"}
        self.daemon_log = (self.artifacts / "daemon.log").open("xb")
        self.process = subprocess.Popen(command, stdout=self.daemon_log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, env=environment, start_new_session=True, close_fds=True)
        self.wait(lambda: self.socket_path.exists() or self.process.poll() is not None, "daemon socket")
        require(self.process.poll() is None, "daemon startup failed; inspect daemon.log")
        self.observer = Connection(self.socket_path, "e2e-observer-reviewer", "human")
        self.agent = Connection(self.socket_path, "e2e-agent", "agent")
        tui_command = [str(self.bin_dir / "namtab-tui"), "--socket", str(self.socket_path)]
        self.record["tui_command"] = tui_command
        self.tui = Tui(tui_command, environment, self.artifacts / "tui.ansi")
        snapshot = self.wait_snapshot(lambda s: len(s["sessions"]) == 1 and s["sessions"][0]["owner"], "real TUI creates and owns first shell")
        self.first = snapshot["sessions"][0]["id"]
        self.tui_endpoint = snapshot["sessions"][0]["owner"]
        self.wait_session(self.first, lambda s: any("mk1$" in line for line in s["screen"]["lines"]), "clean Bash prompt")
        self.wait(lambda: b"mk1$" in self.tui.text_after(), "real frontend paints the prompt before typing")
        self.descendants()
        require(self.runtime.stat().st_mode & 0o777 == 0o700, "runtime directory mode")
        require(self.socket_path.stat().st_mode & 0o777 == 0o600, "socket mode")
        self.passed("real_startup", session=self.first, owner=self.tui_endpoint, source="real broker + worker + controlling PTY + TUI process")

    def type_command(self, session_id, command, marker, wait_prompt=True):
        # Feed ordinary keyboard events exactly once. Waiting for canonical echo
        # avoids hiding dropped human keys with retries or timing assumptions.
        typed = ""
        for character in command:
            self.tui.send(character.encode("ascii"))
            typed += character
            def echoed(session):
                screen = session["screen"]
                current = screen["lines"][screen["cursor_row"]][:screen["cursor_col"]]
                return current == "mk1$ " + typed
            self.wait_session(session_id, echoed, "canonical current-cursor echo of synthetic human prefix " + repr(typed))
        paint_mark = self.tui.mark()
        self.tui.send(b"\r")
        self.wait_session(session_id, lambda s: marker in [line.strip() for line in s["screen"]["lines"]], "real command output " + marker)
        if wait_prompt:
            self.wait_session(session_id, self.at_prompt, "canonical prompt after real command")
            self.wait(lambda: b"mk1$" in self.tui.text_after(paint_mark), "frontend paints fresh prompt before next command")
        else:
            self.wait(lambda: marker.encode() in self.tui.text_after(paint_mark), "frontend paints synthetic foreground output")
        self.record["human_commands"].append({"session": session_id, "command": command, "actual_output_marker": marker})

    @staticmethod
    def at_prompt(session):
        screen = session["screen"]
        return screen["lines"][screen["cursor_row"]][:screen["cursor_col"]] == "mk1$ "

    def shells_and_takeover(self):
        self.type_command(self.first, "printf x>>tui-count;echo MK1_TUI_ONE", "MK1_TUI_ONE")
        require((self.workspace / "tui-count").read_bytes() == b"x", "first real command side effect")
        self.tui.prefix(b"n")
        snapshot = self.wait_snapshot(lambda s: len(s["sessions"]) == 2, "second real PTY")
        self.second = next(s["id"] for s in snapshot["sessions"] if s["id"] != self.first)
        self.tui.prefix(b"\t")
        time.sleep(0.1)
        self.tui.prefix(b"t")
        self.wait_session(self.second, lambda s: s["owner"] == self.tui_endpoint, "TUI takes selected second session")
        self.type_command(self.second, "echo MK1_TUI_TWO", "MK1_TUI_TWO")
        require(not any(line.strip() == "MK1_TUI_TWO" for line in self.session(self.first)["screen"]["lines"]), "session input leaked to wrong canonical session")
        self.type_command(self.second, "echo MK1_SLEEPING;sleep 5", "MK1_SLEEPING", wait_prompt=False)
        paint_mark = self.tui.mark()
        self.tui.send(b"\x03")
        self.wait_session(self.second, self.at_prompt, "shell returns after synthetic foreground Ctrl-C")
        self.wait(lambda: b"mk1$" in self.tui.text_after(paint_mark), "frontend paints post-interrupt prompt")
        self.type_command(self.second, "echo MK1_AFTER_INTERRUPT", "MK1_AFTER_INTERRUPT")
        self.tui.prefix(b"\t")
        time.sleep(0.1)
        self.tui.prefix(b"l")
        current = self.wait_session(self.first, lambda s: s["owner"] is None, "human releases selected first writer")
        taken = self.mutate(self.agent, {"op": "take_control", "session": self.first, "generation": current["generation"], "force": False})
        require(taken["ok"], "agent take refused")
        current = self.session(self.first)
        old_generation = current["generation"]
        self.agent_input(current, {"kind": "text", "text": "echo MK1_AGENT"})
        self.wait_session(self.first, lambda s: any("mk1$ echo MK1_AGENT" in line for line in s["screen"]["lines"]), "agent text echo")
        self.agent_input(self.session(self.first), {"kind": "key", "key": "Enter", "ctrl": False, "alt": False, "shift": False})
        self.wait_session(self.first, lambda s: "MK1_AGENT" in [line.strip() for line in s["screen"]["lines"]], "actual agent-entered command output")
        old_endpoint, resume, next_id = self.agent.endpoint, self.agent.resume, self.agent.next_id
        self.agent.close()
        time.sleep(0.1)
        self.agent = Connection(self.socket_path, "e2e-agent", "agent", resume=resume, next_id=next_id)
        require(self.agent.endpoint == old_endpoint, "live resume created a different endpoint")
        current = self.session(self.first)
        require(current["generation"] == old_generation and current["owner"] == old_endpoint, "live endpoint resume changed writer authority")
        self.tui.prefix(b"t")
        current = self.wait_session(self.first, lambda s: s["owner"] == self.tui_endpoint and s["generation"] > old_generation, "human forced takeover from live agent")
        refused = self.mutate(self.agent, {"op": "input", "session": self.first, "generation": old_generation, "revision": current["screen"]["revision"], "mode_revision": current["screen"]["mode_revision"], "policy": "observed", "intent": {"kind": "text", "text": "echo SHOULD_NOT_EXECUTE"}})
        require(not refused["ok"] and refused["error"] == "writer_or_generation_stale", "old writer generation was not refused")
        self.descendants()
        self.passed("human_typing_session_switch_agent_resume_takeover", first=self.first, second=self.second, old_generation=old_generation, new_generation=current["generation"], refusal=refused["error"], ctrl_c="synthetic foreground sleep interrupted; shell remained usable, not a universal stop guarantee", agent_status="declared connected endpoint; command output observed, no model inference")

    def agent_input(self, current, intent):
        # Observe immediately before constructing each new application mutation.
        snapshot = self.agent.observe()
        current = next(s for s in snapshot["sessions"] if s["id"] == current["id"])
        response = self.agent.call({"op": "input", "session": current["id"], "generation": current["generation"], "revision": current["screen"]["revision"], "mode_revision": current["screen"]["mode_revision"], "policy": "observed", "intent": intent})
        require(response["ok"] and response["receipt"]["state"] == "applied", "agent input refused; never automatically retried: " + str(response.get("error")))

    def managed_and_receipts(self):
        operation = {"op": "run", "task": "literal-argv", "argv": ["/usr/bin/printf", "%s", "literal;$(touch SHOULD_NOT_EXIST)"]}
        self.agent.observe()
        request_id = self.agent.next_id
        response = self.agent.call(operation)
        require(response["ok"], "literal managed launch refused")
        duplicate = self.agent.call(operation, request_id=request_id)
        require(duplicate["receipt"] == response["receipt"], "known duplicate did not retain exact receipt")
        conflict = self.agent.call({"op": "run", "task": "conflicting", "argv": ["/bin/false"]}, request_id=request_id)
        require(not conflict["ok"] and conflict["error"] == "mutation_id_conflict", "changed body with same mutation ID not refused")
        snapshot = self.wait_snapshot(lambda s: len(s["attempts"]) == 1 and s["attempts"][0]["state"] == "exited", "literal attempt exits and drains")
        require(snapshot["attempts"][0]["stdout"] == operation["argv"][2] and snapshot["attempts"][0]["stderr"] == "", "literal argv attribution changed")
        require(not (self.workspace / "SHOULD_NOT_EXIST").exists(), "literal argv passed through unintended shell")
        receipt = self.agent.call({"op": "receipt", "request_id": request_id})
        require(receipt["receipt"] == response["receipt"], "retained receipt query mismatch")
        second = self.mutate(self.agent, {"op": "run", "task": "separate-streams", "argv": ["/bin/sh", "-c", "printf native-out;printf native-err >&2;(sleep 0.6;printf native-tail)&exit 7"]})
        require(second["ok"], "second managed launch refused")
        self.descendants()
        self.wait(lambda: any(a["state"] == "draining" for a in self.snapshot()["attempts"]), "exit-before-pipe-EOF draining state")
        snapshot = self.wait_snapshot(lambda s: len(s["attempts"]) == 2 and all(a["state"] == "exited" for a in s["attempts"]), "both managed attempts finish and drain")
        attempt = next(a for a in snapshot["attempts"] if a["task"] == "separate-streams")
        require((attempt["stdout"], attempt["stderr"], attempt["exit_code"]) == ("native-outnative-tail", "native-err", 7), "managed streams/exit lost attribution")
        capacity = self.mutate(self.agent, {"op": "run", "task": "over-cap", "argv": ["/bin/true"]})
        require(not capacity["ok"] and capacity["error"] == "attempt_capacity_two_total", "two-attempt lifetime cap not enforced")
        self.tui.prefix(b"b")
        self.wait(lambda: b"Task review" in self.tui.text_after() and b"separate-streams" in self.tui.text_after(), "actual task review rendering")
        before = [s["screen"]["lines"] for s in self.snapshot()["sessions"]]
        mark = self.tui.mark()
        self.tui.send(b"z")
        self.wait(lambda: self.tui.mark() > mark, "TUI redraw after review-focus key")
        require([s["screen"]["lines"] for s in self.snapshot()["sessions"]] == before, "review key leaked into an application PTY")
        self.tui.prefix(b"b")
        self.passed("literal_argv_streams_exit_drain_duplicate_conflict_cap", attempts=2, duplicate_request_id=request_id, conflict=conflict["error"], capacity=capacity["error"], stdout=attempt["stdout"], stderr=attempt["stderr"], exit_code=7)

    def artifacts_and_cli(self):
        artifact_path = self.workspace / "review.txt"
        artifact_path.write_text("synthetic revision one\n")
        registered = self.mutate(self.agent, {"op": "artifact", "task": "explicit-review", "path": "review.txt"})
        require(registered["ok"], "explicit artifact registration refused")
        artifact = self.snapshot()["artifacts"][0]
        require(artifact["revision"] == "sha256:" + sha256(artifact_path), "artifact revision not actual file digest")
        denied = self.mutate(self.agent, {"op": "validate_artifact", "artifact": artifact["id"], "revision": artifact["revision"]})
        require(not denied["ok"], "agent incorrectly performed manual human review")
        reviewed = self.mutate(self.observer, {"op": "validate_artifact", "artifact": artifact["id"], "revision": artifact["revision"]})
        require(reviewed["ok"], "human revision-bound review refused")
        current = self.snapshot()["artifacts"][0]
        require(current["validated_revision"] == artifact["revision"] and not current["stale"], "manual review not bound to observed revision")
        artifact_path.write_text("synthetic revision two\n")
        changed = self.wait_snapshot(lambda s: s["artifacts"][0]["stale"], "artifact validation becomes stale")
        stale = changed["artifacts"][0]
        require(stale["revision"] != artifact["revision"], "changed artifact digest remained old")
        # Fourth and final endpoint identity: exercise the actual CLI once.
        command = [str(self.bin_dir / "namtab-agent"), "--socket", str(self.socket_path), "observe"]
        result = subprocess.run(command, capture_output=True, timeout=5, check=True)
        response = json.loads(result.stdout)
        require(response["ok"] and len(response["snapshot"]["sessions"]) == 2, "actual agent CLI did not observe shared state")
        require(response.get("resume") is None, "CLI unexpectedly printed resume capability")
        (self.artifacts / "agent-observe.json").write_text(json.dumps(response, indent=2) + "\n")
        self.passed("artifact_revision_manual_review_staleness_and_real_agent_cli", old_revision=artifact["revision"], new_revision=stale["revision"], agent_review_refusal=denied["error"], cli_command=command)

    def live_tui_reconnect(self):
        before = self.session(self.first)
        mark = self.tui.mark()
        os.kill(self.process.pid, signal.SIGSTOP)  # Only our still-owned child.
        try:
            self.wait(lambda: b"DISCONNECTED" in self.tui.text_after(mark) or b"LAST OBSERVED" in self.tui.text_after(mark), "actual TUI detects blocked live broker", timeout=6)
        finally:
            os.kill(self.process.pid, signal.SIGCONT)
        # The controlled process stop can interrupt other timed socket reads as
        # well. Explicitly resume the same read-only observer identity rather
        # than creating extra endpoints or retrying any application mutation.
        endpoint = self.observer.endpoint
        resume, next_id = self.observer.resume, self.observer.next_id
        self.observer.close()
        time.sleep(0.15)
        self.observer = Connection(self.socket_path, "e2e-observer-reviewer", "human", resume=resume, next_id=next_id)
        require(self.observer.endpoint == endpoint, "controlled fault changed persistent observer identity")
        self.tui.prefix(b"c")
        # Ratatui emits differential ANSI; searching for a whole plain-text
        # notice is not a valid screen oracle. A new actual command is stronger
        # proof that this client resumed, rather than only repainting old state.
        after = self.session(self.first)
        require(after["owner"] == before["owner"] and after["generation"] == before["generation"], "frontend reconnect altered writer identity/generation")
        require((self.workspace / "tui-count").read_bytes() == b"x", "old human command was replayed")
        self.type_command(self.first, "echo MK1_RESUMED", "MK1_RESUMED")
        self.passed("actual_tui_disconnect_explicit_live_resume_no_replay", fault="SIGSTOP/SIGCONT of own live broker", generation=after["generation"], input_replays_observed=0)

    def finish_shells(self):
        self.tui.send(b"exit\r")
        self.wait_session(self.first, lambda s: s["process"] == "exited", "first shell exits and PTY drains")
        self.tui.prefix(b"\t")
        time.sleep(0.1)
        self.tui.send(b"exit\r")
        self.wait_session(self.second, lambda s: s["process"] == "exited", "second shell exits and PTY drains")
        self.tui.prefix(b"q")
        require(self.tui.process.wait(timeout=4) == 0, "normal TUI detach failed")
        require(not self.tui.truncated, "TUI capture exceeded finite cap")
        journal = Path(str(self.socket_path) + ".journal")
        require(journal.stat().st_mode & 0o777 == 0o600, "journal mode")
        journal_bytes = journal.read_bytes()
        require(b"MK1_TUI_ONE" not in journal_bytes and b"literal;$(touch" not in journal_bytes, "capture-off metadata journal persisted synthetic application content")
        for name, metadata in self.record["binaries"].items():
            require(sha256(self.bin_dir / name) == metadata["sha256"], "binary changed during journey: " + name)
        self.passed("real_shell_exit_drain_and_tui_detach", tui_exit=0)

    def cleanup(self):
        # Preserve all synthetic evidence. Clean up only handles whose ancestry
        # was verified while our daemon was live; pidfds cannot retarget reuse.
        if self.process and self.process.poll() is None:
            try:
                os.kill(self.process.pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
            self.descendants()
        for connection in [self.agent, self.observer]:
            if connection:
                connection.close()
        if self.tui:
            self.tui.stop()
        for descriptor in self.owned_pidfds.values():
            try:
                signal.pidfd_send_signal(descriptor, signal.SIGTERM)
            except ProcessLookupError:
                pass
        descriptors = list(self.owned_pidfds.values())
        terminated = select.select(descriptors, [], [], 1)[0] if descriptors else []
        remaining = [descriptor for descriptor in descriptors if descriptor not in terminated]
        for descriptor in remaining:
            try:
                signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            except ProcessLookupError:
                pass
        killed = select.select(remaining, [], [], 1)[0] if remaining else []
        unexited = len(remaining) - len(killed)
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=2)
        for descriptor in descriptors:
            os.close(descriptor)
        if hasattr(self, "daemon_log"):
            self.daemon_log.close()
        self.record["cleanup"] = {
            "direct_popen_children_waited": True,
            "verified_descendant_pidfds": len(descriptors),
            "unexited_verified_descendants": unexited,
            "limit": "The harness cannot reap another process's children or prove discovery of every descendant.",
        }
        require(unexited == 0, "verified owned descendant did not exit within bounded cleanup")

    def run(self):
        status = "fail"
        try:
            self.start()
            self.shells_and_takeover()
            self.managed_and_receipts()
            self.artifacts_and_cli()
            self.live_tui_reconnect()
            self.finish_shells()
            status = "pass"
        except Exception as error:
            # Errors contain stage descriptions, never connection/resume objects.
            self.record["failure"] = {"type": type(error).__name__, "message": str(error)}
            (self.artifacts / "failure.txt").write_text(traceback.format_exc())
        finally:
            try:
                self.cleanup()
            except Exception as error:
                status = "fail"
                self.record["cleanup_failure"] = {"type": type(error).__name__, "message": str(error)}
                (self.artifacts / "cleanup-failure.txt").write_text(traceback.format_exc())
            self.record["status"] = status
            self.record["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
            self.record["runtime_directory"] = str(self.runtime)
            self.record["evidence"] = {p.name: {"bytes": p.stat().st_size, "sha256": sha256(p)} for p in self.artifacts.iterdir() if p.is_file()}
            (self.artifacts / "checks.json").write_text(json.dumps(self.record, indent=2) + "\n")
        print(json.dumps({"status": status, "checks": len(self.results), "evidence": str(self.artifacts / "checks.json"), "failure": self.record.get("failure")}))
        return 0 if status == "pass" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True, help="absolute directory containing the five prebuilt MK1 binaries")
    parser.add_argument("--terminfo", type=Path, required=True, help="project-local compiled namtab-mk1 terminfo")
    parser.add_argument("--artifacts", type=Path, required=True, help="fresh ignored .artifacts result directory (must not already exist)")
    return Journey(parser.parse_args()).run()


if __name__ == "__main__":
    sys.exit(main())
