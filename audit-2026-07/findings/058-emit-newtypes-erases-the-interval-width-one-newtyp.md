## emit_newtypes erases the interval width — one newtype silently spans two encodings

category: bug | severity: medium | verdict: CONFIRMED | finder: macros:core

### Summary

`emit_newtypes` enforces "one newtype name = one inner type" by comparing the *rendered Rust type*, but the interval arm discards the width, so `interval<u64, 7> as Week` and `interval<u64> as Week` (or two different fixed widths) both render `::bumbledb::Interval<u64>` and pass the duplicate-declaration assert silently. Every other encoding dimension is caught by the same assert (`u64 as X` vs `i64 as X`; `bytes<5> as D` vs `bytes<6> as D` render `[u8; 5]` vs `[u8; 6]`). This contradicts the file's own doctrine — the width IS the type — and voids the compile-time half of the nominal-safety promise for exactly this one dimension: the mismatch is only caught at the runtime encode boundary.

### Evidence (all verified against the working tree)

- `crates/bumbledb-macros/src/lib.rs:2118-2126` — the inner-type rendering; the interval arm is `FieldTy::Interval(element, _) => (format!("::bumbledb::Interval<{}>", element_rust(element)), true)` — width `_`-discarded, while `FieldTy::FixedBytes(len)` embeds `len` in `[u8; {len}]`.
- `crates/bumbledb-macros/src/lib.rs:2128-2134` — the duplicate check: `assert_eq!(existing, &inner, "schema!: newtype \`{name}\` declared twice with different inner types")`. For two interval widths the inner types compare equal, so the assert never fires — even though, per doctrine, the inner *types* differ.
- `crates/bumbledb-macros/src/lib.rs:34-37` — the module doc: "the width is the type, the encoding stores only the start". Same doctrine in `docs/architecture/70-api.md:50-51` and `docs/architecture/10-data-model.md:20`.
- **Reproduced:** a temp test declaring `relation A { span: interval<u64, 7> as Week }` and `relation B { span: interval<u64> as Week }` in one `schema!`, then constructing one `Week` and placing it in both `A { span: w }` and `B { span: w }`, passes `cargo check` with zero diagnostics (temp file removed after verification).
- Downstream, nothing recovers the distinction:
  - `crates/bumbledb-theory/src/schema/spec.rs:79-85` — newtype names are "dropped at lowering: two specs differing only in newtype names lower to identical descriptors", so engine-side `validate` never sees them.
  - `crates/bumbledb-theory/src/schema/spec.rs:572-605` (`coherent`) — the `StatementNewtypeMismatch` check compares newtype *names* (`source_newtype == target_newtype` on `Option<&str>`); two faces both labeled `Week` with different widths agree.
  - `crates/bumbledb-macros/src/lib.rs:2287-2301` — the only place the width bites: the fixed-width encode arm calls `::bumbledb::__private::fixed_interval_{u64,i64}(..., {width}u64)?`, a **runtime** typed error.
- No compile-fail test pins the assert at all: `crates/bumbledb/tests/schema-compile-fail/` contains only `statement_newtype_mismatch.rs` and `statement_newtype_half_labeled.rs` — nothing for "declared twice with different inner types", in any dimension.

### Spec check

`docs/architecture/70-api.md:56-62` was checked both ways. It says "`as` is legal on ... intervals (the newtype wraps the engine value; **rustc polices domains**)" — the newtype's stated purpose is compile-time domain policing — and separately that "A fixed-width field's host type is the same checked `Interval<T>`; the typed write boundary checks the declared width ... wide values are unrepresentable at the type, never stored". So the shared host type and the runtime width check are documented design (this bounds the severity: no wrong-width value is ever stored, hence medium, not high). But nothing in the spec licenses one nominal label spanning two encodings; the assert exists precisely to forbid that, its message claims to, and by the width-is-the-type doctrine the two interval encodings are different types that the check's lossy Rust-rendering proxy fails to distinguish.

### Failure scenario

A schema labels two fields `as Week`, one `interval<u64, 7>` and one `interval<u64>` (or `interval<u64, 30>`). Expansion succeeds. Host code reads a `Week` from the general field (e.g. a 30-wide interval) and hands it to the width-7 field — rustc, the advertised domain police, approves. The write dies at the `fixed_interval_u64` boundary with a runtime typed error instead of the schema failing at expansion, where every analogous encoding conflict (element type, bytes width) already fails.

### Suggested fix

Key the equality on the encoding, not the rendered Rust type: carry the width into the comparison — e.g. store `(inner_rust, order_free, ValueType)` or render a comparison-only tag like `interval<u64, 7>` / `interval<u64>` alongside the emitted inner — and word the assert to name the two encodings: ``schema!: newtype `Week` declared twice with different encodings: interval<u64, 7> vs interval<u64>``. Add a `schema-compile-fail` case pinning the assert for the interval-width dimension (none exists for any dimension today).
