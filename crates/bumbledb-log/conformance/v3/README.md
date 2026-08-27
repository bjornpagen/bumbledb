# v:3 codec corpus

Cross-driver goldens for the representation-first codec. Both suites
decode these bytes to one value and re-encode them byte-identically.
Root: `crates/bumbledb-log/conformance/v3/`.

## Layout

| Path | What |
| --- | --- |
| `schemas.json` | Fixture descriptors both drivers assemble. `blank` is the zero-field ordinary relation. |
| `braids/` | Derived braid maps, one file per schema. |
| `batch/*.json` + `*.bin` | Wire batches. Sidecar names schema, fingerprint, expect, and the decoded value (or refusal). |
| `chain/` | `verifyChain` goldens over v:3 batch bytes. |
| `documents/{manifest,checkpoint,sidecar}/` | Canonical binary document records (`.bin`) plus the decoded value (`.json`). |
| `lease/` | `LEASE/1` lease-body goldens, plus `placement.json` — the fs-lock placement table (`~lease/{key}/{n}` tokens, `~head` pointer, TTL constants). |
| `counter/` | id-lease counter body goldens: canonical decimal ASCII u64 in, typed `Counter` refusal otherwise. |
| `scratch/` | ckpt-scratch body goldens: version byte `3` + 32-byte digest; any other body parses to nothing on both drivers. |
| `keys/` | Key-grammar tables: `grammar.json` (named accept/refuse spellings) and `tilde-family.json` (the 15-point reserved tilde set, NFKC-closed). |
| `machine-constants.json` | Protocol constants both machines assert, one value per fact (`wait_for_poll_ms`, `heartbeat_every`, `loss_bound`, `lease_width`). |
| `fuzz/` | Materialised truncations and hostile splices, plus `storm.json` — the Rust mutation lane (`f9_fuzz.rs`) as a recipe the TS codec replays. |
| `inventory.json` | The case roster. |
| `surfaces.json` | The surface roster: every protocol surface names its golden pins (the census pin lane holds the two-way match). |

`r_encode_short_prev.json` is encode-only (no `.bin`): a 2-byte `prev` is unconstructible as `[u8; 32]`.

## Grammar

- **Version is 3.** The batch wire spells it `u16 LE` at offset 4 (after the `BDBL` magic); every document — manifest, checkpoint, sidecar, ckpt-scratch — opens with version byte `3`. A well-formed version-2 body is `Version`.
- **Documents are binary records**: version byte, `u32 LE` counts and raw braid ids, `u64 LE` numbers, raw 32-byte digests, and optional digests as a presence byte (`0x00` absent, `0x01` + 32 bytes). Manifest: version byte, fingerprint digest, optional checkpoint digest. Checkpoint: version byte, `u32 LE` braid count, `(braid u32 LE, g u64 LE, hash digest, ts u64 LE)` entries, catalog digest, `writer u64 LE`, optional prev digest. Sidecar: version byte, `u32 LE` chain count, `(braid u32 LE, g u64 LE, prev digest, ts u64 LE)` entries, then the pending arm — the absence byte, or the presence byte + `braid u32 LE`, `gen u64 LE`, `u32 LE` length, and the held batch bytes verbatim.
- **Braid entries ascend in raw-id order** (which leaves a duplicate no place to stand). An id outside the schema's own decomposition is `UnknownBraid`; a checkpoint whose braid set is not exactly the derived set is `BraidSet`; trailing bytes after any record are `Malformed`/`TrailingBytes` at the offending offset.
- **Intervals are half-open.** `start >= end` is `EmptyInterval`. A fixed-width interval whose end is not in the domain (the ceiling is not a value) is `IntervalOverflow` — `r_fixed_interval_overflow`, `r_fixed_interval_ray`.
- **A row vector cannot outrun its bytes.** A declared `row_count` or `op_count` the remaining bytes cannot back is `Truncated` immediately — `r_row_count_unbacked`, `r_op_count_unbacked`, `r_truncated_row`. Zero-field `Tick` with a nonzero count and no payload is `r_zero_width_rows`.
- **Strings are bytes-in, bytes-out.** A leading U+FEFF is a character (`ok_string_bom`), not a BOM the drivers strip. Invalid UTF-8 is `InvalidUtf8`.
- **Digests are 32 raw bytes on the wire, 64 lowercase hex in sidecars.** A short `prev` cannot encode.
- **Lease bodies are strict-canonical.** `Lease::parse` accepts exactly the `LEASE/1\n{holder}\n{token}\n{expires}\n` bytes `encode` renders: a body missing its final newline (`r_no_final_newline`), a CRLF-terminated body (`r_crlf`), or a non-canonical decimal refuses.

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

- One version number: the batch wire and every document spell version 3.
- Sidecar `.json` numeric fields are decimal strings — `9007199254740993`
  (`2^53+1`) and `18446744073709551615` cannot be JSON numbers; the wire
  carries them `u64 LE`/`i64 LE`.
- A held pending batch rides the sidecar document verbatim, bytes for
  bytes; the document never re-spells the batch grammar.
