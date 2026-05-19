# 02 Panic, Unwrap, And Smell Cleanup Results

**Completed Runtime Cleanup**
- Replaced `SortedTrieIter::key` panic behavior with an `Option<EncodedRef<'_>>` API.
- Updated `key_owned`, `LeapfrogState::key`, `LeapfrogState::init`, `LeapfrogState::next`, and `LeapfrogState::search` to propagate typed internal errors when trie keys are unexpectedly unavailable.
- Kept `trie_key_reads` accounting aligned with actual successful key reads.
- Rewrote hash-trie insertion to avoid the previous `unreachable!()` after replacing a node with `HashNode::Inner`.
- Replaced aggregate finish unwraps with typed internal errors for missing aggregate group keys or states.
- Removed the remaining `let _ = query` unused suppression from `build_hash_atom_indexes` by removing the unused parameter.

**Completed Benchmark CLI Cleanup**
- `Config::from_env()` now returns `Result<Option<Config>, Box<dyn std::error::Error>>`.
- Missing option values, invalid numeric values, invalid output format, and unknown args return typed input errors.
- `--help` prints usage and returns `Ok(None)` without process exit or panic.
- Benchmark row conversion helpers now return typed errors instead of panicking on missing or mismatched fields.

**Completed Test Cleanup**
- Converted crate tests, integration tests, property tests, and trybuild fixtures away from direct `unwrap`, `expect`, and panic-oriented helpers.
- Updated test helpers to return typed errors for missing diagnostics, missing fields, missing aggregate state, and row conversion mismatches.
- Updated `scan_escape.stderr` after removing `.unwrap()` from the compile-fail fixture.

**Smell Checks**
```sh
rg -n 'let _ = query|unwrap\(|panic!\(|todo!\(|unimplemented!\(|unreachable!\(|dbg!\(' crates fuzz --glob '*.rs'
rg -n '#\[allow\(' crates fuzz --glob '*.rs'
```

Both commands return no matches.

`expect(` appears only as approved `#[expect(..., reason = "...")]` lint annotations from PRD 01.

**Benchmark CLI Error Check**
```sh
cargo run -p bumbledb-bench -- --format invalid
```

Observed error:

```text
Error: Custom { kind: InvalidInput, error: "unknown --format invalid" }
```

**Benchmark Smoke**
Focused command:

```sh
BUMBLED_BENCH_REPEATS=3 scripts/bench-focused.sh
```

Exact release gate command:

```sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

Latest post-cleanup release gate run:

| Dataset | Query | Rows | Bumbledb Avg Us | Gate |
|---|---|---:|---:|---|
| ledger | postings_for_holder_range | 3 | 171 | pass |
| ledger | balances_by_instrument | 3 | 234 | pass |
| ledger | tag_lookup_join | 10000 | 18336 | pass |
| sailors | red_boat_sailors | 10000 | 48783 | pass |
| sailors | sailor_range_reserves | 5 | 52 | pass |
| sailors | high_rating_red_boats | 6660 | 12184 | pass |
| joinstress | chain4_from_a | 1 | 110 | pass |
| joinstress | triangle_count | 1 | 16861 | pass |
| tpch | revenue_by_customer_range | 2000 | 68619 | pass |
| tpch | supplier_nation_orders | 5716 | 53298 | pass |

Structural counters remain clean: `cursor_seeks=0`, `rows_scanned=0`, and `dictionary_reverse_lookups=0` for every focused query.

Timing note: the first focused run after replacing panic-based key reads showed `triangle_count` noise above the PRD 00 baseline. After removing an accidental key-vector allocation and using a non-panicking key fast path, the final release gate run is within benchmark noise of the PRD 00 timing baseline while preserving typed key-read failures.

**Verification**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `scripts/check-cutover.sh`
- `scripts/check-prd-map.sh`
- `scripts/check-performance-kill-list.sh`
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown`
- `BUMBLED_BENCH_REPEATS=3 scripts/bench-focused.sh`

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/03_query_observability_data_model.md`
