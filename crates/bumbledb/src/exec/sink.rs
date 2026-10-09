//! Binding consumers: deduplicated set projection with subtree skipping,
//! and aggregate folds with full-binding deduplication. Shared union spans
//! identify bindings consistently across DNF rule layouts.
use crate::encoding::encode_i64;
use crate::exec::wordmap::WordMap;
use std::num::NonZeroU32;

mod aggregate;
mod projection;
#[cfg(test)]
mod tests;

/// A fold aggregate's operator, execution-side: exactly the ops that fold
/// over a slot into an [`Acc`]. Nullary [`AggSpec::Count`] is a sibling
/// arm, not a `FoldOp`.
pub(crate) use crate::ir::FoldOp;

/// Nullary Count vs a fold over a slot. Trusted layer: Count cannot
/// carry a slot and folds cannot omit one. Hostile Count-with-variable
/// is unrepresentable on [`crate::ir::FindTerm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggSpec {
    Count,
    /// F64 argument: Sum/Mean exact, Min/Max over order keys with NaN propagating.
    Float {
        op: FoldOp,
        slot: usize,
    },
    Fold {
        op: FoldOp,
        slot: usize,
        width: usize,
        /// Signed (I64) argument; Sum decodes the biased word before accumulating.
        signed: bool,
    },
}

impl AggSpec {
    pub(in crate::exec::sink) fn seed_acc(self) -> Acc {
        match self {
            Self::Float { .. } => unreachable!("float groups seed their separate accumulator bank"),
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
            Self::Fold {
                op: FoldOp::Mean, ..
            } => unreachable!("Mean has F64 input"),
        }
    }
}

/// One find term in execution form: a projected slot span or a fold
/// aggregate. Widths come from the plan's
/// binding-slot layout (`ValidatedPlan::slots`) — never assumed 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FindSpec {
    Var { slot: usize, width: usize },
    Compute(std::sync::Arc<crate::api::prepared::computed::OutputProgram>),
    Agg(AggSpec),
    Pack { slot: usize },
}

/// What a sink executes after construction parsed [`FindSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkSpec {
    Var { slot: usize, width: usize },
    Agg(AggSpec),
    Pack { slot: usize },
}

/// Crate-internal sink reply L05 consumes after emit or finalize.
/// Work/scratch refusals are Stop; cardinality and corruption are Error.
/// Finalize is Q-ATOMIC: no group publishes after a recorded failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SinkProgress {
    /// More input is welcome; the row was recorded or was an exact duplicate.
    Continue,
    /// Finalize emitted every group/answer; the execution is complete.
    Finish,
    /// Cancellation stopped the sink. Sticky: later
    /// emits drop and finalize refuses (Q-ATOMIC).
    Stop,
    /// Scratch, cardinality, or corruption failure. Sticky: later emits
    /// drop and finalize refuses before any answer publishes.
    Error,
}

pub(in crate::exec::sink) fn classify_progress(error: &crate::error::Error) -> SinkProgress {
    if error.is_cancelled() {
        SinkProgress::Stop
    } else {
        SinkProgress::Error
    }
}

/// A distinct-tuple set over the RAM word table, insertion-ordered when
/// `ordered`. Errors are sticky: the executor's sink interface is
/// infallible, so a failure records itself, later inserts drop, and
/// finalize surfaces the error before any answer publishes. A full index
/// refuses with `Capacity::DistinctRows`.
#[derive(Debug)]
pub(in crate::exec::sink) struct SeenSet {
    ordered: bool,
    ram: WordMap<()>,
    /// Some only under an exact singleton-head uniqueness proof: a dense
    /// insertion-order log with the same cancellation quantum and sticky
    /// failure handling.
    unique_rows: Option<UniqueRows>,
    work: Option<crate::work::WorkContext>,
    error: Option<crate::error::Error>,
    /// Inserts since the last cancellation poll; the context is polled
    /// every [`STEP_QUANTUM`] rows.
    pending_steps: u32,
}

/// The maximum unpolled work quantum of a sink: cancellation is checked at
/// bounded intervals.
pub(crate) const STEP_QUANTUM: u32 = 256;

