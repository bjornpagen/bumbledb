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
use crate::error::{Error, Result};
use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, LeafSource, Sink};
use crate::exec::wordmap::WordMap;
use crate::image::ColumnView;
use crate::ir::AggOp;

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
    /// Per-slot leaf-batch sources, recomputed only when the leaf shape
    /// changes (PRD 05's pointer-keyed cache — pinned leaves emit
    /// batches of one, so per-batch recomputation was per-row work):
    /// `Some(word)` reads the batch keys, `None` the outer bindings.
    batch_sources: Vec<Option<usize>>,
    /// The cache key: `key_slots` pointer + length (stable per prepared
    /// executor; invalidated on reset).
    sources_key: (usize, usize),
    /// Rows consumed by the open scan (docs/perf/ PRD 05).
    scan_count: u64,
}

impl ProjectionSink {
    /// `slots`: the projected variables' binding slots, in find order.
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        let arity = slots.len();
        Self {
            slots,
            seen: WordMap::new(arity),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
            sources_key: (0, 0),
            scan_count: 0,
        }
    }

    /// The distinct projected tuples, unordered (results are sets; the
    /// host sorts).
    pub fn rows(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.iter().map(|(key, ())| key)
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
        self.sources_key = (0, 0);
    }
}

impl Sink for ProjectionSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for (i, slot) in self.slots.iter().enumerate() {
            self.scratch[i] = bindings.get(*slot);
        }
        self.seen.insert(&self.scratch);
        // The doc's first-emit signal (30-execution D2): once a projected
        // tuple lands — new or duplicate — the current suffix can only
        // multiply witnesses. The executor's sink_relevant gating
        // (run.rs's skip-absorption arm) decides how far the skip
        // unwinds — for projections the bits come from the group key
        // (hardening PRD 05); signaling on the *first* emit (not the
        // first duplicate) saves one full suffix descent per distinct
        // output tuple.
        Flow::SkipSuffix
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Sources cached on the leaf shape (pointer-keyed, PRD 05); the
        // outer values refresh per batch (bindings vary per parent), the
        // row loop touches only the varying key words and the seen-set.
        let key = (batch.key_slots.as_ptr() as usize, batch.key_slots.len());
        if key != self.sources_key {
            for (i, slot) in self.slots.iter().enumerate() {
                self.batch_sources[i] = match batch.source_of(*slot) {
                    LeafSource::Key(word) => Some(word),
                    LeafSource::Outer => None,
                };
            }
            self.sources_key = key;
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = batch.bindings.get(*slot);
            }
        }
        for &entry in batch.survivors {
            for (i, source) in self.batch_sources.iter().enumerate() {
                if let Some(word) = source {
                    self.scratch[i] = batch.key(entry, *word);
                }
            }
            self.seen.insert(&self.scratch);
            if stop_on_skip {
                // First-emit semantics (see `emit`): the remaining rows
                // bind nothing sink-relevant — the executor unwinds.
                return Flow::SkipSuffix;
            }
        }
        Flow::Continue
    }

    fn may_skip(&self) -> bool {
        true
    }

    /// The projection scan (docs/perf/ PRD 05): positions insert straight
    /// into the seen-set — outer slots prefilled once, leaf words read
    /// live from the columns. The executor never opens a scan on a leaf
    /// that could skip (D2 leaves stay on the batch path), so every
    /// position inserts.
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool {
        let key = (scan.key_slots.as_ptr() as usize, scan.key_slots.len());
        if key != self.sources_key {
            for (i, slot) in self.slots.iter().enumerate() {
                self.batch_sources[i] = scan.key_slots.iter().position(|k| k == slot);
            }
            self.sources_key = key;
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = scan.bindings.get(*slot);
            }
        }
        self.scan_count = 0;
        true
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        // Run-length-adaptive column resolution (docs/perf/ PRD 05,
        // measured both ways): big runs amortize a hoisted column table,
        // fanout-sized runs resolve per position.
        if run.len() >= 32 {
            assert!(self.batch_sources.len() <= 8, "projection arity cap");
            let cols: [Option<ColumnView<'_>>; 8] = std::array::from_fn(|i| {
                self.batch_sources
                    .get(i)
                    .copied()
                    .flatten()
                    .map(|word| scan.colt.suffix_column(scan.level, word))
            });
            let mut insert = |position: u32| {
                for (i, source) in self.batch_sources.iter().enumerate() {
                    if source.is_some() {
                        self.scratch[i] = match cols[i].as_ref().expect("resolved with sources") {
                            ColumnView::Words(w) => w[position as usize],
                            ColumnView::Bytes(b) => u64::from(b[position as usize]),
                        };
                    }
                }
                self.seen.insert(&self.scratch);
            };
            run_positions(run, &mut insert);
        } else {
            let mut insert = |position: u32| {
                for (i, source) in self.batch_sources.iter().enumerate() {
                    if let Some(word) = source {
                        self.scratch[i] = match scan.colt.suffix_column(scan.level, *word) {
                            ColumnView::Words(w) => w[position as usize],
                            ColumnView::Bytes(b) => u64::from(b[position as usize]),
                        };
                    }
                }
                self.seen.insert(&self.scratch);
            };
            run_positions(run, &mut insert);
        }
    }

    fn end_scan(&mut self, _scan: &LeafScan<'_>) -> u64 {
        self.scan_count
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
    /// The group-run memo (docs/perf/ PRD 02): consecutive constant-group
    /// batches within one node-entry run share their group — remember the
    /// last key words and accumulator index and skip even the
    /// once-per-batch probe when unchanged.
    memo_key: Vec<u64>,
    memo_idx: Option<usize>,
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
    sources_key: (usize, usize),
    /// Group-map probes actually issued (the PRD 02 hoist observable).
    #[cfg(test)]
    group_probes: usize,
}

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count;
    /// `distinct_bindings` is the plan's elision flag (30-execution): when
    /// set, the seen-set is skipped entirely.
    #[must_use]
    pub fn new(finds: Vec<FindSpec>, slot_count: usize, distinct_bindings: bool) -> Self {
        let group_slots: Vec<usize> = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Var { slot } => Some(*slot),
                FindSpec::Agg { .. } => None,
            })
            .collect();
        let n_aggs = finds.len() - group_slots.len();
        Self {
            groups: WordMap::new(group_slots.len()),
            key_scratch: vec![0; group_slots.len()],
            binding_scratch: vec![0; slot_count],
            seen: (!distinct_bindings).then(|| WordMap::new(slot_count)),
            memo_key: vec![0; group_slots.len()],
            memo_idx: None,
            acc_scratch: Vec::with_capacity(n_aggs),
            dedup_survivors: Vec::new(),
            scan_sources: Vec::with_capacity(n_aggs),
            scan_count: 0,
            cached_outer_slots: Vec::new(),
            cached_constant_group: false,
            sources_key: (0, 0),
            #[cfg(test)]
            group_probes: 0,
            group_slots,
            finds,
            accs: Vec::new(),
            n_aggs,
        }
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.memo_idx = None;
        self.sources_key = (0, 0);
        self.groups.clear();
        self.accs.clear();
        if let Some(seen) = &mut self.seen {
            seen.clear();
        }
    }

    /// Finalizes each group's row (values in find order) into `emit`,
    /// assembling rows in a caller-reused scratch. Sums are range-checked
    /// here, once — deterministic by construction (i128 cannot overflow
    /// summing fewer than 2^64 i64 terms). Empty input yields zero rows: a
    /// global aggregate over nothing is the empty set, not a 0 or NULL row.
    ///
    /// # Errors
    ///
    /// `Overflow` when a Sum's final value exceeds its result type; errors
    /// from `emit` propagate.
    pub fn finalize_into(
        &self,
        row_scratch: &mut Vec<u64>,
        mut emit: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        for (key, group_idx) in self.groups.iter() {
            let accs = &self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
            row_scratch.clear();
            let mut key_cursor = 0;
            let mut acc_cursor = 0;
            for (find_idx, find) in self.finds.iter().enumerate() {
                match find {
                    FindSpec::Var { .. } => {
                        row_scratch.push(key[key_cursor]);
                        key_cursor += 1;
                    }
                    FindSpec::Agg { .. } => {
                        row_scratch.push(finalize(accs[acc_cursor], find_idx)?);
                        acc_cursor += 1;
                    }
                }
            }
            emit(row_scratch)?;
        }
        Ok(())
    }

    /// Convenience finalization into fresh vectors (tests).
    ///
    /// # Errors
    ///
    /// As [`Self::finalize_into`].
    #[cfg(test)]
    pub fn into_rows(self) -> Result<Vec<Vec<u64>>> {
        let mut rows = Vec::with_capacity(self.groups.len());
        let mut scratch = Vec::new();
        self.finalize_into(&mut scratch, |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        Ok(rows)
    }
}

