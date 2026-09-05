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
//! Pack claims spill as individual `(start, end)` set entries. Mode is
//! chosen from the checked group layout at first flush — never from a
//! payload byte. Narrow heads stay inline as `(group, start, end)`. Wide
//! heads use one [`ScratchRelation`] roster: `Default` claims,
//! `GroupToToken`, and `TokenToGroup`. Finalize streams one token (or
//! one inline group) through endpoint coalescing and fetches that group's
//! header via [`ScratchRelation::visit_with_lookup`]. Exact sum/count
//! state is not rounded here.
//!
//! Errors are sticky (the executor's emit interface is infallible): a
//! flush failure records itself, later rows drop, and finalize refuses
//! before any group publishes (Q-ATOMIC).

use crate::error::{Error, OverflowKind, Result};
use crate::exec::kernel::numeric::ExactF64Accumulator;
use crate::exec::scratch::{
    MAX_INLINE_KEY, ScratchAppend, ScratchClaimKey, ScratchMapId, ScratchRelation,
};
use crate::exec::sink::{Acc, AggregateSink, GroupState, GroupTable};

/// Wide Pack claim length — [`ScratchClaimKey::BYTE_LEN`]. Always
/// ≤ [`MAX_INLINE_KEY`], so claim iteration is exact key order.
pub(crate) const PACK_WIDE_CLAIM_BYTES: usize = ScratchClaimKey::BYTE_LEN;

const _: () = assert!(
    PACK_WIDE_CLAIM_BYTES <= MAX_INLINE_KEY,
    "wide Pack claims must stay inline-ordered"
);

/// Group-key word count that forces wide (token) Pack keys: inline
/// `(group ‖ start ‖ end)` would exceed [`MAX_INLINE_KEY`].
#[must_use]
pub(crate) const fn pack_requires_wide(group_words: usize) -> bool {
    group_words.saturating_mul(8).saturating_add(16) > MAX_INLINE_KEY
}

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
    /// Checked from the group-key layout at first flush, never inferred
    /// from a leading payload byte (narrow data may start `0xFE`).
    pack_wide_mode: bool,
    /// Next stable group token. Assigned once per distinct wide group;
    /// headers live in `TokenToGroup` on [`Self::table`].
    next_pack_token: u64,
}

impl std::fmt::Debug for GroupSpill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupSpill")
            .field("groups", &self.groups)
            .field("entries", &self.table.len())
            .field("pack_wide_mode", &self.pack_wide_mode)
            .field("next_pack_token", &self.next_pack_token)
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

fn pack_claim_inline(group_key: &[u64], start: u64, end: u64, out: &mut Vec<u8>) {
    encode_key(group_key, out);
    out.extend_from_slice(&start.to_be_bytes());
    out.extend_from_slice(&end.to_be_bytes());
}

