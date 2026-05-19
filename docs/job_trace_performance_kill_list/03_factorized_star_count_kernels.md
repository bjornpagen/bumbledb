# 03 Factorized Star Count Kernels

Priority: P0

## Problem

Broad count joins are still spending nearly all steady-state time in LFTJ traversal. The first kill-list round added final-suffix count range pushdown, but it still iterates most of the join lattice.

Post-kill trace evidence:

| Query | Sample Avg | LFTJ Execute Share | Key Reads/Run | Seeks/Run | Counted Bindings/Run |
|---|---:|---:|---:|---:|---:|
| `job_broad_movie_info_star` | `21.46ms` | `98.8%` | `507,718` | `166,030` | `47,034` |
| `job_broad_cast_keyword_company` | `3.85ms` | `95.2%` | `116,679` | `23,036` | `11,009` |

Both queries return one aggregate row. The engine should not need hundreds of thousands of trie operations to compute one count.

## Technical Cause

LFTJ still recursively binds most variables:

`crates/bumbledb-lmdb/src/query.rs:3301-3387`

```rust
while !leapfrog.at_end {
    let value = leapfrog.key(&self.runtime.iters, &mut self.plan.summary.counters)?;
    self.plan.summary.counters.variable_candidates += 1;
    ...
    self.execute(depth + 1)?;
    ...
    leapfrog.next(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
}
```

The current count suffix optimization only activates at the final variable:

`crates/bumbledb-lmdb/src/query.rs:3396-3424`

```rust
if depth + 1 != self.plan.variable_order_ids.len() || !self.plan.comparisons.is_empty() {
    return false;
}
...
while !leapfrog.at_end {
    let _ = leapfrog.key(...)?;
    count = count.saturating_add(1);
    leapfrog.next(...)?;
}
```

This avoids leaf sink emissions but does not avoid broad traversal.

## Required Solution

Add direct factorized count kernels for central-star count queries.

### Shape Detection

Detect a query where:

- Output is count-only aggregate with no group vars.
- There is a central variable appearing in multiple fact atoms.
- Other variables are independent fanouts from the central variable or dimension existence filters.
- No unsafe predicates or repeated-variable constraints break multiplicity.

For `job_broad_movie_info_star`, central variable is `?movie`.

### Execution Strategy

For each central value:

1. Ensure all required central fact atoms have rows.
2. Compute per-atom fanout counts under the central prefix.
3. Apply dimension filters or FK existence checks.
4. Multiply fanouts.
5. Sum into aggregate via `emit_count_range`.

Pseudo-code:

```rust
let mut total = 0u64;
for movie in central_movie_intersection() {
    let cast = count_cast_info(movie, filters);
    let companies = count_movie_companies(movie, filters);
    let keywords = count_movie_keyword(movie, filters);
    let info = count_movie_info(movie, filters);
    let info_idx = count_movie_info_idx(movie, filters);
    total += cast * companies * keywords * info * info_idx;
}
sink.emit_count_range(empty_binding, total);
```

### Use Prefix Counts

Use existing `HashTrieIndex::count(prefix)` where hash indexes are appropriate, or add `SortedTrieIndex::count_prefix` for sorted indexes.

`crates/bumbledb-lmdb/src/hash_trie.rs:214-225`

```rust
fn count(&self, prefix: &[EncodedRef<'_>]) -> usize {
    find_node(&self.root, prefix).map_or(0, count_node)
}
```

### FK Dimension Elimination

If fact refs are guaranteed valid, dimension atoms like `Keyword(id: ?keyword)` should not add a fanout. They are existence checks and can be removed from the count kernel.

## Strict Passing Criteria

- `job_broad_movie_info_star` steady average drops from `~21.46ms` to `<8ms`.
- `job_broad_movie_info_star` `trie_key_reads` drops by at least `80%`.
- `job_broad_cast_keyword_company` steady average drops from `~3.85ms` to `<2.5ms`.
- `bindings_yielded` remains `0`; `factorized_counted_bindings` or a new `factorized_count_kernel_bindings` reports the counted multiplicity.
- Results match current output exactly.

## Tests

- Synthetic star join count equals reference enumeration.
- Multiplicity with multiple fanouts per spoke is correct.
- Repeated variables disable kernel and fall back.
- Predicates not understood by the kernel disable it.
- FK dimension elimination preserves count.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb aggregate count --all-targets
cargo test -p bumbledb-bench --all-targets
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --query job_broad_movie_info_star --query job_broad_cast_keyword_company --format json
```
