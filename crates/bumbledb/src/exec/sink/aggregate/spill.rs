//! Group-state spill: the aggregate sink's group tables and accumulator
//! banks continue in the one charged scratch map (chapter 12 §4) when the
//! execution's RAM allowance is crossed — exactly like the dedup seen-set
//! and completed results, never through a private partition framework.
//!
//! Mechanism: the RAM group machinery (hashed/dense table, integer `Acc`
//! bank, exact float bank, per-group counts, Pack claims) accumulates
//! unchanged; when its estimated bytes cross the installed allowance, the
//! whole RAM partition FLUSHES into a `ScratchRelation` keyed by the
//! group-key words' exact big-endian bytes, merging with any previously
//! flushed state per group. Partition merges are licensed by the exact
//! accumulator merge laws (`lean/Bumbledb/Float64/Sum.lean` — the limb
//! bank round-trips bit-for-bit through `encode_into`/`decode_from`, so a
//! spilled group's Sum/Mean bits equal the resident bits); integer
//! Sum/Min/Max/Count merges are the same total operations the scan path
//! performs. Bindings were already deduplicated upstream, so partitions
//! are disjoint (merge is NOT idempotent — the dedup seen-set or the
//! plan's `DistinctWitness` is the license, as in `push_repeated`).
//!
//! Pack claims spill as individual `groupkey ‖ start ‖ end` set entries;
//! the scratch map's exact key order (group key first, then start) lets
//! finalize stream the maximal-segment union per group with the same
//! frontier walk as [`crate::interval::sweep`] without rematerializing a
//! group's claim set in RAM.
//!
//! Errors are sticky (the executor's emit interface is infallible): a
//! flush failure records itself, later rows drop, and finalize refuses
//! before any group publishes (Q-ATOMIC).

use crate::error::{Error, OverflowKind, Result};
use crate::exec::kernel::numeric::ExactF64Accumulator;
use crate::exec::scratch::{MAX_INLINE_KEY, ScratchRelation};
use crate::exec::sink::{Acc, AggregateSink, GroupState, GroupTable};

/// The spilled group-state partition store plus its reusable codec
/// buffers. Owned by the sink once the allowance is crossed; disposed by
/// `reset` (drop closes the scratch env before unlinking its directory).
pub(in crate::exec::sink) struct GroupSpill {
    pub(in crate::exec::sink) table: ScratchRelation,
    /// Distinct spilled groups. Exact for fold groups (the table's keys
    /// ARE the groups); an upper bound for Pack (counted once per flushed
    /// RAM group, and a group flushed twice counts twice). Only a
    /// capacity hint downstream — stage-budget judgments never see a
    /// spilled sink (stage sinks carry no allowance).
    pub(in crate::exec::sink) groups: u64,
    key_bytes: Vec<u8>,
    value_bytes: Vec<u8>,
    existing: Vec<u8>,
    decoded: DecodedGroup,
}

impl std::fmt::Debug for GroupSpill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupSpill")
            .field("groups", &self.groups)
            .field("entries", &self.table.len())
            .finish_non_exhaustive()
    }
}

/// One decoded spilled group: the same accumulator shapes the RAM bank
/// holds, with `Acc::Float` indexes pointing into the group-local `floats`
/// vector (the RAM bank's indexes point into the sink-global bank — the
/// encoder is generic over which bank its indexes address).
#[derive(Default)]
pub(in crate::exec::sink) struct DecodedGroup {
    pub(in crate::exec::sink) count: u64,
    pub(in crate::exec::sink) accs: Vec<Acc>,
    pub(in crate::exec::sink) floats: Vec<ExactF64Accumulator>,
}

fn corrupt() -> Error {
    Error::Corruption(crate::error::CorruptionError::MalformedValue(
        "aggregate scratch group",
    ))
}

fn cardinality() -> Error {
    Error::Overflow(OverflowKind::Cardinality)
}

pub(in crate::exec::sink) fn encode_key(key: &[u64], out: &mut Vec<u8>) {
    out.clear();
    for word in key {
        out.extend_from_slice(&word.to_be_bytes());
    }
}

