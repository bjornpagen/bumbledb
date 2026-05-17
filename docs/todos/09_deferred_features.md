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

**Notes**
- Recursion and as-of queries are the most strategically important deferred features.
- Query macros are ergonomic, not foundational.
- Unsafe performance modes should require strong benchmark evidence and explicit user opt-in if they ever exist.
