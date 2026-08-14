//! The two consumers of bindings (docs/architecture/40-execution.md): set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! (`docs/architecture/40-execution.md` D2/D3; semantics normative in
//! `20-query-ir.md`).
//!
//! **The sinks are where union lives** (docs/architecture/40-execution.md
//! § the rule loop): one sink hears every rule of a query, its seen-set
//! spanning rules — reset once per execution, never per rule — so a later
//! rule re-deriving a head fact is absorbed exactly like a within-rule
//! duplicate. No merge node, no concat-then-dedup pass exists anywhere
//! else. The seen-set keys are **rule-independent by provenance**
//! (ruled 2026-07-23, R2): the projection sink keys the projected find
//! tuple; the multi-rule aggregate sink keys the head projection
//! (`union_spans`) for a hand-written rule set — variables are
//! rule-scoped there, so the head is the only shared vocabulary — and
//! the **shared slot arrays** for a DNF-derived rule set (the disjuncts
//! of one written rule share one variable scope, so disjunction widens
//! membership without moving the fold domain — the or-transparency law,
//! `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
//! Rule-disjointness remains diagnostic knowledge, but the executor does
//! not spend it: a measured attempt to replace the spanning map with
//! per-rule drains was slower. See the refutation in
//! `docs/architecture/40-execution.md`.
//!
//! Aggregation never materializes the join: group maps live in sink state;
//! the fold domain of every aggregate is the group's **set of distinct
//! full bindings over all query variables** — two postings of amount 100
//! to one account are two distinct bindings (their fresh ids differ), so
//! `Sum(amount) by account` is 200, under ANY spelling of the rule's
//! conditions (`or` included — the DNF re-key above). Only a
//! hand-written multi-rule query coarsens the domain to the head
//! projection. The stated footgun: joining a
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

/// A fold aggregate's operator, execution-side: exactly the ops that fold
/// over a slot into an [`Acc`]. Nullary [`AggSpec::Count`] is a sibling
/// arm, not a `FoldOp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldOp {
    Sum,
    Min,
    Max,
}

/// Nullary Count vs a fold over a slot. Trusted layer: Count cannot
/// carry a slot and folds cannot omit one (C1/C6). Hostile
/// `FindTerm::Aggregate { over: Option }` stays on `ir.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggSpec {
    Count,
    Fold {
        /// `Sum` / `Min` / `Max` — never `Count`.
        op: FoldOp,
        slot: usize,
        width: usize,
        /// Whether the input is I64 (its column word is the sign-flipped
        /// biased form; Sum must decode before accumulating).
        signed: bool,
    },
}

impl AggSpec {
    pub(in crate::exec::sink) fn seed_acc(self) -> Acc {
        match self {
            Self::Count => Acc::Count(0),
            Self::Fold {
                op: FoldOp::Sum,
                signed: true,
                ..
            } => Acc::SumSigned(0),
            Self::Fold {
                op: FoldOp::Sum,
                signed: false,
                ..
            } => Acc::SumUnsigned(0),
            Self::Fold {
                op: FoldOp::Min, ..
            } => Acc::Min(u64::MAX),
            Self::Fold {
                op: FoldOp::Max, ..
            } => Acc::Max(u64::MIN),
        }
    }
}

/// One find term in execution form: a projected slot span or a fold
/// aggregate. Widths come from the plan's
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
    /// A fold aggregate: nullary Count, or Sum/Min/Max over a slot.
    Agg(AggSpec),
    /// The coalescing fold (`Pack` — 20-query-ir § aggregation): the
    /// interval variable's two-slot span. Relation-shaped group state —
    /// per group the sink accumulates the claim list; finalize sorts by
    /// start word and drives the shared segment sweep
    /// (`crate::interval::sweep`), one head answer per maximal segment.
    /// Validation admits at most one per head and no fold
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
    /// A fold: Count contributes no slot; a Fold reads one.
    Agg(AggSpec),
    /// A coalescing interval claim.
    Pack { slot: usize },
}

/// Live dedup regime (R2). Construction mints the arm; `seen` lives only
/// on Bindings/Union/DnfUnion, the witness only on Elided, and DNF vs
/// head-projection is which union arm — not a sidecar bool.
#[derive(Debug)]
pub(in crate::exec::sink) enum DedupState {
    Bindings {
        seen: WordMap<()>,
    },
    /// Head projection — hand-written multi-rule.
    Union {
        seen: WordMap<()>,
        spans: Vec<(usize, usize)>,
    },
    /// VarId-ordered slots — DNF-derived multi-rule.
    DnfUnion {
        seen: WordMap<()>,
        spans: Vec<(usize, usize)>,
    },
    Elided {
        /// Plan proof that minted this arm. Retained as evidence; the
        /// arm itself is the elision observable.
        #[allow(dead_code)]
        witness: crate::plan::fj::DistinctWitness,
    },
}

impl DedupState {
    /// `true` when this binding should fold (first-seen, or elided).
    pub(in crate::exec::sink) fn consider(
        &mut self,
        binding_scratch: &[u64],
        union_scratch: &mut Vec<u64>,
    ) -> bool {
        match self {
            Self::Elided { .. } => true,
            Self::Bindings { seen } => seen.insert(binding_scratch),
            Self::Union { seen, spans } | Self::DnfUnion { seen, spans } => {
                union_scratch.clear();
                for &(slot, width) in spans.iter() {
                    union_scratch.extend_from_slice(&binding_scratch[slot..slot + width]);
                }
                seen.insert(union_scratch)
            }
        }
    }

