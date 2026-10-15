# Bounded fuzz foundation

`node tests/fuzz/chunk-corpus.mjs` performs 192 fixed-seed partition trials against the six shared terminal fixtures. Rust protocol Proptest covers arbitrary bounded byte frames; domain Proptest covers generation and repeated-admission traces. Those properties run in the fast lane.

No empty cargo-fuzz workspace, nightly toolchain, sanitizer setup or coverage-guided claim is added. A coverage-guided target becomes justified when a production framing/parser/IPC implementation exists and its resource oracle is defined. Current tests do not exercise a PTY, opaque application, malicious-output preemption or a real display.
