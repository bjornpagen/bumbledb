# PRD 08: Final Source And Benchmark Purge

## Status

Not started.

## Severity

Final release gate.

## Prerequisite

PRDs 01 through 07 must be complete.

## Problem

After structural deletion, residue often remains in docs, tests, benchmark JSON, explain output, comments, and counter names. This PRD is the final hard audit and purge.

## Required Actions

1. Run the suite source hygiene gate from `README.md`.
2. Run stale set-engine terminology gates from the broader PRD suite.
3. Run benchmark JSON rendering tests.
4. Run markdown rendering tests.
5. Inspect public exports for deleted names.
6. Inspect `ROSETTA_STONE.md` for stale architecture language.
7. Inspect all PRDs for claims that are now completed or obsolete.
8. Remove obsolete PRDs if project policy requires deletion after completion.

## Required Benchmark Work

- Remove deleted counters from JSON.
- Remove deleted timings from markdown.
- Remove deleted runtime families from grouped output.
- Add replacement Free Join execution metrics where available.
- Ensure old benchmark artifacts are not cited as current architecture evidence.

## Strict Passing Criteria

- Zero source matches for every name in the suite-level source hygiene gate.
- Zero docs matches for deleted public names unless documenting completed removal in this final PRD.
- Full global validation gate passes.
- Query-focused validation gate passes.
- Bench renderer tests pass.
- `git diff --check` passes.

## Failure Modes

- Leaving deleted names in JSON output is failure.
- Leaving deleted names in explain output is failure.
- Leaving deleted names in public exports is failure.
- Leaving tests named after deleted sidecars is failure.

## Non-Goals

- Do not add new features.
- Do not optimize performance.
- This is deletion verification only.
