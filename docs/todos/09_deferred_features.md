# 09: Deferred Features

**Goal**
- Keep important non-v0 features visible without letting them distort the v0 architecture.

**Why This Stage Exists**
- The Rosetta Stone explicitly defers several tempting features.
- Capturing them here prevents accidental scope creep while preserving future direction.

**Deferred Work**
- Recursive rules with semi-naive evaluation.
- Stratified negation.
- As-of query execution over history indexes.
- Compile-time `datalog!` query macro.
- Prepared query caching and plan invalidation rules.
- Ordered output and `limit`.
- String lexical range indexes.
- String prefix indexes.
- User-defined pure functions.
- Transaction functions.
- Check constraints.
- Cascading deletes.
- Explicit long-lived snapshot API.
- Spill-to-LMDB temporary relations for large recursion or aggregation.
- Unsafe performance mode, if ever justified.

**Promotion Rule**
- A deferred feature can move into the active roadmap only when v0 passing criteria are not blocked by more foundational work.
- A deferred feature must get its own design note before implementation if it changes query semantics, storage format, or public API.
- A deferred feature must not weaken the core decisions: typed schema, BCNF logical model, LMDB-only backend, no migrations, current indexes separate from history, and Datalog-only querying.

**Passing Criteria**
- This file remains a holding pen, not an implementation stage.
- No deferred feature is implemented accidentally while completing stages 01 through 08.
- When a deferred feature is promoted, its scope and passing criteria are written before coding begins.

**Stage 09 Audit**
- Recursive rules remain unimplemented; rule markers are rejected intentionally by the Datalog frontend.
- Stratified negation remains unimplemented; `not` is rejected intentionally.
- As-of query execution remains unimplemented; `as_of` is rejected intentionally.
- Compile-time `datalog!` macros remain unimplemented.
- Prepared query caching and plan invalidation remain unimplemented.
- Ordered output and `limit` remain unimplemented; both are rejected intentionally.
- String lexical range and prefix indexes remain unimplemented.
- User-defined pure functions remain unimplemented; unknown lower-case function-like clauses are rejected intentionally.
- Transaction functions remain unimplemented.
- Check constraints remain unimplemented.
- Cascading deletes remain unimplemented; restrict delete is the only delete behavior.
- Explicit long-lived snapshot APIs remain unimplemented; read access remains closure-scoped.
- Spill-to-LMDB temporary relations remain unimplemented.
- Unsafe performance modes remain unimplemented; safe LMDB durability remains the only mode.

**Promotion Checklist**
- Write a design note before implementation.
- State whether the feature changes query semantics, storage format, public API, or file compatibility.
- State whether the feature requires a storage format version bump and ETL.
- Define explicit passing criteria and tests before coding.
- Verify the feature does not weaken typed schema, BCNF modeling, LMDB-only storage, no migrations, current/history separation, or Datalog-only querying.
- Update `docs/ROSETTA_STONE.md` if the feature changes a canonical decision.
- Add the promoted work to a new numbered todo document instead of editing it directly into this holding pen.

**Notes**
- Recursion and as-of queries are the most strategically important deferred features.
- Query macros are ergonomic, not foundational.
- Unsafe performance modes should require strong benchmark evidence and explicit user opt-in if they ever exist.
