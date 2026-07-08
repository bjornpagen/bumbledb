# PRD 08 — Commit: containment, source side

**Depends on:** 07.
**Modules:** `crates/bumbledb/src/storage/commit.rs` (+ a new `commit/judgment.rs`), `crates/bumbledb/src/error.rs`.
**Authority:** `docs/architecture/50-storage.md` (§ commit step 2 R-puts, step 3 source side), `30-dependencies.md` (§ enforcement, § pointwise lifting).

## Goal

Every inserted fact writes its reverse edges and, in the judgment phase, proves its
containment statements' targets exist in the final state — scalar tuples by one
guard probe, interval positions by the coverage walk.

## Technical direction

1. **Selection evaluation:** one helper,
   `fn satisfies(selection: &[(FieldId, LiteralValue)], fact_bytes, layout) -> bool`
   — byte-compare each selected field's slice against the literal's canonical
   encoding (encode literals once per commit, not per fact; precompute per
   statement into a commit-local scratch). Used by both R-puts and target-side
   selection checks.
2. **R-puts (insert phase):** for each inserted fact, for each `outgoing`
   containment statement of its relation where `satisfies(source.selection)`:
   build `key_bytes` = the source fact's projection fields, reordered by
   `key_permutation` into the target key's guard order (16-byte interval fields
   copied whole, always last per the acceptance gate); put
   `R | stmt | key_bytes | source_rel | source_row`. Delete-phase symmetric
   removal is already in PRD 07's step-1 rewrite — confirm it derives the same
   key_bytes.
3. **Judgment phase, source side** (new `commit/judgment.rs`; runs after step 2):
   for each inserted fact, for each satisfying `outgoing` statement:
   - **Scalar** (`interval_position: None`): `U | target_rel | target_key | key_bytes`
     get. Miss ⇒ `ContainmentViolation`. On hit with a nonempty `target.selection`:
     one `F` get via the guard's row_id, `satisfies(target.selection)` must hold.
   - **Interval** (`interval_position: Some(_)`): the **coverage walk**. Let
     `prefix` = the scalar part of `key_bytes`, `(s, e)` = the source interval:
     1. Cursor-seek to `U | target_rel | target_key | prefix | s`; if the exact
        or preceding entry within `prefix` has `start ≤ s < end`, set
        `covered = that.end`; else ⇒ violation (the walk's entry gap).
     2. While `covered < e`: advance the cursor; the next entry within `prefix`
        must have `start ≤ covered` (target key disjointness makes `start ==
        covered` the only non-gap case, but write `≤` and let the key's own
        invariant carry the proof — comment this); set `covered = max(covered,
        its end)`. Gap or prefix exhaustion ⇒ violation.
     3. When `target.selection` is nonempty, each segment consumed by the walk
        pays one `F` get + `satisfies` check.
     All comparisons on the 8-byte encoded halves (order-preserving ⇒ byte
     compare).
4. **Error:** `Error::ContainmentViolation { statement: StatementId, side: Direction, fact: Box<[u8]> }`
   where `Direction = SourceUnsatisfied | TargetRequired` — this PRD emits
   `SourceUnsatisfied` (fact = the source fact whose target is missing);
   PRD 09 emits the other. Replaces `ForeignKeyViolation`; delete it.
5. Skip work honestly: a fact whose relation has no `outgoing` statements touches
   none of this; the per-relation `outgoing` index (PRD 02) is the loop driver —
   never iterate all statements per fact.

## Out of scope

Target-side (delete-direction) checks — PRD 09. Bulk-load chunking semantics
(no code change; the loud failure falls out of judging each chunk's commit).

## Passing criteria

- `[shape]` `ForeignKeyViolation` no longer exists; `ContainmentViolation` carries
  statement id, direction, and fact bytes.
- `[shape]` Selection literals are pre-encoded once per commit (no per-fact
  literal encoding in the loops).
- `[test]` Scalar containment: insert source without target aborts
  (`SourceUnsatisfied`); target+source in one delta commits; source whose target
  fails the target selection aborts; conditional source (fact outside σ) commits
  without a target and writes **no** R entry (assert via a follow-up target delete
  committing cleanly).
- `[test]` Coverage walk matrix: exact single-segment cover; multi-segment
  abutting chain; chain with an interior gap (aborts); source start before first
  segment (aborts); source end past last segment (aborts); `MAX`-sentinel target
  segment covering a bounded source; selected-target segment failing σ mid-chain
  (aborts).
- `[test]` A `==` pair (two statements) enforces both directions on insert: a
  parent inserted alone aborts on the totality statement; parent+child in one
  delta commits.
