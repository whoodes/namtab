import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { normalize } from "../../apps/desktop/dist/input.js";
import { encodeText } from "../../packages/terminal-adapter/dist/index.js";

const profile = JSON.parse(
  readFileSync("tests/ea1/input-profile.json", "utf8"),
);
const event = (key, bits = 0) => ({
  key,
  location: 0,
  shiftKey: !!(bits & 1),
  ctrlKey: !!(bits & 2),
  altKey: !!(bits & 4),
  metaKey: false,
  isComposing: false,
  repeat: false,
});
test("desktop shares the synthetic named-key/modifier intent profile", () => {
  for (const item of profile.keys)
    for (let bits = 0; bits < 8; bits++) {
      assert.deepEqual(normalize(event(item.desktop, bits)), {
        kind: "key",
        key: item.key,
        shift: !!(bits & 1),
        alt: !!(bits & 4),
        ctrl: !!(bits & 2),
      });
    }
});
test("desktop refuses unavailable character/layout/composition/repeat facts", () => {
  for (const key of ["i", "m", "[", "界", "Unidentified"]) {
    assert.equal(normalize(event(key, 2)), undefined);
  }
  for (const extra of [
    { repeat: true },
    { isComposing: true },
    { metaKey: true },
  ]) {
    assert.equal(normalize({ ...event("Tab"), ...extra }), undefined);
  }
  // This synthetic API consumes keydown only. It has no keyup/action field;
  // native release/repeat support must not be inferred from it.
});

test("text and paste payload limits count UTF-8 bytes, not JS code units", () => {
  const modes = {
    applicationCursorKeysMode: false,
    applicationKeypadMode: false,
    bracketedPasteMode: true,
  };
  for (const kind of ["text", "paste"]) {
    const boundary = "😀".repeat(2048);
    assert.equal(
      encodeText({ kind, text: boundary }, modes).byteLength,
      8192 + (kind === "paste" ? 12 : 0),
    );
    for (const text of [boundary + "a", "界".repeat(2731)]) {
      assert.throws(
        () => encodeText({ kind, text }, modes),
        /ResourceExhausted/u,
      );
    }
  }
});