fn decode_narrow_claim(key: &[u8], group_bytes: usize) -> Result<(u64, u64)> {
    if key.len() != group_bytes + 16 {
        return Err(corrupt());
    }
    let start = u64::from_be_bytes(key[group_bytes..group_bytes + 8].try_into().expect("eight bytes"));
    let end = u64::from_be_bytes(key[group_bytes + 8..].try_into().expect("eight bytes"));
    Ok((start, end))
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
        // Pack claims use inline group heads when they fit the scratch
        // key bound; otherwise a dense token keeps `(token,start,end)`
        // ordered and short so the streaming finalize stays exact.
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
            let pack_wide_mode = matches!(&self.group_state, GroupState::Pack { .. })
                && pack_requires_wide(self.key_scratch.len());
            if pack_wide_mode {
                table.open_map(ScratchMapId::GroupToToken)?;
                table.open_map(ScratchMapId::TokenToGroup)?;
            }
            table.force_spill()?;
            self.spill = Some(Box::new(GroupSpill {
                table,
                groups: 0,
                key_bytes: Vec::new(),
                value_bytes: Vec::new(),
                existing: Vec::new(),
                decoded: DecodedGroup::default(),
                pack_wide_mode,
                next_pack_token: 0,
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
                    let mut append = ScratchAppend::new(&mut spill.table);
                    match append.append(
                        ScratchMapId::Default,
                        &spill.key_bytes,
                        &spill.value_bytes,
                    ) {
                        Ok(()) => append.finish(),
                        Err(error) => {
                            drop(append);
                            Err(error)
                        }
                    }
                })
            }
            GroupState::Pack { claims, .. } => flush_pack_claims(spill, &self.groups, claims),
        }
    }

    /// Finalize's spilled arm: the residual RAM partition was already
    /// flushed; walk the merged scratch state in key order and emit each
    /// group exactly once. Consumes the spill (a finished execution's
    /// scratch is disposed here rather than lingering until reset).
    ///
    /// Pack streams one group's ordered claims, coalesces endpoints, and
    /// fetches that group's header boundedly. Unrelated groups are not
    /// sorted against each other (set output).
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
                if spill.pack_wide_mode {
                    finalize_pack_wide(&mut spill, &self.finds, answer_scratch, emit)
                } else {
                    finalize_pack_narrow(
                        &mut spill,
                        self.key_scratch.len(),
                        &self.finds,
                        answer_scratch,
                        emit,
                    )
                }
            }
        }
    }

    /// Whether the group state has crossed into the scratch tier — the
    /// forced-transition tests' witness.
    #[must_use]
    pub fn group_state_spilled(&self) -> bool {
        self.spill.is_some()
    }

    /// Checked Pack key regime after the first flush. `None` until spill.
    #[cfg(test)]
    #[must_use]
    pub fn pack_wide_mode(&self) -> Option<bool> {
        self.spill.as_ref().map(|spill| spill.pack_wide_mode)
    }
}

fn flush_pack_claims(
    spill: &mut GroupSpill,
    groups: &GroupTable,
    claims: &[Vec<[u64; 2]>],
) -> Result<()> {
    let wide = spill.pack_wide_mode;
    if wide {
        let first_new = spill.next_pack_token;
        let mut tokens = vec![0u64; claims.len()];
        for_each_ram_group(groups, &mut |key, group_idx| {
            encode_key(key, &mut spill.key_bytes);
            if spill.table.get_map(
                ScratchMapId::GroupToToken,
                &spill.key_bytes,
                &mut spill.value_bytes,
            )? {
                if spill.value_bytes.len() != 8 {
                    return Err(corrupt());
                }
                tokens[group_idx] = u64::from_be_bytes(
                    spill.value_bytes.as_slice().try_into().expect("eight bytes"),
                );
            } else {
                let token = spill.next_pack_token;
                spill.next_pack_token = token.checked_add(1).ok_or_else(cardinality)?;
                tokens[group_idx] = token;
            }
            Ok(())
        })?;
        let GroupSpill {
            table,
            key_bytes,
            groups: group_count,
            ..
        } = spill;
        let mut append = ScratchAppend::new(table);
        let staged = for_each_ram_group(groups, &mut |key, group_idx| {
            *group_count += 1;
            let token = tokens[group_idx];
            if token >= first_new {
                encode_key(key, key_bytes);
                let token_bytes = token.to_be_bytes();
                append.append(ScratchMapId::GroupToToken, key_bytes, &token_bytes)?;
                append.append(ScratchMapId::TokenToGroup, &token_bytes, key_bytes)?;
            }
            for &[start, end] in &claims[group_idx] {
                let claim = ScratchClaimKey::new([token, start, end]).encode();
                append.append(ScratchMapId::Default, &claim, &[])?;
            }
            Ok(())
        });
        return match staged {
            Ok(()) => append.finish(),
            Err(error) => {
                drop(append);
                Err(error)
            }
        };
    }

    let GroupSpill {
        table,
        key_bytes,
        groups: group_count,
        ..
    } = spill;
    let mut append = ScratchAppend::new(table);
    let staged = for_each_ram_group(groups, &mut |key, group_idx| {
        *group_count += 1;
        for &[start, end] in &claims[group_idx] {
            pack_claim_inline(key, start, end, key_bytes);
            append.append(ScratchMapId::Default, key_bytes, &[])?;
        }
        Ok(())
    });
    match staged {
        Ok(()) => append.finish(),
        Err(error) => {
            drop(append);
            Err(error)
        }
    }
}

