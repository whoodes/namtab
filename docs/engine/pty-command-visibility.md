# Cooperative PTY command visibility — proposed

[Execution / optional shell adapters](../architecture/03-execution-terminal-and-remote.md) · [Issue85](https://github.com/whoodes/namtab/issues/85)

**Scope/design only; implementation deferred until ENG5 is review-ready.** Managed argv/pipe launches have an explicit attempt origin and direct-child wait result. Arbitrary bytes typed into a persistent PTY do not. Cooperative shell integration can add useful observations without turning an opaque shell into a supervised command dispatcher.

## Proposed narrow contract

A fresh session may explicitly opt into one version-qualified shell adapter. It reports command/statement start, end and optional exit status with session and shell incarnation, local sequence, observed stream cuts (or unknown range), source `cooperative_shell`, completeness and trust. A later separately admitted bounded event profile must specify frame/retention/resource caps. No implemented wire capability or supported Bash hook/version is claimed here.

Use the owned canonical parser for any in-band integration markers; never add a second frontend parser. OSC133-style conventions are a candidate, not a universal standard. Programs can forge them. A nonce visible to the shell/children does not establish strong provenance. A future side channel may improve binding but inherited descriptors and hostile same-UID processes require separate analysis. Metadata never authorizes input, approves artifacts, proves a managed attempt completed or changes writer ownership. Never infer completion from prompt text.

Default command text/history/environment/cwd/hostname collection **off**. Start/end/status alone are useful; even command names/status can be sensitive. Any later export/capture needs explicit consent and bounded retention. No universal password detector is promised. Launch-time per-session opt-in must preserve shell options, hooks and behavior; zero-integration sessions remain ordinary opaque PTYs. No global startup files or running sessions are modified by this work.

## Uncertainty that must remain visible

| Situation | Honest interpretation |
| --- | --- |
| Unsupported shell/version or missing hooks | No integration; ordinary terminal behavior |
| Missing/duplicate/out-of-order/truncated/forged marker, disconnect or capture gap | Partial/unknown observation; no success inference or automatic retry |
| Pipeline/compound statement | Shell-reported aggregate status; not each stage's wait result or causal output partition |
| Background job/output after prompt | Overlap/interleaving, no exclusive command ownership of the stream |
| Nested/replaced shell, SSH/tmux | New/unknown shell incarnation; remote marker is no verified host identity; no automatic helper installation |
| Editor/fullscreen/REPL | At most outer shell launch/return; internal operations remain opaque |
| PTY output | stdout/stderr merged; markers cannot reconstruct separate channels |

Future acceptance needs adversarial marker forgery and sequence-gap tests; hook failures; syntax errors/interrupts; pipelines/background output; nesting/SSH/fullscreen; secret-disabled collection; and unchanged exit meaning, options, normal key behavior and unsupported-shell fallback. Evidence must distinguish shell claims from supervised process facts.

## Primary-source ledger and gaps

Retrieved **2026-10-29**, no copied integration code or installation:

- [Microsoft shell integration source](https://github.com/microsoft/vscode-docs/blob/4ad8b6554dd1f408fc0568f16b2f9b38763ef084/docs/terminal/shell-integration.md) (header DateApproved 10/28/2026, pinned source revision): integration availability/quality varies with shell/setup, subshells and SSH. It supports workflow navigation; it does not authenticate command metadata for Namtab.
- [iTerm2 shell integration](https://iterm2.com/documentation-shell-integration.html) and [OSC133 conventions](https://iterm2.com/documentation-escape-codes.html#shell-integration-finalterm) (unversioned current pages): prompt/input/output/end markers and optional status are de facto implementation conventions. Those bytes remain forgeable output in this threat model.
- GNU Bash manual retrieval timed out during this campaign. **Open:** verified version/hook choice and interactions with existing prompt/debug hooks. Do not treat recalled `PS0`/`DEBUG`/`PROMPT_COMMAND` behavior as an admitted adapter API.

This design does not solve universal semantic attribution, reconstitute stderr, promise deterministic replay, or weaken the managed-execution evidence contract.
