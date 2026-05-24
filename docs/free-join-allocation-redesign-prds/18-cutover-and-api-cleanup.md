# PRD 18: Cutover And API Cleanup

## Purpose

Delete stale names, old diagnostics, compatibility remnants, and comments after the allocation-first redesign is complete.

## Required Work

- Remove old PRD comments that no longer match implemented behavior.
- Remove old internal mode names that imply legacy LFTJ or bag semantics.
- Remove stale benchmark fields or docs replaced by allocation gates.
- Remove test-only helpers that accidentally expose old architecture.
- Update public exports tests if the intended surface changed.

## Required Searches

```bash
rg "deprecated|compat|legacy|old path|pending PRD|unavailable until" crates docs/ROSETTA_STONE.md docs/free-join-allocation-redesign-prds
rg "Rc<RefCell|HashMap<EncodedTuple|Vec<usize>" crates/bumbledb-lmdb/src/colt* crates/bumbledb-lmdb/src/query
```

## Passing Criteria

- Search results are either empty or explicitly justified in comments/tests.
- `bash scripts/check-cutover.sh` passes if present and applicable.
- `bash scripts/check-line-counts.sh` passes.
- Full global gates pass.
