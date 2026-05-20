# PRD 14: LFTJ Zero-Allocation Traversal

## Status

Proposed.

## Motivation

PRDs 04, 05, and 13 attack cold LFTJ build cost. The traced run also shows a steady-state LFTJ execution cost that remains after caches are warm:

| Query | Sample LFTJ execute time | Sample query time share |
|---|---:|---:|
| `job_q09_voice_us_actor` | 8.894 s across 30 samples | 99.944% |
| `job_q24_voice_keyword_actor` | 74.78 ms across 30 samples | 93.463% |

Some of this is real join work, but the code still allocates and clones during traversal:

- iterator stacks allocate `Vec`,
- participants are cloned,
- leapfrog keys are copied into `EncodedOwned`,
- bindings store `EncodedValue` with cloned `ValueType`,
- sinks clone encoded values into row/group keys.

This PRD removes avoidable allocation from hot LFTJ traversal.

## Evidence

| Current behavior | Anchor |
|---|---|
| `SortedTrieIndex::iter` allocates `Vec::with_capacity` stack | `crates/bumbledb-lmdb/src/sorted_trie.rs:120-126` |
| `execute_lftj` builds participant vectors and iterator vectors | `crates/bumbledb-lmdb/src/query.rs:4925-4931`, `4958-4969` |
| `LftjExecutor` creates `EncodedBinding::new(query.vars.len())` | `crates/bumbledb-lmdb/src/query.rs:4938-4945` |
| Each recursion clones participants into `LeapfrogState` | `crates/bumbledb-lmdb/src/query.rs:5172` |
| Each candidate binds `EncodedValue::new(value_type.clone(), value)` | `crates/bumbledb-lmdb/src/query.rs:5198-5203` |
| Prefix probe has the same pattern | `crates/bumbledb-lmdb/src/query.rs:5042-5122` |
| Project/aggregate sinks clone bound encoded values | `crates/bumbledb-lmdb/src/query.rs:7640-7645`, `7766-7785` |
| q09 first execution has 645,250 `lftj_execute` alloc calls and 97.4 MB transient allocation | `docs/job-trace-analysis/04-job_q09_voice_us_actor.md:115-143` |

## Goals

- Make steady-state LFTJ traversal allocate zero or near-zero heap memory per query execution for fixed query shapes.
- Remove `ValueType` clones from per-candidate binding.
- Use stack/small fixed storage for iterator frames, participants, and bindings where query sizes are small.
- Keep traversal correctness and set semantics.
- Preserve counters and explain timings.

## Non-Goals

- Do not change cold sorted trie build here.
- Do not change planner variable order here.
- Do not specialize output sinks here except where needed for binding ownership. Sink specialization is PRD 15.

## Current Binding Shape

Current:

```rust
struct EncodedBinding {
    values: SmallVec<[Option<EncodedValue>; 8]>,
}

struct EncodedValue {
    value_type: ValueType,
    encoded: EncodedOwned,
}
```

Anchors: `crates/bumbledb-lmdb/src/query.rs:1081-1145`.

Problem:

- Every bind clones `ValueType`.
- Every bound key is an owned copy.
- Sink code clones owned encoded values again.

## Proposed Binding Shape

Use encoded scalar only in binding:

```rust
struct EncodedBinding {
    values: SmallVec<[Option<EncodedOwned>; 8]>,
}
```

Get value type from query metadata when decoding/comparing/projecting:

```rust
let value_type = &query.vars[variable].value_type;
```

This deletes per-bind `ValueType` clones.

If copying `EncodedOwned` still dominates, move to:

```rust
enum BoundEncoded<'a> {
    Borrowed(EncodedRef<'a>),
    Owned(EncodedOwned),
}
```

But be careful: trie iterators move, so borrowed refs may not survive. Safer first step is scalar copy without `ValueType` clone. `[u8; 8]` copies are cheap and stack-like.

## Iterator Stack

Replace heap `Vec` stack in `SortedTrieIter`:

