# 04: Read Snapshots And Access Paths

**Goal**
- Build the typed read side below Datalog: snapshots, primary lookup, index scans, prefix scans, range scans, and tuple decoding.

**Why This Stage Exists**
- The planner and executor should not talk directly to LMDB cursors.
- We need a clean access path layer that exposes relation indexes as ordered sources.

**Concrete Work**
- Implement closure-scoped read snapshots over LMDB read transactions.
- Implement primary-key row lookup using the primary covering index.
- Implement relation full scan through primary index fallback.
- Implement prefix scan over ref and unique indexes.
- Implement scalar range scan over range indexes.
- Implement cursor wrappers that expose encoded components without eager full decoding.
- Implement tuple decoding from covering index keys.
- Implement `next_while_prefix` or equivalent prefix-bounded iteration.
- Implement access path descriptors that the future planner can enumerate.
- Expose low-level internal scan APIs for tests and later query execution.
- Add tests for snapshot isolation with concurrent read and write closures where feasible.

**Out Of Scope**
- Datalog parser.
- Type inference for query variables.
- Multiway join executor.
- Aggregation.
- User-facing query API.
- Borrowed public result APIs.

**Passing Criteria**
- Primary lookup returns exact typed rows.
- Prefix scans return all and only matching rows.
- Range scans return all and only matching rows for ordered scalar fields.
- Full scans work as a correctness fallback.
- Decoding from each covering index produces the same logical tuple.
- Scan APIs do not expose raw LMDB cursors publicly.
- Cursor wrappers cannot outlive their read transaction through safe Rust APIs.
- A read snapshot sees stable data while a later write commits.
- Long-lived snapshot behavior is documented, even if not deeply tested.

**Notes**
- This stage should make the database useful for hand-written internal reads before Datalog exists.
- Keep the access path API generic enough for the planner, but do not invent a full iterator framework before it is needed.
