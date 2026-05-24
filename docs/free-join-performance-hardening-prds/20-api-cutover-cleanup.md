# PRD 20: API Cutover Cleanup

## Purpose

Delete stale names, compatibility leftovers, and diagnostic scaffolding that no longer matches the hardened Free Join architecture.

## Required Cleanup

- Remove stale comments saying operations are unavailable pending old PRDs when they are implemented.
- Remove old LFTJ-era naming and any aliases caught by `scripts/check-cutover.sh`.
- Remove benchmark fields replaced by trace summaries.
- Remove test helpers that expose internal modes not part of the engine design.
- Rename misleading types so GHT, COLT, Free Join plan, source filter, and sink responsibilities are explicit.
- Split large files instead of letting cleanup make them harder to maintain.

## Required Breaking Changes

- Do not keep old function names as wrappers.
- Do not preserve JSON field aliases.
- Do not keep deprecated modules.
- Do not keep old storage format compatibility.

## Passing Criteria

- `bash scripts/check-cutover.sh` passes.
- `bash scripts/check-line-counts.sh` passes.
- `rg "deprecated|compat|legacy|old path|pending PRD|unavailable until" crates docs/ROSETTA_STONE.md` has no stale matches.
- Public exports test reflects the new intended surface only.
- Global acceptance from PRD 00 passes.
