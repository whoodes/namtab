import { CanonicalProbe } from "@project-x/terminal-adapter/node";
// In-memory observation utility; no managed PTY, sockets, capture or launch authority.
export async function inspectSynthetic(bytes: Uint8Array) {
  const engine = new CanonicalProbe(12, 5);
  try {
    await engine.feed(bytes);
    return { observation: engine.observe(), replies: engine.drainReplies() };
  } finally {
    engine.dispose();
  }
}
