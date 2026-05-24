# PRD 12: COLT Allocation Benchmarks

## Purpose

Create durable, focused COLT allocation benchmarks that protect the arena redesign from regression.

## Required Work

- Add benchmark commands or checked-in bench fixtures for COLT force, suffix iteration, map lookup, and batch fill.
- Include duplicate-heavy and distinct-heavy force cases.
- Include 8-byte and 16-byte key cases.
- Include filtered and unfiltered source cases.
- Render allocation calls, allocated bytes, and net bytes.

## Required Output

Add `COLT_ALLOCATION_BASELINE.md` in this directory containing current numbers after arena cutover.

## Passing Criteria

- The benchmark can be run with one command.
- Bench output is exact and deterministic enough for regression gates.
- Each benchmark explains the intended allocation complexity.
- Current COLT force allocation calls are materially lower than the pre-arena baseline.
- Global gates pass.
