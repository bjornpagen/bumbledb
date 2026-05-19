# PRD 02: Index-Backed Direct Chain

## Status

Draft.

## Problem

Direct chain kernels currently build relation-wide hash tries. This is wasteful for selective chain lookups where existing schema indexes can answer each step by prefix scan.

Examples:

```text
ledger/tag_lookup_join
joinstress/chain4_from_a
```

`joinstress/chain4_from_a` is already fast enough for the gate, but it still reports hash index build rows. `ledger/tag_lookup_join` returns 10k rows and spends significant time in direct kernel execution and sink finish.

## Root Cause Analysis

Current direct chain flow:

```text
query -> normalize -> encode inputs -> query image -> plan -> direct chain -> build hash trie per step -> probe prefixes -> recurse -> materialize rows
```

Waste sources:

- `direct_hash_index` builds a full `HashTrieIndex` for each relation/field set.
- Existing current/durable indexes already have the needed prefix order.
- Direct chain steps often need only a few prefix probes.
- Hash-trie build cost is paid before knowing whether the first probe is tiny.
- For large-output chains, result sink cost dominates after lookup cost is reduced.

Trace/benchmark evidence:

```text
ledger/tag_lookup_join
rows = 10000
runtime = DirectKernel
hash_index_builds = 2
hash_index_build_rows = 60000 at scale 10000
Bumbledb avg ~= 9.7 ms
SQLite avg ~= 1.3 ms
```

```text
joinstress/chain4_from_a
rows = 1
runtime = DirectKernel
hash_index_builds = 4
hash_index_build_rows = 40000 at scale 10000
Bumbledb avg ~= 14 us
SQLite avg ~= 4 us
```

## Goal

Replace relation-wide hash-trie builds in direct chain execution with current-index or durable-index prefix probes when an access path exists.

## Non-Goals

- No general acyclic join planner in this PRD.
- No multi-output streaming rewrite in this PRD.
- No changes to cyclic LFTJ behavior.
- No text/vector/document indexes.

## Eligibility

A direct chain step can use index-backed probing when:

- The step has a non-empty bound prefix.
- The bound prefix maps to a contiguous leading prefix of an access path.
- The step introduces at most one new variable, or it is existence-only.
- All remaining atom predicates can be checked after row fetch.

Examples:

```text
PostingTag(tag: $tag, posting: ?posting)
Posting(id: ?posting, account: ?account)
```

```text
B(id: ?b, a: $a)
C(id: ?c, b: ?b)
D(id: ?d, c: ?c)
```

## Technical Design

Add a direct step executor abstraction:

```rust
enum DirectStepAccess {
    CurrentIndexPrefix { index_name: String, prefix_terms: Vec<NormTerm> },
    HashTrieFallback { cache_key: String, fields: Vec<FieldId> },
}
```

Execution algorithm:

```text
for each chain step:
  build encoded prefix from bound variables/inputs/literals
  if current index access exists:
    prefix scan current index
    fetch row payload if non-covering
    bind introduced variable
  else:
    fallback to existing hash trie path
```

The fallback remains for unusual shapes but should not trigger for the target benchmarks.

## Required Code Areas

- `try_direct_chain_kernel`
- `DirectChainStep`
- `DirectExistenceCheck`
- `DirectChainExecutor`
- `direct_hash_index`
- `ReadTxn::scan_prefix` or lower-level encoded prefix scan
- `StorageSchema::access_paths`

## Required Counters

Add or reuse counters so benchmark output distinguishes:

- index-prefix direct probes
- hash-trie direct probes
- hash-trie fallback builds
- rows returned from index prefix probes
- rows filtered after row fetch

At minimum, target queries must show:

```text
hash_index_builds = 0
hash_index_build_rows = 0
direct_kernel_probes > 0
```

## Required Tests

Add tests for:

- Chain step uses current index and does not build hash trie.
- Existence check uses current index and does not build hash trie.
- Broken chain returns zero rows without building downstream indexes unnecessarily.
- Multi-step chain result equals existing direct-chain result.
- Fallback still works for a shape with no usable current index.

## Required Benchmark Gates

Update gates:

```text
ledger/tag_lookup_join
joinstress/chain4_from_a
```

Required gate fields:

- `plan_family == Direct`
- `runtime == DirectKernel`
- `hash_index_builds == 0`
- `hash_index_build_rows == 0`
- `cursor_seeks == 0`
- `rows_scanned == 0`

Initial performance targets:

```text
joinstress/chain4_from_a avg <= 10 us
ledger/tag_lookup_join avg improves by at least 25% from latest baseline
```

Stretch targets:

```text
joinstress/chain4_from_a avg <= 6 us
ledger/tag_lookup_join within 3x SQLite
```

## Strict Passing Criteria

- `joinstress/chain4_from_a` completes with `hash_index_builds == 0`.
- `ledger/tag_lookup_join` completes with `hash_index_builds == 0`.
- Existing direct chain tests continue passing.
- New index-backed chain tests compare exact rows against the generic/reference result.
- JOB `8/8` wins are preserved.
- `joinstress/triangle_count` remains LFTJ/WCOJ and does not route to direct chain.
- Full workspace test/clippy/fuzz gates pass.

## Verification Commands

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset ledger --query tag_lookup_join
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset joinstress --query chain4_from_a
```
