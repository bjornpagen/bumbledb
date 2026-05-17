# 03: Storage Write Path And Constraints

**Goal**
- Make typed relation rows durable by writing current covering indexes, transaction history, dictionary entries, ID counters, and constraints.

**Why This Stage Exists**
- This is the first real database vertical slice.
- The engine should be able to insert, replace, delete, reopen, and verify current state without any Datalog layer.

**Concrete Work**
- Implement dictionary forward and reverse storage for strings and bytes.
- Implement generated ID allocation and persisted counters.
- Implement current covering index key construction for primary, ref, unique, and range indexes.
- Implement `insert` for primary-key relations.
- Implement `insert_tuple` for composite set relations.
- Implement primary and composite uniqueness checks.
- Implement declared unique constraints using unique guard keys.
- Implement foreign-key existence checks for ref fields.
- Implement `replace` for primary-key relations as whole-row replacement.
- Implement `delete` for primary-key relations with default restrict behavior.
- Implement `delete_tuple` for composite set relations.
- Append transaction log/history records for insert, replace, and delete.
- Maintain basic stats: relation row count and index entry count.
- Add storage-level tests for crash-like abort behavior by returning errors from write closures.

**Out Of Scope**
- Datalog query execution.
- Query planner.
- As-of reads over history.
- Cascading deletes.
- Partial updates.
- Upsert.
- Check constraints.
- Tx functions.

**Passing Criteria**
- Inserts create all expected current covering index entries.
- Duplicate primary keys fail atomically.
- Duplicate composite tuples fail atomically.
- Declared unique violations fail atomically.
- Foreign-key violations fail atomically.
- Rows inserted earlier in the same write transaction can be referenced later in that transaction.
- Preallocated IDs can be used within one write transaction when constraints are satisfied before commit.
- Replaces remove old current index entries and insert new current index entries.
- Deletes remove current index entries and fail when restricted by existing refs.
- Every committed write advances the transaction ID exactly once.
- Every committed write appends enough history to audit what changed.
- Aborted writes leave no partial current index entries, unique guards, dictionary entries, stats changes, or tx metadata visible.
- Reopening the database preserves rows, counters, dictionary mappings, and metadata.

**Notes**
- Write amplification is accepted here.
- Do not optimize away covering indexes before measuring actual pain.
- It is acceptable for early history records to be simple if they preserve enough data for later as-of support.