#[derive(Debug, Default)]
struct UniqueRows {
    words: Vec<u64>,
    /// Avoid dividing the flat word count by runtime arity on every emit.
    len: usize,
}

/// Resident rows have exactly one representation. Keeping the iterator's
/// alternatives exclusive avoids a flatten/chain state machine per result.
/// Bulk consumers select the alternative once before entering their loops.
#[derive(Clone)]
pub(crate) enum ResidentRows<Dense, Hashed> {
    Dense(Dense),
    Hashed(Hashed),
}

impl<T, Dense: Iterator<Item = T>, Hashed: Iterator<Item = T>> Iterator
    for ResidentRows<Dense, Hashed>
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Dense(rows) => rows.next(),
            Self::Hashed(rows) => rows.next(),
        }
    }
}

impl UniqueRows {
    fn clear(&mut self) {
        self.words.clear();
        self.len = 0;
    }
}

impl SeenSet {
    pub(in crate::exec::sink) fn with_capacity_hint(
        arity: usize,
        hint: usize,
        ordered: bool,
    ) -> Self {
        Self {
            ordered,
            ram: WordMap::with_capacity_hint(arity, hint),
            unique_rows: None,
            work: None,
            error: None,
            pending_steps: 0,
        }
    }

    pub(in crate::exec::sink) fn begin(&mut self, work: Option<crate::work::WorkContext>) {
        self.work = work;
    }

    fn elide_output_hashing(&mut self, _: crate::plan::fj::ProjectionDistinctWitness) {
        assert_eq!(self.len(), 0, "proof is installed only before execution");
        assert!(
            self.ordered,
            "only insertion-ordered projections elide hashing"
        );
        // Zero-width output words retain the original unit-set behavior.
        if self.ram.arity() != 0 {
            self.ram = WordMap::new(self.ram.arity());
            self.unique_rows = Some(UniqueRows::default());
        }
    }

    pub(in crate::exec::sink) fn len(&self) -> usize {
        self.unique_rows
            .as_ref()
            .map_or_else(|| self.ram.len(), |rows| rows.len)
    }

    pub(in crate::exec::sink) fn clear(&mut self) {
        self.ram.clear();
        if let Some(rows) = &mut self.unique_rows {
            rows.clear();
        }
        self.error = None;
        self.pending_steps = 0;
    }

    fn release_memory(&mut self) {
        self.ram = WordMap::new(self.ram.arity());
        if let Some(rows) = &mut self.unique_rows {
            *rows = UniqueRows::default();
        }
        self.work = None;
        self.error = None;
        self.pending_steps = 0;
    }

    pub(in crate::exec::sink) fn take_error(&mut self) -> Option<crate::error::Error> {
        self.error.take()
    }

    pub(in crate::exec::sink) fn progress(&self) -> SinkProgress {
        match &self.error {
            None => SinkProgress::Continue,
            Some(error) => classify_progress(error),
        }
    }

    /// Exact insert-if-absent. A recorded failure drops every later row
    /// (the execution is spoiled; finalize refuses).
    #[inline]
    pub(in crate::exec::sink) fn insert(&mut self, key: &[u64]) -> bool {
        if self.unique_rows.is_some() {
            self.insert_inner::<true>(key)
        } else {
            self.insert_inner::<false>(key)
        }
    }

    // Keep the two resident kernels separate: ordinary hashing must not
    // carry dense-row allocation in its frame. Known-mode batches bypass
    // the dynamic dispatcher.
    #[inline(never)]
    fn insert_inner<const UNIQUE: bool>(&mut self, key: &[u64]) -> bool {
        debug_assert_eq!(self.unique_rows.is_some(), UNIQUE);
        if self.error.is_some() {
            return false;
        }
        if self.work.is_some() {
            self.pending_steps += 1;
            if self.pending_steps >= STEP_QUANTUM && !self.poll_steps() {
                return false;
            }
        }
        if !UNIQUE && self.ram.remaining_rows() == 0 {
            if !self.ram.contains_key(key) {
                self.refuse_full();
            }
            return false;
        }
        if UNIQUE {
            let rows = self.unique_rows.as_mut().expect("proved projection");
            debug_assert_eq!(key.len(), self.ram.arity());
            if rows.words.try_reserve(key.len()).is_err() {
                self.error = Some(crate::error::Error::from(
                    crate::work::WorkError::Allocation,
                ));
                return false;
            }
            rows.words.extend_from_slice(key);
            rows.len += 1;
            true
        } else {
            self.ram.insert(key)
        }
    }

