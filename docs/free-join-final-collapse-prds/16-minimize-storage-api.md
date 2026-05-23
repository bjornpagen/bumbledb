# PRD 16: Minimize Storage API

## Status

Not started.

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
- manual query image fetch helpers
- stale diagnostics not used by benchmarks
- public raw access helpers in tests

## Implementation Steps

1. Make storage scans `pub(crate)` where possible.
2. Move test-only raw helpers behind `#[cfg(test)]`.
3. Delete backup/compact APIs unless tests prove they are required.
4. Keep benchmark bulk-load paths intact.
5. Update public docs and tests.

## Passing Criteria

- Public storage API is fact insert/delete plus minimal read/query support.
- Raw encoded components are not public unless required by benchmarks.
- Free Join still has internal access scans.
- Full validation passes.

## Failure Modes

- Breaking query image construction is failure.
- Keeping public raw internals for convenience is failure.
- Removing benchmark bulk-load support is failure.

## Completion

Delete this PRD and commit.
