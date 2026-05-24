# PRD 05: Executor Span Coverage

## Purpose

Instrument the current engine deeply enough to explain where time and allocations go before any major optimization is attempted.

## Required Span Coverage

Instrument these functions or their replacements:

- `executor::execute_query`: normalize, validate, select plan, execute, finish.
- `planner_select::select_plan`: stats collection, candidate generation, scoring.
- `planner_select::collect_planner_stats`: per atom relation stats.
- `base_image::relation_base_image`: cache lookup, cache hit, load.
- `base_image::load_relation_base_image`: live row scan, per-column load.
- `predicate::source_filters_for_atom`: literal/input encoding and false filter decisions.
- `run::build_sources`: per atom source construction.
- `ColtSource::new_filtered`: source filter scan and survivor count.
- `ColtSource::iter`: tuple production count.
- `ColtSource::force`: offsets scanned, map entries, child nodes.
- `ColtSource::get`: probe calls and misses.
- `cover::choose_cover`: candidate key counts and choice.
- `run::execute_node`: node entry count and recursion depth.
- `run::probe_siblings`: probe count and failed probes.
- `Binding::extend_from_tuple`: binding writes and conflicts.
- `ProjectionSink::consume` and `finish`: facts emitted, duplicates suppressed, decode cost.

## Required Counters

Add real counters for:

- base image cache hits and misses;
- live rows scanned;
- column values loaded;
- loaded bytes;
- source filters encoded;
- source filter false decisions;
- source filter rows tested;
- source filter survivors;
- COLT nodes created;
- COLT nodes forced;
- COLT offsets scanned;
- COLT map entries built;
- tuples yielded by `iter`;
- batches yielded by `iter_batch`;
- cover choices;
- probe calls;
- probe misses;
- recursive node entries;
- maximum recursion depth;
- binding clones or frame copies;
- source map clones or frame changes;
- sink consumes;
- projection duplicates suppressed;
- decoded values.

## Passing Criteria

- Every required span appears in at least one focused test or benchmark smoke test.
- Counters are zero when no work occurs and non-zero when corresponding work occurs.
- JOB `job_q09_voice_us_actor` with profiling emits relation-labeled base-image and COLT spans.
- No counter is computed from elapsed time.
- No counter is hardcoded in benchmark rendering.
- Global acceptance from PRD 00 passes.