pub(in crate::exec::sink) fn decode_key(bytes: &[u8], out: &mut Vec<u64>) -> Result<()> {
    if !bytes.len().is_multiple_of(8) {
        return Err(corrupt());
    }
    out.clear();
    for chunk in bytes.as_chunks::<8>().0 {
        out.push(u64::from_be_bytes(*chunk));
    }
    Ok(())
}

/// Encode one group's complete state. `accs` holds this group's bank
/// entries in find order; `floats` is whichever bank the `Acc::Float`
/// indexes address (the sink-global bank on the RAM side, the group-local
/// vector on the decoded side). Aliased float finds encode as a reference
/// to the earlier primary's group-local ordinal, never a second copy —
/// merge must not accumulate a shared argument twice.
fn encode_group(accs: &[Acc], count: u64, floats: &[ExactF64Accumulator], out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&count.to_be_bytes());
    for (position, acc) in accs.iter().enumerate() {
        match *acc {
            Acc::Count(n) => {
                out.push(0x01);
                out.extend_from_slice(&n.to_be_bytes());
            }
            Acc::SumSigned(total) => {
                out.push(0x02);
                out.extend_from_slice(&total.to_be_bytes());
            }
            Acc::SumUnsigned(total) => {
                out.push(0x03);
                out.extend_from_slice(&total.to_be_bytes());
            }
            Acc::Min(word) => {
                out.push(0x04);
                out.extend_from_slice(&word.to_be_bytes());
            }
            Acc::Max(word) => {
                out.push(0x05);
                out.extend_from_slice(&word.to_be_bytes());
            }
            Acc::Float { index, primary } => {
                if primary {
                    out.push(0x06);
                    floats[index].encode_into(out);
                } else {
                    // The group-local ordinal of the aliased primary: how
                    // many primary floats precede it in this group's bank.
                    let ordinal = accs[..position]
                        .iter()
                        .filter_map(|earlier| match *earlier {
                            Acc::Float {
                                index: earlier_index,
                                primary: true,
                            } => Some(earlier_index),
                            _ => None,
                        })
                        .position(|earlier_index| earlier_index == index)
                        .expect("aliases reference an earlier primary float accumulator");
                    out.push(0x07);
                    out.extend_from_slice(
                        &u32::try_from(ordinal)
                            .expect("find arity bounds the float bank")
                            .to_be_bytes(),
                    );
                }
            }
        }
    }
}

impl DecodedGroup {
    fn decode(&mut self, n_aggs: usize, bytes: &[u8]) -> Result<()> {
        self.accs.clear();
        self.floats.clear();
        let (count, mut rest) = bytes.split_at_checked(8).ok_or_else(corrupt)?;
        self.count = u64::from_be_bytes(count.try_into().expect("eight bytes"));
        for _ in 0..n_aggs {
            let (&tag, after) = rest.split_first().ok_or_else(corrupt)?;
            rest = after;
            let acc = match tag {
                0x01 => {
                    let (word, after) = rest.split_at_checked(8).ok_or_else(corrupt)?;
                    rest = after;
                    Acc::Count(u64::from_be_bytes(word.try_into().expect("eight bytes")))
                }
                0x02 => {
                    let (word, after) = rest.split_at_checked(16).ok_or_else(corrupt)?;
                    rest = after;
                    Acc::SumSigned(i128::from_be_bytes(word.try_into().expect("sixteen bytes")))
                }
                0x03 => {
                    let (word, after) = rest.split_at_checked(16).ok_or_else(corrupt)?;
                    rest = after;
                    Acc::SumUnsigned(u128::from_be_bytes(word.try_into().expect("sixteen bytes")))
                }
                0x04 => {
                    let (word, after) = rest.split_at_checked(8).ok_or_else(corrupt)?;
                    rest = after;
                    Acc::Min(u64::from_be_bytes(word.try_into().expect("eight bytes")))
                }
                0x05 => {
                    let (word, after) = rest.split_at_checked(8).ok_or_else(corrupt)?;
                    rest = after;
                    Acc::Max(u64::from_be_bytes(word.try_into().expect("eight bytes")))
                }
                0x06 => {
                    let (accumulator, used) =
                        ExactF64Accumulator::decode_from(rest).ok_or_else(corrupt)?;
                    rest = &rest[used..];
                    let index = self.floats.len();
                    self.floats.push(accumulator);
                    Acc::Float {
                        index,
                        primary: true,
                    }
                }
                0x07 => {
                    let (ordinal, after) = rest.split_at_checked(4).ok_or_else(corrupt)?;
                    rest = after;
                    let ordinal = usize::try_from(u32::from_be_bytes(
                        ordinal.try_into().expect("four bytes"),
                    ))
                    .expect("64-bit usize");
                    if ordinal >= self.floats.len() {
                        return Err(corrupt());
                    }
                    Acc::Float {
                        index: ordinal,
                        primary: false,
                    }
                }
                _ => return Err(corrupt()),
            };
            self.accs.push(acc);
        }
        if !rest.is_empty() {
            return Err(corrupt());
        }
        Ok(())
    }

