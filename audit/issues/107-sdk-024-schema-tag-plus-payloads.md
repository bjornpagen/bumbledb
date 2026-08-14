# sdk-024: discriminator-plus-all-payloads across the C++ schema lane (spec, raii, where, manifest)

- **Severity:** medium
- **Tree:** sdk (cpp schema + foreign/raii)
- **Status:** OPEN
- **Source:** audit/sdk-rest.md #2
- **Depends on:** sdk-023 (the IR restructure); sdk-026 lands inside the statement-form reshape
- **Conflicts with:** sdk-023, sdk-025, sdk-026 (same files)

## The bug

Every schema-lane "sum" is a struct carrying a tag plus EVERY alternative's fields simultaneously — sdk-011's query-IR defect, on the schema table:

- `field_data` (`cpp/src/schema/spec.cc:51-57`) / `field_class` (`cpp/src/relation/classify.cc:36-39`): `kind` plus `fixed_len` plus `width`; `width == 0` is the sentinel for "general interval" (engine `ValueType::Interval { width: Option<u64> }`).
- `selection_literal` (`spec.cc:79-87`), `axiom_literal` (`cpp/src/closed/axioms.cc:24-30`), raii `owned_literal` (`cpp/foreign/raii.cc:860-868`): `is_handle` plus bool, u64, i64, and text at once.
- `bound_data` / raii `owned_bound` (`spec.cc:142-146`, `raii.cc:943-948`): `kind` plus `lit` AND `field`.
- `window_data` / raii `owned_capacity_window` (`spec.cc:148-152`, `raii.cc:953-958`): `kind` plus both bounds always.
- `statement_data` (`spec.cc:159-167`): `form` plus source, target, bidirectional, weight, weight_field, and window at once. A key statement carries a dummy capacity window.
- `class_entry` (`spec.cc:175-179`): `bool classed` as an existence flag beside `class_name`.
- `where_slot` (`cpp/src/closed/where.cc:27-32`): `bool bound` plus a leftover `selection_literal` — sdk-009's wildcard-as-absent, on ψ selection.
- `StatementRow` (`cpp/src/db/manifest.cc:24-28`): `bool is_key` plus relation/projection that only keys read.

`cpp/AGENTS.md` §8 makes `std::variant` the blessed closed sum and forbids "manual discriminator + payload structs" verbatim. raii used a variant for `owned_statement` and an optional for closedness — then rebuilt `owned_literal` as the product. NTTP/consteval wanting trivial types is a constraint, not a representation.

## Why it's wrong

This is Minsky's three-boolean problem as a data layout (Insight 4): the state space is the product of all payloads times the tag, and only a sliver is meaningful; every consumer switch reads the tag and trusts the right payload was the last one written. `width == 0` is a sentinel (Insight 8): absence encoded as a magic value the next reader must remember not to treat as a width. `where_slot.bound` is absence reified as a flag (sdk-009).

## The fix

Per `audit/CONTRACT.md §C1` (trusted layers are sums) and `cpp/AGENTS.md` §8, in preference order:

1. `std::variant` for literals, bounds, windows, field types, statements (containment vs mirrors is sdk-026) — IF variant is NTTP-usable (probe `std::is_structural_v`; record the result in the commit).
2. If variant fails NTTP: per-alternative arrays / parallel closed vs ordinary tables (sdk-023 owns the closed split).

Either way: no recorded-IR struct carries a tag plus more than one alternative's payload. `where_slot` is `std::optional<selection_literal>` (unbound = absence). `StatementRow` is `Key { relation, projection } | Other`. `width == 0` dies as a dialect sentinel — general vs fixed-width are cases. raii `owned_literal` / `owned_bound` / `owned_weight` match.

ABI tagged structs stay (C1 / sdk-008 essential-C). Dummy `has_width` on `bdb_value_type` at the C boundary stays; `scalar_type` filling a dummy `element` is the ABI projection, not the recorded IR.

## Acceptance criteria

- [ ] Gone: `rg -n 'bool is_handle;|bool classed;|bool bound;' cpp/src/schema cpp/src/closed cpp/foreign/raii.cc` → no recorded-IR matches; `rg -n 'width == 0' cpp/src/db/wire.cc cpp/src/relation/classify.cc` → no "0 means general interval" sentinel in dialect types (ABI `has_width` may remain).
- [ ] Unchanged tests: cpp `ctest` green with zero assertion edits; wire bytes / fingerprints identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Consteval/NTTP viability is the real constraint — probe FIRST, choose the arm, say which in the commit. Semantics identical. Coordinate with sdk-023/025/026 (same files). Do not change C ABI field layout (C1). `has_over` / `has_measure` remain sdk-008.
