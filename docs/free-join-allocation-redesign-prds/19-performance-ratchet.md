# PRD 19: Performance Ratchet

## Purpose

Ratchet the allocation and elapsed-time budgets after allocation-first redesign stabilizes.

## Required Work

- Create or update allocation budget config for all eight JOB sample queries.
- Include no-trace allocation calls and allocated bytes.
- Include elapsed nanos as secondary budget.
- Include traced diagnostic counter expectations separately.
- Add dry-run or test mode proving regressions are detected.

## Ratchet Rules

- Budgets may move down after stable improvement.
- Budgets may move up only with written justification and exact correctness unchanged.
- Trace-enabled allocation numbers must never be used as the primary allocation budget.

## Passing Criteria

- Budget file exists and is checked in.
- JOB gate enforces budgets.
- Regression detection is tested.
- Current numbers are materially better than PRD 00 baseline for COLT-heavy queries.
- Global gates pass.
