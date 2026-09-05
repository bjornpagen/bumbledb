use crate::error::{Error, FindIndex, OverflowKind, Result};
use crate::exec::kernel::numeric::ExactF64Accumulator;
use crate::exec::sink::{Acc, AggregateSink, GroupState, SinkSpec, i64_to_word};
use crate::interval::sweep::{Continuation, sweep};

impl AggregateSink {
    /// # Errors
    pub fn finalize_into(
        &mut self,
        answer_scratch: &mut Vec<u64>,
        mut emit: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        // A sticky dedup/spill failure spoiled this execution: no group may
        // publish (the seen-set's misses would have double-counted, and a
        // half-flushed partition would double-fold).
        if let Some(error) = self.take_error() {
            return Err(error);
        }
        if self.cardinality_overflow {
            return Err(Error::Overflow(OverflowKind::Cardinality));
        }
        if self.group_state_spilled() {
            // Flush the residual RAM partition, then emit each merged
            // group exactly once from the scratch tier.
            self.spill_groups()?;
            if let Some(error) = self.take_error() {
                return Err(error);
            }
            if self.cardinality_overflow {
                return Err(Error::Overflow(OverflowKind::Cardinality));
            }
            return self.finalize_spilled(answer_scratch, &mut emit);
        }
        let live = self.group_count();
        if let GroupState::Pack { claims, .. } = &mut self.group_state {
            for claims in &mut claims[..live] {
                claims.sort_unstable_by_key(|&[start, _]| start);
            }
        }

        super::spill::for_each_ram_group(&self.groups, &mut |key, group_idx| {
            self.emit_group(key, group_idx, answer_scratch, &mut emit)
        })
    }

    fn emit_group(
        &self,
        key: &[u64],
        group_idx: usize,
        answer_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        match &self.group_state {
            GroupState::Pack { .. } => self.emit_pack_group(key, group_idx, answer_scratch, emit),
            GroupState::Folds { accs, n_aggs } => {
                let accs = &accs[group_idx * n_aggs..(group_idx + 1) * n_aggs];
                emit_fold_row(
                    &self.finds,
                    key,
                    accs,
                    &self.float_accs,
                    answer_scratch,
                    emit,
                )
            }
        }
    }

    fn emit_pack_group(
        &self,
        key: &[u64],
        group_idx: usize,
        answer_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        struct PackEmit<'a, F> {
            finds: &'a [SinkSpec],
            key: &'a [u64],
            answer_scratch: &'a mut Vec<u64>,
            emit: &'a mut F,
        }

        impl<F: FnMut(&[u64]) -> Result<()>> Continuation<u64, ()> for PackEmit<'_, F> {
            type Error = Error;

            fn segment(&mut self, (): ()) -> Result<()> {
                Ok(())
            }

            fn maximal(&mut self, start: u64, frontier: u64) -> Result<()> {
                emit_pack_row(
                    self.finds,
                    self.key,
                    start,
                    frontier,
                    self.answer_scratch,
                    self.emit,
                )
            }
        }

        let GroupState::Pack { claims, .. } = &self.group_state else {
            unreachable!("emit_pack_group is the Pack arm");
        };
        let claims = claims[group_idx]
            .iter()
            .map(|&[start, end]| Ok((start, end, ())));
        sweep(
            claims,
            None,
            &mut PackEmit {
                finds: &self.finds,
                key,
                answer_scratch,
                emit,
            },
        )
    }

    /// # Errors
    #[cfg(test)]
    pub fn into_answers(mut self) -> Result<Vec<Vec<u64>>> {
        let mut rows = Vec::with_capacity(self.groups.len());
        let mut scratch = Vec::new();
        self.finalize_into(&mut scratch, |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        Ok(rows)
    }
}

/// Assemble and emit one fold group's answer row: Var columns from the
/// group-key words, aggregate columns finalized from the given accumulator
/// bank. Shared by the resident arm (sink-global float bank) and the
/// spilled arm (decoded group-local bank) — one rounding, at exactly this
/// output boundary, in both regimes.
pub(in crate::exec::sink) fn emit_fold_row(
    finds: &[SinkSpec],
    key: &[u64],
    accs: &[Acc],
    floats: &[ExactF64Accumulator],
    answer_scratch: &mut Vec<u64>,
    emit: &mut impl FnMut(&[u64]) -> Result<()>,
) -> Result<()> {
    answer_scratch.clear();
    let mut key_cursor = 0;
    let mut acc_cursor = 0;
    for (find_idx, find) in finds.iter().enumerate() {
        match find {
            SinkSpec::Var { width, .. } => {
                answer_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                key_cursor += width;
            }
            SinkSpec::Agg(spec) => {
                let word = if let Acc::Float { index, .. } = accs[acc_cursor] {
                    let crate::exec::sink::AggSpec::Float { op, .. } = spec else {
                        unreachable!("float accumulator has float find")
                    };
                    let value = match op {
                        crate::ir::FoldOp::Sum => floats[index].sum(),
                        crate::ir::FoldOp::Mean => floats[index].mean(),
                        _ => unreachable!("exact float bank is Sum/Mean only"),
                    }
                    .expect("groups exist only after at least one binding");
                    value.to_order_key()
                } else {
                    finalize_acc(accs[acc_cursor], find_idx)?
                };
                answer_scratch.push(word);
                acc_cursor += 1;
            }
            SinkSpec::Pack { .. } => {
                unreachable!("validated: relation-shaped terms and folds never mix")
            }
        }
    }
    emit(answer_scratch)
}

/// Assemble and emit one Pack group's maximal segment — shared by the
/// resident sweep and the spilled streaming frontier walk.
pub(in crate::exec::sink) fn emit_pack_row(
    finds: &[SinkSpec],
    key: &[u64],
    start: u64,
    frontier: u64,
    answer_scratch: &mut Vec<u64>,
    emit: &mut impl FnMut(&[u64]) -> Result<()>,
) -> Result<()> {
    answer_scratch.clear();
    let mut key_cursor = 0;
    for find in finds {
        match find {
            SinkSpec::Var { width, .. } => {
                answer_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                key_cursor += width;
            }
            SinkSpec::Pack { .. } => {
                answer_scratch.push(start);
                answer_scratch.push(frontier);
            }
            SinkSpec::Agg(_) => {
                unreachable!("validated: Pack mixes with no other aggregate")
            }
        }
    }
    emit(answer_scratch)
}

fn finalize_acc(acc: Acc, find_idx: usize) -> Result<u64> {
    match acc {
        Acc::SumSigned(total) => i64::try_from(total).map(i64_to_word).map_err(|_| {
            Error::Overflow(OverflowKind::Aggregate {
                find: FindIndex(find_idx),
            })
        }),
        Acc::SumUnsigned(total) => u64::try_from(total).map_err(|_| {
            Error::Overflow(OverflowKind::Aggregate {
                find: FindIndex(find_idx),
            })
        }),
        Acc::Min(word) | Acc::Max(word) | Acc::Count(word) => Ok(word),
        Acc::Float { .. } => unreachable!("float results round from the separate bank"),
    }
}
