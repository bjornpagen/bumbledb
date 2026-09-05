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
/// the one temporary-LMDB scratch map (chapter 12 §4). Installed per
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

    pub(in crate::exec::sink) fn len(&self) -> usize {
        self.ram.len()
            + self.spilled.as_ref().map_or(0, |spilled| {
                usize::try_from(spilled.set.len()).expect("64-bit targets")
            })
    }

    pub(in crate::exec::sink) fn spilled(&self) -> bool {
        self.spilled.is_some()
    }

    pub(in crate::exec::sink) fn clear(&mut self) {
        self.ram.clear();
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
    pub(in crate::exec::sink) fn insert(&mut self, key: &[u64]) -> bool {
        if self.error.is_some() {
            return false;
        }
        if self.spilled.is_none()
            && let Some(budget) = &self.budget
        {
            let bytes = (self.ram.len() + 1) * (key.len() * 8 + 16);
            if bytes > budget.ram_bytes {
                if let Err(error) = self.spill() {
                    self.error = Some(error);
                    return false;
                }
            } else {
                self.pending_steps += 1;
                if self.pending_steps >= STEP_QUANTUM {
                    let pending = u64::from(self.pending_steps);
                    self.pending_steps = 0;
                    if let Err(error) = budget.work.step(pending) {
                        self.error = Some(crate::api::prepared::source::work_error(error));
                        return false;
                    }
                }
            }
        }
        match &mut self.spilled {
            None => self.ram.insert(key),
            Some(spilled) => {
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
                            if let Err(error) = append.append(
                                ScratchMapId::OrderLog,
                                &seq,
                                key_bytes,
                            ) {
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
        }
    }

    fn spill(&mut self) -> crate::error::Result<()> {
        let budget = self
            .budget
            .as_ref()
            .expect("spill is reached only under a budget");
        crate::obs::event(
            crate::obs::names::SCRATCH_SPILL,
            crate::obs::TraceArgs::Count(self.ram.len() as u64),
        );
        let mut set = crate::exec::scratch::ScratchRelation::new(&budget.work, 0);
        set.force_spill()?;
        if self.ordered {
            set.open_map(ScratchMapId::OrderLog)?;
        }
        let mut entries: u64 = 0;
        let mut key_bytes = Vec::new();
        let mut append = ScratchAppend::new(&mut set);
        let copied = (|| {
            for (key, ()) in self.ram.iter() {
                key_bytes.clear();
                for word in key {
                    key_bytes.extend_from_slice(&word.to_be_bytes());
                }
                append.append(ScratchMapId::Default, &key_bytes, &[])?;
                if self.ordered {
                    append.append(
                        ScratchMapId::OrderLog,
                        &entries.to_be_bytes(),
                        &key_bytes,
                    )?;
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
    ) -> impl Iterator<Item = &[u64]> {
        debug_assert!(!self.spilled(), "spilled sets drain through for_each_since");
        self.ram.iter_since(since).map(|(key, ())| key)
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
                for (key, ()) in self.ram.iter_since(since) {
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
                    &mut |_, row| {
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

    pub(in crate::exec::sink) fn seen_len(&self) -> Option<usize> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen.len())
            }
            Self::Elided { .. } => None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjSource {
    Slot(usize),
}

#[derive(Debug)]
enum ProjectionSources {
    Plain(Vec<usize>),
}

fn sources_of(finds: &[SinkSpec]) -> ProjectionSources {
    let mut sources = Vec::new();
    extend_sources(finds, &mut sources);
    ProjectionSources::Plain(
        sources
            .into_iter()
            .map(|ProjSource::Slot(slot)| slot)
            .collect(),
    )
}

fn extend_sources(finds: &[SinkSpec], out: &mut Vec<ProjSource>) {
    out.clear();
    for spec in finds {
        match spec {
            SinkSpec::Var { slot, width } => {
                out.extend((*slot..slot + width).map(ProjSource::Slot));
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
    sources: ProjectionSources,
    seen: SpillSet,
    scratch: Vec<u64>,
    batch_sources: Vec<crate::exec::run::LeafSource>,
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
pub struct AggregateSink {
    dedup: DedupState,
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
    scan_count: u64,
    cached_outer_slots: Vec<usize>,
    cached_constant_group: bool,
    #[cfg(test)]
    group_probes: usize,
}
