// Bounded seeded partition stress, not libFuzzer coverage or application execution.
import assert from "node:assert/strict";
import { create, feed, view, cases } from "../system/engine-probe.mjs";
let seed = 0x51a7e;
const next = () => {
  seed ^= seed << 13;
  seed ^= seed >>> 17;
  seed ^= seed << 5;
  return seed >>> 0;
};
let runs = 0;
for (const c of cases) {
  const input = new TextEncoder().encode(c.before + c.after);
  const baseline = create();
  await feed(baseline, input);
  const expected = view(baseline);
  baseline.dispose();
  for (let i = 0; i < 32; i++) {
    const engine = create();
    try {
      for (let offset = 0; offset < input.length; ) {
        const end = Math.min(input.length, offset + 1 + (next() % 7));
        await feed(engine, input.subarray(offset, end));
        offset = end;
      }
      assert.deepEqual(view(engine), expected);
    } finally {
      engine.dispose();
    }
    runs++;
  }
}
console.log(
  `${runs} deterministic parser partition runs passed; seed=0x51a7e, six synthetic cases. No execution replay claim.`,
);
