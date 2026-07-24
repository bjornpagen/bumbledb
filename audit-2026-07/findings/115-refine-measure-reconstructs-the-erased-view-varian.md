## refine_measure reconstructs the erased View variant from an emptiness sentinel

category: inelegance | severity: low | verdict: CONFIRMED | finder: cross:branching

### Summary

The `DurationFieldsCompare` arm of `refine_measure` destroys the `View` sum type up front (`view.len()` then `view.recycle()`) and then infers which variant it had from a boolean emptiness sentinel, resurrecting the all-positions identity when the sentinel says "was All". The sibling `DurationCompare` arm thirty lines above does the honest thing: a direct `match view` on the three variants. The module's own doc for `View` says it is "a three-variant representation, not a sentinel vector" (crates/bumbledb/src/image/view.rs:223-224); this arm converts it back into a sentinel vector and re-derives the variant with a guard — exactly the sentinel-vs-representation pattern docs/design/representation-first.md rules against (illegal states, sentinel nodes, reify-don't-flag).

### Evidence

All verified against the working tree:

- crates/bumbledb/src/image/view/apply.rs:435-440 — the sentinel reconstruction:
  ```rust
  let row_count = view.len();
  let mut positions = view.recycle();
  let survivors_input = !positions.is_empty() || row_count == 0;
  if !survivors_input {
      positions.extend(0..u32::try_from(row_count).expect("positions fit u32"));
  }
  ```
- crates/bumbledb/src/image/view/apply.rs:383-420 — the sibling `DurationCompare` arm matches `View::All(_)`, `View::Survivors { image, mut positions }`, and `View::Unbound => unreachable!("apply binds the view it filters")` directly, doing the same refine-in-place cursor loop per variant.
- crates/bumbledb/src/image/view.rs:220-224 — `View`'s doc: "A three-variant representation, not a sentinel vector."
- crates/bumbledb/src/image/view.rs:290-295 — `recycle()` returns `Vec::new()` for `Unbound | All(_)`; the guard's correctness is entirely parasitic on this.

Truth table of the guard (traced by hand):

| input variant | `len()` | `recycle()` | `survivors_input` | path taken |
|---|---|---|---|---|
| `All`, n > 0 | n | empty | false | extend identity `0..n` (correct) |
| `All`, n = 0 | 0 | empty | true | vacuous loop (correct by coincidence: conflated with empty Survivors) |
| `Survivors`, non-empty | k | positions | true | refine in place (correct) |
| `Survivors`, empty | 0 | empty | true | vacuous loop (correct) |
| `Unbound` | 0 | empty | true | vacuous → `Survivors` bound to the parameter image |

Two structural defects, no behavior bug today:

1. The `All`-with-zero-rows and empty-`Survivors` cases are distinguished by nothing and merely happen to want the same vacuous path — the correctness hinge the type exists to make explicit.
2. The `Unbound` row is worse than the finder claimed: where the sibling arm asserts `unreachable!`, this arm silently launders an unbound view into an empty `Survivors { image: Arc::clone(image), .. }`, masking a programmer-invariant violation instead of surfacing it.

### Failure scenario

None today. The hazard is a future drift in the parts the sentinel leans on: `recycle()` ever returning a pooled buffer with residual elements for `All` (buffer pooling is a live discipline here — view.rs:287-288 cites "buffers belong to the prepared query, the 40-execution doc"), or `len()` semantics shifting, would make the guard misclassify an `All` view as `Survivors` and silently drop or resurrect rows for the fields-measure predicate — the wrong-results class the sum type makes unrepresentable in the sibling arm. An `Unbound` view reaching this arm would also produce a bound empty result instead of a crash, hiding the bug.

### Suggested fix

Match on `View` in the `DurationFieldsCompare` arm exactly as `DurationCompare` does: `All(_)` → extend the identity into a fresh/recycled buffer then run the cursor loop; `Survivors { image, mut positions }` → refine in place; `Unbound` → `unreachable!("apply binds the view it filters")`. Delete `row_count`, `survivors_input`, and the guard. The shared cursor loop body (the `end == u64::MAX` ray check and `op.compare(&(end - start), &scalars[p])` write) is unchanged; only the variant dispatch moves from a boolean back to the type.
