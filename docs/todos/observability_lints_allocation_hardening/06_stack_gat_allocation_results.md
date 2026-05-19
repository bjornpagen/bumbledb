# 06 Stack, GAT, And Hot-Path Allocation Cleanup Results

**Completed Hot-Path Refactors**
- Added `smallvec` as a workspace dependency for stack-backed hot buffers.
- Changed `EncodedValue` to store fixed-width `EncodedOwned` instead of heap-backed `Vec<u8>`.
- Changed `EncodedBinding` to use `SmallVec<[Option<EncodedValue>; 8]>`.
- Changed projection and aggregate output grouping keys to use stack-backed `SmallVec` rows.
- Changed hash prefix and encoded-ref temporary buffers to stack-backed `SmallVec`.
- Precomputed LFTJ and HashProbe participants by variable, avoiding per-depth participant scans and heap `Vec` allocation for common small joins.
- Changed `LeapfrogState` iterator IDs to stack-backed `SmallVec`.
- Added `PrefixRows` GAT trait and concrete `PrefixRowIter` for borrowed hash prefix row streaming without `Box<dyn Iterator>`.
- Replaced HashProbe `rows_owned()` execution path with streaming prefix iteration.
- Changed per-row hash-trie build keys from heap `Vec<EncodedOwned>` to stack-backed `SmallVec`.
- Avoided cloning raw LFTJ column vectors when building temporary relation images.

**Deferred Work**
- Fully typed sorted/hash cache keys remain a follow-up. This PRD removed measured allocation pressure without changing cache semantics.
- LFTJ temporary atom row values still allocate per variable value and remain a target for later LFTJ inner-loop work.

**GAT Iterator Coverage**
- Added `PrefixRows`:

```rust
pub trait PrefixRows {
    type Rows<'a>: Iterator<Item = RowId> + 'a
    where
        Self: 'a;

    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Rows<'a>;
}
```

- `PrefixRowIter` streams empty, one-row, slice, range, and nested hash-node prefixes without boxed iterators.

**Allocation Profile Results**
Command:

```sh
cargo run -p bumbledb-bench --features alloc-profile --release -- --scale 10000 --repeats 3 --format markdown --dataset ledger --dataset sailors --dataset joinstress --dataset tpch --query chain4_from_a --query sailor_range_reserves --query tag_lookup_join --query red_boat_sailors --query triangle_count --query supplier_nation_orders
```

Final profile-enabled target run:

| Dataset | Query | Alloc Calls | Bytes Allocated | Net Bytes | Gate |
|---|---|---:|---:|---:|---|
| ledger | tag_lookup_join | 158076 | 87890779 | 26243444 | pass |
| sailors | red_boat_sailors | 139176 | 74699980 | 15396642 | pass |
| sailors | sailor_range_reserves | 71109 | 39808560 | 29801697 | pass |
| joinstress | chain4_from_a | 43350 | 42213759 | 17943451 | pass |
| joinstress | triangle_count | 732371 | 106833391 | 4653113 | pass |
| tpch | supplier_nation_orders | 175238 | 88285971 | 25343463 | pass |

Measured reductions against the PRD 05 profile baseline and the first PRD 06 profile pass:

| Query | Before Calls | After Calls | Calls Delta | Before Bytes | After Bytes | Bytes Delta |
|---|---:|---:|---:|---:|---:|---:|
| chain4_from_a | 103377 | 43350 | -60027 | 46288043 | 42213759 | -4074284 |
| tag_lookup_join | 258071 | 158076 | -99995 | 87886867 | 87890779 | +3912 |
| red_boat_sailors | 278319 | 139176 | -139143 | 79070620 | 74699980 | -4370640 |
| sailor_range_reserves | 171115 | 71109 | -100006 | 46603368 | 39808560 | -6794808 |
| supplier_nation_orders | 330945 | 175238 | -155707 | 94946083 | 88285971 | -6660112 |

`triangle_count` is LFTJ-build dominated and unchanged by this hash-probe-focused pass.

**Default Release Smoke**
Command:

```sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

All structural gates passed with `cursor_seeks=0`, `rows_scanned=0`, and `dictionary_reverse_lookups=0` for generated queries.

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
- Allocation-profile target query run listed above.

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/07_verification_and_handoff.md`
