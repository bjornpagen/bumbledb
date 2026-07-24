## Fact structs miss free Copy/Eq derives their generated key-struct siblings already have

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: macros:core

### Summary

`emit_fact_struct` emits generated fact structs with `#[derive(Debug, Clone, PartialEq)]` only, even though every field type the macro can put in a fact struct is `Copy` and `Eq` by construction. The generated key structs — whose fields are a projection of the very same declarations — already derive `Copy`, and every other emission in the file (newtypes, host enums, the schema descriptor enum) carries `Copy + Eq`. The fact struct is the single outlier, for no representational reason: `Copy` and `Eq` are free given the representation the machinery guarantees.

### Evidence

All citations verified against the working tree.

- **The outlier derive** — crates/bumbledb-macros/src/lib.rs:2441 (inside `emit_fact_struct`, defined at :2387):
  ```rust
  "#[derive(Debug, Clone, PartialEq)]\n\
   pub struct {name}{struct_params} {{ {struct_fields} }}\n\
  ```
- **The sibling key struct already derives Copy** — lib.rs:2685:
  ```rust
  "#[derive(Debug, Clone, Copy, PartialEq)]\n\
   pub struct {key_name}{struct_params} {{ {struct_fields} }}\n\
  ```
  Key-struct fields are a subset of the fact's fields, produced from the same `Field` values — so the compiler already accepts `Copy` over this exact field vocabulary.
- **The field-type inventory is closed and all-Copy/all-Eq** — `rust_field_ty`, lib.rs:2237-2251: `bool`, `u64`, `i64`, `&'a str` (the one variable-width kind is a shared borrow, per `is_borrowed` lib.rs:2233-2235 — the fixed-width law makes `bytes<N>` owned `[u8; N]`), `[u8; N]`, `::bumbledb::Interval<T>`, or a newtype name.
  - `Interval<T>` derives `Copy, Eq, Hash` (crates/bumbledb-theory/src/interval.rs:20-21), and its element is restricted to `u64`/`i64` (`element_rust`, lib.rs:141-146), so the derived `Copy`/`Eq` bounds are satisfied.
  - Emitted newtypes derive `Copy, Eq, Hash` unconditionally — lib.rs:2142: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash{order})]`.
  - Nothing float-like exists anywhere in the value vocabulary, so derived `Eq` is sound.
- **No compensating manual impls** — a grep of the file for `impl Copy` / `impl Eq` finds none; the derive lists at lib.rs:2012, 2142, 2192, 2685 are the only other emitted derives, and all of them include `Copy` (and all but the key struct include `Eq`).

### Failure scenario

Host code holding a generated fact value (e.g. `Holder<'_>`) must `.clone()` it to insert it twice or keep it after a comparison, even though the value is a handful of words and shared borrows. A `HashSet`/`BTreeSet` of decoded facts, or `assert_eq!`-style total-equality reasoning via `Eq` bounds, is impossible — both are free given the generated representation, and both match the value-semantics doctrine the newtypes and host enums already follow (`docs/design/representation-first.md`: the representation, not host-side ceremony, should carry the semantics).

### Suggested fix

In `emit_fact_struct` (lib.rs:2441), change the derive list to `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` (and consider `Hash`, which every field also supports, for parity with the newtypes and host enums). For symmetry, add `Eq` to the key-struct derive at lib.rs:2685. Both are pure additions — every field type is `Copy + Eq + Hash` by construction, so no schema can make the derives fail to compile.
