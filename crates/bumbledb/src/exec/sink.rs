//! The two consumers of bindings (docs/architecture/40-execution.md): set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! (`docs/architecture/40-execution.md` D2/D3; semantics normative in
//! `20-query-ir.md`).
//!
//! Aggregation never materializes the join: group maps live in sink state;
//! the fold domain of every aggregate is the group's **set of distinct
//! full bindings over all query variables** — two postings of amount 100
//! to one account are two distinct bindings (their serial ids differ), so
//! `Sum(amount) by account` is 200. The stated footgun: joining a
//! multiplicity-adding relation multiplies the binding set, exactly as in
//! SQL.
//!
//! Slots are **words**, not variables: an interval-typed variable occupies
//! two consecutive binding slots (the [`crate::ir::normalize::SlotWidth`]
//! layout), so every [`FindSpec`] carries its slot span and every consumer
//! walks widths — the seen-set keys the full slot array (both interval
//! words hashed), the group key concatenates spans, and emitted rows are
//! word rows the result buffer re-assembles by find type.

use crate::encoding::encode_i64;
use crate::exec::wordmap::WordMap;

mod aggregate;
mod projection;
#[cfg(test)]
mod tests;

/// One find term in execution form: a projected slot span, a fold
/// aggregate, or an Arg-restriction carry. Widths come from the plan's
/// binding-slot layout (`ValidatedPlan::slots`) — never assumed 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindSpec {
    /// A projected (group-key) variable: first binding slot + width in
    /// words (2 for an interval variable, 1 for everything else).
    Var { slot: usize, width: usize },
    /// A fold aggregate over a slot span (`over_slot: None` for the
    /// nullary Count; `over_width` > 1 only for `CountDistinct` — the
    /// arithmetic folds are validated scalar).
    Agg {
        op: FoldOp,
        over_slot: Option<usize>,
        over_width: usize,
        /// Whether the input is I64 (its column word is the sign-flipped
        /// biased form; Sum must decode before accumulating).
        signed: bool,
    },
    /// An Arg-restriction carry (`ArgMax`/`ArgMin` — 20-query-ir
    /// § aggregation): the carried variable's slot span, plus the shared
    /// key. Validation guarantees every Arg term of a query names one key
    /// variable and one direction, so the per-find copies agree.
    Arg {
        slot: usize,
        width: usize,
        /// The key variable's slot (orderable — U64/I64 — so width 1).
        key_slot: usize,
        /// `true` for `ArgMax`, `false` for `ArgMin`.
        max: bool,
    },
}

/// A fold aggregate's operator, execution-side: exactly the ops that fold
/// into an [`Acc`] — the Arg ops are not folds (they restrict the binding
/// set; [`FindSpec::Arg`]) and are unrepresentable here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldOp {
    Sum,
    Min,
    Max,
    Count,
    CountDistinct,
}

/// Decodes a binding word back to the i64 it encodes (the biased word form
/// is order-preserving; arithmetic needs the logical value).
fn word_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

fn i64_to_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

/// The projection sink: dedups projected find tuples, and reports
/// staleness (`SkipSuffix`) so the executor can unwind suffixes that bind
/// nothing projection-relevant (D2 — legal for this sink only).
#[derive(Debug)]
pub struct ProjectionSink {
    /// The projected binding slots in find-**word** order: an interval
    /// find contributes its two consecutive slots (the `SlotWidth` layout,
    /// expanded by the constructor's caller from the plan's layout map).
    slots: Vec<usize>,
    seen: WordMap<()>,
    scratch: Vec<u64>,
    /// Per-slot leaf-batch sources, recomputed at batch entry —
    /// per-slot work, not per-row (the pointer-keyed
    /// skip-if-same-shape cache measured < 2%
    /// at family level and was deleted): `Some(word)` reads the batch
    /// keys, `None` the outer bindings.
    batch_sources: Vec<Option<usize>>,
    /// Row-major staging rows of one hoisted scan run — the
    /// column-outer gather's target, `run length × arity` words with
    /// retained capacity (the allocation contract's touched-data
    /// bound). Sized by the run, never by a width cap: the projection
    /// arity is unbounded by construction.
    scan_rows: Vec<u64>,
    /// Rows consumed by the open scan.
    scan_count: u64,
}

/// One accumulator cell.
#[derive(Debug, Clone, Copy)]
enum Acc {
    /// i128 accumulation: deterministic under any fold order — set folds
    /// have none; one range check at finalization (u128 for unsigned).
    SumSigned(i128),
    SumUnsigned(u128),
    /// Min/Max compare column words — correct because words are
    /// order-preserving (docs/architecture/40-execution.md).
    Min(u64),
    Max(u64),
    Count(u64),
    /// `CountDistinct`: index into the sink's `value_sets` pool — the
    /// group's distinct-value word-set (20-query-ir § aggregation);
    /// finalize is its `len()`.
    CountDistinct(usize),
}

