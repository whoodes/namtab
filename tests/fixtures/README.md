# Sanitized shared fixtures

All fixture payloads are authored synthetic data in this session, not captured user output. `input/request.json` is shared by Rust decoding, service admission and TS shape tests. `terminal/continuations.json` supplies canonical output and subsequent output at the attachment cut; it deliberately includes a known failing serializer continuation and an incomplete control sequence.

No upstream conformance corpus is vendored. See `manifest.json` for SHA-256 integrity and generator origin. Do not regenerate expected results simply to make an engine change pass. Never place real prompts, tokens, home paths, environment dumps or user transcripts here.
