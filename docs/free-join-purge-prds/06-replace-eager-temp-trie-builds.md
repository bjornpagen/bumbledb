# PRD 06: Replace Eager Temporary Trie Builds

## Status

Not started.

## Severity

High performance architecture.

## Prerequisite

PRDs 01 through 05 must be complete.

## Problem

After direct/hash/static sidecars are removed, the main remaining paper-skeptical structure is eager temporary sorted-trie construction for LFTJ atom plans. The paper identifies eager trie building as a major cost of Generic Join-like engines and introduces COLT to avoid it.

## Code To Target

- `build_lftj_sorted_trie`
- `append_indexed_lftj_atom_values`
- temporary relation image construction
- eager sorted trie cache as default atom path
- per-atom copied column builders when avoidable

## Required Replacement

Implement a minimal Free Join access abstraction that can stream or probe relation-image columns without eager full trie construction.

This can be the first COLT/GHT slice from the broader PRD suite.

## Implementation Steps

1. Add access trait for iterate/probe over relation-image columns.
2. Add a lazy root implementation for single-field and two-field relation atoms.
3. Route simple LFTJ atom plans through lazy access when possible.
4. Keep eager sorted trie only as fallback for unsupported shapes.
5. Add counters for eager builds avoided.
6. Add tests proving fewer copied bytes on focused fixtures.

## Strict Passing Criteria

- At least one multi-relation query executes without temp sorted-trie build.
- Exact result equality with eager fallback is proven.
- Eager build counters decrease on focused fixture.
- Full validation gate passes.

## Failure Modes

- Renaming eager sorted trie to lazy without changing behavior is failure.
- Copying full atom columns before proving need is failure.
- Deleting eager fallback before replacement coverage is sufficient is failure.

## Non-Goals

- Do not implement vectorization here.
- Do not implement full cover optimizer here.
