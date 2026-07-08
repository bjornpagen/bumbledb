//! The two consumers of bindings (docs/architecture/30-execution.md): set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! (`docs/architecture/30-execution.md` D2/D3; semantics normative in
//! `20-query-ir.md`).
//!
//! Aggregation never materializes the join: group maps live in sink state;
//! the fold domain of every aggregate is the group's **set of distinct
//! full bindings over all query variables** — two postings of amount 100
//! to one account are two distinct bindings (their serial ids differ), so
//! `Sum(amount) by account` is 200. The stated footgun: joining a
//! multiplicity-adding relation multiplies the binding set, exactly as in
//! SQL.

use crate::encoding::encode_i64;
use crate::exec::wordmap::WordMap;
use crate::ir::AggOp;

mod aggregate;
mod projection;
#[cfg(test)]
mod tests;

/// One find term in execution form: a projected slot or an aggregate spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindSpec {
    /// A projected (group-key) variable's binding slot.
    Var { slot: usize },
    /// An aggregate over a slot (`None` for the nullary Count).
    Agg {
        op: AggOp,
        over_slot: Option<usize>,
        /// Whether the input is I64 (its column word is the sign-flipped
        /// biased form; Sum must decode before accumulating).
        signed: bool,
    },
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
    slots: Vec<usize>,
    seen: WordMap<()>,
    scratch: Vec<u64>,
    /// Per-slot leaf-batch sources, recomputed at batch entry —
    /// per-slot work, not per-row (docs/silicon2/10: the pointer-keyed
    /// skip-if-same-shape cache from docs/perf/ PRD 05 measured < 2%
    /// at family level and was deleted): `Some(word)` reads the batch
    /// keys, `None` the outer bindings.
    batch_sources: Vec<Option<usize>>,
    /// Rows consumed by the open scan (docs/perf/ PRD 05).
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
    /// order-preserving (docs/architecture/30-execution.md).
    Min(u64),
    Max(u64),
    Count(u64),
}

/// The aggregate sink: group map keyed by the group-key words, folding each
/// distinct full binding exactly once. Never returns `SkipSuffix` — the
/// skip is illegal under aggregation (any new bound variable multiplies
/// the binding set the fold is defined over). The illegality is also
/// encoded structurally: aggregate plans mark every node sink-relevant
/// (hardening PRD 05; run.rs's skip-absorption arm), so even a skip
/// signaled by mistake would be absorbed at its producing node.
#[derive(Debug)]
pub struct AggregateSink {
    finds: Vec<FindSpec>,
    /// Group-key slots (the `Var` specs, in find order).
    group_slots: Vec<usize>,
    /// Group key words -> accumulator row index.
    groups: WordMap<usize>,
    /// Flat accumulator rows: `accs[group * n_aggs ..][..n_aggs]`.
    accs: Vec<Acc>,
    n_aggs: usize,
    /// Full-binding dedup, elided when the plan proves distinct bindings.
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
    /// The open scan's per-aggregate leaf-word sources (PRD 05):
    /// `Some(word)` folds a column, `None` finishes from the constant
    /// outer value at `end_scan`.
    scan_sources: Vec<Option<usize>>,
    /// Rows consumed by the open scan.
    scan_count: u64,
    /// The leaf-shape cache (pointer-keyed on `key_slots`): outer slots
    /// for the per-row prefill, and whether the group key is batch-
    /// constant. Pinned leaves emit batches of one — recomputing this
    /// per batch was per-row work.
    cached_outer_slots: Vec<usize>,
    cached_constant_group: bool,
    /// Group-map probes actually issued (the PRD 02 hoist observable).
    #[cfg(test)]
    group_probes: usize,
}
