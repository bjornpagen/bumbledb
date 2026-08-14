# engine-014: `rounds_budget` is an inert field on every prepared query

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F14
- **Depends on:** engine-001 (the Reach arm is where rounds live)

## The bug

`crates/bumbledb/src/api/prepared.rs:219-224` — the budget lives on the product, with a comment naming a field that doesn't exist ("Inert when `rec` is `None`" — there is no `rec` field, only `body`); `reach.rs:116-123`:

```rust
/// Amends this prepared query's derived-tuples / rec-rounds budget.
/// The rounds axis is inert when `rec` is `None` (rounds never
/// advance); the tuples axis judges every query — interiors-only
/// included. Hosts copy-paste.
pub fn set_derived_budget(&mut self, rounds: u32, tuples: u64) {
    self.rounds_budget = rounds;
    self.tuples_budget = tuples;
}
```

Only `run_reach` reads `rounds_budget` (`reach.rs:386`); on Cq/interiors-only queries it is a stored value with no reader.

## Why it's wrong

A field meaningful in one arm, stored on the product, documented by an "inert when" apology (Insight 5: the flag's applicability condition lives in prose instead of in the type). Two budget axes are essential; an inert rounds axis on non-recursive queries is the Program-era "every query might recurse" residue.

## The fix

Per `audit/CONTRACT.md §C3` ("Budgets"):

- `tuples_budget` stays on the prepared query (universal axis — interiors-only trips it; locked test `a_tight_tuple_budget_trips_on_an_interiors_only_query` is the pin).
- `rounds_budget` moves into the `PreparedPipeline::Reach` arm.
- `set_derived_budget` KEEPS its name, signature `(rounds: u32, tuples: u64)`, and observable behavior (locked): on a Cq pipeline the rounds argument is accepted and discarded — the doc comment says so plainly ("rounds applies only to recursive queries") instead of apologizing about a nonexistent field. No error, no new API.

## Acceptance criteria

- [ ] Placement: `rg -n 'rounds_budget' crates/bumbledb/src` shows the field only inside the Reach arm + `set_derived_budget`'s write path; `rg -n 'Inert when .?rec.? is .?None' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `a_tight_derived_budget_trips_under_reach` and `a_tight_tuple_budget_trips_on_an_interiors_only_query` (`crates/bumbledb/tests/api.rs`) pass UNCHANGED; `DEFAULT_REACH_ROUNDS = 1 << 16` and `DEFAULT_DERIVED_TUPLES = 10_000_000` values and names unchanged.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`; the `Bridge.lean` row citing `set_derived_budget` still resolves.

## Constraints

- `set_derived_budget` name/signature/behavior locked (public API + Bridge + docs cite it).
- Lands after engine-001.
