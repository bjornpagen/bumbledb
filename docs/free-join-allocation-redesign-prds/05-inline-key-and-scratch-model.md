# PRD 05: Inline Key And Scratch Model

## Purpose

Replace heap-allocated key bytes in hot paths with compact inline keys and reusable scratch buffers.

## Required Design

The key representation must distinguish borrowed probe keys from owned stored keys:

```rust
struct KeyRef<'a> { bytes: &'a [u8] }
enum KeyOwned { K8([u8; 8]), K16([u8; 16]), K32 { len: u8, bytes: [u8; 32] }, Heap(Vec<u8>) }
struct KeyScratch { bytes: [u8; 32], heap: Vec<u8>, len: usize }
```

Exact shapes may differ. The important properties are no heap allocation for normal 8-byte and 16-byte keys, and no per-probe owned key allocation.

## Required Tests

- Equality and hash equivalence between borrowed and owned keys.
- 8-byte key stores inline.
- 16-byte key stores inline.
- Wider key falls back safely.
- Probe key construction into scratch allocates zero heap objects for 8-byte and 16-byte keys.

## Anti-Regression Rule

The previous naive inline tuple experiment increased allocated bytes. This PRD must not widen all tuple/map entries blindly. Inline keys are for map/probe keys only, not a general replacement for every tuple object unless benchmarks prove it.

## Passing Criteria

- No-trace microbench shows fewer probe/force key allocations.
- No-trace JOB allocation bytes do not regress by more than 2 percent on any query.
- No-trace JOB allocation calls improve on at least q09 and one broad query.
- Global gates pass.