Current:

```rust
stack: Vec<TrieFrame>
```

Anchor: `crates/bumbledb-lmdb/src/sorted_trie.rs:120-126`, `236-239`.

Target:

```rust
stack: SmallVec<[TrieFrame; 8]>
```

Most Datalog atoms in target workloads have small arity. This should eliminate iterator-stack heap allocation for common cases.

## Participants And Leapfrog State

Current code clones `SmallParticipants` per recursion:

```rust
let participants = self.participants(variable);
let mut leapfrog = LeapfrogState::new(participants.clone());
```

Anchor: `query.rs:5158-5173`.

Change to borrow participants from precomputed runtime:

```rust
let participants = &self.runtime.participants_by_variable[variable];
let mut leapfrog = LeapfrogState::new(participants);
```

`LeapfrogState` should store a small borrowed slice or `SmallVec` only if it must sort/reorder participants. If sorting is necessary, keep a stack `SmallVec<[usize; 8]>`.

## Key Copying

`LeapfrogState::key` currently returns `EncodedOwned` or equivalent owned key. Audit functions around `query.rs:5270-5409`.

Target:

- `key_ref` returns `EncodedRef<'_>` where lifetime allows immediate comparison.
- `key_owned` only copies once when binding accepted.
- `seek` takes `EncodedRef<'_>` instead of owned where possible.

If borrow lifetimes are hard, keep `EncodedOwned` but ensure no heap allocation. `EncodedOwned` is fixed-size enum, so copying is acceptable.

## Comparisons

`comparisons_ready_pass` and related helpers should not require `EncodedValue` with embedded `ValueType`. They can resolve value type from query vars/predicate metadata.

Update code paths that use:

- `value.value_type`
- `value.as_bytes()`

to take `(value_type, bytes)` from query metadata and binding.

## Output Sink Interface

The sink trait currently receives `&EncodedBinding` and clones values as needed. Keep that, but after binding shape change, sinks must know value type from query metadata.

PRD 15 will specialize sinks. This PRD can keep existing sink semantics with less binding allocation.

## Acceptance Criteria

- `EncodedBinding` no longer stores `ValueType` per bound value.
- `SortedTrieIter` uses `SmallVec` stack or equivalent no-heap small storage.
- LFTJ recursion does not clone participants per depth.
- q09 `lftj_execute` allocation calls drop substantially from 645,250.
- q09/q24 results unchanged.

## Tests

### Unit Tests

- Binding bind/unbind semantics unchanged.
- Duplicate bind with same value succeeds.
- Duplicate bind with different value fails.
- Comparison predicates still pass/fail correctly.
- Output decoding returns correct `Value`s after removing `ValueType` from binding.
- `SortedTrieIter` tests still pass with `SmallVec` stack.

### Existing Tests

Run:

```sh
cargo test -p bumbledb-lmdb sorted_trie
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Plan

Run q09/q24 allocation profile after PRD 14:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q09_voice_us_actor \
  --query job_q24_voice_keyword_actor
```

Gates:

- q09 `allocations.phases.lftj_execute.alloc_calls` drops from 645,250 by at least 80%.
- q09 `allocations.phases.lftj_execute.bytes_allocated` drops from 97.4 MB by at least 80%.
- q09 sample avg improves or does not regress.
- q24 `lftj_execute` alloc calls drop materially from 6,070.

## Risks

- Borrowed key refs can become invalid after iterator movement. If unsure, copy into `EncodedOwned` and focus on removing heap allocations first.
- Removing `ValueType` from `EncodedValue` requires careful decode/comparison updates.
- `SmallVec` can still spill for very high arity. That is acceptable; target workloads are narrow relation joins.
- Counter semantics must not change.

## Definition Of Done

- LFTJ traversal hot path is near-zero allocation for common query arities.
- Binding no longer clones `ValueType` per candidate.
- Iterator frames and participants avoid heap allocation for JOB query arities.
- q09/q24 sample correctness and allocation gates pass.
