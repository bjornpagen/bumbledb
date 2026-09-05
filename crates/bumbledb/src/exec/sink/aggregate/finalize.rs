use crate::error::{Error, FindIndex, OverflowKind, Result};
use crate::exec::sink::{Acc, AggregateSink, GroupState, SinkSpec, i64_to_word};
use crate::interval::sweep::{Continuation, sweep};

impl AggregateSink {
    /// # Errors
    pub fn finalize_into(
        &mut self,
        answer_scratch: &mut Vec<u64>,
        mut emit: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        if self.cardinality_overflow {
            return Err(Error::Overflow(OverflowKind::Cardinality));
        }
        let live = self.group_count();
        if let GroupState::Pack { claims, .. } = &mut self.group_state {
            for claims in &mut claims[..live] {
                claims.sort_unstable_by_key(|&[start, _]| start);
            }
        }

        match &self.groups {
            crate::exec::sink::GroupTable::Hashed(map) => {
                for (key, group_idx) in map.iter() {
                    self.emit_group(key, *group_idx, answer_scratch, &mut emit)?;
                }
            }
            crate::exec::sink::GroupTable::Dense {
                radixes, ordinals, ..
            } => {
                let mut key = vec![0u64; radixes.len()];
                for (group_idx, ordinal) in ordinals.iter().enumerate() {
                    let mut rest = usize::try_from(*ordinal).expect("capped product");
                    for (word, radix) in key.iter_mut().zip(radixes.iter()).rev() {
                        *word = (rest % usize::from(*radix)) as u64;
                        rest /= usize::from(*radix);
                    }
                    self.emit_group(&key, group_idx, answer_scratch, &mut emit)?;
                }
            }
        }
        Ok(())
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
                answer_scratch.clear();
                let mut key_cursor = 0;
                let mut acc_cursor = 0;
                for (find_idx, find) in self.finds.iter().enumerate() {
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
                                    crate::ir::FoldOp::Sum => self.float_accs[index].sum(),
                                    crate::ir::FoldOp::Mean => self.float_accs[index].mean(),
                                    _ => unreachable!("exact float bank is Sum/Mean only"),
                                }.expect("groups exist only after at least one binding");
                                value.to_order_key()
                            } else {
                                Self::finalize_acc(accs[acc_cursor], find_idx)?
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
                self.answer_scratch.clear();
                let mut key_cursor = 0;
                for find in self.finds {
                    match find {
                        SinkSpec::Var { width, .. } => {
                            self.answer_scratch
                                .extend_from_slice(&self.key[key_cursor..key_cursor + width]);
                            key_cursor += width;
                        }
                        SinkSpec::Pack { .. } => {
                            self.answer_scratch.push(start);
                            self.answer_scratch.push(frontier);
                        }
                        SinkSpec::Agg(_) => {
                            unreachable!("validated: Pack mixes with no other aggregate")
                        }
                    }
                }
                (self.emit)(self.answer_scratch)
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
