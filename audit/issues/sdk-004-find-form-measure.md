# sdk-004: C++ `find_form` has no `Measure` — duration finds silently collapse to `Var`

- **Severity:** high
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #4
- **Depends on:** none (parallel-safe; dummy Var `op` filler is this change — sdks.md #20)

## The bug

Engine `FindTerm = Var | Aggregate | Measure | AggregateMeasure` — four cases. C++ `find_form` (`cpp/src/query/ir.cc:157-161`) has three: `variable | aggregate | aggregate_measure`. Consequences:

- Projected variables and projected measures share `find_form::variable`; `find_of` (`cpp/foreign/query_view.cc:160-169`) always emits `BDB_FIND_TERM_KIND_VAR` for them.
- `find_slot` (`cpp/src/query/rule.cc:229-238`) only constructs from `qvar`, so `(Duration(w))` as a find column is UNWRITABLE in the dialect.
- `find_data` (`ir.cc:177-187`) uses `has_over` as an existence flag; the real discriminator (`over.form`) is not read by the switch (`lower.cc:338-347`).

A legal IR sentence (a Measure find) is a special case of Var until the engine computes the wrong column.

## Why it's wrong

The missing fourth case makes the dialect emit a WRONG tag rather than refuse (Insight 4: the sum is incomplete, so an illegal collapse is representable and silent — the worst failure class). They already distinguished `aggregate_measure`, proving the discrimination is expressible; Measure is simply the missing arm (Insight 2).

## The fix

Per `audit/CONTRACT.md §C6` (C++): four `find_form` cases mirroring `FindTerm` — `variable | aggregate | measure | aggregate_measure`. Concretely:

- `find_slot` accepts a `measure_ref` (the `Duration(w)` spelling) and records `find_form::measure`.
- `find_of` maps `measure` → `BDB_FIND_TERM_KIND_MEASURE`.
- Dialect `has_over` DIES: Count is the no-over aggregate case in the sum (aggregate op carries its over as payload where the op requires one); sdk-020's dummy `op` filler on Var heads dies in the same change. The ABI `bdb_find_term.has_over` *field* is sdk-008's C6 delta — this issue must not remove or reorder that field on its own. `find_of` projects the four-case sum onto whatever encoding sdk-008 ships (`BDB_FIND_TERM_KIND_COUNT` or always-read-`over`).
- Add a compile-SUCCESS test that projects a duration measure column and asserts the wire kind (there is currently neither a compile-fail nor a compile-success for it — sdk-018). The ABI already has `BDB_FIND_TERM_KIND_MEASURE` (`cpp/foreign/bumbledb_c.h:211`); the dialect never emits it.

## Acceptance criteria

- [ ] Gone: `rg -n 'has_over' cpp/src` → no matches; `rg -n 'find_form' cpp/src/query/ir.cc` shows exactly four enumerators.
- [ ] New lock: a cpp test constructing a measure find and asserting `BDB_FIND_TERM_KIND_MEASURE` on the wire (name suggestion: `cpp/tests/runtime/measure_find.cc` or extend conformance).
- [ ] Unchanged tests: all existing cpp tests green with zero assertion edits.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- Semantics identical for existing queries (Var/Aggregate/AggregateMeasure wire bytes unchanged until sdk-008 lands the ABI encoding); the NEW capability is only the dialect gaining a spelling for an already-legal engine sentence — no engine change. Count has no `over`; folds require it (C6) — same split as sdk-008 / sdk-027, not a third Count encoding. Do not change `bdb_query`'s nullable `rec` (C1).
