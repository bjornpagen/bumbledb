//! The two consumers of bindings: set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup.
//! The sinks are where union lives: `union_spans` keys the multi-rule
//! aggregate head projection. `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`.
use crate::encoding::encode_i64;
use crate::exec::scratch::{ScratchAppend, ScratchMapId};
use crate::exec::wordmap::WordMap;

mod aggregate;
mod projection;
#[cfg(test)]
mod tests;

/// A fold aggregate's operator, execution-side: exactly the ops that fold
/// over a slot into an [`Acc`]. Nullary [`AggSpec::Count`] is a sibling
/// arm, not a `FoldOp`.
pub use crate::ir::FoldOp;

/// Nullary Count vs a fold over a slot. Trusted layer: Count cannot
/// carry a slot and folds cannot omit one. Hostile Count-with-variable
/// is unrepresentable on [`crate::ir::FindTerm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggSpec {
    Count,
    /// Exact F64 Sum/Mean; Min/Max operate directly on total-order words.
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
pub enum FindSpec {
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

/// One execution's sink allowance: the operation ledger plus the RAM
/// allowance a sink's distinct-state may occupy before it must continue in
/// the one temporary-LMDB scratch map. Installed per
/// execution by the prepared query on the main sink AND interior stage
/// sinks (the derived-tuples budget still judges stage row counts at
/// seal); a sink without a budget (bare executor harnesses) stays in RAM.
#[derive(Debug, Clone)]
pub(crate) struct SinkBudget {
    pub(crate) work: crate::work::WorkContext,
    pub(crate) ram_bytes: usize,
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
    /// Work, deadline, or cancellation stopped the sink. Sticky: later
    /// emits drop and finalize refuses (Q-ATOMIC).
    Stop,
    /// Scratch, cardinality, or corruption failure. Sticky: later emits
    /// drop and finalize refuses before any answer publishes.
    Error,
}

pub(in crate::exec::sink) fn classify_progress(error: &crate::error::Error) -> SinkProgress {
    match error {
        crate::error::Error::Store(store)
            if matches!(
                store.as_ref(),
                crate::storage::store::StoreError::Work(
                    crate::work::WorkError::Cancelled
                        | crate::work::WorkError::DeadlineExceeded
                        | crate::work::WorkError::Exhausted { .. }
                )
            ) =>
        {
            SinkProgress::Stop
        }
        _ => SinkProgress::Error,
    }
}

/// A distinct-tuple set that starts as the measured RAM word table and
/// continues in the one scratch relation when its owner's allowance is
/// crossed — exact full-key semantics in both tiers, insertion order on
/// [`ScratchMapId::OrderLog`] of that same env when `ordered`. Errors are
/// sticky: the executor's sink interface is infallible, so a scratch
/// failure records itself, subsequent inserts drop, and finalize surfaces
/// the error before any answer publishes (Q-ATOMIC).
#[derive(Debug)]
pub(in crate::exec::sink) struct SpillSet {
    ordered: bool,
    ram: WordMap<()>,
    /// Some only under an exact singleton-head uniqueness proof. The RAM
    /// tier is a dense insertion-order log; the existing scratch protocol,
    /// budget, work quantum and sticky failures remain shared.
    unique_rows: Option<UniqueRows>,
    spilled: Option<SpilledSet>,
    budget: Option<SinkBudget>,
    error: Option<crate::error::Error>,
    key_bytes: Vec<u8>,
    /// Work is charged in bounded quanta, not per emitted row — the warm
    /// path pays one branch and one counter increment per insert; the
    /// ledger (deadline/cancellation included) is polled every
    /// [`STEP_QUANTUM`] rows, the published maximum unpolled quantum.
    pending_steps: u32,
}

/// The maximum unpolled work quantum of a sink's RAM tier (chapter 12 §7:
/// publish the quantum; cancellation is checked at bounded intervals).
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

struct SpilledSet {
    set: crate::exec::scratch::ScratchRelation,
    /// Insertion-order log is [`ScratchMapId::OrderLog`] on [`Self::set`].
    ordered: bool,
    entries: u64,
}

impl std::fmt::Debug for SpilledSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpilledSet")
            .field("entries", &self.entries)
            .field("ordered", &self.ordered)
            .finish_non_exhaustive()
    }
}

