# Owned MK1 terminal core

`project-x-terminal` is a pure, safe-Rust, bounded reducer and logical input
encoder. The worker/keeper controls ordering and dispatches returned reply bytes
once; this library has no process, PTY, OS, clock, storage, rendering or grant
authority. `Frame` is a visible projection, **not a checkpoint/import format**.

The MK1 subset uses two 2–160 by 2–48 grids and ASCII/Latin-1 width-one text
(excluding soft hyphen). Unsupported Unicode becomes a safe replacement cell;
UTF-8 continuation is incremental. It includes classic controls, cursor movement,
erase, eight-color SGR, margins/tabs, save/restore, 1049 alternate screen,
cursor/keypad/backarrow/paste/focus modes and selected status/mode replies.
Unsupported strings discard all payload. ESC/CSI state and parameters are bounded.
Each output operation is at most 512 bytes. Work is bounded by the small fixed
geometry; cooperative work-unit yielding and production allocation admission are
not implemented or claimed.

Logical input covers named editing/function/keypad keys, classic Ctrl aliases,
documented modifiers, committed repertoire text and bracketed paste. Physical
layout/consumed-modifier facts must already be resolved by the endpoint. Release
events are refused; a known repeat emits once. Disabled focus reporting explicitly refuses focus input. Paste requires bracketed mode,
normalizes CRLF/lone CR to LF and rejects ESC/control payloads. Unsupported Kitty
or modifyOtherKeys negotiation fences all input until a terminal reset. This is
an explicit finite profile refusal, not emulation of those protocols.

There is no history/reflow, grapheme/wide-cell/bidi shaping, mouse, graphics,
search/selection model, shell-command inference, password-prompt detection,
portable durable recovery, serialized parser import or full `px-dev-text-0`
qualification. Input mode revision changes on encoding-mode transitions,
alternate/reset/geometry; ordinary echo/cursor output does not change it.
Canonical semantic revision is per accepted nonempty feed event, so callers must
not confuse a transport chunk with an application-command or complete sequence.
Unknown protocol state and unavailable interpretation are distinct facts.

`namtab-terminal-worker` provides a same-build inherited-stdio JSON-lines seam
for at most two reducers. Requests are bounded to 128 KiB, responses to 1 MiB,
with strictly increasing nonzero request IDs. A malformed, oversized or replayed
exchange terminates the worker with a generic error and no payload echo. A lost
response is uncertainty, not permission to retry. The service remains the sole
PTY/input/reply dispatcher and must fence affected sessions on worker failure.
`FinishOutput` resolves trailing partial decoder state and closes input for that
reducer; it is independent of actual child exit and never resurrects a process.

Run `cargo test -p project-x-terminal --offline` and
`cargo clippy -p project-x-terminal --all-targets --offline -- -D warnings` with
the project-local Cargo cache. Existing historical engine experiments remain
separate. Original crate code uses [Apache-2.0 with explicit third-party boundaries](../../docs/licensing.md).
