use crate::error::{Error, FindIndex, OverflowKind, Result};
use crate::exec::sink::{Acc, AggregateSink, GroupState, SinkSpec, i64_to_word};
use crate::interval::sweep::{Continuation, sweep};

impl AggregateSink {
    /// Finalizes each group into `emit` as **word rows** (find order,
    /// each find contributing its width — an interval find is two
    /// words), assembling rows in a caller-reused scratch. Sums are
    /// range-checked here, once — deterministic by construction (i128
    /// cannot overflow summing fewer than 2^64 i64 terms). Empty input
    /// yields zero rows: a global aggregate over nothing is the empty
    /// set, not a 0 or NULL row.
    ///
    /// Fold groups emit one row each; Pack groups emit **one row per
    /// maximal segment** of the group's claim union (relation-shaped —
    /// the claim lists sort here, hence `&mut self`).
    ///
    /// # Errors
    ///
    /// `Overflow` when a Sum's final value exceeds its result type; errors
    /// from `emit` propagate.
    pub fn finalize_into(
        &mut self,
        answer_scratch: &mut Vec<u64>,
        mut emit: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        // Pack's sort pass, ahead of the emit loop (which iterates the
        // group map immutably): each live group's claim list orders by
        // start word — the sweep's ONE precondition, so the comparator
        // is the start word alone (a full `[start, end]` lexicographic
        // compare paid a second word per decision for a tie order the
        // sweep cannot observe: overlapping and identical claims
        // collapse into the same maximal segment whatever their end
        // order). The sort is `sort_unstable` — the existing in-place
        // machinery, allocation-free, so the warm gate covers it; a
        // pooled radix over the start words stays unearned until a
        // bench shows this pass dominating a profile (t5_pack_key's
        // 35µs/44% warm-finalize share is the standing candidate).
        let live = self.group_count();
        if let GroupState::Pack { claims, .. } = &mut self.group_state {
            for claims in &mut claims[..live] {
                claims.sort_unstable_by_key(|&[start, _]| start);
            }
        }
        // The two group representations walk in mint order either way
        // (the map preserves insertion order; the dense ordinals ARE the
        // mint record) — the dense walk reconstructs each key's words
        // from its mixed-radix ordinal (finding 049).
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

    /// One group's emission by head shape — the finalize walk's body,
    /// shared by both group representations.
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
                        SinkSpec::Agg(_) => {
                            answer_scratch.push(Self::finalize_acc(accs[acc_cursor], find_idx)?);
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

    /// One Pack group's emission: the sweep's maximal-run continuation
    /// (`crate::interval::sweep` — the one segment walk, this is its
    /// second caller) over the group's start-sorted claims, one head answer
    /// per maximal segment — group key interleaved per find order, the
    /// segment's two words at the Pack position. Adjacency merges,
    /// identical claims collapse, and a ray (`end == MAX`) is the
    /// frontier no later claim exceeds, so a packed ray is a ray — all
    /// three are the sweep's laws, not cases here.
    fn emit_pack_group(
        &self,
        key: &[u64],
        group_idx: usize,
        answer_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        /// The emit continuation: consumed segments need nothing; a
        /// maximal run is one answer.
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

    /// Range-checks and word-encodes one accumulator.
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
        }
    }

    /// Convenience finalization into fresh vectors (tests).
    ///
    /// # Errors
    ///
    /// As [`Self::finalize_into`].
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