impl SpillSet {
    pub(in crate::exec::sink) fn with_capacity_hint(
        arity: usize,
        hint: usize,
        ordered: bool,
    ) -> Self {
        Self {
            ordered,
            ram: WordMap::with_capacity_hint(arity, hint),
            unique_rows: None,
            spilled: None,
            budget: None,
            error: None,
            key_bytes: Vec::new(),
            pending_steps: 0,
        }
    }

    pub(in crate::exec::sink) fn begin(&mut self, budget: Option<SinkBudget>) {
        self.budget = budget;
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

    fn resident_len(&self) -> usize {
        self.unique_rows
            .as_ref()
            .map_or_else(|| self.ram.len(), |rows| rows.len)
    }

    pub(in crate::exec::sink) fn len(&self) -> usize {
        self.resident_len()
            + self.spilled.as_ref().map_or(0, |spilled| {
                usize::try_from(spilled.set.len()).expect("64-bit targets")
            })
    }

    pub(in crate::exec::sink) fn spilled(&self) -> bool {
        self.spilled.is_some()
    }

    pub(in crate::exec::sink) fn clear(&mut self) {
        self.ram.clear();
        if let Some(rows) = &mut self.unique_rows {
            rows.clear();
        }
        self.spilled = None;
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
    // carry dense-row allocation or a second resident-representation branch
    // in its frame. Known-mode batches bypass the dynamic dispatcher.
    #[inline(never)]
    fn insert_inner<const UNIQUE: bool>(&mut self, key: &[u64]) -> bool {
        debug_assert_eq!(self.unique_rows.is_some(), UNIQUE);
        if self.error.is_some() {
            return false;
        }
        if self.spilled.is_some() {
            return self.insert_scratch(key);
        }
        if let Some(budget) = &self.budget {
            let resident = if UNIQUE {
                self.unique_rows.as_ref().expect("proved projection").len
            } else {
                self.ram.len()
            };
            let bytes = (resident + 1) * (key.len() * 8 + 16);
            if bytes > budget.ram_bytes {
                return self.insert_scratch(key);
            }
            self.pending_steps += 1;
            if self.pending_steps >= STEP_QUANTUM && !self.poll_steps() {
                return false;
            }
        }
        if UNIQUE {
            let rows = self.unique_rows.as_mut().expect("proved projection");
            debug_assert_eq!(key.len(), self.ram.arity());
            if rows.words.try_reserve(key.len()).is_err() {
                self.error = Some(crate::error::Error::from_store(
                    crate::storage::store::StoreError::Allocation,
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

    /// Insert a nonempty-width gathered projection in order. Amortize
    /// resident bookkeeping only inside a prefix that cannot reach a
    /// spill or poll boundary, even if every row adds a new key. Duplicates
    /// may make the next prefix longer; boundary rows retain the ordinary
    /// conservative pre-probe spill and polling behavior.
    fn insert_hashed_rows(&mut self, mut words: &[u64]) {
        let arity = self.ram.arity();
        debug_assert!(self.unique_rows.is_none() && arity != 0);
        debug_assert_eq!(words.len() % arity, 0);
        while !words.is_empty() && self.error.is_none() {
            if self.spilled.is_some() {
                for row in words.chunks_exact(arity) {
                    self.insert_inner::<false>(row);
                }
                return;
            }
            let mut count = words.len() / arity;
            if let Some(budget) = &self.budget {
                let admitted = (budget.ram_bytes / (arity * 8 + 16)).saturating_sub(self.ram.len());
                let before_poll = usize::try_from(STEP_QUANTUM - 1 - self.pending_steps)
                    .expect("less than a work quantum");
                count = count.min(admitted).min(before_poll);
            }
            if count == 0 {
                self.insert_inner::<false>(&words[..arity]);
                words = &words[arity..];
            } else {
                let width = count * arity;
                // Dispatch width once; retain growth before duplicate
                // lookup and each row's ordinary allocation behavior.
                self.ram.insert_rows(&words[..width]);
                if self.budget.is_some() {
                    self.pending_steps += u32::try_from(count).expect("less than a work quantum");
                }
                words = &words[width..];
            }
        }
    }

    /// Generate proved-unique rows directly in their retained allocation.
    /// Only prefixes before the next allocation, poll or spill boundary
    /// are bulk-filled. Boundary rows use the ordinary insertion path.
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
            let mut count = if self.spilled.is_some() {
                0
            } else {
                (len - offset).min(spare)
            };
            if let Some(budget) = &self.budget {
                let admitted = (budget.ram_bytes / (arity * 8 + 16)).saturating_sub(rows.len);
                // The polling row itself must use insert: append its
                // preceding prefix BEFORE checking the work ledger.
                let before_poll = usize::try_from(STEP_QUANTUM - 1 - self.pending_steps)
                    .expect("less than a work quantum");
                count = count.min(admitted).min(before_poll);
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
                if self.budget.is_some() {
                    self.pending_steps += u32::try_from(count).expect("less than a work quantum");
                }
                offset += count;
            }
        }
    }

    // Keep the ledger/error representation and scratch transaction frames
    // out of every resident tuple's stack frame. The polling quantum and
    // the conservative pre-insert spill threshold are unchanged.
    #[cold]
    #[inline(never)]
    fn poll_steps(&mut self) -> bool {
        let pending = u64::from(self.pending_steps);
        self.pending_steps = 0;
        let budget = self.budget.as_ref().expect("only budgeted inserts poll");
        match budget.work.step(pending) {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(crate::api::prepared::source::work_error(error));
                false
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn insert_scratch(&mut self, key: &[u64]) -> bool {
        if self.spilled.is_none()
            && let Err(error) = self.spill()
        {
            self.error = Some(error);
            return false;
        }
        let spilled = self.spilled.as_mut().expect("scratch tier is installed");
        let key_bytes = &mut self.key_bytes;
        key_bytes.clear();
        for word in key {
            key_bytes.extend_from_slice(&word.to_be_bytes());
        }
        match spilled.set.insert_if_absent(key_bytes, &[]) {
            Ok(false) => false,
            Ok(true) => {
                if spilled.ordered {
                    let seq = spilled.entries.to_be_bytes();
                    let mut append = ScratchAppend::new(&mut spilled.set);
                    if let Err(error) = append.append(ScratchMapId::OrderLog, &seq, key_bytes) {
                        drop(append);
                        self.error = Some(error);
                        return false;
                    }
                    if let Err(error) = append.finish() {
                        self.error = Some(error);
                        return false;
                    }
                }
                spilled.entries += 1;
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn spill(&mut self) -> crate::error::Result<()> {
        let budget = self
            .budget
            .as_ref()
            .expect("spill is reached only under a budget");
        let mut set = crate::exec::scratch::ScratchRelation::new(&budget.work, 0);
        set.force_spill()?;
        let mut entries: u64 = 0;
        let mut key_bytes = Vec::new();
        let mut append = ScratchAppend::new(&mut set);
        let copied = (|| {
            for key in self.ram_iter_since(0) {
                key_bytes.clear();
                for word in key {
                    key_bytes.extend_from_slice(&word.to_be_bytes());
                }
                append.append(ScratchMapId::Default, &key_bytes, &[])?;
                if self.ordered {
                    append.append(ScratchMapId::OrderLog, &entries.to_be_bytes(), &key_bytes)?;
                }
                entries += 1;
            }
            Ok(())
        })();
        match copied {
            Ok(()) => append.finish()?,
            Err(error) => {
                drop(append);
                return Err(error);
            }
        }
        // Ownership switches only after the copy completed.
        self.ram.clear();
        if let Some(rows) = &mut self.unique_rows {
            rows.clear();
        }
        self.spilled = Some(SpilledSet {
            set,
            ordered: self.ordered,
            entries,
        });
        Ok(())
    }

    /// RAM-tier insertion-order iteration — the warm drain/watermark path.
    /// Callers branch on [`Self::spilled`] first.
    pub(in crate::exec::sink) fn ram_iter_since(
        &self,
        since: usize,
    ) -> ResidentRows<impl Iterator<Item = &[u64]> + Clone, impl Iterator<Item = &[u64]> + Clone>
    {
        debug_assert!(!self.spilled(), "spilled sets drain through for_each_since");
        match &self.unique_rows {
            Some(rows) => {
                let arity = self.ram.arity();
                let start = since.min(rows.len) * arity;
                ResidentRows::Dense(rows.words[start..].chunks_exact(arity))
            }
            None => ResidentRows::Hashed(self.ram.iter_since(since).map(|(key, ())| key)),
        }
    }

    /// Insertion-ordered drain from `since`, across both tiers. Ordered
    /// sets only. `Ok(false)` stops the walk; `Err` is immediate.
    /// # Errors
    /// Scratch read failure, stopped work, or the visitor's failure.
    pub(in crate::exec::sink) fn for_each_since(
        &mut self,
        since: usize,
        visit: &mut dyn FnMut(&[u64]) -> crate::error::Result<bool>,
    ) -> crate::error::Result<()> {
        debug_assert!(self.ordered, "unordered sets never drain");
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        match &mut self.spilled {
            None => {
                for key in self.ram_iter_since(since) {
                    if !visit(key)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Some(spilled) => {
                debug_assert!(spilled.ordered, "ordered sets keep the row log");
                let mut words: Vec<u64> = Vec::new();
                spilled.set.visit_map_from(
                    ScratchMapId::OrderLog,
                    &(since as u64).to_be_bytes(),
                    &mut |_: &[u8], row: &[u8]| {
                        words.clear();
                        for chunk in row.as_chunks::<8>().0 {
                            words.push(u64::from_be_bytes(*chunk));
                        }
                        visit(&words)
                    },
                )
            }
        }
    }
}

/// Fallible early-stoppable stage-row visit. `Ok(false)` stops; `Err` is
/// immediate.
pub(crate) type StageRowVisit<'v> = &'v mut dyn FnMut(&[u64]) -> crate::error::Result<bool>;

pub(in crate::exec::sink) fn encode_stage_row(row: &[u64], out: &mut Vec<u8>) {
    out.clear();
    for word in row {
        out.extend_from_slice(&word.to_be_bytes());
    }
}

#[derive(Debug)]
pub(in crate::exec::sink) enum DedupState {
    Bindings {
        seen: SpillSet,
    },
    Union {
        seen: SpillSet,
        spans: Vec<(usize, usize)>,
    },
    DnfUnion {
        seen: SpillSet,
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

    pub(in crate::exec::sink) fn seen(&self) -> Option<&SpillSet> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }

    pub(in crate::exec::sink) fn seen_mut(&mut self) -> Option<&mut SpillSet> {
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
pub struct ProjectionSink {
    finds: Vec<SinkSpec>,
    sources: Vec<usize>,
    seen: SpillSet,
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
                table, ordinals, ..
            } => {
                table.fill(0);
                ordinals.clear();
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
pub struct AggregateSink {
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
    /// This execution's allowance: installed by `begin`, consulted by the
    /// group-state pressure check (`maybe_spill_groups`). `None` = derived
    /// stage sink (RAM-only; the derived-tuples budget bounds it at seal).
    budget: Option<SinkBudget>,
    /// The spilled group-state partition store, once the RAM group tables
    /// crossed the allowance ([`aggregate::spill::GroupSpill`]).
    spill: Option<Box<aggregate::spill::GroupSpill>>,
    /// Sticky spill/scratch failure recorded by the infallible fold paths;
    /// finalize refuses before any group publishes (Q-ATOMIC).
    error: Option<crate::error::Error>,
    /// Successful finalize has published every group (L05 Finish).
    finished: bool,
    /// Stop/Error latched across [`Self::take_error`] until reset.
    terminal: SinkProgress,
    /// Tracked Pack-claim bytes (the claims grow per row, not per group).
    pack_bytes: usize,
    union_scratch: Vec<u64>,
    key_scratch: Vec<u64>,
    binding_scratch: Vec<u64>,
    acc_scratch: Vec<Acc>,
    dedup_survivors: Vec<u32>,
    scan_sources: Vec<FoldSource>,
    scan_inputs: Vec<aggregate::scan::ScanInput>,
    scan_count: u64,
    cached_outer_slots: Vec<usize>,
    cached_constant_group: bool,
    #[cfg(test)]
    group_probes: usize,
}
