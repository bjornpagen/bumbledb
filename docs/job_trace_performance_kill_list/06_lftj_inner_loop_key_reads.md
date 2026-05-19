# 06 LFTJ Inner-Loop Borrowed-Key Optimization

Priority: P1

## Problem

After removing repeated planning/build waste, broad LFTJ queries spend most steady-state time in trie traversal. The hottest counters are key reads, seeks, and iterator movement.

Post-kill trace:

| Query | Key Reads/Run | Seeks/Run | Sample Avg |
|---|---:|---:|---:|
| `job_broad_movie_info_star` | `507,718` | `166,030` | `21.46ms` |
| `job_broad_cast_keyword_company` | `116,679` | `23,036` | `3.85ms` |

Even after factorized count suffixes, the engine still does many encoded key reads and clones.

## Technical Cause

Every key access clones into `EncodedOwned`:

`crates/bumbledb-lmdb/src/query.rs:3569-3575`

```rust
fn key_owned_opt(
    iter: &crate::SortedTrieIter<'_>,
    counters: &mut PlanCounters,
) -> Option<EncodedOwned> {
    let key = iter.key()?;
    counters.trie_key_reads += 1;
    Some(EncodedOwned::from_ref(key))
}
```

The leapfrog state repeatedly calls this during sort/search:

`crates/bumbledb-lmdb/src/query.rs:3522-3558`

```rust
let Some(current) = key_owned_opt(&iters[id], counters) else { ... };
if current == max {
    return Ok(());
}
iters[id].seek(max.as_ref());
counters.trie_seek += 1;
```

## Required Solution

Reduce per-key allocation/copy and repeated key reads in `LeapfrogState`.

### Borrowed Key Comparisons

Use borrowed byte slices for comparison where lifetimes permit:

```rust
fn key_ref_opt<'a>(iter: &'a SortedTrieIter<'a>, counters: &mut PlanCounters) -> Option<&'a [u8]>;
```

If direct borrowing is too hard because iterators move, store current key in a small fixed stack buffer for fixed-width types instead of heap-backed `EncodedOwned`.

### Cached Current Keys

Track current key per iterator inside `LeapfrogState`:

```rust
struct LeapfrogState {
    iter_ids: SmallParticipants,
    current_keys: SmallVec<[Option<EncodedOwned>; 8]>,
    ...
}
```

Refresh only after `next` or `seek`, not on every sort/search comparison.

### Specialized Fixed-Width Path

Most encoded values are fixed width 8 or 16 bytes. Add fast compare paths for these widths to avoid heap allocations.

## Strict Passing Criteria

- `job_broad_movie_info_star` key-read overhead reduces enough to cut steady avg by at least `30%` if factorized kernel has not already bypassed it.
- `trie_key_reads` either drops by `30%+` or remains but allocation/copy counters drop in alloc-profile trace.
- No correctness regressions in differential/property tests.

## Tests

- LFTJ query outputs match before/after for single, two, and three-level joins.
- `trie_key_reads` does not increase.
- Allocation-profile test shows fewer allocations/bytes in LFTJ execution for broad joins.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb sorted_trie query --all-targets
cargo run -p bumbledb-bench --release --features alloc-profile -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --query job_broad_movie_info_star --format json
```
