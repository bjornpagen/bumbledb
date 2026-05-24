# PRD 13: JOB Allocation Gates

## Purpose

Turn allocation improvements into enforced no-trace JOB gates.

## Required Work

- Add a checked-in config or script for all eight JOB sample queries.
- Run in release mode without `query-tracing` for allocation budgets.
- Keep exact SQLite comparison mandatory.
- Keep traced runs available only for diagnostics.
- Record p50 or median-of-N allocation calls and bytes where feasible.

## Initial Budget Policy

- Budgets start from the post-arena measured baseline.
- Budgets may only ratchet down unless a documented architectural improvement justifies temporary increase.
- No PRD may claim allocation improvement from traced allocation counts alone.

## Passing Criteria

- One command runs JOB allocation gates.
- Any exact value mismatch fails the command.
- Any allocation call regression over budget fails the command.
- Any allocated byte regression over budget fails the command.
- Output reports per-query allocation calls, bytes, elapsed nanos, result rows, and correctness fingerprint.
- Global gates pass.
