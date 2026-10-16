// Test-only observer of the OUTER host TUI's ANSI paint, never inner canonical
// state, input, replies or authority. Reuse the locked reference dependency;
// neither frontend nor the owned engine imports this helper.
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
const reference = createRequire(
  new URL("../../packages/terminal-adapter/package.json", import.meta.url),
);
const { Terminal } = reference("@xterm/headless");
export async function outerScreen(bytes, cols = 110, rows = 38) {
  if (
    bytes.byteLength > 2_097_152 ||
    !Number.isInteger(cols) ||
    !Number.isInteger(rows) ||
    cols < 2 ||
    cols > 160 ||
    rows < 2 ||
    rows > 60
  )
    throw new Error("outer test observation exceeds finite profile");
  const terminal = new Terminal({
    cols,
    rows,
    scrollback: 0,
    allowProposedApi: true,
  });
  try {
    await new Promise((done) => terminal.write(bytes, done));
    // Public proposed buffer read API, tied to pinned headless 6.0.0. No
    // serialization/checkpoint/private field and no onData/reply route.
    const buffer = terminal.buffer.active;
    return Array.from(
      { length: rows },
      (_, row) =>
        buffer.getLine(buffer.baseY + row)?.translateToString(true) ?? "",
    );
  } finally {
    terminal.dispose();
  }
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const chunks = [];
  let length = 0;
  for await (const chunk of process.stdin) {
    length += chunk.length;
    if (length > 2_097_152) throw new Error("outer test log too large");
    chunks.push(chunk);
  }
  process.stdout.write(
    JSON.stringify(await outerScreen(Buffer.concat(chunks))) + "\n",
  );
}
