// Intent normalization only. No universal semantic navigation or byte encoder.
export interface LogicalKey {
  kind: "key";
  key:
    | "Up"
    | "Down"
    | "Left"
    | "Right"
    | "Tab"
    | "Enter"
    | "Escape"
    | "Backspace"
    | "F1"
    | "F12"
    | "KeypadEnter";
  shift: boolean;
  alt: boolean;
  ctrl: boolean;
}
export type Intent = LogicalKey | { kind: "text" | "paste"; text: string };
export interface Modes {
  applicationCursorKeysMode: boolean;
  applicationKeypadMode: boolean;
  bracketedPasteMode: boolean;
}
export function encodeText(intent: Intent, modes: Modes): Uint8Array {
  if (intent.kind === "key")
    throw new Error(
      "UnsupportedCapability: complete logical-key encoder failed feasibility",
    );
  if (intent.kind === "text" && /[\u0000-\u001f\u007f]/u.test(intent.text))
    throw new Error(
      "InvalidRequest: control character requires key/paste intent",
    );
  if (/[\uD800-\uDFFF]/u.test(intent.text))
    throw new Error("InvalidRequest: unpaired surrogate");
  if (intent.text.length > 8192) throw new Error("ResourceExhausted");
  // Bound payload bytes as well as pre-encoding allocation. UTF-16 code units
  // alone allow multibyte Unicode payloads to exceed the 8 KiB input limit.
  const payload = new TextEncoder().encode(intent.text);
  if (payload.byteLength > 8192) throw new Error("ResourceExhausted");
  // Do not manufacture a trusted paste boundary from embedded delimiters.
  if (intent.kind === "paste" && /\x1b\[20[01]~/u.test(intent.text))
    throw new Error("UnsupportedCapability: embedded paste boundary");
  if (intent.kind === "paste" && modes.bracketedPasteMode)
    return new TextEncoder().encode("\x1b[200~" + intent.text + "\x1b[201~");
  return payload;
}
