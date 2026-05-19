# 07 Static-Empty Frontend Cache And Proof Instrumentation

Priority: P1

## Problem

`job_q33_linked_series_companies` is now `StaticEmpty`, with no planning or executor runtime, but it still averages `69us` traced versus SQLite `61us`. The trace cannot fully explain the remaining time:

- visible normalize/image/validate/encode spans are about `22.6%` of sample time,
- about `77.4%` is unattributed parent time,
- static proof scans no counters and has no span.

## Technical Cause

Static empty proof happens before prepared plan caching and has no diagnostics:

`crates/bumbledb-lmdb/src/query.rs:1311-1331`

```rust
if normalized.inputs.is_empty()
    && static_literal_atoms_prove_empty(image.as_ref(), &normalized, &encoded_inputs)?
{
    let mut plan = static_empty_plan(...);
    ...
    return Ok(QueryOutput { rows: Vec::new(), plan });
}
```

The proof scans rows and returns only a boolean:

`crates/bumbledb-lmdb/src/query.rs:1432-1460`

```rust
for row in 0..relation.row_count {
    if static_atom_row_matches(relation, atom, RowId(row as u32), inputs)? {
        matched = true;
        break;
    }
}
```

## Required Solution

Cache static-empty decisions and instrument proof work.

### Static Empty Cache

Attach a `StaticQueryProofCache` to `QueryImage`, keyed by normalized query fingerprint and encoded literals.

Cached value:

```rust
enum StaticProofResult {
    Empty { proving_atom: AtomId, rows_scanned: u64 },
    Unknown,
}
```

### Counters

Add to `PlanCounters`:

```rust
static_empty_atoms_checked: u64
static_empty_rows_scanned: u64
static_empty_cache_hits: u64
static_empty_cache_misses: u64
```

### Spans

Add `bumbledb.query.static_empty.prove` with fields:

- atoms checked,
- rows scanned,
- proving relation,
- cache hit/miss.

## Strict Passing Criteria

- `job_q33_linked_series_companies` steady average drops from `~69us` to `<40us` traced or untraced.
- Static proof counters appear in `explain()` and benchmark JSON.
- Repeated q33 executions show static-empty cache hits after first proof.
- No result changes.

## Tests

- Static-empty cache hits on second execution.
- Cache invalidates across write/QueryImage `tx_id` changes.
- Counters report rows scanned and proving atom.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb static_empty --all-targets
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --query job_q33_linked_series_companies --format json
```
