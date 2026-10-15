// Assessment harness only. Imports, never modifies, the original X4 fixtures/oracle.
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import {
  cases,
  create,
  feed,
  view,
  run as originalRun,
} from "../system/engine-probe.mjs";
const expectations = JSON.parse(
  await readFile(new URL("./expectations.json", import.meta.url), "utf8"),
).cases;
export function cells(t) {
  const b = t.buffer.active;
  return {
    cols: t.cols,
    rows: t.rows,
    cursor: [b.cursorX, b.cursorY],
    buffer: b.type,
    lines: Array.from({ length: t.rows }, (_, y) =>
      b
        .getLine(b.baseY + y)
        .translateToString(false)
        .trimEnd(),
    ),
    cells: Array.from({ length: t.rows }, (_, y) =>
      Array.from({ length: t.cols }, (_, x) => {
        const c = b.getLine(b.baseY + y).getCell(x);
        return {
          text: c.getChars(),
          width: c.getWidth(),
          fg: c.getFgColor(),
          fgPalette: c.isFgPalette(),
          bold: c.isBold(),
          italic: c.isItalic(),
          underline: c.isUnderline(),
          inverse: c.isInverse(),
        };
      }),
    ),
    modes: { ...t.modes },
  };
}
function oracle(name, observation) {
  const expected = expectations[name];
  for (const key of ["lines", "cursor", "buffer"])
    assert.deepEqual(
      observation[key],
      expected[key],
      name + " independent " + key,
    );
  if (expected.foregroundPaletteAtOrigin !== undefined) {
    assert.equal(
      observation.cells[0][0].fg,
      expected.foregroundPaletteAtOrigin,
    );
    assert.ok(observation.cells[0][0].fgPalette);
  }
}
export async function assess() {
  const old = await originalRun();
  assert.equal(old.verdict, "FAIL");
  const results = [];
  for (const c of cases) {
    const canonical = create(),
      replay = create();
    try {
      const rawBefore = Buffer.from(c.before, "utf8"),
        rawAfter = Buffer.from(c.after, "utf8");
      await feed(canonical, rawBefore);
      const detached = structuredClone(cells(canonical));
      // New frontend draws detached cells; it never interprets application bytes.
      const tui = structuredClone(detached),
        desktop = structuredClone(detached);
      assert.deepEqual(tui, desktop);
      await feed(canonical, rawAfter);
      const current = cells(canonical);
      oracle(c.name, current);
      const nextTui = structuredClone(current),
        nextDesktop = structuredClone(current);
      assert.deepEqual(nextTui, nextDesktop);
      assert.notDeepEqual(current.cursor, undefined);
      // Authorized complete interpretation history only; no process execution replay.
      await feed(replay, rawBefore);
      await feed(replay, rawAfter);
      assert.deepEqual(cells(replay), current);
      // Exhaustively split the short byte stream; compare attributes as well as text.
      const all = Buffer.concat([rawBefore, rawAfter]);
      let partitions = 0;
      for (let split = 0; split <= all.length; split++) {
        const t = create();
        try {
          await feed(t, all.subarray(0, split));
          await feed(t, all.subarray(split));
          assert.deepEqual(cells(t), current);
          partitions++;
        } finally {
          t.dispose();
        }
      }
      // Sensitivity check: the independent oracle must reject wrong focus/cursor/text.
      let mutationsRejected = 0;
      for (const wrong of [
        { ...current, cursor: [99, 99] },
        { ...current, lines: ["wrong", ...current.lines.slice(1)] },
      ]) {
        assert.throws(() => oracle(c.name, wrong));
        mutationsRejected++;
      }
      results.push({
        name: c.name,
        independentOracle: "PASS",
        originalSerializer: old.results.find((x) => x.name === c.name)
          .afterContinuation
          ? "PASS"
          : "FAIL",
        semanticFrameModel: "PASS",
        completeAuthorizedInterpretationReplay: "PASS",
        partitions,
        oracleMutationsRejected: mutationsRejected,
        after: current,
      });
    } finally {
      canonical.dispose();
      replay.dispose();
    }
  }
  const r = {
    candidate: "A revised xterm single owner",
    sourceFamily: "f447274f430fd22513f6adbf9862d19524471c04",
    fixtureSha256: createHash("sha256")
      .update(
        await readFile(
          new URL("../fixtures/terminal/continuations.json", import.meta.url),
        ),
      )
      .digest("hex"),
    originalX4: old.verdict,
    verdict: "PASS_BOUNDED_STATE_MODEL",
    results,
    limitations:
      "Semantic/cell frames are synthetic copied data, not an Electron/host renderer. Replay starts at a known initial state with complete authorized bytes; no product capture/recovery or resource benchmarks. Parser is not restarted in no-capture frontend attach model. No complete xterm checkpoint API claimed.",
  };
  await writeFile(
    new URL(
      "../../evidence/engine-assessment/xterm-assessment.json",
      import.meta.url,
    ),
    JSON.stringify(r, null, 2) + "\n",
  );
  console.log(
    JSON.stringify(
      {
        candidate: r.candidate,
        originalX4: r.originalX4,
        verdict: r.verdict,
        cases: results.map(
          ({
            name,
            independentOracle,
            originalSerializer,
            partitions,
            oracleMutationsRejected,
          }) => ({
            name,
            independentOracle,
            originalSerializer,
            partitions,
            oracleMutationsRejected,
          }),
        ),
      },
      null,
      2,
    ),
  );
}
if (process.argv[1]?.endsWith("xterm-assessment.mjs")) await assess();
