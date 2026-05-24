# PRD 08: Planner Stats Without Base Images

## Purpose

Stop the planner from paying execution-level base-image costs before a plan is selected.

## Current Problem

`planner_select::collect_planner_stats` calls `txn.relation_base_image` for every atom to derive row and distinct stats. This violates the measurement-first finding and makes empty-result queries pay broad scan/materialization costs during planning.

## Required Design

- Planner stats must use durable relation counts from storage stats first.
- Distinct and prefix estimates must come from persisted stats, sampled stats, or explicit cheap estimates, not full base-image construction.
- If a distinct estimate is unavailable, use a conservative estimate with `estimate` labeling.
- Plan scoring must not require loading column values.
- The trace must show planner stats separately from execution source building.

## Required Breaking Changes

- Replace `PlannerRelationStats::base_image_rows` with names that distinguish durable `relation_fact_count` from estimated filtered/source cardinality.
- Delete planner code that accepts a full `RelationBaseImage` merely to estimate distinct counts.
- If tests depend on exact prefix distinct from base images, rewrite them to use explicit stored stats fixtures or assert estimate status.

## Passing Criteria

- A unit test proves `select_plan` on a populated relation does not call `relation_base_image`.
- Trace for `job_q09_voice_us_actor` shows zero base-image load spans inside planner stats.
- Planner still emits deterministic candidate ordering on ties.
- All existing correctness tests pass.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
