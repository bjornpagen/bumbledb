## Shared probe hash lacks a final avalanche: tail-padded bytes<N> keys collapse into one home bucket

category: perf | severity: high | verdict: CONFIRMED | finder: engine:colt

### Summary

`hash_words`/`hash_core` in `crates/bumbledb/src/exec/swar.rs` fold each key word as `h ^= w; h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15); h ^= h >> 29;` with no post-loop finalizer. Multiplication mod 2^64 only propagates input bits upward (bit i of a product depends only on bits 0..=i of the operands), and the single `h ^= h >> 29` reaches down only 29 bits. Therefore two keys whose difference is confined to bits >= p produce hashes identical in the low p−29 bits. Both probe structures derive their home index from the LOW bits, so any key family whose distinguishing bits sit high in the word piles into a handful of home buckets — in the worst realistic case, exactly one.

This case is not adversarial; it is the canonical encoding of every short `bytes<N>` column. `FixedBytesValue::padded()` zero-pads raw bytes at the TAIL (encoding.rs:104-109, "pad already zero by construction"), and the exec lane decodes each 8-byte chunk big-endian (dispatch/fact_word.rs:45, `u64::from_be_bytes`; the `FixedBytes` arm at :49-62). So a 2-4 character ticker / country / currency code puts all of its distinguishing bits in the top bytes of the word and constant zeros in the low bits — precisely the blind end of the hash. The ctrl tag, by contrast, is taken from the top 7 bits (swar.rs:50, `hash >> 57`), i.e. the well-mixed end.

### Evidence (all verified against the code)

- `crates/bumbledb/src/exec/swar.rs:18-26` (`hash_words`) and `:36-45` (`hash_core`): the per-word fold shown above; the function returns `h` immediately after the last round — no finalizer.
- Low-bit index derivation: `crates/bumbledb/src/exec/colt/probe.rs:114` and `:153` — `let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;`; `crates/bumbledb/src/exec/wordmap/probe.rs:54` — `let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;`. High-bit tag: `crates/bumbledb/src/exec/swar.rs:50`.
- No call site pre-mixes: grepped every `hash_words`/`hash_core` caller — colt `force.rs:97-98` (the force-pass insert), `grow.rs:30` (rehash), `select.rs:36/71`, wordmap `entry.rs`/`grow.rs`, and `image/cardinality.rs:49` (the distinct-word counter shares this hash, per swar.rs:13-16). All mask the raw fold output.
- Tail-zero big-endian encoding: `crates/bumbledb/src/encoding.rs:104-109`; `crates/bumbledb/src/exec/dispatch/fact_word.rs:44-56`.
- Empirical reproduction (exact constants, `rustc -O`), 1000 distinct keys per family, masked at colt-realistic sizes:
  - bytes<2>-style (bits 48-63 vary): **1 home bucket** at 512 AND 4096 buckets, max load 1000.
  - bytes<3>-style (bits 40-63 vary): **1 home** at 512 buckets; 2 homes at 4096.
  - bytes<4>-style (bits 32-63 vary): 64 of 512 homes (8x pile-up), 512 of 4096.
  - control (low-bit-varying integers): 433 of 512 homes, max load 6.
  This matches the algebra: low p−29 bits constant → bytes<2> freezes 19 low bits, bytes<3> 11 bits, bytes<4> 3 bits.
- Degeneration mechanism: colt buckets overflow bucket-linearly (`probe.rs:141`, `b = (b + 1) & nbm`) and inserts land at the first empty slot found from the home bucket (`force.rs:97-102`), so a single shared home forms one ever-growing chain walked per insert — O(n²/8) group loads across the force pass. The wordmap window scan (`probe.rs:81`, `idx = (idx + WINDOW) & mask`) forms the same contiguous run for seen-sets/group maps keyed on such a column. Growth laws (colt `nbuckets = next_pow2(guess·5/16)` at force.rs:21-25, load cap 0.4) do not help: doubling the table does not separate keys whose hashes agree in more low bits than the mask width.
- Spec check: the Free Join paper (docs/free-join-paper, COLT section) presents the lazy trie as a hash-map-per-level structure with expected O(1) probes; hash quality is assumed, not specified — so this is an implementation defect, not a divergence question. The one-definition-shared-hash doctrine stated in swar.rs:1-16 means a single fix covers colt, wordmap, and the image cardinality counter simultaneously.
- The pinned test the fix must preserve exists: `crates/bumbledb/src/exec/wordmap/tests/contracts.rs:150-159` (`hash_core_is_identical_to_hash_words`) — apply the same finalizer to both functions and it stays green.

### Bench impact

Any relation with a bytes<N ≤ 8> fixed-code column used as a join key, group key, or deduped find projection: forcing that trie level and deduping those bindings degrade toward quadratic. For 2-3-byte codes the collapse is total (one home bucket at any realistic table size); for 4-byte codes it is an 8x pile-up (the original finding's "ONE bucket" slightly overstates the 4-byte case — the only correction). 10^6 rows of distinct short codes turn ~10^6 expected bucket-group loads into ~10^11-10^12 in the force pass alone. Integer, interned-string, and bool/enum keys are unaffected today (their entropy is in the low bits) and would pay one extra multiply+shift per hash after the fix.

### Suggested fix

Add a full-avalanche finalizer once, after the fold, in BOTH `hash_words` and `hash_core` (splitmix/fmix style, e.g. `h = h.wrapping_mul(C2); h ^= h >> 32;`), keeping the two functions hash-identical so the pinned contract test at wordmap/tests/contracts.rs:155 survives. One definition in swar.rs fixes colt home buckets, wordmap windows, and the image cardinality counter together. The alternative representation-level fix — deriving the home index from the high bits the way `ctrl_tag` already does — also works but correlates index and tag unless offset carefully; the shared finalizer is smaller and keeps them decorrelated.
