# ADR E3: retain the input veto until a public compatibility policy exists

> Superseded direction: [ADR E4](0004-owned-agent-native-engine.md) chooses ownership. This ADR preserves the prior rationale/input veto and invariants; it is no longer the active adoption or next-SDK recommendation. Historical X4/EA1 remain FAIL.

Date: 2026-10-15. **Accepted-for-design:** preserve classic legacy aliases and negotiated distinctions; do not infer negotiation state or falsify key events. **Measured investigation:** the pinned public seam is insufficient for the proposed correction. **EA1 remains FAIL; EA2 remains stopped.** This is a bounded decision about commit `22fa134a07f5749edd9e92c6bc17d8aaf61843be`, not rejection of all libghostty-vt versions.

[Index](../index.md) · [Exact investigation and checks](../engineering/23-legacy-input-investigation.md) · [Ownership ADR](0001-engine-ownership-and-presentation.md) · [Historical EA1](../engineering/21-ea1-native-worker-results.md)

## Context and diagnosis

The product requires Ctrl-I/M/[ to emit `09`/`0d`/`1b` in its ordinary legacy profile. The original test still enforces this. The pinned encoder intentionally applies fixterms without Kitty negotiation: those character chords emit `CSI 105;5u`, `CSI 109;5u`, `CSI 91;5u`. Upstream tests explicitly require these encodings. This is a product-policy mismatch, not a demonstrated parser defect or upstream regression. See the [pinned implementation](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/src/input/key_encode.zig) and [fixterms proposal](https://www.leonerd.org.uk/hacks/fixterms/).

Our character construction supplies printable, unmodified text before Ctrl/Meta processing, modifiers, an ASCII logical codepoint and an approximate US physical key. That follows the relevant [public text contract](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/key/event.h) for these three probes. Removing text loses legacy output; changing physical-key identity does not restore the aliases. This does not establish correct physical keyboard/layout/IME translation in general.

The [public encoder API](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/key/encoder.h) has eight options, including Kitty flags and modifyOtherKeys state 2, but no fixterms/classic-legacy option and no encoder-option getter. `setopt_from_terminal` correctly imports canonical negotiation state. The [public terminal data API](https://github.com/ghostty-org/ghostty/blob/22fa134a07f5749edd9e92c6bc17d8aaf61843be/include/ghostty/vt/terminal.h) exposes Kitty flags, but no typed modifyOtherKeys state. ANSI/DEC mode access is not access to that CSI `>` protocol. Reading private flags or decoding formatter/snapshot/query bytes would create the prohibited second state interpretation.

## Decision

Do not change production worker encoding at this pin. Keep the unchanged raw selection gate independently failing. The diagnostic target may pass because it verifies the mismatch; its report must also say `selection_gate: FAIL` and `correction: BLOCKED_PUBLIC_SEAM`.

Require a future evaluated **public encoder compatibility option** that applies classic aliases only in the intended legacy profile and preserves negotiated behavior, or a sufficient typed effective-keyboard-state API plus a separately reviewed small policy. Such an API is a prerequisite, not an implementation delivered here. Do not assume a setter is a state getter, force Kitty/MOK off, or treat Kitty flags equal to zero as proof of ordinary legacy mode. A future policy must specify MOK levels, Kitty stacks, precedence/reset and Alt/Shift treatment before claiming support.

Reject these apparent fixes:

- Replacing returned CSI bytes or probing the encoder to infer modes: corrupts the single semantic authority and can change negotiated meaning.
- Always translating character chords into unmodified Tab/Enter/Escape: loses distinct Kitty character identity. Named keys remain separate intents.
- Passing transformed C0 bytes as text, clearing real modifiers or inventing consumed modifiers: violates event facts/contracts or drops the intended chord.
- Parsing output twice, reading private structs or patching/forking the dependency: exceeds the authorized boundary and maintenance budget.
- Redefining the legacy requirement as fixterms support: a product requirement change, not a repair. No such weakening was accepted.

## Supported experimental profile and refusals

The worker remains an **unadopted diagnostic press-only profile**. Named keys and explicit logical characters use one canonical encoder; selected Kitty and MOK transitions are exercised. This is not a supported product navigation capability. The private DTO has no physical layout, consumed-modifier, composing, release or repeat fields. Strict decoding refuses supplied action/unknown fields; modified non-ASCII characters and control-character text are refused. A latched unknown interpreter state refuses subsequent encoding. Those refusals do not make the three legacy aliases safe: they still return the wrong bytes for the product requirement, so the entire selection stays blocked.

The current scalar `Text` path is not a complete committed-text/IME interface. In Kitty report-all modes it generates encoded scalar presses. A native text-producing Shift event can require consumed-modifier facts that the DTO cannot express. Both synthetic human normalizers currently refuse character events; only their shared named-key subset is compared. Do not advertise full human/agent character parity or let a future frontend guess missing host information.

## Consequences and next decision

No dependency, native C/unsafe wrapper, protocol/schema, renderer or authority change is justified. Original X4 remains a distinct serializer failure. There is no new engine selection or EA2 work. Investigative fixtures, source hashes, append-only attempts and preservation checks make this stopping point reproducible.

The next bounded task should evaluate an explicitly chosen public API revision/policy against this corpus and the original raw gate, with separate consumed-modifier/layout/committed-text design. If no such seam is available, request an explicit change of constraints or revisit the existing BUILD/ADOPT/REDESIGN decision. Do not start another engine or claim that this investigation authorizes an upstream patch. Production adoption remains pending, with rendering, PTY delivery, isolation, accessibility, performance and recovery gates untouched.
