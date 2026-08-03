## verify_store's marks pass (closed-parent capacity re-check) has zero test coverage

incoherence | low | CONFIRMED | capacity-judge
outcome: fixed 2b1e87b0

### Summary

The marks pass (`crates/bumbledb/src/verify_store/marks.rs`) is the only offline re-verification closed-parent capacity statements receive — those parents have no `F` rows to ride the fact scan (module doc, marks.rs:1-8; facts.rs:258 explicitly defers: "closed parents re-check in the marks pass"). No test in the repository ever executes its working arm: no test anywhere opens a `Db` whose schema contains a capacity statement with a closed target relation, so the pass's roster iteration, `encode_u64(row_index)` parent encoding, `CapacityId`-from-enumerate derivation, member-set ψ filtering, and its `CapacityViolation`/`CapacityRayMeasure` arms all have zero coverage.

### Evidence (verified)

- `crates/bumbledb/src/verify_store/marks.rs:24-27` — the sweep skips every statement that is not `Enforcement::Closed`; the pass is wired at `verify_store.rs:349`, so it "runs" in every verify test but iterates zero statements.
- `crates/bumbledb/src/schema/validate.rs:1357-1368` — `Enforcement::Closed` for capacity is minted only when the target relation is closed (`extension: Some`) with the handle projection `[FieldId(0)]`. The shape is legal and constructible: `schema/tests/valid.rs:683` (`a_window_into_a_closed_target_validates`) and `:712` (closed-to-closed) — but both stop at `decl.validate()`; no `Db` is opened.
- `crates/bumbledb/src/verify_store/tests.rs` — both capacity fixtures use ordinary keyed parents: `marks_schema` (1548-1592, Holder/Account, `extension: None` at 1557/1562) and `weighted_fixture` (1686-1736, Pool/Device, `extension: None` at 1695/1700). The closed-relation tests (1252-1536) cover containment and domain-quantification arms only.
- Exhaustive co-occurrence scan (every `.rs` containing both `StatementDescriptor::Capacity` and a closed relation): only schema validate/render/codec/macro tests (no `Db`) and the bench ledger `bumbledb-bench/src/querygen/target.rs`, whose sole capacity statement (`TAG_BUDGET`, target.rs:541-547) targets the ordinary keyed `POSTING` — its `verify_store()` test (target.rs:988) never enters the marks arm.
- Correction to the finding as filed: `commit/tests/marks.rs` `exclusion_*` (389-427) is NOT closed-parent coverage — `exclusion_schema` (346-384) declares `extension: None` on both relations; it is an ordinary keyed parent with a `{0}` window. Consequently the shared `Enforcement::Closed` arm of `judgment.rs::check_capacity` (`storage/commit/judgment.rs:1138-1156` — the axiom-id decode, member-set gate, and extension-row fact resolution) is untested at commit time too. The untested surface is wider than the finding claimed.

### Failure scenario / impact

A regression anywhere in the closed-parent chain — the marks pass's parent encoding drifting from `encode_u64(row_index)` (judgment.rs:1141 decodes it back with `u64::from_be_bytes`), the `CapacityId(enumerate index)` desyncing from the `schema.capacities()` arena order, or the judgment arm's member-set/extension-row resolution — would silently blind both the offline authority and the commit-time check for closed-parent capacity violations. No test would fail. This also violates the campaign law that every change lands with its test: the pass landed test-free.

### Suggested fix

One `verify_store/tests.rs` fixture with a closed parent relation and a capacity statement targeting its handle (the `a_window_into_a_closed_target_validates` shape from `schema/tests/valid.rs:683`, given a keyed child relation): commit children satisfying the floor, then raw-delete a child's `F`/`R` rows (the tests.rs:1617 `a_missing_capacity_edge_is_found_and_the_group_remeasured` pattern) and assert the marks pass reports `CapacityViolation` with the axiom's fact bytes (`rows[index].fact`) and the re-measured value. A companion commit-time test (a member child pushed past the ceiling under `exclusion_schema`-style assertions in `commit/tests/marks.rs`, but with a closed parent) would close the judgment-arm half of the same gap.