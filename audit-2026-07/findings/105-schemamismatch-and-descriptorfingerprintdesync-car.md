## SchemaMismatch and DescriptorFingerprintDesync carry both fingerprints but Display renders neither

category: incoherence | severity: low | verdict: CONFIRMED | finder: engine:interval-allen
outcome: fixed b7ddb7e0

### Summary

The error taxonomy documents a rendering contract — structured variants "carry data payloads, not nested errors … and the structured detail renders through `Display`" (`crates/bumbledb/src/error.rs:1171-1176`). Most variants honor it: `FormatMismatch` prints `found`/`expected`, `GenerationMoved` prints both generations, `CounterDesync` prints `claimed`/`witness`. The two fingerprint-carrying variants break it: `Error::SchemaMismatch { found, expected }` and `CorruptionError::DescriptorFingerprintDesync { fingerprint, descriptor_hash }` both destructure with `{ .. }` and print fixed, payload-free sentences — discarding exactly the 32-byte values that answer the diagnostic question the error exists to raise.

### Evidence (verified against the code)

- `crates/bumbledb/src/error/display.rs:853-855` — `Self::SchemaMismatch { .. } => { write!(f, "the compiled schema's fingerprint is not the stored one") }`. Payload dropped.
- `crates/bumbledb/src/error/display.rs:242-245` — `Self::DescriptorFingerprintDesync { .. } => write!(f, "the persisted schema descriptor hashes to something other than the stored fingerprint")`. Both hashes dropped.
- Payloads exist: `crates/bumbledb/src/error.rs:1186-1189` (`found`/`expected: SchemaFingerprint`) and `error.rs:112-117` (`fingerprint`/`descriptor_hash: [u8; 32]`, each with its own doc comment naming which value it is).
- Sibling discipline: `display.rs:847-851` (FormatMismatch renders `{found}`/`{expected}`), `display.rs:908-913` (GenerationMoved renders `({witnessed} → {current})`), `display.rs:227-235` (CounterDesync renders `{claimed}`/`{witness}`), `display.rs:212-221` (WrongFactWidth renders both widths).
- Contract: `crates/bumbledb/src/error.rs:1171-1176`, quoted above.
- `SchemaFingerprint` is `pub struct SchemaFingerprint(pub [u8; 32])` with derived Debug only (`crates/bumbledb/src/schema/fingerprint.rs:41`) — no Display impl in the crate. The crate's own tests hand-roll a `hex_of` helper (`fingerprint.rs:324-329`) to render fingerprints as hex, demonstrating both that the rendering is trivial and that the need already exists in-repo.
- No doc records a decision to omit the bytes: `docs/architecture/70-api.md` (§ store contracts, lines 333-337 and 435-436) specifies when these errors are raised but says nothing about their Display shape; the only stated rendering policy is the error.rs contract they violate.
- Partial mitigation, noted for completeness: `Error` derives `Debug` (error.rs:1177), so `{:?}` formatting does expose the raw arrays — but Display is the documented contract and the form hosts log.

### Failure scenario

A host opens a store under the wrong build of an evolving schema and logs the error's Display string: "the compiled schema's fingerprint is not the stored one". With several deployed schema versions, the log line cannot say which fingerprint the store carries, so the operator must run the out-of-band `exhume` workflow (`crates/bumbledb/src/api/db/exhume.rs`) against the store to learn a value the error it already received owns. Same story for `DescriptorFingerprintDesync` during exhume itself: the corruption report names neither the stored fingerprint nor the recomputed hash.

### Suggested fix

Give `SchemaFingerprint` a hex `Display` (promote the test-only `hex_of` fold at `fingerprint.rs:324-329`, or render a short prefix such as the first 8 bytes), then render the payloads in both arms, matching `FormatMismatch`'s discipline — e.g. `stored schema fingerprint {found}, this build's schema is {expected}` and `the persisted schema descriptor hashes to {descriptor_hash}, the stored fingerprint is {fingerprint}`.
