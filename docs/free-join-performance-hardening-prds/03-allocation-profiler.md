# PRD 03: Allocation Profiler

## Purpose

Add allocation tracking that can report allocation deltas per trace span during benchmark runs.

## Required Architecture

- Implement allocation tracking in the benchmark binary or a dedicated support module where a global allocator can be installed safely.
- Use `std::alloc::System` as the underlying allocator.
- Count allocation calls, deallocation calls, realloc calls, allocated bytes, deallocated bytes, and net bytes.
- Expose snapshots that can be sampled at trace span enter and exit.
- Make allocation tracking opt-in for benchmark/profile builds and explicit in output.

## Required Safety

- The global allocator must not allocate while updating counters.
- Counter updates must use atomics.
- The profiler must tolerate nested spans.
- The profiler must work with single-thread benchmark execution first.
- If multi-thread support is not implemented, output must say counters are process-global, not thread-local.

## Required API Shape

Provide equivalents of:

```rust
pub struct AllocationSnapshot {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
}

pub struct AllocationDelta {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
    pub net_bytes: i128,
}
```

## Forbidden Shortcuts

- Do not use external profilers as the only allocation source.
- Do not report total process allocations as if they were query-only unless spans prove the delta.
- Do not add a dependency just to format allocation output.
- Do not fake allocation numbers in tests.

## Passing Criteria

- A unit test or benchmark test proves allocation deltas increase after allocating a `Vec`.
- A test proves allocation deltas are attached to a trace span.
- Benchmark JSON can include allocation fields without panicking when allocation tracking is disabled.
- Allocation output clearly states whether counters are enabled.
- Global acceptance from PRD 00 passes.
