# engine-003: rec identity is `Option` + the `interiors.len()` pun, recomputed at every layer

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED(9c77c002) (accepted half; boundary re-encoding remains refused per CONTRACT §C1/§C2)
- **Source:** audit/engine.md F3
- **Depends on:** engine-005, engine-016 (the witness sum is where the stored ids live; engine-028 is a duplicate of this issue)

## The bug

The rec's identity is a numbering convention restated at every layer. `crates/bumbledb/src/ir.rs:74-85` documents it ("the rec, when present, is `interiors.len()`"); then:

```rust
// api/prepared/reach.rs:303
let rec_id = self.interiors.len();
// api/prepared/build.rs:373-375
let rec_id = crate::ir::InteriorId(
    u32::try_from(witness.interiors().len()).expect("overflow judged at validate"),
);
// ir/render.rs:153
let id = query.interiors.len();
// bumbledb-bench/src/naive/query.rs:269
let rec_id = sets.len() - 1;
// bumbledb-bench/src/translate/reach.rs:43
let rec_id = query.interiors.len();
```

An `InteriorId` one past the interiors vec is either the rec or `UnknownInterior` depending on a *flag the id does not carry* (`rec.is_some()`).

## Why it's wrong

Dijkstra: the off-by-one lives in the numbering. "Last id, if the Option is Some" is a two-part coordinate whose halves live in different fields, so six independent sites re-derive it with arithmetic and two of them re-`expect` an overflow proof validation already established (Insight 9: derived facts recomputed instead of stored; Insight 6: the proof discarded).

## The fix

Scoped per `audit/CONTRACT.md §C2`:

- **Accepted half:** the id is computed ONCE, at validate, and stored: the witness (engine-005's `ValidatedQuery::Reach` arm) carries `rec_id: InteriorId` and `derived_count: u32` as data; prepare copies them into the `PreparedPipeline::Reach` arm; execute/render/introspect read the stored value. No site re-derives `len() + usize::from(is_some())`; the overflow `expect("overflow judged at validate")` sites die. The bench naive/translate oracles read `query.interiors.len()` from the *boundary* object — legal there (it is the boundary numbering) but each computes it exactly once at its own entry, with engine-019/021 restructuring those.
- **Refused half (do NOT attempt):** re-encoding the boundary as `derived: Vec<Derived>` / `enum AtomSource { Edb, Derived(DerivedId) }` — refused per CONTRACT §C1: the hostile boundary `Query { interiors, rec: Option<Rec>, head, rules }`, the JSON corpus, the C ABI, and the TS wire type stay shape-unchanged; `Option<Rec>` IS the sum's boundary spelling. The engine-internal sums (witness, prepared) are where absence stops being a flag.

## Acceptance criteria

- [ ] One computation site: `rg -n 'interiors\(\)\.len\(\)|interiors\.len\(\)' crates/bumbledb/src/api crates/bumbledb/src/ir/render.rs` → only the validate-time computation (and boundary-facing render if it reads the stored witness value instead, better); `rg -n 'expect\("overflow judged at validate"\)' crates/bumbledb/src` → no matches.
- [ ] Boundary untouched: `git diff --stat crates/bumbledb/src/ir.rs` shows no change to `Query`/`Rec`/`AtomSource` shapes; serde round-trip tests unchanged.
- [ ] Unchanged tests: full `cargo test -p bumbledb` and `-p bumbledb-bench` green, zero assertion edits.
- [ ] Green: `./scripts/check.sh`; `./scripts/lean.sh` (three-way comparator still agrees).

## Constraints

- Semantics identical; `InteriorIdOverflow` stays the boundary refusal name.
- Lands after engine-005/engine-016 (mostly their acceptance checklist, plus render/bench single-site cleanups). Absorbs engine.md F28.
