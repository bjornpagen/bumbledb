# PRD 13 — Resolvable filters: the chase gate parses

**Depends on:** baseline (self-contained in `plan/chase/evaluate.rs`;
if the 09–11 spine has landed, enforcement reads use the witness
accessors — otherwise the old shapes; either baseline works, the file is
the boundary).
**Modules:** `crates/bumbledb/src/plan/chase/evaluate.rs` +
`plan/chase/evaluate/tests.rs` only.
**Authority:** the audit's finding 2 (~15 `unreachable!` sites in one
file, messages "filters_prepare_resolvable admits literal points/masks/
interval constants only", "folded filters are prepare-resolved" ×5);
King: a validator returns a bool and throws away what it learned; every
downstream match re-checks and refuses the impossible.
**Representation move:** the gate becomes a parser. It returns the
narrowed filter vocabulary it proved, and the evaluator consumes that
vocabulary totally — the ~15 re-refusals become unrepresentable.

## Context (decided shape)

```rust
/// A closed atom's filter, proven prepare-resolvable — constants only,
/// over the sealed extension's column words. Minted exclusively by
/// `parse_resolvable`; consumed totally by `surviving_ids`.
enum ResolvableFilter {
    /// Eq/Ne/Lt/Le/Gt/Ge against one encoded word (scalar columns).
    WordCompare { field: FieldId, op: CmpOp, word: u64 },
    /// Eq against a plan-constant word set (attached memberships).
    WordSetEq { field: FieldId, words: Box<[u64]> },
    /// A constant point inside the column's interval.
    PointIn { field: FieldId, point: u64 },
    /// The column's interval within a constant outer interval.
    Within { field: FieldId, start: u64, end: u64 },
    /// Literal-mask Allen between the column and a constant interval.
    Allen { field: FieldId, other: (u64, u64), mask: AllenMask },
    // Exactly the vocabulary the CURRENT gate admits — enumerate from
    // the existing `filters_prepare_resolvable` arms, one variant per
    // admitted shape, none extra. If the audit of those arms finds a
    // shape admitted but unevaluated (or vice versa) that is a PRD-05
    // -era latent bug: fix it here and pin it with a test.
}

fn parse_resolvable(filters: &[FilterPredicate]) -> Option<Vec<ResolvableFilter>>;
```

`filters_prepare_resolvable` (bool) dies; the fold's condition 2 becomes
`let Some(parsed) = parse_resolvable(...)` — `None` refuses the fold
exactly as `false` did (param-bearing, measure, str-pending, and every
other shape refuse by non-parse; the refusal comments move onto the
parser's `None` arms so the derivations survive). `surviving_ids`
consumes `&[ResolvableFilter]` with a total match — its `unreachable!`s
die. The EXPLAIN picture path (`folded_picture`) keeps rendering from the
ORIGINAL retained `FilterPredicate` list on the folded occurrence (the
picture is the user's spelling, not the parsed form) — unchanged.
`into_stats`' diagnostic re-run of `surviving_ids` parses again at that
call site (cold path; the parse is cheap and the Option is already
proven Some by the fold mark — consume `.expect("folded occurrences
parsed at fold time")`? NO: re-parse and treat `None` as the impossible-
by-construction arm via the mark carrying the parsed set instead —
decide: store `Box<[ResolvableFilter]>` alongside the sibling
attachment at fold time if `FoldedMark` can carry it without breaking
`Role: Copy`; the recorded constraint says it cannot — so the stats path
re-parses and maps `None` to an empty fold line with a debug_assert,
never a panic. Record this choice in the module doc.)

## Technical direction

1. Enumerate the current gate's admitted shapes by reading
   `filters_prepare_resolvable` and `surviving_ids`/`row_satisfies`
   side by side — the variant list above is a sketch; the CODE's
   admitted set is normative. Any asymmetry found (admitted-but-
   unevaluated / evaluated-but-unadmitted) is a bug: fix, test, record.
2. Land `ResolvableFilter` + `parse_resolvable`; rewrite
   `surviving_ids` total; delete the boolean gate; move each refusal
   comment (param v0 trigger, measure error-timing, etc.) onto its
   `None` arm.
3. Keep the public shape of the fold unchanged: conditions, marks,
   attachments, complement logic, rule-death — this PRD touches HOW
   filters are judged resolvable and evaluated, nothing else.
4. Tests: the existing condition tests re-anchor mechanically (they call
   the gate — now the parser). Add: (a) parser totality — for every
   `FilterPredicate` shape the file's own vocabulary can construct,
   `parse_resolvable` returns Some(exact expected variant) or None,
   exhaustively, so a future filter kind fails THIS test instead of
   growing a new unreachable; (b) the evaluator agreement test — for a
   fixture extension, folded results via the parsed path equal the
   pre-PRD expected id-sets already pinned in the tests.

## Passing criteria

- `[shape]` `grep -c "unreachable!" crates/bumbledb/src/plan/chase/evaluate.rs`
  → ≤ 2 (from ~17), each survivor's message naming a genuine boundary,
  none saying "filters_prepare_resolvable" or "prepare-resolved";
  `grep -n "filters_prepare_resolvable" crates` → zero hits.
- `[test]` The parser-totality test and the evaluator-agreement test;
  every pre-existing evaluate test green with unchanged id-set pins.
- `[shape]` The refusal derivations (param v0 + trigger, measure
  error-timing) survive verbatim on the parser's None arms — grep the
  trigger strings.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`40-execution.md` § the chase, one sentence: condition 2 is a parse — the
gate returns the resolvable vocabulary and the evaluator is total over
it.
