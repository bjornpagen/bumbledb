//! The two consumers of bindings (docs/architecture/40-execution.md): set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! (`docs/architecture/40-execution.md` D2/D3; semantics normative in
//! `20-query-ir.md`).
//!
//! **The sinks are where union lives** (docs/architecture/40-execution.md
//! § the rule loop): one sink hears every rule of a program, its seen-set
//! spanning rules — reset once per execution, never per rule — so a later
//! rule re-deriving a head fact is absorbed exactly like a within-rule
//! duplicate. No merge node, no concat-then-dedup pass exists anywhere
//! else. The seen-set keys are **head-shaped** by construction: the
//! projection sink keys the projected find tuple, and the multi-rule
//! aggregate sink keys the head projection (`union_spans`), never the
//! rule's full slot array — dedup keys must be rule-independent.
//! Rule-disjointness remains diagnostic knowledge, but the executor does
//! not spend it: a measured attempt to replace the spanning map with
//! per-rule drains was slower. See the refutation in
//! `docs/architecture/40-execution.md`.
//!
//! Aggregation never materializes the join: group maps live in sink state;
//! the fold domain of every aggregate is the group's **set of distinct
//! full bindings over all query variables** — two postings of amount 100
//! to one account are two distinct bindings (their fresh ids differ), so
//! `Sum(amount) by account` is 200. The stated footgun: joining a
//! multiplicity-adding relation multiplies the binding set, exactly as in
//! SQL.
//!
//! Slots are **words**, not variables: a multi-word variable occupies
//! consecutive binding slots — two for an interval, ⌈N/8⌉ for a
//! bytes<N> value (the [`crate::ir::normalize::SlotWidth`] layout) — so
//! every [`FindSpec`] carries its slot span and every consumer walks
//! widths: the seen-set keys the full slot array (every span word
//! hashed), the group key concatenates spans, and emitted rows are word
//! rows the result buffer re-assembles by find type.

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
    /// words (2 for an interval variable, ⌈N/8⌉ for bytes<N>, 1 for
    /// everything else).
    Var { slot: usize, width: usize },
    /// The measure at a find position: ONE projected u64 word computed
    /// from the interval variable's two-slot span at `slot` —
    /// `end − start`, the two-slot read + subtraction (docs/architecture/20-query-ir.md, § the measure). Exact
    /// for both element types: the encodings are unit-spaced
    /// order-preserving maps onto u64 words (u64 the identity, I64 the
    /// +2⁶³ bias, which cancels in the difference), and the constructor
    /// invariant `end > start` keeps it positive. `end == MAX` is the
    /// ray — no finite measure: the sink poisons and the execution
    /// raises the typed [`crate::Error::MeasureOfRay`].
    Duration { slot: usize },
    /// A fold over the measure (`Sum`/`Min`/`Max` of `Duration`): the
    /// interval variable's two-slot span at `slot`, folded as an
    /// unsigned u64 input — Sum in the wide accumulator with the single
    /// finalize range check, like every Sum. Ray semantics as
    /// [`FindSpec::Duration`].
    AggDuration { op: FoldOp, slot: usize },
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
    /// The coalescing fold (`Pack` — 20-query-ir § aggregation): the
    /// interval variable's two-slot span. Relation-shaped group state —
    /// per group the sink accumulates the claim list; finalize sorts by
    /// start word and drives the shared segment sweep
    /// (`crate::interval::sweep`), one head answer per maximal segment.
    /// Validation admits at most one per head and no fold or Arg
    /// companions.
    Pack { slot: usize },
}

/// What a sink executes after construction parsed [`FindSpec`]. Measures
/// have already become derived scratch words, so the symbolic
/// `Duration`/`AggDuration` shapes cannot reach any execution consumer.
/// Minted only by `aggregate::parse_finds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkSpec {
    /// A projected/group-key slot span.
    Var { slot: usize, width: usize },
    /// A fold over a slot span (`over_slot: None` for nullary Count).
    Agg {
        op: FoldOp,
        over_slot: Option<usize>,
        over_width: usize,
        signed: bool,
    },
    /// An Arg-restriction carry and its shared extreme key.
    Arg {
        slot: usize,
        width: usize,
        key_slot: usize,
        max: bool,
    },
    /// A coalescing interval claim.
    Pack { slot: usize },
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