    #[cold]
    #[inline(never)]
    fn refuse_full(&mut self) {
        self.error = Some(crate::error::Error::Capacity(
            crate::error::Capacity::DistinctRows,
        ));
    }

    /// Insert a nonempty-width gathered projection in order. Amortize
    /// bookkeeping only inside a prefix that cannot reach the index limit
    /// or a poll boundary, even if every row adds a new key. Boundary rows
    /// take the ordinary exact duplicate lookup and polling path.
    fn insert_hashed_rows(&mut self, mut words: &[u64]) {
        let arity = self.ram.arity();
        debug_assert!(self.unique_rows.is_none() && arity != 0);
        debug_assert_eq!(words.len() % arity, 0);
        while !words.is_empty() && self.error.is_none() {
            let mut count = (words.len() / arity).min(self.ram.remaining_rows());
            if self.work.is_some() {
                let before_poll = usize::try_from(STEP_QUANTUM - 1 - self.pending_steps)
                    .expect("less than a work quantum");
                count = count.min(before_poll);
            }
            if count == 0 {
                self.insert_inner::<false>(&words[..arity]);
                words = &words[arity..];
            } else {
                let width = count * arity;
                // Dispatch width once; duplicate lookup still precedes growth.
                self.ram.insert_rows(&words[..width]);
                if self.work.is_some() {
                    self.pending_steps += u32::try_from(count).expect("less than a work quantum");
                }
                words = &words[width..];
            }
        }
    }

    /// Generate proved-unique rows directly in their retained allocation.
    /// Only prefixes before the next allocation or poll boundary are
    /// bulk-filled. Boundary rows use the ordinary insertion path.
    ///
    /// # Safety
    /// `write(offset, words)` must initialize EVERY word before returning,
    /// with consecutive rows beginning at `offset`. It must only initialize
    /// slots, never deinitialize an existing value, including on unwind.
    /// The projection route proves output-slot coverage; the distinctness
    /// witness installed on this set independently licenses eliding
    /// duplicate lookup.
    #[expect(unsafe_code, reason = "Publish only fully initialized generated rows")]
    unsafe fn insert_unique_with(
        &mut self,
        len: usize,
        scratch: &mut [u64],
        mut write: impl FnMut(usize, &mut [std::mem::MaybeUninit<u64>]),
    ) {
        let arity = self.ram.arity();
        debug_assert!(self.unique_rows.is_some() && arity != 0);
        assert_eq!(scratch.len(), arity);
        let mut offset = 0;
        while offset < len && self.error.is_none() {
            let rows = self.unique_rows.as_mut().expect("proved projection");
            let spare = (rows.words.capacity() - rows.words.len()) / arity;
            let mut count = (len - offset).min(spare);
            if self.work.is_some() {
                // The polling row itself must use insert: append its
                // preceding prefix BEFORE checking the work ledger.
                let before_poll = usize::try_from(STEP_QUANTUM - 1 - self.pending_steps)
                    .expect("less than a work quantum");
                count = count.min(before_poll);
            }
            if count == 0 {
                // SAFETY: u64 and MaybeUninit<u64> have identical layout.
                // `write` restores every initialized word before scratch
                // is read again; it cannot invalidate a drop-bearing value.
                let uninit =
                    unsafe { std::slice::from_raw_parts_mut(scratch.as_mut_ptr().cast(), arity) };
                write(offset, uninit);
                self.insert_inner::<true>(scratch);
                offset += 1;
            } else {
                let width = count * arity;
                let base = rows.words.len();
                write(offset, &mut rows.words.spare_capacity_mut()[..width]);
                // SAFETY: `spare` bounds the reserved extent; the writer
                // initialized every slot. On panic the old length remains
                // valid and no partially generated row is published.
                unsafe { rows.words.set_len(base + width) };
                rows.len += count;
                if self.work.is_some() {
                    self.pending_steps += u32::try_from(count).expect("less than a work quantum");
                }
                offset += count;
            }
        }
    }

