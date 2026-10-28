# Bounded xterm adapter: authority, API risk and veto gates

> Historical phase record. [ADR E4](../adr/0004-owned-agent-native-engine.md) chooses an [owned agent-native engine](../engine/index.md). Prior findings, failed gates and evidence remain unchanged; adoption-specific next-step recommendations below are superseded. They are not the new product acceptance definition.

> Subsequent [EA1 results](21-ea1-native-worker-results.md): experimental Rust/public-C state and reply seams demonstrated; required legacy control aliases **FAIL**. No EA2/dependent application work. Original X4 stays independently red.


> Historical xterm candidate finding: serializer X4 remains FAIL and the original independent gate is preserved. [19](19-engine-build-adopt-redesign-assessment.md) independently audits canonical parsing, tests a cell-rendering redesign and official libghostty-vt, and recommends the native public core for the next prototype. ADR E1 explicitly accepts owned rendering; this revises the old no-custom-renderer cost preference without changing the old gate verdict or product fidelity requirements. The remainder records the earlier candidate boundary/budget.


[Index](../index.md) · [Baseline ADR-A2](../architecture/10-recommended-baseline-and-adrs.md#adr-a2-one-interpretation-authority-replaceable-presentation) · [Dual frontend](../ux/host-terminal-profile.md) · [Conformance/CI](16-test-and-ci-strategy.md)

**Historical recommendation, not the current selected prototype direction.** xterm was the first candidate for the Electron + host-TUI requirements, with an explicit possibility of rejection. Public/proposed cell APIs, a private key seam and replica recovery are substantive risks; naming an adapter does not remove them. The authorized engineering continuation ran bounded probes without dependency source edits; its failed gate supersedes the earlier unmeasured recommendation.

## Exact research source family and packaging gate

The official 6.0.0 release links to commit **f447274f430fd22513f6adbf9862d19524471c04**. This resolves the earlier tag-only evidence. It is a reviewed source-family anchor, not a security endorsement or verification of registry artifact bytes. [Release](https://github.com/xtermjs/xterm.js/releases/tag/6.0.0), [full commit](https://github.com/xtermjs/xterm.js/commit/f447274f430fd22513f6adbf9862d19524471c04)

| Component | Inspected package metadata at that commit/tag | Required pairing |
| --- | --- | --- |
| Browser | @xterm/xterm **6.0.0** | Same reviewed source tree as headless |
| Headless | @xterm/headless **5.5.0** | This manifest version alone does not identify the 6.0.0 tree |
| WebGL | @xterm/addon-webgl **0.19.0** | Visible desktop presentation only |
| Serialize | @xterm/addon-serialize **0.14.0** | Experimental presentation candidate, not full checkpoint |
| Search | @xterm/addon-search **0.16.0** | Optional desktop search helper; canonical range identity comes from broker |
| Grapheme addon | @xterm/addon-unicode-graphemes **0.4.0** | Candidate only if the same tested width provider works in both headless/browser |

Metadata sources: [browser](https://raw.githubusercontent.com/xtermjs/xterm.js/f447274f430fd22513f6adbf9862d19524471c04/package.json), [headless](https://raw.githubusercontent.com/xtermjs/xterm.js/f447274f430fd22513f6adbf9862d19524471c04/headless/package.json), [WebGL](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/addons/addon-webgl/package.json), [serialize](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/addons/addon-serialize/package.json), [search](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/addons/addon-search/package.json), [graphemes](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/addons/addon-unicode-graphemes/package.json).

**Update: published @xterm/xterm 6.0.0 and @xterm/headless 6.0.0, plus serialize 0.14.0, now have bounded verified artifact mapping in 18.** The source-tree headless manifest version 5.5.0 did not match the publication version; installed/registry commit fields and embedded parser source establish the tested pairing. The requirements below still apply to future families. Bootstrap must either prove published tarballs correspond to this family or build private internal artifacts from one immutable tree, with content digests, provenance and explicit internal version labels. Never substitute the registry's headless 5.5.0 merely because the string matches the manifest. One lockfile pins the resulting exact artifacts; retain source/build/patch manifests and upstream MIT notices. Do not publish private repackaging. Validate new shipping families again, including transitive licenses. The direct API tag endpoint was unavailable through the research tool; the release's commit link supplied the full identity instead.

## Required API surface and uncertainty

The classification below comes from the inspected declarations and sources; usage rules are Project X proposals. Stable declaration does not mean every required composition is supported. [Headless declarations](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/typings/xterm-headless.d.ts), [browser declarations](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/typings/xterm.d.ts), [internal key helper](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/src/common/input/Keyboard.ts), [serializer status](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/addons/addon-serialize/README.md)

| Need | Upstream status / required seam | Gate consequence |
| --- | --- | --- |
| Consume output at explicit prefix | Public write(Uint8Array, callback), resize, dispose | Use completion callbacks and ordered resize events; frame-level onWriteParsed can leave writes pending |
| Base mode inspection | Public modes includes cursor/keypad/paste and selected other modes | Do not assume it exposes every encoder flag; inventory actual required profile fields |
| Canonical cells/cursor/buffer | buffer API is declared experimental; related line/cell access is mutable | Opt into proposed API only inside adapter; copy a coherent bounded snapshot, never retain live cell objects |
| Parser/Unicode extension | parser and unicode surfaces declared experimental | Only reviewed built-ins; no arbitrary repository/custom JS handlers; width tuple must match replicas |
| Structured logical keys | No public headless logical-key encoder established; input accepts an already encoded string | Private helper evaluation is a feasibility candidate, not a complete keypad/modifier implementation |
| Browser human interaction | Public custom key handler, onKey/onData/onBinary and DOM interaction surfaces | Prove committed text/IME/paste/mouse origin separation; generic onData is never an input tunnel |
| Desktop replica reattach | Serialize addon is experimental framebuffer serialization | Test future behavior after attachment, not merely equal current pixels; hidden modes/parser carry matter |
| Complete canonical checkpoint | No complete public API established by this research | Do not promise engine-crash restoration without complete authorized history/state |

A mode-aware encoder must cover required normal/application cursor and keypad behavior, Return versus keypad Enter, modifier/function keys, Backspace conventions, legacy aliases, paste controls and negotiated keyboard enhancements. Enhanced wire protocols may remain unsupported if truthfully negotiated; required conventional keys cannot be waived. Do not implement guessed escape tables separately in Rust TUI, React and agents. If the private evaluator is insufficient within the budget below, xterm fails this requirement.

## Narrow adapter and authority

One internal TS package has explicit node/browser entries, with a closed dependency surface:

| Operation | Contract |
| --- | --- |
| create/feed/resize/dispose | Engine instance/incarnation, interpretation tuple and exact sequence/prefix; completion or bounded failure |
| observe | Detached immutable cells/cursor/buffer/modes/profile plus prefix, generations, completeness and lag |
| encode intent | Typed logical key/text/paste/mouse or explicit raw payload + canonical profile revision → immutable bytes or unsupported/stale; no write side effect |
| collect replies | Bounded canonical protocol responses tagged with engine incarnation and parse work identity; no clipboard/credentials/authority |
| presentation attach/update | Validated desktop presentation basis at a prefix, following chunks/geometry and mismatch status; TUI receives canonical cells |

For a snapshot, finish a named parser write/resize prefix, defer subsequent interpretation only while copying a bounded view into immutable storage, then resume. Paged frames carry one snapshot identity/prefix; never combine pages from different cuts into a supposedly coherent screen. Oversized requests return a bounded range/limit error rather than allocating an unlimited full history. The copy stall is measured under X5, not assumed cheap.

These are internal operation shapes, not a public plugin ABI. The Rust supervisor checks identity/lease/approval/freshness before keeper delivery and rechecks relevant revisions after asynchronous encoding. Encoding cannot freeze an application's internal state or unread output. TUI key decoding normalizes only what its outer terminal delivered; the same inner encoder never fabricates missing outer distinctions. Native agents observe canonical state without any frontend attached and never synthesize desktop shortcuts.

Only the canonical engine has a protocol-reply lane. Keep its xterm instance **feed-only**: human/agent input never calls headless input to obtain a reply event. Encoded application bytes instead return to Rust admission. Thus canonical onData/onBinary handling is reserved for known parse-generated replies, with explicit provenance/size limits; unsupported output-triggered clipboard/window actions are disabled. This origin-separation design must be demonstrated under delayed callbacks/custom handlers, not assumed because the API event is called onData.

Desktop replicas drop their generated reply/data streams. Human key/composition/paste/mouse intent is captured separately and encoded against canonical context; matching byte contents cannot tell a human key from a device reply. A browser's key handler alone does not establish complete IME/mouse coverage. TUI never forwards inner escape sequences or host query replies to application input. Both clients use the public broker API, never private engine IPC.

Reply suppression is **per authority/parse event**, not payload deduplication. Two legitimate identical DSR queries need two replies. Keeper reconnection must not resend an uncertain old reply: response bytes are also non-idempotent. Protocol queries that depend on unavailable GUI geometry/clipboard must follow the declared conservative terminal profile, not whichever view is currently focused.

## Presentation, capture and recovery limits

A TUI viewport is an immutable rendering of canonical cells and can reattach without reconstructing an emulator. A desktop xterm replica needs compatible **future interpretation**, including current attributes, cursor/saved state, scroll regions, wrap state, tabs, primary/alternate buffers, partial UTF-8/escape strings and keyboard/charset modes. The list is a risk checklist, not a claim the serializer saves all of it. Framebuffer equality at one instant is insufficient. Cropped/limited-history TUI presentation is also not a complete terminal checkpoint.

Permit attachment only at a tested compatible boundary with a validated representation and following ordered prefix. If no safe representation exists for a current state (for example an incomplete control string), wait under bounded limits or mark view unavailable; do not reset the application to force a picture. A continually growing output history is not a bounded memory-only reattach design. For the original no-custom-renderer candidate, ordinary desktop attach failed. ADR E1 now explicitly accepts owned rendering as a REDESIGN with its own fidelity/accessibility costs; this supersedes that old cost veto, not the failed serializer result. A text inspector still does not qualify.

GUI/TUI loss can preserve keeper+canonical state. Canonical engine loss can leave a live PTY with unknown interpretation; application input stays disabled until valid state is reconstructed. Authorized complete byte/resize/options history from a known initial state can support **interpretation reconstruction** with a pinned engine; filtered/gapped history cannot. Persisting complete history or raw-equivalent state requires capture consent. Memory-only sessions may therefore lose recoverability. No checkpoint is mandated merely to advertise persistence.

History reconstruction does not execute commands or promise deterministic execution replay. No collection of terminal bytes recreates OS scheduling, mutable files, secrets or remote effects. Re-execution is always a new attempt. [Capture and lifecycle policy](../architecture/10-recommended-baseline-and-adrs.md#adr-a6-capture-and-retention-defaults)

## Caps, backpressure and malicious output

Enforce existing shared G6 limits across canonical state, snapshots, replica buffers and both clients. The adapter must copy bounded pages, coalesce only presentation updates, expose parse/output/durable frontiers separately, and never drop arbitrary parser bytes. Per-session fair parse chunks and process limits address a flood; a single Node event loop can still be starved by one pathological parser call, so PP1/PP3 must measure worst fixture behavior and cancellation responsiveness. Do not claim quotas can preempt arbitrary synchronous code.

Cap geometry/cell allocation, OSC/DCS payloads, pending callbacks, reply traffic, snapshot reassembly and subscriber credit. Limits inside upstream code may differ from Project X limits and must be inspected. A slow browser/TUI may be disconnected; capture may explicitly gap; canonical parsing loss makes state unknown. No silent “complete” result after truncation. If required limits need broad parser modification beyond the budget, fail the gate rather than introduce a second sanitizing terminal parser.

## Acceptable adaptation budget and go/no-go

Planning limits for a solo/small team, not universal cost estimates:

- One private bridge module, at most **two upstream private entry points across two upstream source files**; no arbitrary _core graph traversal outside it. Enumerate every symbol and transitive addon risk in the manifest.
- At most **two small carried patches, 200 changed non-generated upstream lines total**, for packaging/exposure/origin plumbing; no terminal parsing or rendering semantics fork. Zero dependency edits occur in this session.
- Initial adapter target **≤800 non-generated production lines**, excluding generated bindings and tests. Exceeding it reopens the design rather than moving code into another package to hide the count.
- Upgrade rehearsal across two independently pinned released families: allow roughly **two engineering days** for adaptation plus existing corpus, recorded prospectively. Exceeding it triggers reassessment; there is no promise a later upgrade is safe within a stopwatch budget.

| Gate | Required falsifiable evidence | Fail action |
| --- | --- | --- |
| X1 — Provenance | One exact source tree, built/published artifact mapping and addon hashes; manifest mismatch explained | No engine-dependent foundation expansion |
| X2 — Keys and origins | Required N07/N08/N12/N15 plus human IME/mouse; native/TUI/desktop intents encode identically where representable; no query reply enters key path | Reject xterm if budget cannot fix; no raw-only substitute |
| X3 — Observation and replies | Headless observations without UI; cell/mode/prefix consistency; exactly one response per query event across two attached clients | Block interactive certification |
| X4 — Attach/recovery | Reattach followed by adversarial future output, resize and alternate-screen sequences; losses/unknown states honest under capture off/on | Reject unsafe replica model; no serializer-as-checkpoint claim |
| X5 — Resource/security | G6 caps/flood fairness, bounded payloads/replies, no output side effects, timely independent revocation | Fix within budget or reject candidate |
| X6 — Maintenance | Private surface/patch budget honored, upgrade rehearsal and fixture-license review completed | Reopen baseline, do not normalize a long-lived fork |

X1–X6 refine PP1/PP3/PP4 and G0/G7; current verdicts are in 18. X1 has bounded provenance evidence, X4 failed, and other full gates remain incomplete. First spike may establish X1–X4 minimal cases before undertaking full performance coverage. A failed essential gate blocks a two-frontend release rather than postponing the TUI as optional.

## Replacement path

Keep Rust domain/store/client contracts and immutable fixture/evidence identities. Replace engine-host and terminal-adapter behind a new interpretation/profile version, then recertify **both** presentations and native navigation. A headless core change never silently reinterprets old citations. libghostty-vt is the first targeted alternative for a proven key/state seam failure, but its evolving C API and complete renderer/accessibility integration remain costs. A GTK/VTE desktop would be an explicit Linux-first product change; TUI cell export/agent encoding still need proof. Tauri changes the desktop shell, not these engine problems.

Do not build a Ghostty parser and continue trusting xterm as a second independent interpretation authority. Do not migrate live parser state across engines by guess. Drain or preserve old sessions with their old certified tuple; new sessions use the new profile. If no bounded candidate satisfies the initial two-frontend requirements, stop with a documented infeasibility finding or revise scope explicitly. This is preferable to describing fragile internals as a finished architecture.
