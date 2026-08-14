# sdk-008: ABI optionals reconstruct illegal Query / Rec / FindTerm — marshal parses tags then forgets

- **Severity:** high
- **Tree:** sdk (C ABI bridge + ts crate)
- **Status:** OPEN
- **Source:** audit/sdks.md #8
- **Depends on:** none at the bridge; the `has_over` death coordinates with sdk-004 (cpp dialect) and sdk-006 (TS `FindTermIr` split)

## The bug

C cannot have sums, so the ABI uses NULL/counts/`has_*` bytes (`cpp/foreign/bumbledb_c.h:522-528, 546-553, 573-578, 611-621`) — essential, including `bdb_query.rec == NULL` and empty counts copying through to the engine roster (C1). The defect is discriminators-beside-payloads the bridge *parses then forgets*:

- `bdb_find_term.has_over: u8` (`query.rs:143`) with `over: bool_in(view.has_over)?.then_some(VarId(view.over))` (`query.rs:339`) — false discards `over`, true accepts any id; Count-with-over is reconstructible. This is the C6 leftover: Count is a kind, not a bool.
- `bdb_condition` always carries `cmp` AND `children`: Leaf reads leftover children, And leftover cmp, if a consumer drops the tag.
- NAPI: `obj.get::<f64>("over")?` optional on aggregate finds (`marshal.rs:728-738`) — missing `over` on Sum and present `over` on Count both parse. TS `FindTermIr` is the same optional (`native.ts:115`).
- `bdb_violation.has_measure` + two u64 words — the same pattern outbound (writer must zero unused words; dialect types echoing it are sdk-028).

Empty rec/main lists becoming engine values is **not** a marshal defect — that is the hostile boundary's job (`EmptyRecursiveBase` / `EmptyRuleSet` at validate).

The `query_in` comment says the engine validator is the trust boundary. For emptiness, that is correct (C1). For `has_over`, marshal LEARNED the tag and threw away the refined FindTerm.

## Why it's wrong

Flat tagged C structs and empty-list hostility are essential (C1). `has_over` as a parallel optional on a tagged find is not (Insight 4; C6). NAPI `over?: number` on every aggregate is the same bool-plus-payload (sdk-006's wire type). Reconstructing leftover condition payloads by always copying both arms is accidental; unread-by-kind is enough without a second roster.

## The fix

Per `audit/CONTRACT.md §C6` (Count is a kind) and §C1 (hostile C ABI; engine is the one roster authority):

- **`has_over` as discriminator dies.** Pick ONE C6 alternative; document it in `bumbledb_c.h`; move `cpp/bridge/src/query.rs` + `cpp/foreign/query_view.cc` + cbindgen + sdk-004's `find_of` in ONE commit:
  1. Preferred: append `BDB_FIND_TERM_KIND_COUNT` (existing kind *values* stay). Count carries no `over`; folds always read `over`. Dropping the `has_over` *field* is this alternative's ABI delta — the only C struct field this issue may remove. `bdb_query.rec` stays a nullable pointer (C1).
  2. Equivalent: keep field layout and always read `over` for aggregate kinds (`AGG_OP_COUNT` ignores it). Then `has_over` is an unread leftover — prefer (1) so it is not a second discriminator.
- **Do not steal engine roster refusals.** Empty rec lists (`base_count == 0` / `rec_count == 0`), empty main `rules`, empty interior rule-lists, and empty heads copy through. C1: the ABI admits hostile states so `validate` refuses them by name (`EmptyRecursiveBase`, `EmptyRecursiveStep`, `EmptyRuleSet`, `EmptyInterior`). New marshal errors are only un-tag-able nonsense (unknown `kind`, `bool_in` failures) — not emptiness the engine already names.
- NAPI marshal: aggregate finds parse as the C6 sum — Count requires absent `over`, folds require present `over`; mismatch is a marshal error (sdk-006's `FindTermIr` split). This is the TS dialect parse, not a second query validator.
- `bdb_condition`: parse by kind — Leaf reads `cmp` only, And/Or read `children` only; leftover payloads are never *read* (document field-validity-by-kind). Do not add a marshal refusal for leftover bytes when the kind is valid.
- Outbound `bdb_violation.has_measure` stays (C essential); the WRITER fills unused words with zero deterministically.

## Acceptance criteria

- [ ] Gone: `rg -n 'has_over' cpp/bridge/src cpp/foreign ts/crate/src` → no discriminator reads (or, if always-read-over is chosen: `rg -n 'then_some' cpp/bridge/src/query.rs` → no tag-conditional payload reads). `bdb_query` still has nullable `rec`.
- [ ] New lock: `count_with_over_is_refused` (ts crate marshal test). Do **not** add `rec_with_zero_base_is_refused` — that path must still reach the engine as `EmptyRecursiveBase`.
- [ ] Unchanged tests: all existing bridge/marshal round-trip tests green with zero assertion edits; engine adversarial tests (`empty rule set`, empty rec lists) still reach the engine and return the SAME error names.
- [ ] Green: `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`; `cd ts/crate && PATH="$HOME/.cargo/bin:$PATH" cargo test`; `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd ts && pnpm test`.

## Constraints

- Locked error names (`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) untouched; engine validator refusals keep their names. Do not introduce marshal errors that shadow them.
- Count has no `over`; folds require it — same split as sdk-004 / sdk-027, not a third encoding.
- `bdb_query` (nullable `rec`) shape-unchanged (C1). The `bdb_find_term` discriminator change is the C6-blessed ABI delta; no other C struct fields reorder.