    // Keep the ledger/error representation out of every resident tuple's
    // stack frame.
    #[cold]
    #[inline(never)]
    fn poll_steps(&mut self) -> bool {
        self.pending_steps = 0;
        let work = self.work.as_ref().expect("only cancellable inserts poll");
        match work.checkpoint() {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(crate::error::Error::from(error));
                false
            }
        }
    }

    /// Insertion-order iteration from `since`.
    pub(in crate::exec::sink) fn ram_iter_since(
        &self,
        since: usize,
    ) -> ResidentRows<impl Iterator<Item = &[u64]> + Clone, impl Iterator<Item = &[u64]> + Clone>
    {
        match &self.unique_rows {
            Some(rows) => {
                let arity = self.ram.arity();
                let start = since.min(rows.len) * arity;
                ResidentRows::Dense(rows.words[start..].chunks_exact(arity))
            }
            None => ResidentRows::Hashed(self.ram.iter_since(since).map(|(key, ())| key)),
        }
    }

    /// Insertion-ordered drain from `since`. Ordered sets only. `Ok(false)`
    /// stops the walk; `Err` is immediate.
    /// # Errors
    /// The sticky set failure or the visitor's failure.
    pub(in crate::exec::sink) fn for_each_since(
        &mut self,
        since: usize,
        visit: &mut dyn FnMut(&[u64]) -> crate::error::Result<bool>,
    ) -> crate::error::Result<()> {
        debug_assert!(self.ordered, "unordered sets never drain");
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        for key in self.ram_iter_since(since) {
            if !visit(key)? {
                return Ok(());
            }
        }
        Ok(())
    }
}

/// Fallible early-stoppable stage-row visit. `Ok(false)` stops; `Err` is
/// immediate.
pub(crate) type StageRowVisit<'v> = &'v mut dyn FnMut(&[u64]) -> crate::error::Result<bool>;

#[derive(Debug)]
pub(in crate::exec::sink) enum DedupState {
    Bindings {
        seen: SeenSet,
    },
    Union {
        seen: SeenSet,
        spans: Vec<(usize, usize)>,
    },
    DnfUnion {
        seen: SeenSet,
        spans: Vec<(usize, usize)>,
    },
    Elided {
        #[allow(dead_code)]
        witness: crate::plan::fj::DistinctWitness,
    },
}

impl DedupState {
    #[inline]
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

    pub(in crate::exec::sink) fn seen(&self) -> Option<&SeenSet> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }

    pub(in crate::exec::sink) fn seen_mut(&mut self) -> Option<&mut SeenSet> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }
}

fn word_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

fn i64_to_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn sources_of(finds: &[SinkSpec]) -> Vec<usize> {
    let mut sources = Vec::new();
    extend_sources(finds, &mut sources);
    sources
}

fn extend_sources(finds: &[SinkSpec], out: &mut Vec<usize>) {
    out.clear();
    for spec in finds {
        match spec {
            SinkSpec::Var { slot, width } => {
                out.extend(*slot..slot + width);
            }
            SinkSpec::Agg(_) | SinkSpec::Pack { .. } => {}
        }
    }
}

/// The projection sink: dedups projected find tuples, and reports
/// staleness (`SkipSuffix`) so the executor can unwind suffixes that bind
/// nothing projection-relevant (D2 — legal for this sink only).
#[derive(Debug)]
pub(crate) struct ProjectionSink {
    finds: Vec<SinkSpec>,
    sources: Vec<usize>,
    seen: SeenSet,
    scratch: Vec<u64>,
    batch_route: projection::ProjectionRoute,
    /// Scan routing is independent of batch routing: pinned batches may
    /// temporarily use a combined outer-plus-leaf key-slot layout. Both
    /// routing capacities are reserved at construction, bounded by the
    /// fixed projection arity; preparation and group entry never grow them.
    scan_route: projection::ProjectionRoute,
    scan_rows: Vec<u64>,
    scan_count: u64,
}