/// Range-checks and word-encodes one accumulator.
fn finalize(acc: Acc, find_idx: usize) -> Result<u64> {
    match acc {
        Acc::SumSigned(total) => i64::try_from(total)
            .map(i64_to_word)
            .map_err(|_| Error::Overflow { find: find_idx }),
        Acc::SumUnsigned(total) => {
            u64::try_from(total).map_err(|_| Error::Overflow { find: find_idx })
        }
        Acc::Min(word) | Acc::Max(word) | Acc::Count(word) => Ok(word),
    }
}

impl AggregateSink {
    /// The memoized group resolution (PRD 02): consecutive batches of one
    /// run share their group — compare key words before hashing.
    fn resolve_group_memoized(&mut self) -> usize {
        match self.memo_idx {
            Some(idx) if self.memo_key == self.key_scratch => idx,
            _ => {
                let idx = self.probe_group();
                self.memo_key.copy_from_slice(&self.key_scratch);
                self.memo_idx = Some(idx);
                idx
            }
        }
    }

    /// Refreshes the leaf-shape cache (outer slots + group constancy) —
    /// pointer-keyed on `key_slots`, so pinned batch-of-one leaves pay
    /// nothing after the first batch (PRD 05).
    fn refresh_shape_cache(&mut self, batch: &LeafBatch<'_>) {
        let key = (batch.key_slots.as_ptr() as usize, batch.key_slots.len());
        if key == self.sources_key {
            return;
        }
        self.cached_outer_slots.clear();
        for slot in 0..self.binding_scratch.len() {
            if matches!(batch.source_of(slot), LeafSource::Outer) {
                self.cached_outer_slots.push(slot);
            }
        }
        self.cached_constant_group = self
            .group_slots
            .iter()
            .all(|slot| matches!(batch.source_of(*slot), LeafSource::Outer));
        self.sources_key = key;
    }

    /// Probes the group map with the key currently in `key_scratch`,
    /// seeding a fresh accumulator row on first sight. The one place a
    /// group probe happens — the batch path memoizes around it.
    fn probe_group(&mut self) -> usize {
        #[cfg(test)]
        {
            self.group_probes += 1;
        }
        let next = self.groups.len();
        let (idx, inserted) = self.groups.get_or_insert_with(&self.key_scratch, || next);
        let group_idx = *idx;
        if inserted {
            // Fresh accumulator row, seeded per op.
            for find in &self.finds {
                if let FindSpec::Agg { op, signed, .. } = find {
                    self.accs.push(match (op, signed) {
                        (AggOp::Sum, true) => Acc::SumSigned(0),
                        (AggOp::Sum, false) => Acc::SumUnsigned(0),
                        (AggOp::Min, _) => Acc::Min(u64::MAX),
                        (AggOp::Max, _) => Acc::Max(u64::MIN),
                        (AggOp::Count, _) => Acc::Count(0),
                    });
                }
            }
        }
        group_idx
    }

