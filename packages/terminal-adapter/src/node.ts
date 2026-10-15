import headless from "@xterm/headless";
const { Terminal } = headless;
// Feed-only characterization seam, not an IPC service or a certified parser sandbox.
export class CanonicalProbe {
  #terminal: InstanceType<typeof Terminal>;
  #busy = false;
  #prefix = 0;
  #unknown = false;
  #replies: { prefix: number; bytes: string }[] = [];
  constructor(cols = 80, rows = 24) {
    if (
      cols < 1 ||
      rows < 1 ||
      !Number.isInteger(cols) ||
      !Number.isInteger(rows) ||
      cols * rows > 80000
    )
      throw new Error("ResourceExhausted");
    this.#terminal = new Terminal({
      cols,
      rows,
      scrollback: 20,
      allowProposedApi: true,
    });
    this.#terminal.onData((bytes) => {
      if (!this.#busy || this.#replies.length >= 64 || bytes.length > 4096) {
        this.#unknown = true;
        return;
      }
      this.#replies.push({ prefix: this.#prefix + 1, bytes });
    });
  }
  async feed(bytes: Uint8Array): Promise<void> {
    if (this.#unknown || this.#busy || bytes.byteLength > 16384)
      throw new Error("Unavailable");
    this.#busy = true;
    await new Promise<void>((resolve) =>
      this.#terminal.write(Uint8Array.from(bytes), resolve),
    );
    this.#prefix++;
    this.#busy = false;
  }
  resize(cols: number, rows: number): void {
    if (
      this.#busy ||
      this.#unknown ||
      !Number.isInteger(cols) ||
      !Number.isInteger(rows) ||
      cols < 1 ||
      rows < 1 ||
      cols * rows > 80000
    )
      throw new Error("Unavailable");
    this.#terminal.resize(cols, rows);
    this.#prefix++;
  }
  observe() {
    if (this.#busy || this.#unknown) throw new Error("UnknownState");
    const b = this.#terminal.buffer.active;
    const lines = Array.from({ length: this.#terminal.rows }, (_, row) =>
      Object.freeze(
        Array.from({ length: this.#terminal.cols }, (_, col) => {
          const cell = b.getLine(b.baseY + row)?.getCell(col);
          return Object.freeze({
            text: cell?.getChars() ?? "",
            width: cell?.getWidth() ?? 1,
            fg: cell?.getFgColor() ?? 0,
            bg: cell?.getBgColor() ?? 0,
          });
        }),
      ),
    );
    return Object.freeze({
      prefix: this.#prefix,
      cols: this.#terminal.cols,
      rows: this.#terminal.rows,
      buffer: b.type,
      cursor: Object.freeze([b.cursorX, b.cursorY]),
      modes: Object.freeze({ ...this.#terminal.modes }),
      lines: Object.freeze(lines),
    });
  }
  drainReplies() {
    return this.#replies.splice(0);
  }
  dispose() {
    this.#terminal.dispose();
    this.#unknown = true;
  }
}