    pub(in crate::exec::sink) fn seen(&self) -> Option<&WordMap<()>> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }

    pub(in crate::exec::sink) fn seen_mut(&mut self) -> Option<&mut WordMap<()>> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }
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
            SinkSpec::Agg(_) | SinkSpec::Pack { .. } => {}
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
    /// at family level and was deleted): `Key` reads the batch keys,
    /// `Outer` the outer bindings.
    batch_sources: Vec<crate::exec::run::LeafSource>,
    /// Row-major staging rows of one hoisted scan run — the
    /// column-outer gather's target, `run length × arity` words with
    /// retained capacity (the allocation contract's touched-data
    /// bound). Sized by the run, never by a width cap: the projection
    /// arity is unbounded by construction.
    scan_rows: Vec<u64>,
    /// Rows consumed by the open scan.
    scan_count: u64,
}

/// The group map's representation, structural at construction (finding
/// 049): GROUP BY over enum-like closed dimensions — the dominant OLAP
/// grouping shape — takes pure arithmetic instead of a hash.
#[derive(Debug)]
enum GroupTable {
    /// Open-domain group keys: the SWAR map.
    Hashed(WordMap<usize>),
    /// Every group-key word ranges over a schema-proven dense domain —
    /// a closed extension's row ids are its declaration indices `0..N`
    /// (containment keeps every committed referencing word in-domain)
    /// and bool's strict 0/1 encoding is `0..2` — so the group index is
    /// mixed-radix arithmetic into a flat table: no hash, no ctrl-line
    /// probe, no insert branch on the hit path.
    Dense {
        /// Per key word, in key order: the domain size.
        radixes: Box<[u16]>,
        /// ordinal → group index + 1 (`0` = untouched). `Π radix` words,
        /// capped at construction ([`DENSE_GROUPS_CAP`]).
        table: Box<[u32]>,
        /// group index → ordinal, in mint order — finalize reconstructs
        /// the key words from it (the map regime's insertion order).
        ordinals: Vec<u32>,
    },
}

/// The dense table's size ceiling — the group-map capacity hint's
/// existing clamp: past it the open-domain map wins on memory.
pub(crate) const DENSE_GROUPS_CAP: u32 = 4096;

impl GroupTable {
    fn len(&self) -> usize {
        match self {
            Self::Hashed(map) => map.len(),
            Self::Dense { ordinals, .. } => ordinals.len(),
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Hashed(map) => map.clear(),
            Self::Dense {
                table, ordinals, ..
            } => {
                table.fill(0);
                ordinals.clear();
            }
        }
    }
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
}

/// Where a scan-fold input reads. Count contributes no slot.
#[derive(Debug, Clone, Copy)]
enum FoldSource {
    Outer,
    Column(usize),
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
    /// Live R2 regime: seen-set on Bindings/Union/DnfUnion, witness on
    /// Elided. Construction mints the arm and keeps it.
    dedup: DedupState,
    /// The measure-free sink specs in **derived-slot form**: construction
    /// parses every measure onto a derived binding-scratch word —
    /// `Duration { slot }` becomes `Var { slot: derived, width: 1 }` and
    /// `AggDuration { op, slot }` becomes an unsigned
    /// `Agg(Fold { slot: derived })` — so group keys, dedup keys, folds,
    /// and finalize consume plain words with zero measure awareness. The
    /// representation move: the measure gets a word in the sink's row,
    /// not a branch in its folds.
    finds: Vec<SinkSpec>,
    /// The measure table minted by that parse: (derived scratch word,
    /// interval variable's first slot) — computed once per row landing
    /// in `binding_scratch` (`fold_scratch_row`), ray-checked
    /// (`end == MAX` poisons [`Self::ray`]). Non-empty forces the
    /// per-row fold arm: derived words exist only in
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
    /// total width. Representation is bind-time data ([`GroupTable`]):
    /// dense arithmetic when the schema proves every key word a small
    /// domain, the SWAR map otherwise — never a hot-loop branch beyond
    /// the enum's own dispatch (finding 049).
    groups: GroupTable,
    /// Flat accumulator rows: `accs[group * n_aggs ..][..n_aggs]`.
    accs: Vec<Acc>,
    n_aggs: usize,
    /// The Pack term's interval slot span start, when the head carries
    /// one (validation: at most one, never beside folds).
    /// Re-aimed per rule like every slot table.
    pack: Option<usize>,
    /// Per group: `Pack`'s claim accumulation list — `[start, end]`
    /// encoded word pairs, appended raw at fold time (identical and
    /// overlapping claims collapse in the finalize sweep, never here)
    /// and pooled by group index (capacity
    /// retained across executions, cleared at group creation). Memory is
    /// O(the group's claims) — the allocation contract's retained
    /// high-water scratch.
    /// Measures and Pack fold per row — derived words exist only in
    /// the scratch row, and Pack's group state is a claim list, so no
    /// gather kernel or scan pushdown applies; batches route through
    /// the per-row scratch fold. Tested as `pack.is_some() ||
    /// !measures.is_empty()`, not a stored flag.
    pack_claims: Vec<Vec<[u64; 2]>>,
    /// Head-projection / DNF-span key assembly scratch (union regimes).
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
    /// The open scan's per-fold leaf-word sources (Count contributes
    /// no slot — it rides [`AggSpec::Count`]). `Column` folds a leaf
    /// word; `Outer` finishes from the constant outer value at `end_scan`.
    scan_sources: Vec<FoldSource>,
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
