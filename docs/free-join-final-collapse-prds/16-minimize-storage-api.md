# PRD 16: Minimize Storage API

## Status

Not started.

## Current State

The storage layer is correct and durable, but its public surface still exposes convenience/raw APIs beyond the minimal embedded set database:

- `FieldValues`, `FactCursor`, `FactCursorRecord`, and `EncodedComponent` are public.
- Public relation/access scan helpers exist for tests and diagnostics.
- backup/compact copy helpers are public.
- `bulk_load_streaming` is misleading scaffolding: review found it only invokes a closure and returns while comments imply deferred publication behavior.
- Storage diagnostics expose index internals that may only be needed by tests/benchmarks.

## Objective

Keep storage semantics but narrow public storage APIs to what the embedded set database needs.

## Keep

- fact insert
- exact fact delete
- exact duplicate insert no-op
- schema verification
- bulk load required by benchmarks
- internal access scans required by Free Join/query images

## Audit And Delete Or Privatize

- public relation/access scan APIs
- encoded component exposure
- backup/compact copy helpers
- misleading `bulk_load_streaming` wrapper/comment/API
- manual query image fetch helpers
- stale diagnostics not used by benchmarks
- public raw access helpers in tests
- public `FactCursor`/`FieldValues` if no external non-test use remains

## Implementation Steps

1. Make storage scans `pub(crate)` where possible.
2. Move test-only raw helpers behind `#[cfg(test)]`.
3. Delete backup/compact APIs unless tests prove they are required.
4. Delete or rename `bulk_load_streaming`; do not keep a wrapper whose name promises streaming/deferred publication behavior it does not implement.
5. Keep benchmark bulk-load paths intact.
6. Update public docs and tests.

## Passing Criteria

- Public storage API is fact insert/delete plus minimal read/query support.
- Raw encoded components are not public unless required by benchmarks.
- Free Join still has internal access scans.
- Misleading storage APIs/comments are gone.
- Full validation passes.

## Failure Modes

- Breaking query image construction is failure.
- Keeping public raw internals for convenience is failure.
- Removing benchmark bulk-load support is failure.

## Completion

Delete this PRD and commit.