    /// Folds the full binding currently in `binding_scratch`: dedup
    /// (unless elided), group resolution, accumulator update. The
    /// per-row paths land here — the scratch row is the one
    /// representation.
    fn fold_scratch_row(&mut self) {
        // Binding dedup: fold only the first occurrence of each distinct
        // full binding — unless the plan proved distinctness (elision).
        if let Some(seen) = &mut self.seen {
            if !seen.insert(&self.binding_scratch) {
                return;
            }
        }

        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = self.binding_scratch[*slot];
        }
        let group_idx = self.probe_group();

        let accs = &mut self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
        let mut acc_cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg {
                op,
                over_slot,
                signed,
            } = find
            else {
                continue;
            };
            let acc = &mut accs[acc_cursor];
            acc_cursor += 1;
            match (op, acc) {
                (AggOp::Count, Acc::Count(n)) => *n += 1,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    debug_assert!(*signed);
                    *total += i128::from(word_to_i64(word));
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Sum has a variable")];
                    *total += u128::from(word);
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Min has a variable")];
                    *best = (*best).min(word);
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let word =
                        self.binding_scratch[over_slot.expect("validated: Max has a variable")];
                    *best = (*best).max(word);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
    }
}

impl AggregateSink {
    /// The per-row batch arm: outer slots prefilled once, leaf key slots
    /// overwritten per row, each full binding folded through the scratch
    /// (dedup and varying-group regimes).
    fn fold_batch_rows(&mut self, batch: &LeafBatch<'_>) {
        for &slot in &self.cached_outer_slots {
            self.binding_scratch[slot] = batch.bindings.get(slot);
        }
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                self.binding_scratch[*slot] = batch.key(entry, word);
            }
            self.fold_scratch_row();
        }
    }

    /// The constant-group fast path (docs/perf/ PRD 02): one group probe
    /// per batch (memoized across consecutive batches of the same run),
    /// accumulators staged out of the group row, per-op dispatch outside
    /// the row loop, and the row loops themselves shaped as the gather
    /// folds PRD 03 kernelizes.
    /// The dedup-regime batch arm (docs/perf/ PRD 02): the seen-set pass
    /// runs per row (semantically required — the plan could not prove
    /// distinct bindings), collecting first-seen entries; those then
    /// gather-fold through the same constant-group core as the elided
    /// path, group probe hoisted and all.
    fn fold_batch_dedup_constant_group(&mut self, batch: &LeafBatch<'_>) {
        // The dedup key is the full binding: outer slots constant,
        // prefilled once (cached shape); key slots overwritten per row.
        for &slot in &self.cached_outer_slots {
            self.binding_scratch[slot] = batch.bindings.get(slot);
        }
        let mut survivors = std::mem::take(&mut self.dedup_survivors);
        survivors.clear();
        let seen = self.seen.as_mut().expect("dedup regime");
        for &entry in batch.survivors {
            for (word, slot) in batch.key_slots.iter().enumerate() {
                self.binding_scratch[*slot] = batch.key(entry, word);
            }
            if seen.insert(&self.binding_scratch) {
                survivors.push(entry);
            }
        }
        if !survivors.is_empty() {
            self.fold_batch_constant_group(batch, &survivors);
        }
        self.dedup_survivors = survivors;
    }

    fn fold_batch_constant_group(&mut self, batch: &LeafBatch<'_>, survivors: &[u32]) {
        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = batch.bindings.get(*slot);
        }
        let group_idx = self.resolve_group_memoized();

        let range = group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs;
        self.acc_scratch.clear();
        self.acc_scratch
            .extend_from_slice(&self.accs[range.clone()]);
        let count = survivors.len() as u64;
        let mut cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg {
                op,
                over_slot,
                signed,
            } = find
            else {
                continue;
            };
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            match (op, acc) {
                // Count is arithmetic, never a loop.
                (AggOp::Count, Acc::Count(n)) => *n += count,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    debug_assert!(*signed);
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += match batch.source_of(slot) {
                        // Constant over the batch: value × count, i128 —
                        // identical to `count` additions.
                        LeafSource::Outer => {
                            i128::from(word_to_i64(batch.bindings.get(slot))) * i128::from(count)
                        }
                        LeafSource::Key(word) => {
                            gather_sum_signed(batch.keys, batch.arity, word, survivors)
                        }
                    };
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += match batch.source_of(slot) {
                        LeafSource::Outer => {
                            u128::from(batch.bindings.get(slot)) * u128::from(count)
                        }
                        LeafSource::Key(word) => {
                            gather_sum_unsigned(batch.keys, batch.arity, word, survivors)
                        }
                    };
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let slot = over_slot.expect("validated: Min has a variable");
                    let word = match batch.source_of(slot) {
                        LeafSource::Outer => batch.bindings.get(slot),
                        LeafSource::Key(word) => {
                            gather_min(batch.keys, batch.arity, word, survivors)
                        }
                    };
                    *best = (*best).min(word);
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let slot = over_slot.expect("validated: Max has a variable");
                    let word = match batch.source_of(slot) {
                        LeafSource::Outer => batch.bindings.get(slot),
                        LeafSource::Key(word) => {
                            gather_max(batch.keys, batch.arity, word, survivors)
                        }
                    };
                    *best = (*best).max(word);
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
        self.accs[range].copy_from_slice(&self.acc_scratch);
    }
}

/// Drives `f` over every position of a run (the projection scan's loop).
fn run_positions(run: SuffixRun<'_>, f: &mut impl FnMut(u32)) {
    match run {
        SuffixRun::Identity { start, len } => {
            for position in start..start + len {
                f(u32::try_from(position).expect("positions fit u32"));
            }
        }
        SuffixRun::Positions(positions) => {
            for &position in positions {
                f(position);
            }
        }
    }
}

