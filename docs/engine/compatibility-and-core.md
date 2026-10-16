# Compatibility boundary and owned core

[Campaign](index.md) · [Native contract](native-contract.md) · [Recovery](runtime-persistence-security.md) · [Validation matrix](delivery-and-validation.md#compatibility-matrix)

**Proposed design.** No new parser or engine implementation exists. Sources S02–S16/S20–S22 distinguish OS contracts, standards and implementation practice; decisions below define namtab profiles rather than universal terminal behavior.

## Execution boundary

Unmodified interactive programs receive a real PTY slave with controlling-terminal/session setup, sane initial termios, geometry and an honestly advertised terminal profile. The keeper owns the master. The shell/program owns its normal job-control behavior; it can change termios and its foreground process group. Do not run an emulated shell inside the engine or implement libc/system calls. A terminal is neither the shell's grammar nor its filesystem/environment.

The platform path must handle allocation/setup/exec error reporting, close-on-exec FDs, child reaping, nonblocking partial reads/writes, EINTR/EAGAIN and Linux PTY close/EIO behavior. A process/session leader PID alone is not a safe job-control abstraction. Track runner incarnation and stable handles where supported; observe group/foreground relationships with platform APIs. The keeper must not fight an interactive shell's `tcsetpgrp` transitions. PTY EOF, a child exit, shell job completion, stopped jobs and a drained output frontier are distinct facts (S02–S05).

With ordinary fd 0/1/2 attached to one slave, stdout and stderr are **merged terminal bytes**; their original provenance cannot be recovered. Managed pipe execution preserves separately read stdout/stderr, each with offsets, EOF and timestamps. Their combined UI order is the order the runner observed reads, not a total order of original writes across pipes. Pipes change isatty/buffering/line-discipline behavior; never force pipe and PTY fixtures to have identical output. Programs may open `/dev/tty` or retain FDs in descendants; drain/timeout facts must remain explicit.

Native pipe output is raw stream evidence with a safe text/escaped-control projection by default, not implicitly fed into a legacy terminal model. Structured application records are opt-in typed side-channel data. If a program needs terminal behavior, request a PTY at launch; changing a pipe-backed process into a PTY in place is not supported. Initial pipe stdin is explicitly closed or a bounded predeclared authorized byte payload; general streaming stdin can be a separately scoped later operation, not an application-keystroke route.

Ctrl-C sends a key/input byte according to the profile. Kernel VINTR/ISIG/foreground state may turn it into a signal; the program can change or ignore that behavior. `SignalExecution` and runtime termination are separate granted operations. Group signals do not guarantee all descendants or remote effects stop. Resize records a new canonical geometry cut and a separately observed ioctl result; SIGWINCH/repaint is asynchronous (S03–S06).

Shell integration is optional. v1 candidate adapters report prompt/command boundaries and cwd as attributed hints, without trusting OSC 133/title/CWD text as authority. A stronger execution adapter must use an authenticated execution-bound side channel and still report only what it can observe. Basic shells work without hooks, Zsh, a prompt theme or rc modification. Adapter installation/migration is a later explicit workflow, never startup side effect.

## Profiles and negotiation

Profiles are immutable versioned feature manifests, shared by parser, encoder, terminfo, observations and test matrix. Capabilities distinguish **implemented**, **negotiated**, **available at this endpoint**, and **authorized**. A frontend lacking a physical chord does not remove an inner agent capability. An unsupported profile is refused before launch/input; a well-formed unsupported output sequence follows that profile's documented consume/ignore policy and produces bounded diagnostics, not guessed semantics.

| Profile stage | Scope / advertisement | Exclusions and gate |
| --- | --- | --- |
| `px-dev-text-0`, N1 | Private development profile: UTF-8 text, CR/LF/BS/TAB, basic cursor/erase/SGR, saved cursor, vertical scroll region, tab stops, primary/alternate buffers, one shell viewport and declared legacy keys | Custom project-local terminfo with only implemented capabilities. No xterm-256color impersonation. Not general Vim/less compatibility. |
| `px-linux-text-1`, v1 target | Modern Linux text terminal; primary/alternate screens, scroll/margins/tabs, 256/RGB color, DEC line drawing, published cursor/keypad modes, bracketed paste/focus/SGR mouse, selected DA/DSR/mode queries and reset | Each advertised sequence/query and mode transition requires a fixture. ncurses/readline/less/Vim profiles qualify this target. No promise to implement all ECMA/xterm functions. |
| Negotiated keyboard capabilities | Ordinary classic legacy default; separate MOK and Kitty enhancement levels; explicit query/set/push/pop/reset policies | N1 may advertise none. v1 certification specifies supported levels, precedence, stacks and format variants. Do not advertise an incomplete Kitty flag or collapse MOK into “enhanced=true”. |
| Later profiles | Graphics, richer text/bidi, extended device controls or alternate encodings | Separate version and memory/security/compatibility gates; never silently enabled by a renderer or TERM string. |

N0 creates a capability-by-capability table from S08/S09 and fixture needs, including private/intermediate parameters, defaults and query bytes. v1 uses an owned `project-x` terminfo entry; package/user-local installation and remote-host availability must be tested without global writes. An optional xterm-compatible TERM alias can only be published after its actual capability subset is certified. DA/query replies describe what is implemented, not a copied xterm identity. Unrecognized queries are ignored or explicitly reported unsupported as specified, never answered with a false capability.

## Parser and deterministic application

Recommended starting point: a small owned incremental byte FSM, with a reviewed transition table and semantic dispatch table. Reuse standards techniques, checked UTF-8/Unicode primitives and test generators. Do not import an existing emulator's screen/modes as the authority. vte action dispatch is a useful comparator, but its private continuation makes direct reuse conditional; a maintained lexer dependency is eligible only with a public complete bounded state representation and compatible licensing. No lexer fork or dependency is created in this campaign.

```text
ordered keeper output/resize/control events
    → owned incremental lexer + UTF-8 continuation
    → profile-specific semantic reducer
    → canonical state + immutable observation changes + proposed effects
    → broker fences / keeper delivery (effects), subscribers (observations)
```

One session owner applies an explicitly ordered sequence. Pure reduction is deterministic **only for fixed event order, profile/data versions, initial state and specified limit/error policy**. Clock reads, randomness, OS calls and direct callbacks do not occur inside it. Scheduling, application execution, interleaving of external processes and model behavior are not deterministic replay claims.

That equivalence concerns successful bounded reductions with available resources. It does not promise identical allocation failures, scheduling or OOM behavior across machines. A classified resource failure must refuse/latch the relevant state rather than invent a successful next frame.

The FSM represents ground, escape/intermediates, CSI entry/parameters/intermediates/ignore, OSC/string payload and terminator handling, DCS/APC/PM/SOS consumption, UTF-8 continuation and cancellation/error states. Own pending ESC in a string terminator, partially accumulated numeric/subparameters, UTF-8 prefixes and parser ignore/discard state. Recognize bytes in the right parser context; do not decode a UTF-8 codepoint and re-run its value as an escape control. v1 chooses UTF-8 plus supported 7-bit control forms; legacy 8-bit encodings are a separate profile. CAN/SUB, ESC inside sequences, invalid UTF-8, unexpected final bytes and EOF have explicit testable rules, not ad hoc recovery at chunk boundaries.

Caps cover geometry/cell counts, parameter/subparameter counts and arithmetic, string payload, grapheme bytes, history, mode stacks, effect/reply size and diagnostics. Initial safety-cap candidates for N0 are 64 CSI parameter entries, 16 KiB retained string payload, 256 bytes per retained grapheme and a 16-entry keyboard stack; these are **proposed tunables**, not measured sufficiency or new performance SLOs. Final limits and exact overflow policy become part of the profile before advertisement. Saturating diagnostic counters must not become unbounded logs.

Counted operations such as repeat, scroll and insert must do work bounded by affected grid/history limits, not loop an attacker-supplied integer billions of times. Parsing limits alone do not establish a CPU bound. Snapshot import likewise validates structure/lengths before allocating declared collections, with a bounded serialized input and no decompression in v1.

Oversized strings enter a bounded discard-until-terminator state, retaining only what future parsing needs. This is not arbitrary loss of canonical input: every byte still drives the defined state machine. Unsupported graphics/file protocols are consumed without decoding images or host actions. Malformed/unsupported input under a specified policy is not automatically “unknown”; unknown is required for missing parser bytes, failed state invariants or unclassified partial application. No guessed resynchronization makes an opaque TUI correct after an actual stream gap.

## Complete semantic state

| State family | Required ownership / continuation obligations |
| --- | --- |
| Buffers and history | Primary and alternate grids; active buffer; stable logical-line IDs, soft-wrap markers, bounded primary history and viewport anchors. Alternate content is not flattened into a command log. |
| Cursor and saved state | Cursor, wrap-pending, visibility/style; each supported save/restore slot with associated attributes/charset/origin state specified by the profile. Saved cursor alone is insufficient. |
| Editing geometry | Rows/columns; top/bottom and supported left/right margins; origin/insert/autowrap/newline modes; tab-stop set; scroll/erase/insert/delete rules and blank-cell attributes. |
| Attributes and character sets | Foreground/background/default/index/RGB distinctions, palette and supported decorations; G0/G1 DEC line drawing and shift state for the initial text profile. |
| Protocol modes | Application cursor/keypad, numlock policy, backarrow, focus/paste/mouse, synchronized output, keyboard negotiation and **separate primary/alternate Kitty stacks**, resets and query state. |
| Text | Incremental UTF-8, Unicode/profile versions, grapheme continuation, lead/trailing wide cells and stable text-to-cell mapping. No dependency-private iterator or shaping pointer in snapshots. |
| Extensions and errors | Bounded supported hints/hyperlinks, sanitized title metadata, unsupported/discard states and diagnostic/loss flags. No host authority in extension payloads. |

A reducer transaction either commits a valid next state/frontier and its effects or latches unknown without exporting a fabricated partial frame. Large byte events are reduced in bounded quanta; no observer sees half a committed semantic mutation. A background snapshot may use a bounded immutable/COW view, but cannot make an unbounded stop-the-world copy an interaction requirement.

## Unicode, reflow and rendering boundary

Recommend versioned Unicode 18.0.0 data and approved UAX #29/#11 references from S11–S13, with bootstrap hash/license checks and a matching implementation primitive. Width is an owned profile: ambiguous width defaults narrow, emoji/variation/ZWJ and combining behavior are explicitly table-driven/tested, not inferred from font metrics. No normalization changes application bytes. Invalid UTF-8 has a specified replacement/diagnostic policy and optional authorized raw evidence. Unknown/private-use/unassigned glyph width needs a fixed profile fallback.

Keep stream byte offsets, Unicode scalars, grapheme clusters, logical lines, terminal cells and glyph positions distinct. A cluster may grow across writes; a later variation selector can require a bounded correction to the lead/trailing cells, cursor and delta revision. Editing part of a wide cell clears/repairs the entire occupancy according to a fixture. Leading combining marks and right-edge width changes need hand-authored cases. Reflow cannot rewrite original byte citations.

Primary-buffer soft-wrapped logical lines reflow on resize under the published policy; hard breaks and alternate-screen content do not magically become reflowable prose. Map history/selection anchors or invalidate them explicitly; cap shrink/expansion work. Resizing and subsequent application repaint are separate events. v1 terminal cells use logical order without full bidi reordering; expose that limitation, safe bidi markers in review chrome, and readable logical inspection. Full bidirectional terminal editing requires a later profile and mapping tests, not an assertion that Unicode storage implies bidi support.

Shaping, font fallback, ligatures and rasterization are presentation concerns constrained by canonical occupancy. They cannot change agent cell positions. Electron and the outer TUI host may paint differently; that is a visible capability/quality issue, not permission for two width authorities.

## Input policy

Keep three planes: namtab UI commands, application input intents, and native automation/control operations. Keyboard packets distinguish named logical key; text/layout facts; optional physical code; modifiers and **consumed** modifiers; composing state; press/repeat/release; endpoint capabilities. Unknown facts stay absent. `CommitText` represents an IME/text commit, not a loop inventing physical key presses; its treatment under report-all/associated-text modes must have a documented encoder contract before that combination is advertised.

The required inner set covers arrows, Tab/Shift-Tab, Enter/Escape, Home/End/PageUp/PageDown/Insert/Delete/Backspace, F1–F12, keypad identities and advertised Ctrl/Alt/Shift chords. A structured key conveys intent and one encoding authority; raw bytes convey an exact authorized payload but require the caller to know its profile. Neither supplies opaque application meaning. Unsupported modifiers, event phases or encodings are refused before writing, without silent raw fallback. `Tab` asks the application to handle Tab; the engine does not run a universal completer. Text preserves declared characters without normalization; ordinary text fields cannot hide C0 controls. Raw application bytes go toward the program, never into the output parser.

The owned encoder reads canonical typed modes directly. Ordinary legacy policy includes Ctrl-I/M/[ → `09/0d/1b`, real Tab/Enter/Escape and documented Alt/Shift/Ctrl behavior. Enhanced Kitty/MOK preserves distinctions after explicit negotiation; no unnegotiated fixterms default. Cover cursor/keypad modes, numlock/backarrow, function keys, consumed Shift/AltGraph/layout facts, per-buffer mode stacks, reset and protocol precedence. N0 must resolve the detailed MOK format/level table before v1; unsupported combinations return a typed capability error before writing, not guessed bytes. Native agents can supply explicit logical keys that the host cannot physically report.

Mouse coordinates bind to geometry/buffer/revision and negotiated tracking/encoding; selection in namtab is distinct from editor selection via keys/mouse. Focus events are protocol hints tied to view identity, not permission. Paste is a separate typed bounded operation: wrapper policy reads **inner** mode, dangerous controls/delimiter injection and newline transformation are explicit, never append Enter automatically. Outer TUI paste wrappers are decoded once, not tunneled inward. Legacy Escape can be a lone key or an Alt/sequence prefix; transport fragmentation and application timeouts preserve ambiguity even after one local write. Outer decoding reports only facts it knows. Enhanced release/repeat distinctions require end-to-end negotiated support; bounded repeated presses remain separate non-idempotent events. Do not infer certainty from sleeps or invent key-up events.

Ordinary paste rejects embedded controls/delimiters unless a separately explicit policy permits them; unbracketed fallback requires declared policy rather than silently pretending equivalence. Record every newline/control transformation in the operation identity. Partial paste can leave incomplete wrappers and uncertain application state. Bracketed paste is not an execution-safety guarantee.

`SendRaw` is a granted bounded compatibility fallback through the same writer/freshness/receipt path. It may have effects the logical encoder cannot describe and must say so; it does not repair a missing canonical state, bypass protected prompts or promise safe retry. An engine in unknown state refuses all application-input routes. Separately authorized OS controls remain possible.

## Single reply authority

Only the canonical reducer proposes responses to terminal queries. Reply IDs derive from session incarnation, originating event and ordinal; the broker/keeper owns dispatch receipts. Preserve repeated identical legitimate query replies; never deduplicate by byte content. Renderers and reattached views produce no replies. Historical interpretation replay suppresses all external effects, and a delivery-unknown reply is not resent automatically. Palette/geometry/device responses use the state/profile at the defined cut, never whichever frontend happens to be focused. Host-side TUI capability negotiation uses a separate bounded setup channel and cannot answer inner application queries.
