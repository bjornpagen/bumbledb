use crate::error::{Error, Result};
use crate::exec::sink::{i64_to_word, Acc, AggregateSink, FindSpec};

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
    /// 20-query-ir § aggregation).
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
            if self.arg.is_some() {
                self.emit_arg_group(key, *group_idx, row_scratch, &mut emit)?;
                continue;
            }
            let accs = &self.accs[group_idx * self.n_aggs..(group_idx + 1) * self.n_aggs];
            row_scratch.clear();
            let mut key_cursor = 0;
            let mut acc_cursor = 0;
            for (find_idx, find) in self.finds.iter().enumerate() {
                match find {
                    FindSpec::Var { width, .. } => {
                        row_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                        key_cursor += width;
                    }
                    FindSpec::Agg { .. } => {
                        row_scratch.push(self.finalize_acc(accs[acc_cursor], find_idx)?);
                        acc_cursor += 1;
                    }
                    FindSpec::Arg { .. } => {
                        unreachable!("validated: Arg terms and folds never mix")
                    }
                }
            }
            emit(row_scratch)?;
        }
        Ok(())
    }

    /// One Arg group's emission: every row of the restricted set,
    /// interleaved with the group key per find order (restrict-then-
    /// project — the stored row was projected whole from one surviving
    /// binding, so multi-carry coherence needs no per-term bookkeeping).
    fn emit_arg_group(
        &self,
        key: &[u64],
        group_idx: usize,
        row_scratch: &mut Vec<u64>,
        emit: &mut impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        for (carry_row, ()) in self.arg_rows[group_idx].iter() {
            row_scratch.clear();
            let mut key_cursor = 0;
            let mut carry_cursor = 0;
            for find in &self.finds {
                match find {
                    FindSpec::Var { width, .. } => {
                        row_scratch.extend_from_slice(&key[key_cursor..key_cursor + width]);
                        key_cursor += width;
                    }
                    FindSpec::Arg { width, .. } => {
                        row_scratch
                            .extend_from_slice(&carry_row[carry_cursor..carry_cursor + width]);
                        carry_cursor += width;
                    }
                    FindSpec::Agg { .. } => {
                        unreachable!("validated: Arg terms and folds never mix")
                    }
                }
            }
            emit(row_scratch)?;
        }
        Ok(())
    }

    /// Range-checks and word-encodes one accumulator.
    fn finalize_acc(&self, acc: Acc, find_idx: usize) -> Result<u64> {
        match acc {
            Acc::SumSigned(total) => i64::try_from(total)
                .map(i64_to_word)
                .map_err(|_| Error::Overflow { find: find_idx }),
            Acc::SumUnsigned(total) => {
                u64::try_from(total).map_err(|_| Error::Overflow { find: find_idx })
            }
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
