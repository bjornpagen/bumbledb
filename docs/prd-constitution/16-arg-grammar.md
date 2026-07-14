# PRD 16 — Arg aggregates enter the grammar: the render/parse asymmetry dies

**Depends on:** 09 (the macro's agg table is quiet).
**Modules:** `crates/bumbledb-query/src/lib.rs` (the agg grammar :15,
the macro `AggOp` enum :241-245, the name table :479-483, the error
message :557-559), `ir/render.rs` (:194-200, already renders the Arg
forms), round-trip tests, `docs/architecture/20-query-ir.md` +
`docs/cookbook.md` if a recipe wants them.
**Authority:** audit #13, verified: `AggOp::ArgMax { key }` /
`ArgMin { key }` exist in the IR, execute, and render — but the
`query!` macro cannot write them, so the "round-trip golden" pins only
a subset of the IR and hosts must hand-build raw IR for a first-class
feature. The spec offered expose-or-formally-reject; the campaign
chooses EXPOSE (it is executable, tested machinery — hiding it is the
asymmetry, not the feature).
**Representation move:** none in the engine. The macro grammar grows
two forms; the notation becomes total over the executable IR's
aggregate surface.

## Context (decided shape)

- Surface grammar: `ArgMax(value, key)` / `ArgMin(value, key)` in find
  position — the projected term and the extremized key, matching the
  IR's `{ op-over + key }` decomposition. Exact spelling follows the
  existing macro convention (read the grammar block and mirror
  `CountDistinct`'s two-token style).
- The macro `AggOp` enum, name table, and error message ("takes
  Sum/Min/Max/Count/CountDistinct/Pack") all gain the two forms.
- `ir/render.rs`'s existing Arg rendering is the round-trip anchor:
  parse(render(q)) == q for Arg queries — the new round-trip goldens
  assert both directions over singleton and composite cases, including
  the self-carry form (`ArgMax(x, x)`) the signature table already
  types.
- `ArgAcrossRules` refusal (post-DNF) is untouched and re-pinned from
  the macro side: a multi-rule query with an Arg form rejects with the
  existing typed error — now writable in notation, so the rejection
  gets a notation-level test.
- The PRD-04-era signature table already carries Arg rows — unchanged;
  if the macro exposure reveals a typing hole the table missed, policy
  5.
- **The capability matrix (brief B7, approved):** this PRD closes with
  the full parity table — every semantic operation × {macro grammar,
  IR, validator, renderer, planner, executor} — recorded in this file's
  Results section. Every row is either accepted-everywhere (with its
  round-trip test named) or refused-at-the-earliest-phase (with its
  typed error named). Any asymmetry the matrix surfaces beyond the Arg
  forms is a policy-5 record + its own fix in this PRD if mechanical.

## Technical direction

Grammar first (parse → macro AggOp → lowering to IR), then the name
table and error string, then round-trips. The macro crate's tests
follow its existing golden style. Bench querygen may NOW gain Arg
shapes as a follow-up — explicitly OUT of scope here (generator
coverage expansion is registered future work; this PRD only closes the
notation asymmetry).

## Passing criteria

- `[shape]` The macro grammar block, enum, name table, and error
  message all list the Arg forms (grep each).
- `[test]` Round-trip goldens green both directions (singleton,
  composite, self-carry); the ArgAcrossRules notation-level rejection
  green; full bumbledb-query suite green.
- `[shape]` Zero engine-crate source changes (`git diff --stat` shows
  bumbledb-query + docs + tests only).
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`20-query-ir.md`'s aggregate table marks the Arg forms
notation-writable; the grammar block in the macro's module docs
follows; cookbook mention optional (no new recipe required).

## Results

The capability audit found one executable/renderer operation absent from the
macro grammar: Arg restriction, resolved mechanically by this PRD. No further
operation has a grammar/IR/validator/renderer/planner/executor asymmetry. In the
table, `A` means the layer accepts and preserves the operation; the evidence test
names the notation-to-render fixed point (and prepares it, therefore traversing
validation and planning). Executor semantics remain pinned by the engine suite.
Raw `ConditionTree::And`/`Or` nodes are decomposition syntax rather than additional
semantic operations: the macro writes their normal form as comma-conjunction and
clause union, and validation lowers both representations to the same ordered rules;
the renderer's `and(..)`/`or(..)` diagnostic form remains total for malformed raw IR.

| Semantic operation | Macro grammar | IR | Validator | Renderer | Planner | Executor | Round-trip evidence |
|---|---:|---:|---:|---:|---:|---:|---|
| Positive atom / join | A | A | A | A | A | A | `conflicts_normalized_text_is_a_fixed_point` |
| Literal or scalar-param selection | A | A | A | A | A | A | `closed_reference_handles_are_a_fixed_point`, `scalar_comparisons_round_trip` |
| Set-param membership binding | A | A | A | A | A | A | `mask_union_and_set_param_round_trip` |
| Negated atom | A | A | A | A | A | A | `negation_and_bare_handle_round_trip` |
| `Eq` / `Ne` comparison | A | A | A | A | A | A | `scalar_comparisons_round_trip` |
| `Lt` / `Le` / `Gt` / `Ge` comparison | A | A | A | A | A | A | `scalar_comparisons_round_trip` |
| Allen-mask comparison | A | A | A | A | A | A | `mask_union_and_set_param_round_trip` |
| Point-in-interval membership | A | A | A | A | A | A | `tax_rate_normalized_text_is_a_fixed_point` |
| Conjunction | A (`,` items) | A | A | A | A | A | `tax_rate_normalized_text_is_a_fixed_point` |
| Disjunction / rule union | A (clauses) | A | A | A | A | A | `calendar_union_golden` |
| Variable find | A | A | A | A | A | A | `tax_rate_normalized_text_is_a_fixed_point` |
| `Duration` find | A | A | A | A | A | A | `pack_and_duration_round_trip` |
| `Sum` / `Min` / `Max` | A | A | A | A | A | A | `aggregate_heads_golden` |
| `Sum` / `Min` / `Max` over `Duration` | A | A | A | A | A | A | `pack_and_duration_round_trip` |
| `Count` | A | A | A | A | A | A | `aggregate_heads_golden` |
| `CountDistinct` | A | A | A | A | A | A | `aggregate_heads_golden` |
| `Pack` | A | A | A | A | A | A | `pack_and_duration_round_trip` |
| `ArgMax` / `ArgMin` | A | A | A | A | A | A | `arg_heads_round_trip_singleton_composite_and_self_carry` |

Invalid compositions are representable as data (and, where meaningful, notation)
but stop at the first semantic boundary, validation. None reaches planning or
execution:

| Refused composition | Earliest phase | Typed error |
|---|---|---|
| Arg restriction across rules | validator after DNF | `ArgAcrossRules` |
| Arg beside a fold | validator | `MixedArgAndFold` |
| Arg terms with different key or direction | validator | `ArgKeyMismatch` |
| Arg keyed by an unordered value | validator | `NonOrderableArgKey` |
| Multiple `Pack` terms | validator | `MultiplePackTerms` |
| `Pack` beside a fold | validator | `MixedPackAndFold` |
| `Pack` beside Arg restriction | validator | `MixedPackAndArg` |
| `Pack` over a non-interval | validator | `PackInputType` |
| Typed fold over an illegal input | validator | `AggregateInputType` |
| `Count` carrying a variable | validator | `CountWithVariable` |
| Non-`Count` aggregate without a variable | validator | `AggregateWithoutVariable` |
| Aggregate input repeated as a group key | validator | `AggregateOverGroupKey` |
| Unsafe negation variable | validator | `NegatedVariableUnbound` |
| Variable bound only by point membership | validator | `MembershipOnlyVariable` |
| `Duration` in a binding | validator | `DurationInBinding` |
| `Duration` over a non-interval | validator | `DurationOverNonInterval` |
| `Duration` under a non-Sum/Min/Max aggregate | validator | `DurationAggregateOp` |
| `Duration` under a non-order comparison | validator | `DurationComparisonOperator` |
| `Duration` on both comparison sides | validator | `DurationBothSides` |

`arg_heads_round_trip_singleton_composite_and_self_carry` pins both parse/render
directions for all decided Arg shapes. `arg_across_rules_is_the_typed_notation_level_refusal`
pins the cross-rule row from macro notation through the existing typed error.