/// The measure over encoded interval words: `Some(end − start)`, or
/// `None` for the ray (`end == MAX` is ∞ in both element encodings — no
/// finite measure; the caller poisons and the execution raises the typed
/// [`crate::Error::MeasureOfRay`]). One subtraction, exact for both
/// element types (see [`FindSpec::Duration`]).
fn measure(start: u64, end: u64) -> Option<u64> {
    (end != u64::MAX).then(|| end - start)
}

fn i64_to_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

/// One projected word's source: a binding slot read verbatim, or the
/// measure of an interval variable's two-slot span (`end − start`, one
/// computed word — [`FindSpec::Duration`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjSource {
    Slot(usize),
    Measure { start: usize },
}

/// Projection execution is either all direct slots (the fast paths) or
/// includes a computed measure (the ray-checking paths). The sum removes
/// the former `has_measures` flag + per-source assertion agreement.
#[derive(Debug)]
enum ProjectionSources {
    Plain(Vec<usize>),
    Measured(Vec<ProjSource>),
}

/// Expands find specs into projected word sources, find-**word** order:
/// an interval find contributes its two consecutive slots (the
/// `SlotWidth` layout), a measure find one computed word.
fn sources_of(finds: &[SinkSpec], measures: &[(usize, usize)]) -> ProjectionSources {
    let mut sources = Vec::new();
    extend_sources(finds, measures, &mut sources);
    if measures.is_empty() {
        ProjectionSources::Plain(
            sources
                .into_iter()
                .filter_map(|source| match source {
                    ProjSource::Slot(slot) => Some(slot),
                    ProjSource::Measure { .. } => None,
                })
                .collect(),
        )
    } else {
        ProjectionSources::Measured(sources)
    }
}

/// [`sources_of`]'s in-place body — the rule loop's re-aim path rebuilds
/// into retained capacity (the warm allocation contract).
fn extend_sources(finds: &[SinkSpec], measures: &[(usize, usize)], out: &mut Vec<ProjSource>) {
    out.clear();
    for spec in finds {
        match spec {
            SinkSpec::Var { slot, width } => {
                if let Some((_, start)) = measures.iter().find(|(derived, _)| derived == slot) {
                    out.push(ProjSource::Measure { start: *start });
                } else {
                    out.extend((*slot..slot + width).map(ProjSource::Slot));
                }
            }
            SinkSpec::Agg { .. } | SinkSpec::Arg { .. } | SinkSpec::Pack { .. } => {}
        }
    }
}

/// One projected word's batch-resolved source on the measured emit paths
/// (rebuilt at batch/scan entry, per-word work): prefilled in the
/// scratch row (outer slot, or an outer measure computed once), a batch
/// key / leaf column word, or a measure over two key/column words.
#[derive(Debug, Clone, Copy)]
enum MeasuredSource {
    Const,
    Key(usize),
    MeasureKeys(usize, usize),
}

