# PRD 01: Allocation Instrumentation Truth

## Purpose

Add small deterministic allocation fixtures so COLT changes can be judged without trace-span overhead or full JOB noise.

## Problem

Current allocation evidence is either global no-trace JOB totals or traced spans with substantial instrumentation overhead. We need focused tests and benchmark fixtures that isolate COLT force, suffix iteration, and probe-key allocation.

## Required Work

- Add test-only or bench-only helpers that run with `with_allocation_tracking_for_test` or the benchmark allocator.
- Add a deterministic COLT fixture with many duplicate keys.
- Add a deterministic COLT fixture with all distinct keys.
- Add a deterministic suffix-iteration fixture that should not force a map.
- Add a deterministic probe fixture that repeatedly probes already-forced maps.
- Measure allocation calls and bytes for each fixture.

## Technical Direction

- The fixtures may live in `colt_tests.rs`, a new `colt_alloc_tests.rs`, or `bumbledb-bench` if they need release-mode measurements.
- Prefer no trace spans inside these fixtures.
- The fixtures must verify exact tuple/key correctness, not only allocations.
- Do not introduce unstable external profiling dependencies.

## Passing Criteria

- A fixture proves suffix iteration of a range source allocates zero or a bounded constant number of heap objects, independent of row count.
- A fixture proves forcing a duplicate-heavy source allocates proportional to distinct keys, not row count.
- A fixture proves repeated probes into an already-forced source allocate zero or a bounded constant number of heap objects.
- CI tests do not assert platform-fragile exact allocator counts unless guarded by generous, justified upper bounds.
- Global gates pass.
