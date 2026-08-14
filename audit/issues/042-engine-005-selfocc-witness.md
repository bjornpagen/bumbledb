# engine-005: validation discards the unique-self proof; prepare searches again — witness sum + `self_occ`

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F5
- **Depends on:** none (foundation of the validate-side wave; engine-004, engine-016, engine-022, engine-028 build on it)

## The bug

`ir/validate/validate.rs` (`rec_roster`, ~258-265) counts positive self-atoms and refuses 0 (`RecArmMissingSelf`) / ≥2 (`NonlinearRecArm`), then throws the found occurrence away. Prepare re-runs the search — `crates/bumbledb/src/api/prepared/build.rs:407-413`:

```rust
let delta = normalized_rule
    .occurrences
    .iter()
    .filter(|occ| occ.role.participates())
    .find(|occ| occ.source.interior() == Some(rec_id))
    .map(|occ| occ.occ_id)
    .expect("RecArmMissingSelf judged at validate");
```

And the witness accessors re-buy rec-presence with `expect` — `crates/bumbledb/src/ir/validate.rs:523-541`:

```rust
pub(crate) fn rec_base_rule(&self, index: usize) -> RuleWitness<'_> {
    let rec = self.rec.as_ref().expect("rec present");
```

while the iterators (`rec_base_rules`, 555-558) are safe only because `map_or(0, …)` makes the range empty when rec is absent — two conventions for one fact.

## Why it's wrong

King's validate-then-forget, verbatim (Insight 6): the validator held the exact `OccId` of the unique self-atom in its hand and returned `()`-shaped knowledge, so prepare re-derives it with a search whose failure is spelled `expect(<the validator's error name>)` — a runtime restatement of a static fact. `Option<ValidatedRec>` on the witness is the same discard one level up: validation *knows* which shape the query is; the type doesn't say.

## The fix

Per `audit/CONTRACT.md §C3` ("Witness"):

```rust
enum ValidatedQuery {                       // param tables etc. stay query-global
    Cq    { interiors: Vec<ValidatedInterior>, main: ValidatedMain, ... },
    Reach { interiors: Vec<ValidatedInterior>, rec: ValidatedRec, main: ValidatedMain, ... },
}
struct ValidatedRecArm { self_occ: OccId, rule: LoweredRule, typing: RuleTyping }
```

- `rec_roster` RETURNS the occurrence it found; the arm stores `self_occ`. Nonlinear/missing-self stay boundary refusals with locked names — the witness cannot spell them.
- `rec_base_rule`/`rec_step_rule`/`rec_base_rules`/`rec_step_rules` move onto the `Reach` arm (no `expect`, no `map_or(0)` convention); Cq callers never see them.
- Prepare (`prepare_reach`) reads `arm.self_occ`; the `.find(...).expect(...)` deletes (note: `self_occ` is a *written-rule* occurrence — confirm the normalize mapping preserves it or store the normalized `OccId` at lowering, whichever the code actually needs; the acceptance grep below is the arbiter).
- `derived_count`/`rec_id` stored once on the witness per engine-028.

## Acceptance criteria

- [x] Proof carried: `rg -n 'expect\("rec present"\)|expect\("RecArmMissingSelf' crates/bumbledb/src` → no matches; `rg -n 'rec\(\)\.is_some\(\)' crates/bumbledb/src` → no matches.
- [x] Refusals unchanged: adversarial tests for `RecArmMissingSelf`, `NonlinearRecArm`, `SelfInBase`, `NegationInRec` pass UNCHANGED.
- [x] New locks: none beyond compile — the sum + non-optional `self_occ` is the lock.
- [x] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb` (`--lib` 1055 passed; `--test api --test adversarial_ir` 29 passed). `./scripts/check.sh` / `./scripts/lean.sh` not required green for this lane.

## Constraints

- Roster error names and trigger conditions locked. Boundary `ir.rs` unchanged (§C1).
- Coordinate with engine-016 (prepare consumes the sum) and engine-001 (pipeline arms mirror witness arms).
