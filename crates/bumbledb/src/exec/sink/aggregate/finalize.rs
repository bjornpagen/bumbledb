use crate::error::{Error, OverflowKind, Result};
use crate::exec::sink::{Acc, AggregateSink, SinkSpec, i64_to_word};
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
    /// Fold groups emit one row each; Arg-restriction groups emit
    /// **every stored row** — the rows projected from the bindings
    /// attaining the key's extreme, ties included (set-honest,
    /// 20-query-ir § aggregation); Pack groups emit **one row per
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
        // start word — the sweep's precondition. The sort is
        // `sort_unstable` — the existing in-place machinery, allocation-
        // free, so the warm gate covers it; a pooled radix over the start
        // words stays unearned until PRD 16's bench shows this pass on a
        // profile (the measured-choice record).
        if self.pack.is_some() {
            let live = self.group_count();
            for claims in &mut self.pack_claims[..live] {
                claims.sort_unstable();
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
        if self.pack.is_some() {
            return self.emit_pack_group(key, group_idx, answer_scratch, emit);
        }
        if self.arg.is_some() {
            return self.emit_arg_group(key, group_idx, answer_scratch, emit);
        }
        let accs = &self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
        answer_scratch.clear();
        let mut key_cursor = 0;
        let mut acc_cursor = 0;
        for (find_idx, find) in self.finds.iter().enumerate() {
            match find {
                SinkSpec::Var { width, .. } => {
                    answer_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                    key_cursor += width;
                }
                SinkSpec::Agg { .. } => {
                    answer_scratch.push(self.finalize_acc(accs[acc_cursor], find_idx)?);
                    acc_cursor += 1;
                }
                SinkSpec::Arg { .. } | SinkSpec::Pack { .. } => {
                    unreachable!("validated: relation-shaped terms and folds never mix")
                }
            }
        }
        emit(answer_scratch)
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
                        SinkSpec::Agg { .. } | SinkSpec::Arg { .. } => {
                            unreachable!("validated: Pack mixes with no other aggregate")
                        }
                    }
                }
                (self.emit)(self.answer_scratch)
            }
        }

        let claims = self.pack_claims[group_idx]
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

    /// One Arg group's emission: every row of the restricted set,
    /// interleaved with the group key per find order (restrict-then-
    /// project — the stored row was projected whole from one surviving
    /// binding, so multi-carry coherence needs no per-term bookkeeping).
    fn emit_arg_group(
        &self,
        key: &[u64],
        group_idx: usize,
        answer_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        for (carry_row, ()) in self.arg_answers[group_idx].iter() {
            answer_scratch.clear();
            let mut key_cursor = 0;
            let mut carry_cursor = 0;
            for find in &self.finds {
                match find {
                    SinkSpec::Var { width, .. } => {
                        answer_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                        key_cursor += width;
                    }
                    SinkSpec::Arg { width, .. } => {
                        answer_scratch
                            .extend_from_slice(&carry_row[carry_cursor..carry_cursor + width]);
                        carry_cursor += width;
                    }
                    SinkSpec::Agg { .. } | SinkSpec::Pack { .. } => {
                        unreachable!("validated: Arg terms mix with no other aggregate")
                    }
                }
            }
            emit(answer_scratch)?;
        }
        Ok(())
    }

    /// Range-checks and word-encodes one accumulator.
    fn finalize_acc(&self, acc: Acc, find_idx: usize) -> Result<u64> {
        match acc {
            Acc::SumSigned(total) => i64::try_from(total)
                .map(i64_to_word)
                .map_err(|_| Error::Overflow(OverflowKind::Aggregate { find: find_idx })),
            Acc::SumUnsigned(total) => u64::try_from(total)
                .map_err(|_| Error::Overflow(OverflowKind::Aggregate { find: find_idx })),
            Acc::Min(word) | Acc::Max(word) | Acc::Count(word) => Ok(word),
            // |distinct values of the group| — the value set's size.
            Acc::CountDistinct(set) => Ok(self.value_sets[set].len() as u64),
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