fn finalize_pack_wide(
    spill: &mut GroupSpill,
    finds: &[crate::exec::sink::SinkSpec],
    answer_scratch: &mut Vec<u64>,
    emit: &mut impl FnMut(&[u64]) -> Result<()>,
) -> Result<()> {
    let mut group_header: Vec<u64> = Vec::new();
    let mut header = Vec::new();
    let mut current_token: Option<u64> = None;
    let mut run_start = 0u64;
    let mut frontier = 0u64;
    let mut have_run = false;
    spill
        .table
        .visit_with_lookup(ScratchMapId::Default, &mut |lookup, key, _| {
            let claim = ScratchClaimKey::decode(key).ok_or_else(corrupt)?;
            let [token, start, end] = claim.words();
            if current_token != Some(token) {
                if have_run {
                    super::finalize::emit_pack_row(
                        finds,
                        &group_header,
                        run_start,
                        frontier,
                        answer_scratch,
                        emit,
                    )?;
                }
                if !lookup.get(ScratchMapId::TokenToGroup, &token.to_be_bytes(), &mut header)? {
                    return Err(corrupt());
                }
                decode_key(&header, &mut group_header)?;
                current_token = Some(token);
                run_start = start;
                frontier = end;
                have_run = true;
            } else if start > frontier {
                super::finalize::emit_pack_row(
                    finds,
                    &group_header,
                    run_start,
                    frontier,
                    answer_scratch,
                    emit,
                )?;
                run_start = start;
                frontier = end;
            } else {
                frontier = frontier.max(end);
            }
            Ok(true)
        })?;
    if have_run {
        super::finalize::emit_pack_row(
            finds,
            &group_header,
            run_start,
            frontier,
            answer_scratch,
            emit,
        )?;
    }
    Ok(())
}

fn finalize_pack_narrow(
    spill: &mut GroupSpill,
    group_words: usize,
    finds: &[crate::exec::sink::SinkSpec],
    answer_scratch: &mut Vec<u64>,
    emit: &mut impl FnMut(&[u64]) -> Result<()>,
) -> Result<()> {
    let group_bytes = group_words * 8;
    let mut group_header: Vec<u64> = Vec::new();
    let mut prev_prefix: Vec<u8> = Vec::new();
    let mut have_group = false;
    let mut run_start = 0u64;
    let mut frontier = 0u64;
    let mut have_run = false;
    spill.table.for_each(&mut |key, _| {
        let (start, end) = decode_narrow_claim(key, group_bytes)?;
        let same = have_group && key.len() >= group_bytes && prev_prefix == key[..group_bytes];
        if !same {
            if have_run {
                super::finalize::emit_pack_row(
                    finds,
                    &group_header,
                    run_start,
                    frontier,
                    answer_scratch,
                    emit,
                )?;
            }
            decode_key(&key[..group_bytes], &mut group_header)?;
            prev_prefix.clear();
            prev_prefix.extend_from_slice(&key[..group_bytes]);
            have_group = true;
            run_start = start;
            frontier = end;
            have_run = true;
        } else if start > frontier {
            super::finalize::emit_pack_row(
                finds,
                &group_header,
                run_start,
                frontier,
                answer_scratch,
                emit,
            )?;
            run_start = start;
            frontier = end;
        } else {
            frontier = frontier.max(end);
        }
        Ok(true)
    })?;
    if have_run {
        super::finalize::emit_pack_row(
            finds,
            &group_header,
            run_start,
            frontier,
            answer_scratch,
            emit,
        )?;
    }
    Ok(())
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
