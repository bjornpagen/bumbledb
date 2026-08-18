# One predicate algebra and the executor's interior sums

## Ruling

The execution layer evaluates one small predicate algebra — scalar compare,
same-fact compare, point membership, Allen mask, duration measure — and it
spells that algebra **four separate times**, differing only in where an
operand's words come from. This proposal collapses the four interpreters
into one predicate walk over an operand-provider capability, and settles the
executor-layer structures whose state is currently spread across flags,
parallel vectors, and in-band sentinels.

Everything here is crate-private. No storage format, no ABI, no public API
changes. This pass is independent of
[`instance-lifetime.md`](instance-lifetime.md) and gates nothing there; it
may land before or after. It extends recorded audit finding 053, whose
ledger entry ("two FilterPredicate interpreters") undercounts the debt —
there are four.

## The four interpreters

| Interpreter | Operand source | Site |
|---|---|---|
| `row_matches` | image columns | `image/view/apply.rs:115` |
| `fact_matches` | LMDB fact bytes | `exec/dispatch/key_probe_fact.rs:115` — its own doc: "the same word compositions the view evaluator runs over image columns" |
| `verdict::Leaf` | ray-probe binding slots | `exec/verdict.rs:60` |
| the `Placed*` family (`PlacedComparison` / `PlacedAllen` / `PlacedWordComparison` / `PlacedDuration`) | batch words + slots | `ir/normalize.rs:214-296` |

The `point_in` / `const_interval` / `resolve` helper trio is copied verbatim
into three of them. The eleven-variant `FilterPredicate` is additionally
matched exhaustively in five modules (`apply.rs`, `key_probe_fact.rs`,
`plan/ground/evaluate.rs`, `plan/selectivity.rs`, `api/prepared/bind.rs`) —
a new predicate kind costs edits in five files, and each copy re-derives the
"interval compares under `Eq` only" rule independently.

## The fix

The repo's own established idiom, applied once more:
`interval::sweep::Continuation` and `exec::run::Counters` are monomorphized
traits that let one algorithm serve several callers. The predicate walk gets
the same shape — an operand-provider capability:

```rust
pub(crate) trait Operands {
    fn word(&self, at: OperandAddr) -> u64;
    fn pair(&self, at: OperandAddr) -> (u64, u64);
    fn block(&self, at: OperandAddr) -> &[u8];
}
```

One predicate evaluator generic over `Operands`; four providers below it —
image columns, fact bytes, binding slots, batch words. Static dispatch,
monomorphized per source, exactly like the kernel's `Sink`/`Counters`
generics. This:

- deletes three interpreters and the copied helper trio;
- collapses the `Placed*` family into the predicate type it mirrors;
- makes a new filter kind a one-file change;
- and removes the *reason* four of the `Executor`'s seven shadow spines
  exist (below).

By parametricity, a predicate walk generic over `Operands` cannot depend on
which source it reads — the property the four copies maintain today by
review.

## The executor and plan sums

Each entry: the current shape, the illegal or duplicated state, the
replacement.

1. **`Executor`'s seven aligned spines** (`exec/run.rs:600-671`).
   `residual_slots`, `word_residual_slots`, `allen_residual_slots`,
   `allen_masks`, `duration_residual_slots`, `point_probe_slots`,
   `anti_probe_slots` — every one a `Vec<Vec<_>>` "aligned with" a per-node
   list, one of them aligned with another aligned vector (a three-deep index
   chain), carried by nineteen "aligned with" comments and no types. Four of
   the seven dissolve with the `Placed*` family; the survivors move into one
   `Vec<NodePrecompute>` owned by the node they describe. This does **not**
   re-litigate the recorded refusals: the kind-grouped batching lists on
   `PlanNode` stay (that grouping *is* the batching law), and no `NodeScratch`
   extraction happens (the recorded refusal stands — genuinely per-execution
   scratch keeps its shape). What deletes is the shadow *copy* of the spine.
2. **`ValidatedPlan.estimates` parallel to `nodes`** (`plan/fj.rs:293,307`).
   Stored as an unchecked same-length vector; both readers compensate
   (`if i < estimates.len()`, `.get(node_idx).unwrap_or(0)`). Becomes
   `PlanNode { estimate: u64, .. }`; `fold_split`'s duplicate-on-split
   becomes copying a field; both guards delete.
