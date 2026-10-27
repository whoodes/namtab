// Retained synthetic keydown oracle from the retired desktop input adapter.
// Test support only: no DOM, Electron, IME, runtime input authority or GUI claim.
export function normalize(event) {
  if (event.isComposing || event.repeat || event.metaKey) return undefined;
  const names = {
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
  let key = Object.hasOwn(names, event.key) ? names[event.key] : undefined;
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
