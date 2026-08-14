# sdk-008: ABI optionals reconstruct illegal Query / Rec / FindTerm — marshal parses tags then forgets

- **Severity:** high
- **Tree:** sdk (C ABI bridge + ts crate)
- **Status:** OPEN
- **Source:** audit/sdks.md #8
- **Depends on:** none at the bridge; the `has_over` death coordinates with sdk-004 (cpp dialect) and sdk-006 (TS `FindTermIr` split)

## The bug

C cannot have sums, so the ABI uses NULL/counts/`has_*` bytes (`cpp/foreign/bumbledb_c.h:522-528, 546-553, 573-578, 611-621`) — essential. The defect is what the bridges DO with them (`cpp/bridge/src/query.rs:271-283, 333-345, 452-475`; `ts/crate/src/marshal.rs:721-746, 920-960`):

- `bdb_query.rec == NULL` → `None`; non-NULL with `base_count == 0` or `rec_count == 0` → `Some(Rec { base: [], rec: [] })` — empty rec lists become a typed engine value instead of a marshal refusal.
- `rule_count == 0` / `head_count == 0` copy through: Program-shaped empty main is a well-formed `bdb_query`.
- `bdb_find_term.has_over: u8` (`query.rs:143`) with `over: bool_in(view.has_over)?.then_some(VarId(view.over))` (`query.rs:339`) — false discards `over`, true accepts any id; Count-with-over is reconstructible.
- `bdb_condition` always carries `cmp` AND `children`: leaf-with-leftover-children and And-with-leftover-cmp are representable.
- NAPI: `obj.get::<f64>("over")?` optional on aggregate finds (`marshal.rs:728-738`) — missing `over` on Sum and present `over` on Count both parse.
- `bdb_violation.has_measure` + two u64 words — the same pattern outbound.

The `query_in` comment says the engine validator is the trust boundary — "validate, then forget": marshal LEARNED each tag and threw away the refined type.

## Why it's wrong

Flat tagged C structs are essential; reconstructing illegal engine states from them without a refined parse result is not (Insight 6: parse don't validate — the bridge is exactly a parser, and it currently emits unparsed values; Insight 4: `has_over` is a bool beside a payload, admitting both mismatches). Every state the host dialects can no longer mint (sdk-001, sdk-005, sdk-006) remains mintable by any FFI caller through this seam.

## The fix

Per `audit/CONTRACT.md §C6` (C ABI): the layout stays; the bridge parses.

- `has_over` DIES at the ABI: `bdb_find_term_kind` distinguishes the nullary Count as its own kind (or equivalently: aggregate kinds always read `over`, and `AGG_OP_COUNT` ignores none — pick ONE, document in `bumbledb_c.h`, and move `cpp/bridge/src/query.rs` + `cpp/foreign/query_view.cc` + the cbindgen header together).
- Bridge-side, immediately inside `query_in`: refuse (typed marshal error, not engine error) the shapes the boundary IR should never carry from a dialect — `rec` non-NULL with `base_count == 0 || rec_count == 0` is refused, not built. Beyond tag-selection the bridge stays a spelling transform: emptiness of MAIN is still the engine validator's call (CONTRACT §C1: the engine is the ONE refusal authority; the bridge refuses only un-tag-able nonsense: contradictory tag/payload combinations).
- NAPI marshal: aggregate finds parse as the sum — Count requires absent `over`, folds require present `over`; mismatch is a marshal error (aligns with sdk-006's `FindTermIr` split).
- `bdb_condition`: parse by kind — Leaf reads `cmp` only, And/Or read `children` only; leftover payloads are never read (document field-validity-by-kind in the header comment).
- Outbound `bdb_violation.has_measure` stays (C essential) but the WRITER fills the unused words with zero deterministically.

## Acceptance criteria

- [ ] Gone: `rg -n 'has_over' cpp/bridge/src cpp/foreign ts/crate/src` → no matches (or, if the always-read-over alternative is chosen: `rg -n 'then_some' cpp/bridge/src/query.rs` → no tag-conditional payload reads).
- [ ] New locks: bridge unit tests — `rec_with_zero_base_is_refused`, `count_with_over_is_refused` (ts crate marshal test), named accordingly.
- [ ] Unchanged tests: all existing bridge/marshal round-trip tests green with zero assertion edits; engine adversarial tests (`empty rule set` etc.) still reach the engine and return the SAME error names.
- [ ] Green: `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`; `cd ts/crate && PATH="$HOME/.cargo/bin:$PATH" cargo test`; `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd ts && pnpm test`.

## Constraints

- Locked error names (`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) untouched; engine validator refusals keep their names — the new marshal refusals are NEW bridge-level errors, not renames.
- The `bdb_find_term_kind` change is an ABI-visible enum change: cbindgen header, cpp dialect `find_of` (sdk-004), and bridge move in ONE commit. C struct field LAYOUT otherwise frozen.