/// The one Arg-restriction unit of a query (validation: all Arg terms
/// share one key variable and one direction).
#[derive(Debug, Clone, Copy)]
struct ArgSpec {
    key_slot: usize,
    max: bool,
}

/// The aggregate sink: group map keyed by the group-key words, folding each
/// distinct full binding exactly once. Never returns `SkipSuffix` — the
/// skip is illegal under aggregation (any new bound variable multiplies
/// the binding set the fold is defined over). The illegality is also
/// encoded structurally: aggregate plans mark every node sink-relevant
/// (run.rs's skip-absorption arm), so even a skip
/// signaled by mistake would be absorbed at its producing node.
#[derive(Debug)]
pub struct AggregateSink {
    finds: Vec<FindSpec>,
    /// Group-key slot spans (the `Var` specs, in find order): (first
    /// slot, width in words) — the `SlotWidth` layout, never assumed 1.
    group_spans: Vec<(usize, usize)>,
    /// Group key words -> accumulator row index. Key arity = the spans'
    /// total width.
    groups: WordMap<usize>,
    /// Flat accumulator rows: `accs[group * n_aggs ..][..n_aggs]`.
    accs: Vec<Acc>,
    n_aggs: usize,
    /// `CountDistinct` per-group value sets, pooled (arena-backed, reused
    /// across executions by allocation index — the allocation order is
    /// (group, `CountDistinct` find) and the arity sequence repeats, so a
    /// reused map always matches its span width). Exactly the
    /// projection-dedup mechanism scoped per group, keyed on the value's
    /// 1–2 word span with the seen-set's tuple hashing.
    value_sets: Vec<WordMap<()>>,
    /// Pool high-water: sets `< value_sets_live` belong to this
    /// execution's groups.
    value_sets_live: usize,
    /// The Arg-restriction unit, when the finds carry Arg terms
    /// (validation: never alongside folds).
    arg: Option<ArgSpec>,
    /// Per group: the extreme key word so far. Encoded words compare
    /// correctly unsigned for both orderable key types — U64 words are
    /// the value, I64 words are the sign-flipped biased form, and both
    /// encodings are order-preserving (docs/architecture/40-execution.md).
    arg_best: Vec<u64>,
    /// Per group: the restricted set's projected rows (all Arg carries
    /// concatenated, in find order) — a word-set, because ties are
    /// set-honest: two distinct bindings may project equal rows, and the
    /// answer is a set (20-query-ir § aggregation). Pooled by group
    /// index, capacity retained across executions.
    arg_rows: Vec<WordMap<()>>,
    /// Words per Arg row (the carries' total width).
    carry_words: usize,
    /// Arg row assembly scratch.
    carry_scratch: Vec<u64>,
    /// `CountDistinct` and Arg fold per row — their group state is a set,
    /// not a scalar accumulator, so no gather kernel or scan pushdown
    /// applies; batches route through the per-row scratch fold.
    row_fold_only: bool,
    /// Full-binding dedup, elided when the plan proves distinct bindings.
    /// Keyed on the whole slot array — an interval variable's two words
    /// are both hashed (the `SlotWidth` layout).
    seen: Option<WordMap<()>>,
    key_scratch: Vec<u64>,
    binding_scratch: Vec<u64>,
    /// Batch-fold accumulator staging: the group's row is copied here,
    /// folded, and written back once per batch.
    acc_scratch: Vec<Acc>,
    /// Dedup-pass survivors (the seen-set regime's batch fold): entries
    /// whose full binding was first-seen this batch, gather-folded after
    /// the dedup pass exactly like the elided path.
    dedup_survivors: Vec<u32>,
    /// The open scan's per-aggregate leaf-word sources:
    /// `Some(word)` folds a column, `None` finishes from the constant
    /// outer value at `end_scan`.
    scan_sources: Vec<Option<usize>>,
    /// Rows consumed by the open scan.
    scan_count: u64,
    /// The leaf-shape classification, recomputed at each batch entry
    /// (per-slot work, never per-row): outer slots for the per-row
    /// prefill, and whether the group key is batch-constant.
    cached_outer_slots: Vec<usize>,
    cached_constant_group: bool,
    /// Group-map probes actually issued (the group-probe hoist observable).
    #[cfg(test)]
    group_probes: usize,
}