#[derive(Debug)]
enum GroupTable {
    Hashed(WordMap<usize>),
    Dense {
        radixes: Box<[u16]>,
        table: Box<[u32]>,
        ordinals: Vec<u32>,
    },
}

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
                radixes,
                table,
                ordinals,
            } => {
                if table.is_empty() {
                    let size = radixes.iter().map(|&radix| usize::from(radix)).product();
                    *table = vec![0; size].into_boxed_slice();
                }
                table.fill(0);
                ordinals.clear();
            }
        }
    }

    fn release_memory(&mut self) {
        match self {
            Self::Hashed(map) => *map = WordMap::new(map.arity()),
            Self::Dense {
                table, ordinals, ..
            } => {
                *table = Box::default();
                *ordinals = Vec::new();
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Acc {
    SumSigned(i128),
    SumUnsigned(u128),
    Min(u64),
    Max(u64),
    Count(u64),
    /// Small handle, so integer accumulator arrays remain compact. Aliases
    /// share an exact total/count but never accumulate an argument twice.
    Float {
        index: usize,
        primary: bool,
    },
}

#[derive(Debug, Clone, Copy)]
enum FoldSource {
    Outer,
    /// Index into the shared column-reduction program, not a physical column.
    Column(usize),
}

#[derive(Debug)]
pub(in crate::exec::sink) enum GroupState {
    Folds {
        accs: Vec<Acc>,
        n_aggs: usize,
    },
    Pack {
        slot: usize,
        claims: Vec<Vec<[u64; 2]>>,
    },
}

/// The aggregate sink: group map keyed by the group-key words, folding each
/// distinct full binding exactly once. Never returns `SkipSuffix` — the
/// skip is illegal under aggregation (any new bound variable multiplies
/// the binding set the fold is defined over). The illegality is also
/// encoded structurally: aggregate plans mark every node sink-relevant
/// (run.rs's skip-absorption arm), so even a skip signaled by mistake
/// would be absorbed at its producing node.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent aggregate capabilities and terminal flags are not mutually exclusive states"
)]
pub(crate) struct AggregateSink {
    dedup: DedupState,
    physical_distinct: Option<crate::plan::fj::ScalarSetTraversal>,
    finds: Vec<SinkSpec>,
    real_slots: usize,
    group_spans: Vec<(usize, usize)>,
    groups: GroupTable,
    group_state: GroupState,
    float_accs: Vec<crate::exec::kernel::numeric::ExactF64Accumulator>,
    share_float_inputs: bool,
    group_counts: Vec<u64>,
    cardinality_overflow: bool,
    /// Cancellation shared with scratch and deduplication; absent only in
    /// standalone kernel callers that have no operation context.
    work: Option<crate::work::WorkContext>,
    /// Sticky failure recorded by the infallible fold paths; finalize
    /// refuses before any group publishes.
    error: Option<crate::error::Error>,
    /// Successful finalize has published every group (L05 Finish).
    finished: bool,
    /// Stop/Error latched across [`Self::take_error`] until reset.
    terminal: SinkProgress,
    union_scratch: Vec<u64>,
    key_scratch: Vec<u64>,
    binding_scratch: Vec<u64>,
    dedup_survivors: Vec<u32>,
    fold_sources: Vec<FoldSource>,
    fold_inputs: Vec<aggregate::reduce::FoldInput>,
    scan_count: u64,
    cached_slot_count: Option<usize>,
    cached_key_slots: Vec<usize>,
    /// Slot-indexed: None is outer; Some stores the one-based leaf word.
    cached_leaf_words: Vec<Option<NonZeroU32>>,
    cached_constant_group: bool,
    #[cfg(test)]
    group_probes: usize,
}