/// The projection sink: dedups projected find tuples, and reports
/// staleness (`SkipSuffix`) so the executor can unwind suffixes that bind
/// nothing projection-relevant (D2 — legal for this sink only).
#[derive(Debug)]
pub struct ProjectionSink {
    /// Parsed, measure-free head specs. Kept for allocation-silent
    /// re-aiming across rules; emit paths consume `sources` below.
    finds: Vec<SinkSpec>,
    /// Derived measure word → original interval start slot, minted with
    /// `finds` by the constructor parse.
    measures: Vec<(usize, usize)>,
    /// The projected word sources in find-**word** order: an interval
    /// find contributes its two consecutive slots (the `SlotWidth` layout,
    /// expanded by the constructor's caller from the plan's layout map);
    /// a measure find contributes ONE computed word
    /// ([`ProjSource::Measure`]).
    sources: ProjectionSources,
    /// The measure poison: the first ray the projection reached
    /// (`end == MAX` has no finite measure) — surfaced after the run as
    /// the typed [`crate::Error::MeasureOfRay`].
    ray: Option<[u64; 2]>,
    /// The measured paths' batch-resolved sources, aligned with
    /// `sources` (rebuilt at batch/scan entry; empty on the fast paths).
    measured_sources: Vec<MeasuredSource>,
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
    /// Evidence retained exactly when the binding seen-set is absent.
    /// Construction cannot enter that regime without a plan proof.
    distinct_witness: Option<crate::plan::fj::DistinctWitness>,
    /// The measure-free sink specs in **derived-slot form**: construction
    /// parses every measure onto a derived binding-scratch word —
    /// `Duration { slot }` becomes `Var { slot: derived, width: 1 }` and
    /// `AggDuration { op, slot }` becomes an unsigned
    /// `Agg { over_slot: derived }` — so group keys, dedup keys, folds,
    /// and finalize consume plain words with zero measure awareness. The
    /// representation move: the measure gets a word in the sink's row,
    /// not a branch in its folds.
    finds: Vec<SinkSpec>,
    /// The measure table minted by that parse: (derived scratch word,
    /// interval variable's first slot) — computed once per row landing
    /// in `binding_scratch` (`fold_scratch_row`), ray-checked
    /// (`end == MAX` poisons [`Self::ray`]). Non-empty forces the
    /// per-row fold arm (`row_fold_only`): derived words exist only in
    /// the scratch row, so no gather kernel or scan pushdown can read
    /// them.
    measures: Vec<(usize, usize)>,
    /// The rule's real binding-slot count — `binding_scratch` extends
    /// past it by one derived word per measure.
    real_slots: usize,
    /// The measure poison (see [`ProjectionSink::ray`]).
    ray: Option<[u64; 2]>,
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
    /// 1–8 word span with the seen-set's tuple hashing.
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
    /// Per group: the restricted set's projected answers (all Arg carries
    /// concatenated, in find order) — a word-set, because ties are
    /// set-honest: two distinct bindings may project equal rows, and the
    /// answer is a set (20-query-ir § aggregation). Pooled by group
    /// index, capacity retained across executions.
    arg_answers: Vec<WordMap<()>>,
    /// Words per Arg row (the carries' total width).
    carry_words: usize,
    /// Arg row assembly scratch.
    carry_scratch: Vec<u64>,
    /// The Pack term's interval slot span start, when the head carries
    /// one (validation: at most one, never beside folds or Arg terms).
    /// Re-aimed per rule like every slot table.
    pack: Option<usize>,
    /// Per group: `Pack`'s claim accumulation list — `[start, end]`
    /// encoded word pairs, appended raw at fold time (identical and
    /// overlapping claims collapse in the finalize sweep, never here)
    /// and pooled by group index exactly like the Arg row sets (capacity
    /// retained across executions, cleared at group creation). Memory is
    /// O(the group's claims) — the allocation contract's retained
    /// high-water scratch.
    pack_claims: Vec<Vec<[u64; 2]>>,
    /// `CountDistinct` and Arg fold per row — their group state is a set,
    /// not a scalar accumulator, so no gather kernel or scan pushdown
    /// applies; batches route through the per-row scratch fold.
    row_fold_only: bool,
    /// Binding dedup, elided (`None`) when the emitted key stream is
    /// proven duplicate-free: single-rule, the plan's distinct-bindings
    /// proof. A multi-rule sink always retains it. Single-rule key: the
    /// whole slot array — an interval variable's two words are both
    /// hashed (the `SlotWidth` layout). Multi-rule key: the head projection
    /// ([`Self::union_spans`]).
    seen: Option<WordMap<()>>,
    /// The multi-rule union regime's dedup-key spans (`None` =
    /// single-rule, key = the whole slot array): per head position in
    /// head order, the slot span the position reads from THIS rule's
    /// binding layout — group variables and fold inputs; the nullary
    /// `Count` contributes nothing. The extracted words are the
    /// **head projection** of the binding — rule-independent by
    /// construction, so one seen-set spanning rules is the fold domain's
    /// union (20-query-ir § aggregation, "aggregates read the head").
    /// Re-aimed per rule by [`Self::aim`].
    union_spans: Option<Vec<(usize, usize)>>,
    /// Head-projection key assembly scratch (union regime only).
    union_scratch: Vec<u64>,
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