3. **`AggregateSink` multiplexes two sinks on one struct**
   (`exec/sink.rs:473-489`). Fold state and Pack state are co-resident though
   validation proves them exclusive; the mode is re-derived by `pack_slot()`
   re-scanning `finds` at six sites, two of them hot-path gates; the
   exclusivity survives as two `unreachable!`s. Becomes
   `enum GroupState { Folds { accs, n_aggs }, Pack { claims } }` — one match,
   both `unreachable!`s and all six re-derivations delete; `n_aggs` is
   derived from `finds`.
4. **`ProjectionSink`'s 2×2 with a defensive illegal arm**
   (`exec/sink.rs:328-341`). `measures`, a `ProjectionSources` tag computed
   *from* `measures.is_empty()`, and a "empty on the fast paths" third field
   encode one fact three ways, and `aim` keeps a cold fallback arm for the
   two illegal cells. The measure table and resolved sources move inside the
   `Measured` variant; the illegal cells become unrepresentable.
5. **`Colt`'s three encodings of selection state** (`exec/colt.rs:352-364`).
   `selection_levels` duplicates `selection_kinds.len()` (asserted at three
   construction sites); `(select_state, start)` is a two-field machine whose
   `Pending` cell holds a meaningless cursor guarded by a **release**
   `assert!`. `selection_levels` deletes;
   `enum Start { Vacuous(Cursor), Pending, Selected(Cursor) }` returns a
   cursor only where one exists — the release assert becomes a type.
6. **`OverlapCache::Dir`'s four meaningless zeros**
   (`interval/overlap.rs:53-65`). A tallied-but-unbuilt entry stores four
   zeroed fields and readers test `p != 0`; `probe()`'s `Option` means
   "declined by the amortization gate", not "absent". Becomes
   `enum Dir { Tallied { .. }, Built { .., p: NonZeroU32 } }` and
   `enum Probe { Declined, Ready(u32) }`.
7. **`AntiProbeSpec`'s `key_words == 0` gate form** (`exec/run.rs:730-744`).
   The documented "three probe forms" live in one flat struct where the gate
   form is the zero of a count and `parts` co-varies silently. Becomes
   `enum AntiProbeForm { Gate, Keyed { parts, key_words: NonZeroUsize } }`.
8. **Reach unit labels as `format!` prose**
   (`exec/introspection.rs:95-102`). Kind, index, and delta occurrence —
   all structured, all available — are flattened into strings no consumer
   can read back. Becomes
   `enum UnitLabel { Base(usize), Rec { idx, delta: OccId }, Main(usize) }`
   with `Display` at the edge.
9. **`LeafPrecompute::Fast` retains a list and its own partition**
   (`exec/run.rs:759-767`). `residual_sources` is fully re-derived into two
   split lists and all three are kept. Keep the partition only.
10. **`pairs_off(.., exact: bool)`** (`plan/ground.rs:527-534`). A positional
    boolean whose meaning lives in a doc comment. Becomes
    `enum Matching { Containment, Multiset }`; the neighbouring
    comment-only parallel `(rules, finds)` slices become one slice of pairs.

## Sequencing

Land the predicate algebra first — it shrinks item 1 from seven spines to
three before that refactor starts. Everything else is independent and can
land in any order, one item per change.

## Gates

- The kernel-purity gate is unchanged and still holds: `Executor::execute`'s
  signature names no catalog, transaction, identity, or dictionary type.
- Scenario lanes produce byte-identical answers before and after each item —
  this pass changes representation, never results.
- One predicate evaluator: the `point_in`/`const_interval`/`resolve` trio
  exists once; `FilterPredicate` is matched exhaustively in at most two
  modules (the evaluator and the planner's selectivity reader).
- The audit ledger's finding-053 row closes with a pointer here.
- No new `Vec` on `Executor` is documented as "aligned with" anything.

## Refused designs

- Re-litigating the kind-grouped `PlanNode` batching representation (recorded
  refusal stands — the grouping is the batching law).
- A `NodeScratch` extraction (recorded refusal stands — the grouping buys no
  new invariant).
- `dyn Operands` — the providers monomorphize like `Sink` and `Counters`.
- Changing `OVERLAP_CROSSOVER` / `FLAT_SWEEP_CEILING` here — those are
  measurement debts owned by the TODO's re-pin item, not representation
  debts.
