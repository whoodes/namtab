#!/usr/bin/env python3
"""Real, bounded two-agent MK1 task/ownership journey and local RPC samples.

Run after building the five MK1 binaries and private terminfo:
  python3 tests/mk1/agents.py --bin-dir ABS --terminfo ABS --artifacts FRESH_ABS

The agents are scripted protocol clients, not models or a scheduler. The shared
task is declared intent; stdout/stderr attribution is direct managed-process
evidence, while file hashes do not prove which process caused a file. No input
is automatically retried. Measurements include local Python/JSON/UDS/broker and
receipt persistence costs, not inference, GUI rendering or application latency.
"""

import argparse
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import sys
import threading
import time
import traceback

import e2e
from e2e import Connection, Journey, require, sha256


SAMPLES = 30
NOISE_LINES = 64
NOISE_LINE_BYTES = 512
RUN_REQUEST_ID = 10_000  # Deliberately the same ID in two endpoint namespaces.


def percentile(values, quantile):
    """Nearest-rank empirical percentile; all raw nanosecond samples survive."""
    require(values, "no latency samples")
    return sorted(values)[max(0, math.ceil(len(values) * quantile) - 1)]


class Agents(Journey):
    def __init__(self, args):
        super().__init__(args)
        self.agent_b = None
        self.samples = []
        self.run_receipts = {}
        self.record.update({
            "journey": "two-scripted-agents-shared-task-and-human-takeover",
            "agent_harness": {"path": str(Path(__file__).resolve()), "sha256": sha256(Path(__file__))},
            "helper": {"path": str(Path(e2e.__file__).resolve()), "sha256": sha256(Path(e2e.__file__))},
            "clock": {
                "name": "perf_counter_ns",
                "implementation": time.get_clock_info("perf_counter").implementation,
                "monotonic": time.get_clock_info("perf_counter").monotonic,
                "adjustable": time.get_clock_info("perf_counter").adjustable,
                "resolution_seconds": time.get_clock_info("perf_counter").resolution,
            },
            "host_context": {
                "cpu_count": os.cpu_count(),
                "memory_bytes": os.sysconf("SC_PHYS_PAGES") * os.sysconf("SC_PAGE_SIZE"),
                "load_average_at_start": os.getloadavg(),
                "load_limit": "Shared VM load is recorded, not controlled; quiet refers only to the other PTY.",
            },
            "workload": {
                "rpc_samples_per_metric_per_phase": SAMPLES,
                "phases": ["quiet", "other-pty-finite-output"],
                "metrics": ["observe", "take_control_receipt", "input_end_key_receipt"],
                "input": "one logical End press at an empty Bash/readline prompt per sample; no command completion claim",
                "control": "current writer explicitly takes its same session, advancing generation; genuine A-to-B handoff is separately asserted",
                "noise": {"lines": NOISE_LINES, "bytes_per_line": NOISE_LINE_BYTES, "sleep_seconds_per_line": 0.04, "body_bytes": NOISE_LINES * NOISE_LINE_BYTES, "trailer": "MK1_NOISE_DONE\\n"},
                "managed": "two finite commands start concurrently, emit distinct streams/files and wait on independent local fixture gates for at most six seconds",
                "concurrency": "observer + real TUI + two scripted agent endpoints; one RPC at a time on each connection",
            },
            "measurement_limits": [
                "Host load is uncontrolled; quiet means the other PTY is idle, not an idle VM. Small empirical samples on this VM, including required receipt fsync; no achieved SLO, comparative speed or tail-confidence claim",
                "End-to-end local RPC response timing includes test-client overhead and live TUI polling; not model, OS input-to-photon, renderer or CPU timing",
                "No GUI, accessibility, broad compatibility, hostile same-UID isolation or production qualification",
                "Screen/stream/frame bounds are checked at public observations; no claim that every private queue was instrumented",
                "Task contract declaration and retained file hashes do not establish artifact correctness, automatic dependency scheduling or universal command causality",
            ],
        })
        (self.artifacts / "agents-harness.py").write_bytes(Path(__file__).read_bytes())
        monitor = Path(e2e.__file__).with_name("outer-screen.mjs")
        if monitor.exists():
            self.record["outer_monitor"] = {"path": str(monitor), "sha256": sha256(monitor), "role": "read-only OUTER Ratatui paint observer, never canonical application authority"}
            (self.artifacts / "outer-screen.mjs").write_bytes(monitor.read_bytes())

    def timed(self, client, operation, phase, metric):
        start = time.perf_counter_ns()
        response = client.call(operation)
        elapsed = time.perf_counter_ns() - start
        self.samples.append({"phase": phase, "metric": metric, "elapsed_ns": elapsed, "endpoint": client.endpoint, "request_id": response["id"], "ok": response["ok"], "receipt_state": (response.get("receipt") or {}).get("state")})
        return response

    @staticmethod
    def applied(response, description, result_kind=None):
        require(response["ok"], description + " refused: " + str(response.get("error")))
        receipt = response.get("receipt")
        require(receipt and receipt["state"] == "applied", description + " has no applied receipt")
        if result_kind:
            require((receipt.get("result") or {}).get("kind") == result_kind, description + " lacks typed result " + result_kind)
        return receipt

    @staticmethod
    def refused(response, expected, description):
        require(not response["ok"] and response.get("error") == expected, description + " wrong refusal: " + str(response.get("error")))
        receipt = response.get("receipt")
        if receipt:
            require(receipt["state"] == "rejected" and receipt["delivered_bytes"] == 0, description + " did not retain a zero-effect rejection")
        return receipt

    @staticmethod
    def from_snapshot(snapshot, session):
        return next(value for value in snapshot["sessions"] if value["id"] == session)

    def input(self, client, session, intent):
        current = self.from_snapshot(client.observe(), session)
        response = client.call({"op": "input", "session": session, "generation": current["generation"], "revision": current["screen"]["revision"], "mode_revision": current["screen"]["mode_revision"], "policy": "observed", "intent": intent})
        receipt = self.applied(response, "fresh single input; no retry")
        require(receipt["delivered_bytes"] > 0, "applied input delivered no bytes")
        return receipt

    def command(self, client, session, command):
        self.wait_session(session, self.at_prompt, "empty controlled prompt before command")
        self.input(client, session, {"kind": "text", "text": command})
        self.wait_session(session, lambda s: s["screen"]["lines"][s["screen"]["cursor_row"]][:s["screen"]["cursor_col"]] == "mk1$ " + command, "canonical echo of one complete command text intent")
        return self.input(client, session, {"kind": "key", "key": "Enter", "ctrl": False, "alt": False, "shift": False})

    def take(self, client, session, force=False):
        current = self.from_snapshot(client.observe(), session)
        response = client.call({"op": "take_control", "session": session, "generation": current["generation"], "force": force})
        self.applied(response, "explicit writer take")
        return self.wait_session(session, lambda s: s["owner"] == client.endpoint and s["generation"] > current["generation"], "writer generation transition")

    def release(self, client, session):
        current = self.from_snapshot(client.observe(), session)
        response = client.call({"op": "release_control", "session": session, "generation": current["generation"]})
        self.applied(response, "explicit writer release")
        return self.wait_session(session, lambda s: s["owner"] is None and s["generation"] > current["generation"], "released generation transition")

    def shared_task_and_concurrent_attempts(self):
        self.agent_b = Connection(self.socket_path, "mk1-agent-b-scripted", "agent")
        snapshot = self.snapshot()
        require(snapshot["degraded"] is None, "fresh broker reports degradation")
        require({"shared_tasks_v1", "attempt_origin_v1", "receipt_results_v1"}.issubset(snapshot.get("capabilities", [])), "new shared-contract capabilities absent")
        require(len(snapshot["endpoints"]) == 4 and all(e["connected"] for e in snapshot["endpoints"]), "exactly four expected endpoints are not connected")
        self.task = "shared-agent-evidence"
        goal = "Produce distinct evidence files and streams under one immutable declared task"
        paths = ["agent-a.txt", "agent-b.txt"]
        operation = {"op": "define_task", "task": self.task, "goal": goal, "expected_artifacts": paths}
        defined = self.mutate(self.agent, operation)
        self.task_revision = self.applied(defined, "shared task definition", "task_defined")["result"]["revision"]
        canonical = json.dumps(["shared_tasks_v1", self.task, goal, paths], separators=(",", ":"), ensure_ascii=False).encode()
        require(self.task_revision == "sha256:" + hashlib.sha256(canonical).hexdigest(), "shared contract revision differs from specified canonical tuple")
        repeated = self.mutate(self.agent_b, operation)
        require(self.applied(repeated, "second agent same definition", "task_defined")["result"]["revision"] == self.task_revision, "same task definition diverged between agents")
        conflict = self.mutate(self.agent_b, {**operation, "goal": "different goal"})
        self.refused(conflict, "task_contract_immutable_conflict", "immutable shared contract")
        stale = self.mutate(self.agent_b, {"op": "run", "task": self.task, "task_revision": "sha256:" + "0" * 64, "argv": ["/bin/sh", "-c", "printf bad > SHOULD_NOT_EXIST"]})
        require(self.refused(stale, "task_revision_required_or_stale", "stale task revision") is not None, "stale task rejection receipt absent")
        missing = self.mutate(self.agent, {"op": "run", "task": self.task, "argv": ["/bin/true"]})
        require(self.refused(missing, "task_revision_required_or_stale", "missing shared task revision") is not None, "missing revision rejection receipt absent")
        require(not self.snapshot()["attempts"] and not (self.workspace / "SHOULD_NOT_EXIST").exists(), "stale task launched an effect")
        self.run_operations = {}
        self.expected = {}
        for label in ["a", "b"]:
            stdout = f"AGENT_{label.upper()}_OUT\nAGENT_{label.upper()}_DONE\n"
            stderr = f"AGENT_{label.upper()}_ERR\n"
            artifact = f"exact independent artifact {label}\n".encode()
            script = f"printf 'AGENT_{label.upper()}_OUT\\n';printf 'AGENT_{label.upper()}_ERR\\n' >&2;printf 'exact independent artifact {label}\\n' > agent-{label}.txt;i=0;while [ ! -e release-{label} ] && [ \"$i\" -lt 120 ];do i=$((i+1));sleep .05;done;printf 'AGENT_{label.upper()}_DONE\\n';exit {3 if label == 'a' else 7}"
            self.run_operations[label] = {"op": "run", "task": self.task, "task_revision": self.task_revision, "argv": ["/bin/sh", "-c", script]}
            self.expected[label] = {"stdout": stdout, "stderr": stderr, "artifact": artifact, "exit_code": 3 if label == "a" else 7}
        barrier = threading.Barrier(3, timeout=5)
        errors = []

        def launch(label, client):
            try:
                client.next_id = RUN_REQUEST_ID + 1
                client.observe()
                barrier.wait()
                response = client.call(self.run_operations[label], request_id=RUN_REQUEST_ID)
                self.run_receipts[label] = self.applied(response, "concurrent attempt " + label, "attempt_started")
            except Exception as error:
                errors.append((label, type(error).__name__, str(error)))

        threads = [threading.Thread(target=launch, args=("a", self.agent)), threading.Thread(target=launch, args=("b", self.agent_b))]
        for thread in threads:
            thread.start()
        barrier.wait()
        for thread in threads:
            thread.join(timeout=6)
        require(not any(thread.is_alive() for thread in threads), "bounded concurrent launch did not finish")
        require(not errors, "concurrent launch failed: " + repr(errors))
        concurrent = self.wait_snapshot(lambda s: len(s["attempts"]) == 2 and all(a["state"] == "running" and a["stdout"] and a["stderr"] for a in s["attempts"]), "both real managed processes running in one authoritative observation")
        refs = []
        for label, client in [("a", self.agent), ("b", self.agent_b)]:
            receipt = self.run_receipts[label]
            attempt_id = receipt["result"]["attempt"]
            refs.append(attempt_id)
            attempt = next(a for a in concurrent["attempts"] if a["id"] == attempt_id)
            require(receipt["endpoint"] == client.endpoint and receipt["id"] == RUN_REQUEST_ID, "receipt endpoint/request association wrong")
            require(attempt["origin"] == {"endpoint": client.endpoint, "request_id": RUN_REQUEST_ID} and attempt["task_revision"] == self.task_revision and attempt["task"] == self.task, "typed receipt reference/direct attempt origin misattributed")
            duplicate = client.call(self.run_operations[label], request_id=RUN_REQUEST_ID)
            require(duplicate["receipt"] == receipt, "known duplicate returned a different effect/receipt")
            retained = client.call({"op": "receipt", "request_id": RUN_REQUEST_ID})
            require(retained["ok"] and retained["receipt"] == receipt, "retained typed receipt query lost endpoint-local effect association")
            changed = client.call({**self.run_operations[label], "argv": ["/bin/false"]}, request_id=RUN_REQUEST_ID)
            self.refused(changed, "mutation_id_conflict", "changed body in existing endpoint/request namespace")
        require(len(set(refs)) == 2 and len(self.snapshot()["attempts"]) == 2, "two endpoint-local same IDs aliased or duplicate launched more attempts")
        self.descendants()
        self.passed("two_real_agent_attempts_one_immutable_contract", task=self.task, revision=self.task_revision, attempt_refs=refs, endpoint_local_request_id=RUN_REQUEST_ID, concurrent_cut=[{"id": a["id"], "state": a["state"], "origin": a["origin"]} for a in concurrent["attempts"]], duplicate="retained exact receipt", conflict="mutation_id_conflict", stale="task_revision_required_or_stale")

    def sample_phase(self, phase, client):
        for _ in range(SAMPLES):
            response = self.timed(client, {"op": "observe"}, phase, "observe")
            require(response["ok"], "timed observation refused")
            current = self.from_snapshot(response["snapshot"], self.first)
            require(self.at_prompt(current) and current["owner"] == client.endpoint, "sampling lost expected empty prompt/writer")
            response = self.timed(client, {"op": "take_control", "session": self.first, "generation": current["generation"], "force": False}, phase, "take_control_receipt")
            self.applied(response, "timed current-writer generation advance")
            current = self.from_snapshot(client.observe(), self.first)
            response = self.timed(client, {"op": "input", "session": self.first, "generation": current["generation"], "revision": current["screen"]["revision"], "mode_revision": current["screen"]["mode_revision"], "policy": "observed", "intent": {"kind": "key", "key": "End", "ctrl": False, "alt": False, "shift": False}}, phase, "input_end_key_receipt")
            receipt = self.applied(response, "timed input, no retry")
            require(receipt["delivered_bytes"] > 0, "timed input receipt lacks real delivery")
        self.passed("rpc_samples_" + phase, samples_per_metric=SAMPLES, metrics=["observe", "take_control_receipt", "input_end_key_receipt"])

    def coordination_and_noisy_takeover(self):
        # The observer is explicitly human; use protective force only to move
        # the startup grant off the TUI, then give agents a normal released slot.
        self.take(self.observer, self.first, force=True)
        self.release(self.observer, self.first)
        initial_a = self.take(self.agent, self.first)
        self.sample_phase("quiet", self.agent)
        old_a = self.session(self.first)
        released = self.release(self.agent, self.first)
        owner_b = self.take(self.agent_b, self.first)
        require(owner_b["generation"] > released["generation"] > old_a["generation"] >= initial_a["generation"], "A release/B take did not preserve generation order")
        refused_a = self.mutate(self.agent, {"op": "input", "session": self.first, "generation": old_a["generation"], "revision": owner_b["screen"]["revision"], "mode_revision": owner_b["screen"]["mode_revision"], "policy": "observed", "intent": {"kind": "text", "text": "SHOULD_NOT_EXECUTE_A"}})
        receipt = self.refused(refused_a, "writer_or_generation_stale", "old agent A generation")
        require(receipt is not None, "old agent refusal lacks durable zero-effect receipt")
        created = self.mutate(self.agent, {"op": "create_session", "name": "finite-noise"})
        self.applied(created, "second real PTY")
        snapshot = self.wait_snapshot(lambda s: len(s["sessions"]) == 2, "second session present")
        self.second = next(s["id"] for s in snapshot["sessions"] if s["id"] != self.first)
        self.wait_session(self.second, self.at_prompt, "second shell prompt")
        self.take(self.agent, self.second)
        noise_script = f"#!/bin/bash\nfor ((i=0;i<{NOISE_LINES};i++));do printf 'N%0510d\\n' \"$i\";/bin/sleep .04;done\nprintf 'MK1_NOISE_DONE\\n'\n"
        (self.workspace / "noise.sh").write_text(noise_script)
        self.record["noise_fixture_sha256"] = sha256(self.workspace / "noise.sh")
        self.command(self.agent, self.second, "bash noise.sh")
        noisy = self.wait_session(self.second, lambda s: any(line.startswith("N000") for line in s["screen"]["lines"]), "actual finite second-session output")
        noise_before = noisy["screen"]["revision"]
        self.sample_phase("other-pty-finite-output", self.agent_b)
        before = self.session(self.second)
        require(before["screen"]["revision"] > noise_before and not self.at_prompt(before), "noisy sampling phase did not overlap advancing real output")
        old_b = self.session(self.first)
        take_start = time.perf_counter_ns()
        self.tui.prefix(b"t")  # One real UI gesture, not a synthetic daemon call.
        taken = self.wait_session(self.first, lambda s: s["owner"] == self.tui_endpoint and s["generation"] > old_b["generation"], "real human TUI takes B writer during other-session output")
        require(hasattr(self.tui, "screen"), "current outer paint observer required for explicit TUI re-arm acknowledgement")
        self.wait(lambda: "Input re-armed" in self.tui.screen(), "actual outer TUI acknowledges queue-drain/re-arm before any human application typing")
        take_end = time.perf_counter_ns()
        after = self.session(self.second)
        require(after["screen"]["revision"] > before["screen"]["revision"] and not self.at_prompt(after), "other-session output did not advance across human takeover acknowledgement window")
        refused_b = self.mutate(self.agent_b, {"op": "input", "session": self.first, "generation": old_b["generation"], "revision": taken["screen"]["revision"], "mode_revision": taken["screen"]["mode_revision"], "policy": "observed", "intent": {"kind": "text", "text": "SHOULD_NOT_EXECUTE_B"}})
        receipt = self.refused(refused_b, "writer_or_generation_stale", "old agent B after human takeover")
        require(receipt is not None, "takeover refusal lacks zero-effect receipt")
        final_noise = self.wait_session(self.second, self.at_prompt, "finite noise completes and returns to shell")
        require("MK1_NOISE_DONE" in [line.strip() for line in final_noise["screen"]["lines"]], "finite noise trailer absent")
        first = self.session(self.first)
        require(not any("SHOULD_NOT_EXECUTE" in line or line.startswith("N000") or "MK1_NOISE_DONE" in line for line in first["screen"]["lines"]), "stale input or cross-session output leaked into session one")
        require(not any("SHOULD_NOT_EXECUTE" in line for line in final_noise["screen"]["lines"]), "stale writer input leaked into session two")
        self.check_public_bounds(self.snapshot())
        self.descendants()
        self.passed("agent_handoff_and_real_tui_takeover_under_output", released_generation=released["generation"], agent_b_generation=owner_b["generation"], human_generation=taken["generation"], agent_a_refusal=refused_a["error"], agent_b_refusal=refused_b["error"], noise_revision_before=before["screen"]["revision"], noise_revision_after=after["screen"]["revision"], tui_gesture_to_observed_rearm_ns=take_end - take_start, timing_limit="includes polling and outer paint monitor process; not atomic take-only or render latency", queued_or_refused_input_retries=0)

    def check_public_bounds(self, snapshot):
        require(snapshot["degraded"] is None, "broker degraded during bounded workload")
        require(len(snapshot["sessions"]) <= 2 and len(snapshot["attempts"]) <= 2 and len(snapshot["endpoints"]) == 4, "public object caps exceeded")
        for session in snapshot["sessions"]:
            screen = session["screen"]
            require(2 <= screen["cols"] <= 160 and 2 <= screen["rows"] <= 48, "public screen geometry cap")
            require(len(screen["lines"]) == screen["rows"] and all(len(line) <= screen["cols"] for line in screen["lines"]), "bounded ASCII screen rows/cells exceeded")
            require(screen["input_known"] and session["process"] == "running", "bounded synthetic workload lost interpretation or live shell")
        for attempt in snapshot["attempts"]:
            require(len(attempt["stdout"].encode()) <= 65_536 and len(attempt["stderr"].encode()) <= 65_536 and not attempt["truncated"], "small managed evidence exceeded retained stream cap")
        require(not self.tui.truncated, "outer TUI synthetic capture cap exceeded")

    def complete_and_review(self):
        for label in ["a", "b"]:
            (self.workspace / ("release-" + label)).write_bytes(b"explicit fixture release\n")
        snapshot = self.wait_snapshot(lambda s: len(s["attempts"]) == 2 and all(a["state"] == "exited" for a in s["attempts"]), "both managed attempts exit and drain independently")
        evidence = []
        for label, client in [("a", self.agent), ("b", self.agent_b)]:
            expected = self.expected[label]
            attempt = next(a for a in snapshot["attempts"] if a["id"] == self.run_receipts[label]["result"]["attempt"])
            require((attempt["stdout"], attempt["stderr"], attempt["exit_code"]) == (expected["stdout"], expected["stderr"], expected["exit_code"]), "managed attempt " + label + " stream/exit attribution mismatch")
            path = self.workspace / f"agent-{label}.txt"
            require(path.read_bytes() == expected["artifact"], "actual independent file bytes wrong")
            registered = self.mutate(client, {"op": "artifact", "task": self.task, "path": path.name})
            self.applied(registered, "explicit observed artifact registration")
            artifact = next(a for a in self.snapshot()["artifacts"] if a["path"] == path.name)
            digest = "sha256:" + sha256(path)
            require(artifact["revision"] == digest and artifact["task"] == self.task, "artifact's actual file digest/task association wrong")
            reviewed = self.mutate(self.observer, {"op": "validate_artifact", "artifact": artifact["id"], "revision": digest})
            self.applied(reviewed, "manual human revision-bound artifact review")
            evidence.append({"attempt": attempt["id"], "origin": attempt["origin"], "task_revision": attempt["task_revision"], "stdout": attempt["stdout"], "stderr": attempt["stderr"], "exit_code": attempt["exit_code"], "artifact": artifact["id"], "path": path.name, "sha256_revision": digest, "causality_limit": "file observed under shared task; not a sealed producer/attempt proof"})
        capacity = self.mutate(self.agent_b, {"op": "run", "task": self.task, "task_revision": self.task_revision, "argv": ["/bin/true"]})
        self.refused(capacity, "attempt_capacity_two_total", "third attempt lifetime cap")
        require(len(self.snapshot()["attempts"]) == 2, "third attempt had an effect")
        require(not (self.workspace / "SHOULD_NOT_EXIST").exists(), "stale task effect appeared later")
        self.record["attempt_evidence"] = evidence
        self.passed("independent_streams_exit_and_actual_file_revision_review", evidence=evidence, capacity_refusal=capacity["error"])

    def finish(self):
        self.take(self.observer, self.first, force=True)
        self.command(self.observer, self.first, "exit")
        self.wait_session(self.first, lambda s: s["process"] == "exited", "first finite shell exits")
        self.command(self.agent, self.second, "exit")
        self.wait_session(self.second, lambda s: s["process"] == "exited", "second finite shell exits")
        self.tui.prefix(b"q")
        require(self.tui.process.wait(timeout=4) == 0, "TUI detach failed")
        journal = Path(str(self.socket_path) + ".journal").read_bytes()
        require(b"AGENT_A_OUT" not in journal and b"exact independent artifact" not in journal and b"N000" not in journal, "capture-off journal retained synthetic application content")
        for name, metadata in self.record["binaries"].items():
            require(sha256(self.bin_dir / name) == metadata["sha256"], "binary changed during frozen journey: " + name)
        self.passed("finite_real_children_exit_and_capture_off_metadata_only", tui_exit=0)

    def write_samples(self):
        summaries = []
        for phase in ["quiet", "other-pty-finite-output"]:
            for metric in ["observe", "take_control_receipt", "input_end_key_receipt"]:
                rows = [sample for sample in self.samples if sample["phase"] == phase and sample["metric"] == metric]
                values = [sample["elapsed_ns"] for sample in rows]
                if values:
                    summaries.append({"phase": phase, "metric": metric, "n": len(values), "all_responses_ok": all(sample["ok"] for sample in rows), "min_ns": min(values), "p50_ns": percentile(values, .5), "p95_ns": percentile(values, .95), "p99_ns": percentile(values, .99), "max_ns": max(values), "method": "nearest-rank empirical percentile; N=30 is not a tail distribution guarantee"})
        self.record["host_context"]["load_average_at_end"] = os.getloadavg()
        self.record["latency_summaries"] = summaries
        (self.artifacts / "rpc-samples.json").write_text(json.dumps({"clock": self.record["clock"], "host_context": self.record["host_context"], "workload": self.record["workload"], "limits": self.record["measurement_limits"], "samples": self.samples, "summaries": summaries}, indent=2) + "\n")

    def run(self):
        status = "fail"
        try:
            self.start()
            self.shared_task_and_concurrent_attempts()
            self.coordination_and_noisy_takeover()
            self.complete_and_review()
            self.finish()
            status = "pass"
        except Exception as error:
            self.record["failure"] = {"type": type(error).__name__, "message": str(error)}
            (self.artifacts / "failure.txt").write_text(traceback.format_exc())
        finally:
            try:
                if self.agent_b:
                    self.agent_b.close()
                self.cleanup()
            except Exception as error:
                status = "fail"
                self.record["cleanup_failure"] = {"type": type(error).__name__, "message": str(error)}
                (self.artifacts / "cleanup-failure.txt").write_text(traceback.format_exc())
            self.write_samples()
            self.record["status"] = status
            self.record["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
            self.record["runtime_directory"] = str(self.runtime)
            self.record["evidence"] = {p.name: {"bytes": p.stat().st_size, "sha256": sha256(p)} for p in self.artifacts.iterdir() if p.is_file()}
            (self.artifacts / "checks.json").write_text(json.dumps(self.record, indent=2) + "\n")
        print(json.dumps({"status": status, "checks": len(self.results), "evidence": str(self.artifacts / "checks.json"), "failure": self.record.get("failure")}))
        return 0 if status == "pass" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--terminfo", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True, help="fresh repository-local ignored evidence directory")
    return Agents(parser.parse_args()).run()


if __name__ == "__main__":
    sys.exit(main())
