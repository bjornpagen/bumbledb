# engine-006: `InteriorSignatures` seals through `Option` holes and screens the same id three times

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F6
- **Depends on:** none (validate-internal; parallel-safe with engine-005, same files — coordinate)

## The bug

`crates/bumbledb/src/ir/validate.rs:284-300`:

```rust
pub(super) struct InteriorSignatures<'a> {
    arities: &'a [usize],
    /// ... A `None` slot is a table not yet sealed ...
    sealed: &'a [Option<Predicate>],
    reader: Option<InteriorId>,
    /// `interiors.len() + rec.is_some()` — the well-formedness screen's
    /// address space, independent of how many signatures have sealed.
    derived_count: usize,
}
```

"This id is live" is encoded three times and re-checked three times: `screen` fails `index >= derived_count` (`validate.rs:306-318`); `column` calls `screen` AGAIN (line 328) even though `check_atoms` (`context.rs:502-503`) already screened the atom, then treats `arities.get(index)` miss as `UnknownInterior` a second time (330-332), then `sealed.get(index).and_then(Option::as_ref)` miss as `UnknownInterior` a third time (336-343).

## Why it's wrong

The `None` slot is a *phase flag stuffed into the data* (Insight 5): the rec's slot is pushed as `None`, base types against the hole, the hole fills, rec arms type — the sequencing invariant lives in a mutable Option array instead of in what the slice *contains*. Three encodings of liveness (count, arity slot, Some-predicate) shotgun-parse one linear seal, and disagreements between them are representable states each check papers over (Insight 4).

## The fix

Per `audit/CONTRACT.md §C3` ("Sealing"): the phase is the slice's extent, not a hole in it. **Named refusals stay as they fire today** — collapsing both screens into one `UnknownInterior` would change adversarial assertions.

- Type interiors against `sealed: &[Predicate]` containing exactly the already-sealed tables in declaration order. Interior *i* types against `&sealed[..i]`. Keep **two** named screens, one each: `UnknownInterior` iff the id is `>= derived_count`; `InteriorNotPrior` iff the reader is interior *i* and the target is `j >= i` (even when `j < derived_count`). Slice extent makes the unsealed-`None` hole unrepresentable; it does **not** get to rename a later-but-in-range read.
- Type rec BASE against the full interiors slice (rec's own predicate not yet present). `SelfInBase` stays the roster name for a base arm's self-atom, fired in `rec_roster` before typing; do not retarget those inputs to `UnknownInterior`.
- Type rec ARMS and MAIN against `sealed + rec_predicate` (a second slice or a chained lookup — no `Option` in the element type).
- `column` does not re-screen; its precondition is the screen `check_atoms` already ran (make it `debug_assert!` if belt-and-braces is wanted). The `arities` parallel array merges into the predicate slice (a `Predicate` knows its column count).

## Acceptance criteria

- [x] Gone: `rg -n 'Option<Predicate>' crates/bumbledb/src/ir/validate.rs` → no matches; `UnknownInterior` is constructed only for `id >= derived_count`; `InteriorNotPrior` stays the reader-`j >= i` screen (`rg -n 'UnknownInterior|InteriorNotPrior' crates/bumbledb/src/ir/validate.rs`).
- [x] Unchanged tests: every adversarial/validate test asserting `UnknownInterior`, `InteriorNotPrior`, `InteriorColumnOutOfRange`, `SelfInBase` passes UNCHANGED (same inputs → same error names).
- [x] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb` (`--lib` 1055 passed; `--test api --test adversarial_ir` 29 passed).

## Constraints

- Refusal names locked; hostile inputs keep producing the same errors — this is a representation change inside the validator only.
- Coordinate textually with engine-005 (same module).