    /// Merge one RAM group into this decoded (previously flushed) group.
    /// `ram_floats` is the sink-global bank the RAM `Acc::Float` indexes
    /// address. Cardinality failure (group count or float count past
    /// `u64::MAX`) is `Error::Overflow(Cardinality)` — the same judgment
    /// the resident fold makes.
    fn merge_ram(
        &mut self,
        ram_accs: &[Acc],
        ram_count: u64,
        ram_floats: &[ExactF64Accumulator],
    ) -> Result<()> {
        if self.accs.len() != ram_accs.len() {
            return Err(corrupt());
        }
        self.count = self.count.checked_add(ram_count).ok_or_else(cardinality)?;
        for (mine, ram) in self.accs.iter_mut().zip(ram_accs) {
            match (mine, *ram) {
                (Acc::Count(total), Acc::Count(n)) => *total = total.saturating_add(n),
                (Acc::SumSigned(total), Acc::SumSigned(partial)) => *total += partial,
                (Acc::SumUnsigned(total), Acc::SumUnsigned(partial)) => *total += partial,
                (Acc::Min(best), Acc::Min(word)) => *best = (*best).min(word),
                (Acc::Max(best), Acc::Max(word)) => *best = (*best).max(word),
                (
                    Acc::Float {
                        index,
                        primary: true,
                    },
                    Acc::Float {
                        index: ram_index,
                        primary: true,
                    },
                ) => {
                    self.floats[*index]
                        .merge(&ram_floats[ram_index])
                        .map_err(|_| cardinality())?;
                }
                (Acc::Float { primary: false, .. }, Acc::Float { primary: false, .. }) => {}
                _ => return Err(corrupt()),
            }
        }
        Ok(())
    }
}

impl AggregateSink {
    /// Estimated bytes of the RAM group partition — the allowance's
    /// measure, not a ledger figure (the scratch tier's growth IS
    /// ledger-charged, by the scratch map itself).
    fn ram_group_bytes(&self) -> usize {
        let table = match &self.groups {
            GroupTable::Hashed(map) => map.len() * (self.key_scratch.len() * 8 + 24),
            // The dense radix table is fixed at construction (never
            // growth); only the live ordinals count against the allowance.
            GroupTable::Dense { ordinals, .. } => ordinals.len() * 8,
        };
        let state = match &self.group_state {
            GroupState::Folds { accs, .. } => {
                accs.len() * 24
                    + self.group_counts.len() * 8
                    + self.float_accs.len() * (34 * 8 + 40)
            }
            GroupState::Pack { .. } => self.pack_bytes,
        };
        table + state
    }

    /// The fold paths' pressure check: flush the RAM partition into the
    /// scratch tier when the allowance is crossed. Infallible interface —
    /// a failure records the sticky error and later rows drop.
    pub(in crate::exec::sink) fn maybe_spill_groups(&mut self) {
        let Some(budget) = &self.budget else {
            return;
        };
        if self.error.is_some() || self.cardinality_overflow {
            return;
        }
        // Pack claim keys must stay inline in the scratch map: the
        // streaming maximal-segment union at finalize is licensed by the
        // map's exact key order, which oversized (bucketed) keys lose.
        // Group keys wide enough to overflow the inline bound stay in RAM
        // (find arity keeps real heads far below it).
        if matches!(self.group_state, GroupState::Pack { .. })
            && self.key_scratch.len() * 8 + 16 > MAX_INLINE_KEY
        {
            return;
        }
        let over = self.ram_group_bytes() > budget.ram_bytes;
        let eager = budget.ram_bytes == 0 && self.spill.is_none();
        if !(over || eager) {
            return;
        }
        if let Err(error) = self.spill_groups() {
            self.error = Some(error);
        }
    }

