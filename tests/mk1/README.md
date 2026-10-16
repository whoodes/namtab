# Real MK1 journeys

`e2e.py` runs the actual prebuilt broker, owned worker, PTY helper, host TUI and
agent CLI on Linux. Python 3.10+, `/proc`, pidfd APIs, Bash, `/bin/sh`,
`/usr/bin/printf` and `sleep` are required. It uses only the Python standard
library, installs nothing and never invokes a compiler. Build matching binaries
and compile the private `namtab-mk1` terminfo in the normal project build first.

```sh
python3 tests/mk1/e2e.py \
  --bin-dir "$PWD/target/debug" \
  --terminfo "$PWD/.artifacts/terminfo" \
  --artifacts "$PWD/.artifacts/mk1-e2e-UNIQUE_RUN_ID"
```

The artifact directory must be fresh and inside this repository's ignored
`.artifacts`. Each run also creates a fresh short, private project-local socket
directory, avoiding Linux Unix-socket path truncation. Paths and synthetic
workspace data remain available for investigation; no prior artifacts or user
work are deleted. Its direct `Popen` children are terminated and waited. Only
verified descendant pidfds are signalled, and bounded readiness checks establish
those processes exited; the harness cannot reap another process's children or
prove discovery of every descendant. Test capture consists of bounded synthetic TUI ANSI,
daemon diagnostics and machine-readable `checks.json`; live resume capabilities
remain in RAM and are not logged.

The run drives ordinary TUI keyboard input through a real host PTY and inspects
the broker's canonical observations. It checks session creation/switching,
actual command effects/output, human/agent writer handoff and stale-generation
refusal; live endpoint resume; literal managed argv, distinct streams and
exit/drain, duplicate/conflicting mutation IDs and the two-attempt limit;
revision-bound manual artifact review/staleness; real CLI observation; explicit
TUI reconnect after a controlled live-broker pause; no replayed command effect;
and normal shell exit/TUI detach. Inputs are sent exactly once; polling applies
only to observations. A failure retains logs and returns nonzero rather than
retrying uncertain input or weakening the assertion.

The result binds the harness Git observation, exercised binary SHA-256 values,
launch commands, caps, assertions and UTC times. A dirty-source observation or
binary hash alone does not prove compiler provenance; the integrating check must
record its actual build and reviewed source revision. No binary may be replaced
while a run is active.

This is a **headless real host-TUI/process check**. It neither runs nor qualifies
Electron, demonstrates visual fidelity/IME/accessibility, measures performance,
contains descendants, restores crashed processes or establishes full NG1. The
separate real graphical demonstration and user assessment remain necessary for
MK1's human-feel handoff.
