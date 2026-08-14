# engine-022: `rec_roster` / `lower_rec_pool` / `measure_in_rec` are three walks of one parse

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(1af537e5)
- **Source:** audit/engine.md F22
- **Depends on:** engine-005 (the typed `ValidatedRec` is the parse's output), engine-004 (owns the emptiness-check split)

## The bug

`crates/bumbledb/src/ir/validate/validate.rs` — the rec is judged in three passes that each re-walk what an earlier pass established: `rec_roster` (~245-293) rejects empty base/step, self-in-base, missing/nonlinear self, negation; `lower_rec_pool` (~306-346) rejects empty base/step AGAIN after DNF plus pool caps/nesting/width; `measure_in_rec` walks the conditions a third time after typing to refuse `Term::Measure` in rec bodies.

## Why it's wrong

Parse, don't validate (Insight 6): each pass should *refine the type* so the next pass starts from what is known. Instead three passes each start from the raw material, and the same fact (arm emptiness) is tested at two stages under one error name (engine-004). A third full condition-walk for measure is a screen bolted after typing when the typing pass itself visits every condition and could refuse in place.

## The fix

Per `audit/CONTRACT.md §C3`: one rec parser with a typed result.

- Pipeline: written roster (names locked: `EmptyRecursiveBase`, `EmptyRecursiveStep`, `SelfInBase`, `RecArmMissingSelf`, `NonlinearRecArm`, `NegationInRec`) → lower/DNF (pool caps stay; post-DNF emptiness per engine-004's ruling) → typing — producing `ValidatedRec` with nonempty arms and per-arm `self_occ` (engine-005). Each stage consumes the previous stage's OUTPUT type, not the raw `ir::Rec`.
- `measure_in_rec` stops being a post-pass walk: the measure refusal (`MeasureInRec`, name locked) is raised during `type_rules`' condition visit under a rec-body context flag — one walk, refusal in place. (Making `Term::Measure` unrepresentable in a distinct rec-condition type is over-engineering at the hostile boundary — the boundary type is shared by design, §C1; the fix is one-walk refusal, not a parallel condition grammar.)
- Nonlinearity is NOT re-tested after DNF (prepare's re-find dies via engine-005).

## Acceptance criteria

- [ ] One walk per fact: `rg -n 'measure_in_rec' crates/bumbledb/src` → no matches (refusal moved into typing); arm emptiness tested once per distinct fact per engine-004.
- [ ] Unchanged tests: every adversarial test asserting the six roster names + `MeasureInRec` passes UNCHANGED (same inputs → same errors).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- All refusal names and trigger inputs locked; `MAX_RULES` pool cap value unchanged; no new caps.
- Lands with/after engine-005; shares text with engine-004 — one fixer should take 004+005+022 together.