    /// Flush every RAM group into the scratch tier (creating it on first
    /// crossing), merging per group with previously flushed state, then
    /// clear the RAM partition. Also finalize's residual-flush entry.
    pub(in crate::exec::sink) fn spill_groups(&mut self) -> Result<()> {
        let budget = self
            .budget
            .as_ref()
            .expect("group spill is reached only under a budget");
        if self.spill.is_none() {
            crate::obs::event(
                crate::obs::names::SCRATCH_SPILL,
                crate::obs::TraceArgs::Count(self.groups.len() as u64),
            );
            let mut table = ScratchRelation::new(&budget.work, 0);
            table.force_spill()?;
            self.spill = Some(Box::new(GroupSpill {
                table,
                groups: 0,
                key_bytes: Vec::new(),
                value_bytes: Vec::new(),
                existing: Vec::new(),
                decoded: DecodedGroup::default(),
            }));
        }
        let mut spill = self.spill.take().expect("installed above");
        let result = self.flush_ram_groups(&mut spill);
        self.spill = Some(spill);
        result?;
        // Ownership switched: the scratch tier holds the merged state;
        // the RAM partition restarts empty (capacities retained — Pack
        // claim slots stay as a pool, cleared by `probe_group` on reuse).
        self.groups.clear();
        self.group_counts.clear();
        self.float_accs.clear();
        if let GroupState::Folds { accs, .. } = &mut self.group_state {
            accs.clear();
        }
        self.pack_bytes = 0;
        Ok(())
    }

    fn flush_ram_groups(&mut self, spill: &mut GroupSpill) -> Result<()> {
        match &self.group_state {
            GroupState::Folds { accs, n_aggs } => {
                let n_aggs = *n_aggs;
                let group_counts = &self.group_counts;
                let float_accs = &self.float_accs;
                for_each_ram_group(&self.groups, &mut |key, group_idx| {
                    encode_key(key, &mut spill.key_bytes);
                    let group_accs = &accs[group_idx * n_aggs..(group_idx + 1) * n_aggs];
                    let count = group_counts[group_idx];
                    if spill.table.get(&spill.key_bytes, &mut spill.existing)? {
                        spill.decoded.decode(n_aggs, &spill.existing)?;
                        spill.decoded.merge_ram(group_accs, count, float_accs)?;
                        encode_group(
                            &spill.decoded.accs,
                            spill.decoded.count,
                            &spill.decoded.floats,
                            &mut spill.value_bytes,
                        );
                    } else {
                        encode_group(group_accs, count, float_accs, &mut spill.value_bytes);
                        spill.groups += 1;
                    }
                    spill.table.put(&spill.key_bytes, &spill.value_bytes)
                })
            }
            GroupState::Pack { claims, .. } => {
                for_each_ram_group(&self.groups, &mut |key, group_idx| {
                    encode_key(key, &mut spill.key_bytes);
                    spill.groups += 1;
                    let prefix = spill.key_bytes.len();
                    for &[start, end] in &claims[group_idx] {
                        spill.key_bytes.truncate(prefix);
                        spill.key_bytes.extend_from_slice(&start.to_be_bytes());
                        spill.key_bytes.extend_from_slice(&end.to_be_bytes());
                        spill.table.insert_if_absent(&spill.key_bytes, &[])?;
                    }
                    Ok(())
                })
            }
        }
    }

