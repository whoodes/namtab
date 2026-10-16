# Native identity contract review data

These JSON files are hand-authored synthetic examples for the [authoritative native contract](../../../docs/engine/native-contract.md#n0-01-review-examples-and-follow-on-seams), drafted on 2026-10-16 for issue 7. This directory owns example provenance and integrity only; it is not another specification, executable harness, generated protocol schema or engine implementation.

- `valid-managed-workflow.json` describes a pipe execution, bounded observation, exact artifact registration and deferred review. Its source binding is an initial snapshot, not proven sealed input. The command is fictional and is never executed.
- `valid-event-cuts.json` describes independent session cuts, explicit dependencies, pipe observation limits and the four fact levels.
- `invalid-cases.json` contains valid JSON describing forbidden semantic changes/claims against those examples. Its prose expectations are proposed review oracles, not passing runtime tests or a new error vocabulary.

All IDs, arguments, output, artifact bytes and apparent prompt/control text are invented. There are no captured user sessions, credentials, environment values, home paths, upstream code or upstream conformance data. The artifact digest identifies only the included synthetic artifact bytes under the example's explicit artifact-retention authorization; it is not a default request/input hash or a redaction technique. No new dependency or project license is adopted.

Source rationale is the existing native contract and its linked Product 02/runtime/frontend requirements. Requirement mappings are R01/R02/R05 and V01/V02/V05. Each example records its assumptions; N0-02 request/receipt admission, N0-03 writer fencing and N0-06 public schemas remain separate work. Review must challenge attribution, completeness and stale subjects independently of the eventual implementation before these can become executable test oracles. Drafting and JSON parsing do not establish NG0 acceptance.

[manifest.json](manifest.json) records SHA-256 hashes of these JSON data files and their origin. It deliberately leaves the original shared fixture manifest and historical X4/EA1 evidence unchanged. To change an expectation, review the contract change, explain the reason, and then update this local manifest; never regenerate it merely to make an implementation pass. Raw user data must never be added here.
