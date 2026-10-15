import type { LogicalKey } from "@project-x/terminal-adapter";
// Synthetic keydown normalization only; no Electron window/DOM/IME implementation.
export interface KeyEvent {
  key: string;
  location: number;
  shiftKey: boolean;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  isComposing: boolean;
  repeat: boolean;
}
export function normalize(event: KeyEvent): LogicalKey | undefined {
  if (event.isComposing || event.repeat || event.metaKey) return undefined;
  const names: Record<string, LogicalKey["key"]> = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Tab: "Tab",
    Enter: "Enter",
    Escape: "Escape",
    Backspace: "Backspace",
    F1: "F1",
    F12: "F12",
  };
  let key = names[event.key];
  if (key === "Enter" && event.location === 3) key = "KeypadEnter";
  if (key === undefined) return undefined;
  return {
    kind: "key",
    key,
    shift: event.shiftKey,
    alt: event.altKey,
    ctrl: event.ctrlKey,
  };
}
