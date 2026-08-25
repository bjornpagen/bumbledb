# v:3 codec corpus

Cross-driver goldens for the representation-first codec. Both suites
decode these bytes to one value and re-encode them byte-identically.
The v:2 tree under `conformance/corpus/` is not this corpus.

Root: `crates/bumbledb-log/conformance/v3/`.

## Layout

| Path | What |
| --- | --- |
| `schemas.json` | Fixture descriptors both drivers assemble. `blank` is the zero-field ordinary relation. |
| `braids/` | Derived braid maps, one file per schema. |
| `batch/*.json` + `*.bin` | Wire batches. Sidecar names schema, fingerprint, expect, and the decoded value (or refusal). |
| `chain/` | `verifyChain` goldens over v:3 batch bytes. |
| `documents/{manifest,checkpoint,sidecar}/` | Canonical single-line document bytes (`.bin`) plus the decoded value (`.json`). |
| `fuzz/` | Materialised truncations and hostile splices, plus `storm.json` — the Rust mutation lane (`f9_fuzz.rs`) as a recipe the TS codec replays. |
| `inventory.json` | The case roster. |

`r_encode_short_prev.json` is encode-only (no `.bin`): a 2-byte `prev` is unconstructible as `[u8; 32]`.

## Grammar

- **Version is 3.** Batch `u16 LE` at offset 4 is `3`. Manifest, checkpoint, and sidecar documents begin `{"v":3,…}`. A well-formed `v:2` document or batch is `Version`.
- **Pending bytes are lowercase hex** of the codec's canonical batch rendering. Base64 is `Malformed` (`documents/sidecar/r_pending_base64`).
- **Every `u64`/`i64` that is not the `v` discriminator is a decimal string.** `9007199254740993` (`2^53+1`) and `18446744073709551615` cannot be a JSON number. A JSON-number `g`/`ts`/`writer`/`gen`, or a fractional string, is `Malformed`.
- **Intervals are half-open.** `start >= end` is `EmptyInterval`. A fixed-width interval whose end is not in the domain (the ceiling is not a value) is `IntervalOverflow` — `r_fixed_interval_overflow`, `r_fixed_interval_ray`.
- **A row vector cannot outrun its bytes.** A declared `row_count` or `op_count` the remaining bytes cannot back is `Truncated` immediately — `r_row_count_unbacked`, `r_op_count_unbacked`, `r_truncated_row`. Zero-field `Tick` with a nonzero count and no payload is `r_zero_width_rows`.
- **Strings are bytes-in, bytes-out.** A leading U+FEFF is a character (`ok_string_bom`), not a BOM the drivers strip. Invalid UTF-8 is `InvalidUtf8`.
- **Digests are 32 bytes / 64 lowercase hex.** A short `prev` cannot encode.

## Sidecar JSON

Ok batch:

```json
{ "expect": "ok", "schema": "kitchen", "fingerprint": "<64 hex>", "header": {…}, "ops": […] }
```

Refusal batch:

```json
{ "expect": "refusal", "schema": "kitchen", "fingerprint": "<64 hex>", "refusal": "Truncated" }
```

Document:

```json
{ "kind": "sidecar", "expect": "ok", "schema": "kitchen", "value": {…} }
```

`header.*` and `value.*` numeric fields are decimal strings. Fingerprints are the corpus's own `blake3("bumbledb-log corpus fingerprint: " + schema name)`.

## Fuzz

`fuzz/storm.json` pins the XorShift64, seeds, iteration counts, operator set, and golden list from `crates/bumbledb-log/tests/f9_fuzz.rs`. Materialised prefixes sit under `fuzz/batch/` and `fuzz/documents/` so a suite can assert typed in-bounds refusals without a PRNG. An accepted mutant must be a canonical fixpoint.

## Rulings

On representation the cutover subdirectory wins; 60 owns this seam.

- Batch wire version is 3 (one version number; parent `20-command-codec.md`'s `version 2` is dead text).
- `"v":3` stays a JSON number — the discriminator 70 spells. Every other u64/i64 is a decimal string.
- Checkpoint documents carry `"v":3` as the first field.
- Pending bytes are hex, never base64.
- This path is new so the v:2 suite is not rewritten mid-cutover.