/// The batch gather folds, kerneled (docs/perf/ PRD 03): dense survivor
/// runs (ascending with no gaps — the common all-survived batch) take
/// the contiguous strided kernels with zero index loads; everything
/// else takes the `_idx` gather kernels. All take non-empty survivor
/// lists (the executor skips empty batches).
fn dense_run(survivors: &[u32]) -> Option<u32> {
    let (first, last) = (survivors[0], survivors[survivors.len() - 1]);
    (last as usize - first as usize + 1 == survivors.len()).then_some(first)
}

fn gather_sum_signed(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> i128 {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_sum_biased_i64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_sum_biased_i64_idx(keys, arity, word, survivors),
    }
}

fn gather_sum_unsigned(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u128 {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_sum_u64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_sum_u64_idx(keys, arity, word, survivors),
    }
}

fn gather_min(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u64 {
    gather_min_max(keys, arity, word, survivors).0
}

fn gather_max(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> u64 {
    gather_min_max(keys, arity, word, survivors).1
}

fn gather_min_max(keys: &[u64], arity: usize, word: usize, survivors: &[u32]) -> (u64, u64) {
    match dense_run(survivors) {
        Some(first) => crate::exec::kernel::fold_min_max_u64(
            keys,
            arity,
            first as usize * arity + word,
            survivors.len(),
        ),
        None => crate::exec::kernel::fold_min_max_u64_idx(keys, arity, word, survivors),
    }
}

impl Sink for AggregateSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..bindings.slot_count() {
            self.binding_scratch[slot] = bindings.get(slot);
        }
        self.fold_scratch_row();
        Flow::Continue
    }

    /// The scan-fold pushdown (docs/perf/ PRD 05): supported for the
    /// elided constant-group regime over word columns — positions fold
    /// straight through the kernels with no key batch materialized.
    /// Partials are identity-seeded and merged at `end_scan`, so an
    /// empty scan creates no group row (matching the batch paths).
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool {
        if self.seen.is_some() {
            return false;
        }
        if self
            .group_slots
            .iter()
            .any(|slot| scan.key_slots.contains(slot))
        {
            return false;
        }
        self.scan_sources.clear();
        for find in &self.finds {
            let FindSpec::Agg { over_slot, .. } = find else {
                continue;
            };
            let source = over_slot.and_then(|slot| scan.key_slots.iter().position(|k| *k == slot));
            if let Some(word) = source {
                // Aggregates fold integer columns; a byte-backed column
                // here would be a validation hole — decline, don't trust.
                if !matches!(
                    scan.colt.suffix_column(scan.level, word),
                    ColumnView::Words(_)
                ) {
                    return false;
                }
            }
            self.scan_sources.push(source);
        }
        // Identity-seeded partials; the group key resolves now (outer
        // bindings are constant for this node entry).
        self.acc_scratch.clear();
        for find in &self.finds {
            if let FindSpec::Agg { op, signed, .. } = find {
                self.acc_scratch.push(match (op, signed) {
                    (AggOp::Sum, true) => Acc::SumSigned(0),
                    (AggOp::Sum, false) => Acc::SumUnsigned(0),
                    (AggOp::Min, _) => Acc::Min(u64::MAX),
                    (AggOp::Max, _) => Acc::Max(u64::MIN),
                    (AggOp::Count, _) => Acc::Count(0),
                });
            }
        }
        self.scan_count = 0;
        for (i, slot) in self.group_slots.iter().enumerate() {
            self.key_scratch[i] = scan.bindings.get(*slot);
        }
        true
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        let mut cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg { op, .. } = find else {
                continue;
            };
            let source = self.scan_sources[cursor];
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            let Some(word) = source else {
                continue; // outer-constant / Count: finished at end_scan
            };
            let ColumnView::Words(col) = scan.colt.suffix_column(scan.level, word) else {
                unreachable!("begin_scan declined byte columns")
            };
            match (op, acc, run) {
                (AggOp::Sum, Acc::SumSigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_biased_i64(col, 1, start, len);
                }
                (AggOp::Sum, Acc::SumSigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_biased_i64_idx(col, 1, 0, p);
                }
                (AggOp::Sum, Acc::SumUnsigned(total), SuffixRun::Identity { start, len }) => {
                    *total += kernel::fold_sum_u64(col, 1, start, len);
                }
                (AggOp::Sum, Acc::SumUnsigned(total), SuffixRun::Positions(p)) => {
                    *total += kernel::fold_sum_u64_idx(col, 1, 0, p);
                }
                (AggOp::Min, Acc::Min(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).min(kernel::fold_min_max_u64(col, 1, start, len).0);
                }
                (AggOp::Min, Acc::Min(best), SuffixRun::Positions(p)) => {
                    *best = (*best).min(kernel::fold_min_max_u64_idx(col, 1, 0, p).0);
                }
                (AggOp::Max, Acc::Max(best), SuffixRun::Identity { start, len }) => {
                    *best = (*best).max(kernel::fold_min_max_u64(col, 1, start, len).1);
                }
                (AggOp::Max, Acc::Max(best), SuffixRun::Positions(p)) => {
                    *best = (*best).max(kernel::fold_min_max_u64_idx(col, 1, 0, p).1);
                }
                _ => unreachable!("accumulators are seeded per op; Count has no source"),
            }
        }
    }

    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let count = self.scan_count;
        if count == 0 {
            return 0;
        }
        // Finish the outer-sourced and Count partials.
        let mut cursor = 0;
        for find in &self.finds {
            let FindSpec::Agg { op, over_slot, .. } = find else {
                continue;
            };
            let source = self.scan_sources[cursor];
            let acc = &mut self.acc_scratch[cursor];
            cursor += 1;
            if source.is_some() {
                continue;
            }
            match (op, acc) {
                (AggOp::Count, Acc::Count(n)) => *n += count,
                (AggOp::Sum, Acc::SumSigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += i128::from(word_to_i64(scan.bindings.get(slot))) * i128::from(count);
                }
                (AggOp::Sum, Acc::SumUnsigned(total)) => {
                    let slot = over_slot.expect("validated: Sum has a variable");
                    *total += u128::from(scan.bindings.get(slot)) * u128::from(count);
                }
                (AggOp::Min, Acc::Min(best)) => {
                    let slot = over_slot.expect("validated: Min has a variable");
                    *best = (*best).min(scan.bindings.get(slot));
                }
                (AggOp::Max, Acc::Max(best)) => {
                    let slot = over_slot.expect("validated: Max has a variable");
                    *best = (*best).max(scan.bindings.get(slot));
                }
                _ => unreachable!("accumulators are seeded per op"),
            }
        }
        // Merge the partials into the group's row (identity seeds make
        // the merge exact for every op).
        let group_idx = self.resolve_group_memoized();
        let range = group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs;
        for (acc, partial) in self.accs[range].iter_mut().zip(&self.acc_scratch) {
            match (acc, partial) {
                (Acc::SumSigned(t), Acc::SumSigned(p)) => *t += p,
                (Acc::SumUnsigned(t), Acc::SumUnsigned(p)) => *t += p,
                (Acc::Min(t), Acc::Min(p)) => *t = (*t).min(*p),
                (Acc::Max(t), Acc::Max(p)) => *t = (*t).max(*p),
                (Acc::Count(t), Acc::Count(p)) => *t += p,
                _ => unreachable!("partials are seeded from the same finds"),
            }
        }
        count
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Aggregate plans mark every node sink-relevant (hardening
        // PRD 05), so the executor never asks a fold to stop on skip.
        debug_assert!(!stop_on_skip, "folds never stop on skip");
        if batch.survivors.is_empty() {
            return Flow::Continue;
        }
        // Group-key classification, cached on the leaf shape: every
        // group slot outer means the whole batch folds into ONE
        // accumulator row — the trie already grouped it (PRD 02).
        self.refresh_shape_cache(batch);
        match (self.seen.is_some(), self.cached_constant_group) {
            // Dedup required (the plan could not prove distinctness):
            // the seen-set pass runs per row, but the group probe still
            // hoists — surviving entries gather-fold like the elided
            // path.
            (true, true) => self.fold_batch_dedup_constant_group(batch),
            (false, true) => self.fold_batch_constant_group(batch, batch.survivors),
            // Varying group keys: the per-row correctness arm.
            (_, false) => self.fold_batch_rows(batch),
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::exec::colt::Colt;
    use crate::exec::run::{Counters, Executor};
    use crate::image::view::apply;
    use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence};
    use crate::ir::VarId;
    use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
    use crate::plan::planner::JoinOrder;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema,
        SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// Posting(id serial u64, account u64, amount i64) +
    /// PostingTag(posting u64, tag u64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Posting".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "account".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "amount".into(),
                            value_type: ValueType::I64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "PostingTag".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "posting".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "tag".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                },
            ],
        }
        .validate()
        .expect("valid fixture")
    }

    const POSTING: RelationId = RelationId(0);
    const TAG: RelationId = RelationId(1);

    fn views_of(
        dir: &TempDir,
        schema: &Schema,
        postings: &[(u64, u64, i64)],
        tags: &[(u64, u64)],
    ) -> Vec<Arc<crate::image::RelationImage>> {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (id, account, amount) in postings {
            let mut bytes = Vec::new();
            encode_fact(
                &[
                    ValueRef::U64(*id),
                    ValueRef::U64(*account),
                    ValueRef::I64(*amount),
                ],
                schema.relation(POSTING).layout(),
                &mut bytes,
            );
            delta.insert(&view, POSTING, &bytes).expect("insert");
        }
        for (posting, tag) in tags {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*posting), ValueRef::U64(*tag)],
                schema.relation(TAG).layout(),
                &mut bytes,
            );
            delta.insert(&view, TAG, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        [POSTING, TAG]
            .iter()
            .map(|rel| crate::image::build(&txn, schema, *rel).expect("build"))
            .collect()
    }

    fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
        plan.occurrences()
            .iter()
            .map(|occurrence| {
                let columns: Vec<Vec<usize>> = occurrence
                    .trie_schema
                    .iter()
                    .map(|level| {
                        level
                            .iter()
                            .map(|var| {
                                let (field, _) = occurrence
                                    .vars
                                    .iter()
                                    .find(|(_, v)| v == var)
                                    .expect("plan vars");
                                usize::from(field.0)
                            })
                            .collect()
                    })
                    .collect();
                Colt::new(
                    apply(
                        &images[usize::try_from(occurrence.relation.0).expect("small")],
                        &[],
                        &[],
                        Vec::new(),
                    ),
                    &[],
                    columns,
                )
            })
            .collect()
    }

    fn occurrence(occ: u16, relation: RelationId, vars: &[(u16, u16)]) -> Occurrence {
        Occurrence {
            occ_id: OccId(occ),
            relation,
            vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
            filters: vec![],
        }
    }

    fn planned(
        normalized: &NormalizedQuery,
        schema: &Schema,
        order: &[u16],
        sink_vars: &[u16],
    ) -> ValidatedPlan {
        let join_order = JoinOrder {
            order: order.iter().map(|o| OccId(*o)).collect(),
            estimates: vec![0; order.len()],
        };
        let mut plan = binary2fj(normalized, &join_order);
        factor(&mut plan);
        let sinks: BTreeSet<VarId> = sink_vars.iter().map(|v| VarId(*v)).collect();
        validate(&plan, normalized, schema, vec![0; order.len()], &sinks).expect("valid plan")
    }

    fn run_aggregate(
        plan: &ValidatedPlan,
        views: &[Arc<crate::image::RelationImage>],
        finds: Vec<FindSpec>,
    ) -> Result<Vec<Vec<u64>>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds, plan.slots().len(), plan.distinct_bindings());
        Executor::new(plan).execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        let mut rows = sink.into_rows()?;
        rows.sort_unstable();
        Ok(rows)
    }

    /// Counters recording D2 skips.
    #[derive(Default)]
    struct SkipCounter {
        skips: usize,
    }

    impl Counters for SkipCounter {
        fn batch(&mut self, _: usize, _: usize) {}
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {
            self.skips += 1;
        }
    }

    /// PRD 05: the projection scan with leaf residuals (the spread
    /// shape) — positions filter through the residual, insert into the
    /// seen-set, and match the brute-force pair set exactly.
    #[test]
    fn projection_scan_filters_residuals_like_the_oracle() {
        let dir = TempDir::new("sink-projection-scan");
        let schema = schema();
        // Pairs within an account: Q(lo, hi) :- Posting(acct, lo),
        // Posting(acct, hi), lo < hi.
        let postings: Vec<(u64, u64, i64)> = (0..60)
            .map(|i| (i, i % 5, i64::try_from(i * 7 % 23).expect("small")))
            .collect();
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(1, 0), (2, 1)]),
                occurrence(1, POSTING, &[(1, 0), (2, 2)]),
            ],
            residuals: vec![crate::ir::normalize::PlacedComparison {
                op: crate::ir::CmpOp::Lt,
                lhs: VarId(1),
                rhs: VarId(2),
            }],
        };
        let plan = planned(&normalized, &schema, &[0, 1], &[1, 2]);
        let views2 = vec![views[0].clone(), views[0].clone()];
        let mut expected = BTreeSet::new();
        for (_, ka, va) in &postings {
            for (_, kb, vb) in &postings {
                if ka == kb && va < vb {
                    expected.insert(vec![i64_to_word(*va), i64_to_word(*vb)]);
                }
            }
        }
        for batch in [1usize, 128] {
            let mut colts = colts_for(&plan, &views2);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink =
                ProjectionSink::new(vec![plan.slot_of(VarId(1)), plan.slot_of(VarId(2))]);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let got: BTreeSet<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
            assert_eq!(got, expected, "batch {batch}");
        }
    }

    /// PRD 05: the pinned-leaf elision preserves D2 exactly — a fanout-1
    /// leaf that binds nothing sink-relevant skips per parent element,
    /// and the parent's absorption still runs, at every batch size.
    #[test]
    fn pinned_leaf_skips_preserve_d2() {
        let dir = TempDir::new("sink-pinned-d2");
        let schema = schema();
        // One tag per posting: the tag leaf pins to Cursor::Row.
        let postings: Vec<(u64, u64, i64)> = (0..40)
            .map(|i| (i, i % 4, i64::try_from(i).expect("small")))
            .collect();
        let tags: Vec<(u64, u64)> = (0..40).map(|i| (i, 900 + i)).collect();
        let views = views_of(&dir, &schema, &postings, &tags);
        // Q(account) :- Posting(id=p, account=a), PostingTag(posting=p, tag=t).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(0, 0), (1, 1)]),
                occurrence(1, TAG, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1], &[1]);
        for batch in [1usize, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1))]);
            let mut counters = SkipCounter::default();
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut counters,
            );
            let mut rows: Vec<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
            rows.sort_unstable();
            assert_eq!(
                rows,
                vec![vec![0], vec![1], vec![2], vec![3]],
                "batch {batch}"
            );
            assert!(counters.skips > 0, "batch {batch}: pinned leaves skip");
        }
    }

    /// PRD 02 (docs/perf/): the constant-group fast path — one group
    /// probe per run (memoized across batches), gather folds for every
    /// op — is value-identical to the per-row seen path at every batch
    /// size, on the stats shape (group key bound above the leaf).
    #[test]
    fn constant_group_batches_fold_once_per_run() {
        let dir = TempDir::new("sink-constant-group");
        let schema = schema();
        // 8 accounts x 300 postings: each account's leaf subtree spans
        // several batches at size 128 — the run memo holds probes at 8.
        let mut postings = Vec::new();
        let mut id = 0u64;
        for account in 0..8u64 {
            for i in 0..300i64 {
                postings.push((id, account, i - 150));
                id += 1;
            }
        }
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        // Hand-factored GJ plan: n0 binds the account, n1 the
        // (id, amount) suffix — the stats shape, where the leaf's group
        // key is outer.
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(1)],
                    }],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0), VarId(2)],
                    }],
                },
            ],
        };
        let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1), VarId(2)].into();
        let plan =
            validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Count,
                    over_slot: None,
                    signed: false,
                },
                FindSpec::Agg {
                    op: AggOp::Min,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Max,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
            ]
        };
        // The fast path (elided) vs the per-row seen path, across sizes.
        let mut reference: Option<Vec<Vec<u64>>> = None;
        for (batch, distinct) in [(1usize, true), (7, true), (128, true), (128, false)] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            if distinct && batch == 128 {
                assert_eq!(
                    sink.group_probes, 8,
                    "one probe per group run, memoized across batches"
                );
            }
            let mut rows = sink.into_rows().expect("in range");
            rows.sort_unstable();
            // Per account: Sum = -150, Count = 300, Min = -150, Max = 149.
            assert_eq!(rows.len(), 8, "batch {batch} distinct {distinct}");
            assert_eq!(
                rows[0],
                vec![
                    0,
                    i64_to_word(-150),
                    300,
                    i64_to_word(-150),
                    i64_to_word(149)
                ],
                "batch {batch} distinct {distinct}"
            );
            match &reference {
                None => reference = Some(rows),
                Some(r) => assert_eq!(*r, rows, "batch {batch} distinct {distinct}"),
            }
        }
    }

    /// PRD 02: the dedup-then-gather arm — duplicate full bindings
    /// collapse before the fold, identically at every batch size, with
    /// the group probe still hoisted.
    #[test]
    fn dedup_constant_group_collapses_duplicates_before_folding() {
        let dir = TempDir::new("sink-dedup-constant");
        let schema = schema();
        // Serials exist in storage but the query does not bind them:
        // (account, amount) bindings collapse. Account 1 holds amounts
        // {5, 5, 7} -> {5, 7}; account 2 holds {5, 5, 5} -> {5}.
        let postings = vec![
            (1u64, 1u64, 5i64),
            (2, 1, 5),
            (3, 1, 7),
            (4, 2, 5),
            (5, 2, 5),
            (6, 2, 5),
        ];
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
            residuals: vec![],
        };
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0)],
                    }],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(1)],
                    }],
                },
            ],
        };
        let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1)].into();
        let plan =
            validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(0)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(1))),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Count,
                    over_slot: None,
                    signed: false,
                },
            ]
        };
        for batch in [1usize, 2, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            // distinct_bindings = false: the dedup arm is mandatory.
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), false);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let mut rows = sink.into_rows().expect("in range");
            rows.sort_unstable();
            assert_eq!(
                rows,
                vec![vec![1, i64_to_word(12), 2], vec![2, i64_to_word(5), 1],],
                "batch {batch}"
            );
        }
    }

    /// PRD 02: an aggregate over a slot bound above the leaf folds as
    /// value x count (i128/u128 — identical to count additions),
    /// including the deterministic finalize-time overflow.
    #[test]
    fn constant_over_slot_folds_value_times_count() {
        let dir = TempDir::new("sink-constant-over");
        let schema = schema();
        // Sum(account) grouped by account: the over-slot is the group
        // slot itself — outer at the leaf. Account big enough that
        // value x count overflows u64 (caught at finalize) for one
        // group, stays in range for the other.
        let big = u64::MAX / 2;
        let mut postings = vec![];
        for id in 0..5u64 {
            postings.push((id, big, 1i64));
        }
        for id in 5..8u64 {
            postings.push((id, 7u64, 1i64));
        }
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(1)],
                    }],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0), VarId(2)],
                    }],
                },
            ],
        };
        let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1), VarId(2)].into();
        let plan =
            validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(1))),
                    signed: false,
                },
            ]
        };
        // Overflow parity: the batch path and the per-row path yield the
        // same typed error (big x 5 > u64::MAX).
        for distinct in [true, false] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
            Executor::with_batch_size(&plan, 128).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let err = sink.into_rows().unwrap_err();
            assert!(matches!(err, Error::Overflow { find: 1 }), "{err:?}");
        }
        // Value parity in range: drop the big account.
        let dir2 = TempDir::new("sink-constant-over-ok");
        let views = views_of(&dir2, &schema, &postings[5..], &[]);
        for distinct in [true, false] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
            Executor::with_batch_size(&plan, 128).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let rows = sink.into_rows().expect("in range");
            assert_eq!(rows, vec![vec![7, 21]], "distinct {distinct}");
        }
    }

    #[test]
    fn duplicate_witness_projection_dedups_and_skips_suffixes() {
        let dir = TempDir::new("sink-projection-skip");
        let schema = schema();
        // One posting, many tags: projecting only the account, the tag
        // suffix multiplies witnesses without changing the projection.
        // The tag node is the LEAF and is not sink-relevant: at batch
        // size 128 all 50 tags arrive in one leaf batch and the batch
        // emit must stop at the first row (PRD 01's stop_on_skip) — the
        // same skip the recursive path signaled per-row.
        let postings = vec![(1u64, 7u64, 100i64)];
        let tags: Vec<(u64, u64)> = (0..50).map(|t| (1, t)).collect();
        let views = views_of(&dir, &schema, &postings, &tags);
        // Q(account) :- Posting(id=p, account=a), PostingTag(posting=p).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(0, 0), (1, 1)]),
                occurrence(1, TAG, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        // Sink-relevant vars: just the account (var 1).
        let plan = planned(&normalized, &schema, &[0, 1], &[1]);
        for batch in [1usize, 2, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1))]);
            let mut counters = SkipCounter::default();
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut counters,
            );

            let rows: Vec<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
            assert_eq!(rows, vec![vec![7]], "batch {batch}");
            assert!(
                counters.skips > 0,
                "batch {batch}: the tag suffix must be skipped after the first witness"
            );
        }
    }

    /// PRD 01 (docs/perf/): the aggregate leaf batch folds bit-identically
    /// to the scalar degenerate case at every batch size, including the
    /// deterministic-overflow class at the i64 boundary.
    #[test]
    fn aggregate_leaf_batches_match_the_scalar_fold_at_the_boundary() {
        let dir = TempDir::new("sink-batch-boundary");
        let schema = schema();
        // Account 7 sums to exactly i64::MAX (in range); account 8
        // overflows deterministically.
        let postings = vec![
            (1u64, 7u64, i64::MAX),
            (2, 7, 1),
            (3, 7, -2),
            (4, 7, 1),
            (5, 8, i64::MAX),
            (6, 8, 1),
        ];
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Count,
                    over_slot: None,
                    signed: false,
                },
            ]
        };
        for batch in [1usize, 2, 7, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            // Account 8's Sum overflows: the error is deterministic and
            // carries the find index, at every batch size.
            let err = sink.into_rows().unwrap_err();
            assert!(
                matches!(err, Error::Overflow { find: 1 }),
                "batch {batch}: {err:?}"
            );
        }
        // Remove the overflowing account: values identical at every size.
        let dir2 = TempDir::new("sink-batch-boundary-ok");
        let views = views_of(&dir2, &schema, &postings[..4], &[]);
        let mut reference: Option<Vec<Vec<u64>>> = None;
        for batch in [1usize, 2, 7, 128] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
            let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
            Executor::with_batch_size(&plan, batch).execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            );
            let mut rows = sink.into_rows().expect("in range");
            rows.sort_unstable();
            assert_eq!(
                rows,
                vec![vec![7, i64_to_word(i64::MAX), 4]],
                "batch {batch}"
            );
            match &reference {
                None => reference = Some(rows),
                Some(r) => assert_eq!(*r, rows, "batch {batch}"),
            }
        }
    }

    #[test]
    fn sum_distinguishes_bound_serials_and_collapses_unbound_ones() {
        let dir = TempDir::new("sink-footgun");
        let schema = schema();
        // Two postings of amount 100 to account 7.
        let postings = vec![(1u64, 7u64, 100i64), (2, 7, 100)];
        let views = views_of(&dir, &schema, &postings, &[]);

        // Serials bound: two distinct bindings -> Sum = 200.
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(200)]]);

        // Serials unbound: the two facts collapse to one binding -> 100.
        // This documents the set-semantics footgun deliberately.
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[0]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(0)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(100)]]);
    }

    #[test]
    fn joining_a_three_tag_relation_triples_the_sum() {
        let dir = TempDir::new("sink-tag-triple");
        let schema = schema();
        let postings = vec![(1u64, 7u64, 100i64)];
        let tags = vec![(1u64, 10u64), (1, 11), (1, 12)];
        let views = views_of(&dir, &schema, &postings, &tags);
        // Sum(amount) by account joined with tags: the 3 tag bindings
        // multiply the binding set — exactly the documented footgun.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)]),
                occurrence(1, TAG, &[(0, 0), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1], &[1]);
        let finds = vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
        ];
        let rows = run_aggregate(&plan, &views, finds).expect("rows");
        assert_eq!(rows, vec![vec![7, i64_to_word(300)]]);
    }

    #[test]
    fn distinct_flag_elision_matches_the_seen_set_path() {
        let dir = TempDir::new("sink-elision");
        let schema = schema();
        let postings = vec![(1u64, 7u64, 10i64), (2, 7, 20), (3, 8, 30)];
        let views = views_of(&dir, &schema, &postings, &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[1]);
        assert!(plan.distinct_bindings(), "serials are bound");
        let finds = |plan: &ValidatedPlan| {
            vec![
                FindSpec::Var {
                    slot: plan.slot_of(VarId(1)),
                },
                FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(plan.slot_of(VarId(2))),
                    signed: true,
                },
            ]
        };

        // Elided path (as the plan proves) vs forced seen-set path.
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut elided = AggregateSink::new(finds(&plan), plan.slots().len(), true);
        Executor::new(&plan).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut elided,
            &mut crate::exec::run::NoopCounters,
        );
        let mut colts = colts_for(&plan, &views);
        let mut checked = AggregateSink::new(finds(&plan), plan.slots().len(), false);
        Executor::new(&plan).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut checked,
            &mut crate::exec::run::NoopCounters,
        );
        let mut a = elided.into_rows().expect("rows");
        let mut b = checked.into_rows().expect("rows");
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn global_aggregate_over_empty_input_yields_zero_rows() {
        let dir = TempDir::new("sink-empty-global");
        let schema = schema();
        let views = views_of(&dir, &schema, &[], &[]);
        let normalized = NormalizedQuery {
            occurrences: vec![occurrence(0, POSTING, &[(0, 0), (2, 1)])],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0], &[]);
        let finds = vec![
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            },
        ];
        let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
        // The empty set — not a [NULL] or [0] row (documented divergence
        // from SQL's ungrouped-aggregate behavior).
        assert!(rows.is_empty());
    }

    #[test]
    fn sum_is_order_independent_near_the_boundary() {
        // {i64::MAX, 1, -2} sums to MAX-1 under any fold order thanks to
        // i128 accumulation; {MAX, 1} overflows deterministically.
        for order in [[0usize, 1, 2], [2, 1, 0], [1, 2, 0]] {
            let values = [i64::MAX, 1, -2];
            let mut sink = AggregateSink::new(
                vec![FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(0),
                    signed: true,
                }],
                1,
                true,
            );
            let mut bindings = Bindings::new(1);
            bindings.reset();
            for idx in order {
                bindings.set(0, i64_to_word(values[idx]));
                assert_eq!(sink.emit(&bindings), Flow::Continue);
            }
            let rows = sink.into_rows().expect("in range");
            assert_eq!(rows, vec![vec![i64_to_word(i64::MAX - 1)]]);
        }
        for order in [[0usize, 1], [1, 0]] {
            let values = [i64::MAX, 1];
            let mut sink = AggregateSink::new(
                vec![FindSpec::Agg {
                    op: AggOp::Sum,
                    over_slot: Some(0),
                    signed: true,
                }],
                1,
                true,
            );
            let mut bindings = Bindings::new(1);
            bindings.reset();
            for idx in order {
                bindings.set(0, i64_to_word(values[idx]));
                sink.emit(&bindings);
            }
            let err = sink.into_rows().unwrap_err();
            assert!(matches!(err, Error::Overflow { find: 0 }), "{err:?}");
        }
    }

    #[test]
    fn min_and_max_honor_logical_i64_order_across_the_sign_boundary() {
        let mut sink = AggregateSink::new(
            vec![
                FindSpec::Agg {
                    op: AggOp::Min,
                    over_slot: Some(0),
                    signed: true,
                },
                FindSpec::Agg {
                    op: AggOp::Max,
                    over_slot: Some(0),
                    signed: true,
                },
            ],
            1,
            true,
        );
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for v in [-5i64, 3, -100, 42, 0] {
            bindings.set(0, i64_to_word(v));
            sink.emit(&bindings);
        }
        let rows = sink.into_rows().expect("rows");
        assert_eq!(rows, vec![vec![i64_to_word(-100), i64_to_word(42)]]);
    }
}