    /// Finalize's spilled arm: the residual RAM partition was already
    /// flushed; walk the merged scratch state in key order and emit each
    /// group exactly once. Consumes the spill (a finished execution's
    /// scratch is disposed here rather than lingering until reset).
    pub(in crate::exec::sink) fn finalize_spilled(
        &mut self,
        answer_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        let mut spill = self
            .spill
            .take()
            .expect("finalize_spilled follows spill_groups");
        let mut key_words: Vec<u64> = Vec::new();
        match &self.group_state {
            GroupState::Folds { n_aggs, .. } => {
                let n_aggs = *n_aggs;
                let finds = &self.finds;
                let decoded = &mut spill.decoded;
                spill.table.for_each(&mut |key, value| {
                    decode_key(key, &mut key_words)?;
                    decoded.decode(n_aggs, value)?;
                    super::finalize::emit_fold_row(
                        finds,
                        &key_words,
                        &decoded.accs,
                        &decoded.floats,
                        answer_scratch,
                        emit,
                    )?;
                    Ok(true)
                })
            }
            GroupState::Pack { .. } => {
                // Streaming maximal-segment union: claims arrive in exact
                // key order (group key, then start), so one frontier walk
                // per group — the same merge judgment as
                // `crate::interval::sweep` with no window.
                let finds = &self.finds;
                let group_bytes = self.key_scratch.len() * 8;
                let mut run: Option<(Vec<u64>, u64, u64)> = None;
                let mut claim_key: Vec<u64> = Vec::new();
                spill.table.for_each(&mut |key, _| {
                    if key.len() != group_bytes + 16 {
                        return Err(corrupt());
                    }
                    decode_key(&key[..group_bytes], &mut claim_key)?;
                    let start = u64::from_be_bytes(
                        key[group_bytes..group_bytes + 8]
                            .try_into()
                            .expect("eight bytes"),
                    );
                    let end =
                        u64::from_be_bytes(key[group_bytes + 8..].try_into().expect("eight bytes"));
                    match &mut run {
                        Some((group, run_start, frontier)) if *group == claim_key => {
                            if start > *frontier {
                                super::finalize::emit_pack_row(
                                    finds,
                                    group,
                                    *run_start,
                                    *frontier,
                                    answer_scratch,
                                    emit,
                                )?;
                                *run_start = start;
                                *frontier = end;
                            } else {
                                *frontier = (*frontier).max(end);
                            }
                        }
                        _ => {
                            if let Some((group, run_start, frontier)) = run.take() {
                                super::finalize::emit_pack_row(
                                    finds,
                                    &group,
                                    run_start,
                                    frontier,
                                    answer_scratch,
                                    emit,
                                )?;
                            }
                            run = Some((claim_key.clone(), start, end));
                        }
                    }
                    Ok(true)
                })?;
                if let Some((group, run_start, frontier)) = run {
                    super::finalize::emit_pack_row(
                        finds,
                        &group,
                        run_start,
                        frontier,
                        answer_scratch,
                        emit,
                    )?;
                }
                Ok(())
            }
        }
    }

    /// Whether the group state has crossed into the scratch tier — the
    /// forced-transition tests' witness.
    #[must_use]
    pub fn group_state_spilled(&self) -> bool {
        self.spill.is_some()
    }
}

/// Walk every live RAM group as `(key words, group index)` — the shared
/// iteration finalize's resident arm and the flush both consume (dense
/// ordinals reconstruct their key words positionally).
pub(in crate::exec::sink) fn for_each_ram_group(
    groups: &GroupTable,
    visit: &mut dyn FnMut(&[u64], usize) -> Result<()>,
) -> Result<()> {
    match groups {
        GroupTable::Hashed(map) => {
            for (key, group_idx) in map.iter() {
                visit(key, *group_idx)?;
            }
            Ok(())
        }
        GroupTable::Dense {
            radixes, ordinals, ..
        } => {
            let mut key = vec![0u64; radixes.len()];
            for (group_idx, ordinal) in ordinals.iter().enumerate() {
                let mut rest = usize::try_from(*ordinal).expect("capped product");
                for (word, radix) in key.iter_mut().zip(radixes.iter()).rev() {
                    *word = (rest % usize::from(*radix)) as u64;
                    rest /= usize::from(*radix);
                }
                visit(&key, group_idx)?;
            }
            Ok(())
        }
    }
}
