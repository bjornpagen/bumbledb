use crate::error::{Error, Result};
use crate::exec::sink::{i64_to_word, Acc, AggregateSink, FindSpec};

impl AggregateSink {
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
