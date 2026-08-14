# sdk-018: compile-fail holes — C++ and TS do not pin the cutover the way `query!` does

- **Severity:** medium
- **Tree:** sdk (cpp + ts test suites)
- **Status:** FIXED(9527e64e)
- **Source:** audit/sdks.md #18
- **Depends on:** sdk-001 (cpp phase machine), sdk-004 (measure find), sdk-005 (ts phase type), sdk-013 (condition trees) — this issue is their TEST deliverable; it lands last in the SDK wave.

## The bug

The Rust `query!` suite pins the cutover statically (`named_head_without_keyword`, phase order, one rec, nonempty base/rec, bare main required). C++ and TS do not pin **types**:

- C++ already has consteval-trap fixtures, which is not the same as a phase machine: `cpp/tests/compile_fail/query_second_recursive.cc`, `query_interior_after_recursive.cc`, `query_interior_after_main.cc`, `query_duplicate_interior.cc`. They compile the illegal *call* and fail inside `a_second_recursive_is_refused` / `interior_after_recursive` / `interior_or_recursive_after_a_main_rule`. After sdk-001 those methods leave the overload set; the same files should fail as ill-formed calls — do **not** add a second `query_recursive_twice.cc` (that is `query_second_recursive.cc`).
- Still missing: recursive-after-main (`query.cc:208` fires `interior_or_recursive_after_a_main_rule` for `.recursive` too; no `query_recursive_after_main.cc` twin of `query_interior_after_main.cc`). Interiors-only / rec-only `prepare<>` compiles — `cpp/src/db/db.cc:279-281` is unconstrained `template<auto Query>` (empty main is a typed `query_value` — sdk-001). `FindTerm::Measure` as a column is unwritable so neither a compile-fail nor a compile-success exists (sdk-004). Condition trees unwritable (sdk-013). Negation-in-rec is a runtime/consteval wall with no fixture.
- TS: second-recursive, interior-after-recursive, duplicate interior name, empty name are runtime throws (`ts/src/query/lower.ts:1614-1642`; `ts/test/query.test.ts:1161-1215`) while after-main IS `never` — inconsistent coordinates.

## Why it's wrong

A refusal that lives in a trap instead of a type has no regression net: nothing fails when the trap is deleted (Insight 6 — the proof is enacted, not carried; Insight 12: the test suite is where representation claims become checkable). Rust proved these errors are static; the siblings assert less than they know.

## The fix

Once the phase machines land (sdk-001, sdk-005), the compile-fail suite IS the type. Keep the existing trap fixtures (they become ill-formed-method tests). Add only the holes:

- C++ `cpp/tests/compile_fail/`: `query_recursive_after_main.cc`, `query_prepare_without_main.cc`, `query_negation_in_rec.cc` (consteval-fail if that wall stays consteval). Do not add `query_recursive_twice.cc`. Compile-SUCCESS: measure find column (sdk-004), condition tree (sdk-013), >4-rule interior (sdk-012).
- TS: type tests (`@ts-expect-error` or `.test-d.ts`): second recursive, interior-after-recursive; runtime pins stay for the string-name walls TypeScript cannot see (duplicate/empty interior name — those are VALUE facts, correctly runtime).

## Acceptance criteria

- [ ] Each named fixture exists and fails/passes compilation as specified; the cpp compile-fail harness (`cpp/tests/compile_fail/CMakeLists.txt` pattern) runs them in `ctest`.
- [ ] Unchanged tests: all existing fixtures byte-identical; no runtime test weakened — TS throw-tests convert to type tests only where the call became unspellable (per sdk-005's criteria).
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd ts && pnpm test`; `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` (unchanged, as the reference standard).

## Constraints

- Blocked by sdk-001/004/005/012/013 — fixtures for states those issues make unrepresentable cannot be written first. Do not add fixtures asserting engine-side refusals (those live in engine adversarial tests).
