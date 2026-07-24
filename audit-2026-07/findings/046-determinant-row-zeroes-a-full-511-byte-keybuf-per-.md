## determinant_row zeroes a full 511-byte KeyBuf per probe on the point-read hot lanes

category: perf | severity: medium | verdict: CONFIRMED | finder: perf:points

### Summary

The `U` determinant probe — the storage read behind every key-probe query execution (bench lanes `p1_by_id`, `p2_by_key`) and every keyed get (`p5_keyed_get`, `Snapshot::get`/`get_dyn`, `WriteTx` reads) — initializes a full `[0u8; 511]` stack buffer per call to compose a key that is 15–23 bytes on these lanes (1-byte namespace + 4-byte relation + 2-byte statement + an 8–16-byte determinant). This is the exact oversized-zeroing pattern the key codec's own module header bans ("no oversized zeroing (post-mortem §25)") and that the sibling `M` probe was already right-sized to avoid after in-repo measurement.

### Evidence (all verified against the working tree)

- `crates/bumbledb/src/storage/read/determinant_row.rs:22-24`:
  ```rust
  let mut key: KeyBuf = [0; MAX_KEY];
  let len = keys::determinant_key(&mut key, rel, statement, determinant);
  row_id_value(txn.env().data().get(txn.raw(), &key[..len])?)
  ```
  `MAX_KEY = 511` (`storage/keys.rs:27`), so this is an 8-cache-line memset per probe.
- `crates/bumbledb/src/storage/keys.rs:16-18` (module header, citing `docs/architecture/50-storage.md` § Key layout): "Writers fill a caller-provided `[u8; MAX_KEY]` scratch and return the written length — no oversized zeroing (post-mortem §25)". The codec keeps its half of the promise — `KeyWriter` (`keys.rs:245-273`) writes contiguously from byte 0 and returns the length; nothing ever reads the unwritten tail — but this caller re-introduces the zeroing the header banned.
- The measured precedent, `crates/bumbledb/src/storage/read/fact_row.rs:29-34`: "Right-sized stack buffer: this probe runs once per user write operation — zeroing 511 bytes for a 37-byte key was measurable waste" with `let mut key = [0u8; keys::MEMBERSHIP_KEY_LEN];`. The repo's own comment establishes that this exact memset, on a colder path, was measurable.
- Hot callers, all verified:
  - `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs:261` — the `Some(statement)` arm of the prepared key-probe path (p1/p2's timed lane). Note the caller already holds the determinant bytes in a reused scratch (`key_scratch: &mut Vec<u8>`), then `determinant_row` zeroes a fresh 511-byte buffer just to prepend the 7-byte header.
  - `crates/bumbledb/src/api/db/get.rs:334` (`fact_by_determinant`, the WriteTx keyed read), `crates/bumbledb/src/api/db/snapshot.rs:194` (`get_dyn`) and `snapshot.rs:243` (`Snapshot::get`, the p5 lane).
- The persistent scratch the fix can use exists: the prepared query owns `determinant_key: Vec<u8>` (`crates/bumbledb/src/api/prepared/build.rs:172`, threaded through `api/prepared/execute.rs:222,338` and `fixpoint.rs:362,513`).
- Signature precedent in the same file: `fact_key(buf: &mut [u8], ...)` (`keys.rs:285`) already takes a plain slice; `determinant_key` (`keys.rs:309-321`) is the one that demands a full `&mut KeyBuf`.

### Bench impact

`points` is the repo's tightest world by its own README (README.md:164-171): `p5_keyed_get` is a **1.00×** dead heat against SQLite's prepared point SELECT and `p2_by_key` is **1.50×**. On lanes measured in hundreds of nanoseconds, a 511-byte memset per probe is a fixed few-percent tax the codebase has already once measured and eliminated on a colder path (one probe per user write op vs. one per point read). LLVM cannot elide the memset: the written length is runtime and the buffer escapes into the LMDB FFI get.

### Corrections to the original finding

The suggested extension to the write lanes overstates their heat: `commit/write.rs:309` (`flush_escaped_fresh_ids`) and `:333` (`flush_counters`) zero one KeyBuf per commit flush, and `commit/applier.rs:282` (`next_row_id`) zeroes one only on the first lazy high-water read per relation per commit. The true per-fact-op applier path (`applier.rs:17,25,36,58,100`) reuses a persistent `self.key` scratch and never re-zeroes. The finding is a read-path finding; the write-side sites are cold and fine as-is.

### Suggested fix

Compose the full `U` key once in a caller-owned buffer instead of a fresh KeyBuf per probe:
1. Relax `keys::determinant_key` to take `&mut [u8]` (matching `fact_key`), or add a variant that extends a `Vec<u8>`.
2. On the prepared path, write the 7-byte `U | relation | statement` header into the persistent `determinant_key` scratch before `const_bytes` appends the determinant, and add a raw pre-keyed probe (`read::row_for_key(&txn, key_bytes)`) so no per-call buffer exists at all.
3. On the snapshot/get paths, either reuse the same pre-keyed probe with the existing per-call determinant Vec (prepend the header there) or use a right-sized stack cursor: header is fixed at 7 bytes and `MAX_DETERMINANT_WIDTH` bounds the rest, so a `KeyWriter` over an uninit-free, exactly-sized compose is straightforward.

This restores the keys.rs header's own contract (post-mortem §25) on the last hot caller still violating it.
