//! Column reductions shared by leaf scans and constant-group batches: one
//! partial per input column and kernel, so Min and Max of a column share
//! its comparisons and Sum and Mean of an F64 column share one exact total.

use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::kernel::numeric::ExactF64Accumulator;
use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, FoldSource, GroupState, SinkSpec};
use bumbledb_theory::F64;

#[derive(Debug)]
pub(in crate::exec::sink) struct FoldInput {
    pub word: usize,
    pub partial: Partial,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "one partial per input column, reused across runs without allocating"
)]
pub(in crate::exec::sink) enum Partial {
    Sum(u128),
    Extrema { min: u64, max: u64 },
    Float(ExactF64Accumulator),
}

impl Partial {
    pub(crate) fn seed(spec: AggSpec) -> Self {
        match spec {
            AggSpec::Fold {
                op: FoldOp::Sum, ..
            } => Self::Sum(0),
            AggSpec::Fold {
                op: FoldOp::Min | FoldOp::Max,
                ..
            }
            | AggSpec::Float {
                op: FoldOp::Min | FoldOp::Max,
                ..
            } => Self::Extrema {
                min: u64::MAX,
                max: u64::MIN,
            },
            AggSpec::Float { .. } => Self::Float(ExactF64Accumulator::default()),
            _ => unreachable!("only sums, extrema and exact float totals reduce columns"),
        }
    }

    pub(crate) fn same_kernel(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub(crate) fn reset(&mut self) {
        *self = match self {
            Self::Sum(_) => Self::Sum(0),
            Self::Extrema { .. } => Self::Extrema {
                min: u64::MAX,
                max: u64::MIN,
            },
            Self::Float(_) => Self::Float(ExactF64Accumulator::default()),
        };
    }

    /// Folds the run's words of `column` (`stride` words per row, input at
    /// `word`). Callers bound the run by a checked binding count, so a float
    /// total cannot overflow its cardinality.
    pub(crate) fn fold(&mut self, column: &[u64], stride: usize, word: usize, run: SuffixRun<'_>) {
        match self {
            Self::Sum(total) => {
                *total += match run {
                    SuffixRun::Identity { start, len } => {
                        kernel::fold_sum_u64(column, stride, start * stride + word, len)
                    }
                    SuffixRun::Positions(p) => kernel::fold_sum_u64_idx(column, stride, word, p),
                };
            }
            Self::Extrema { min, max } => {
                let (lo, hi) = match run {
                    SuffixRun::Identity { start, len } => {
                        kernel::fold_min_max_u64(column, stride, start * stride + word, len)
                    }
                    SuffixRun::Positions(p) => {
                        kernel::fold_min_max_u64_idx(column, stride, word, p)
                    }
                };
                *min = (*min).min(lo);
                *max = (*max).max(hi);
            }
            Self::Float(total) => {
                if run.is_empty() {
                    return;
                }
                let pushed = match run {
                    SuffixRun::Identity { start, len } => total.push_keys(
                        column[start * stride + word..]
                            .iter()
                            .step_by(stride)
                            .take(len)
                            .copied(),
                    ),
                    SuffixRun::Positions(p) => {
                        total.push_keys(p.iter().map(|&i| column[i as usize * stride + word]))
                    }
                };
                pushed.expect("a checked binding count bounds every run");
            }
        }
    }

    fn repeated(&self, word: u64, count: u64) -> Self {
        match self {
            Self::Sum(_) => Self::Sum(u128::from(word) * u128::from(count)),
            Self::Extrema { .. } => Self::Extrema {
                min: word,
                max: word,
            },
            Self::Float(_) => unreachable!("outer float constants push into the group bank"),
        }
    }

    fn output(&self, spec: AggSpec, count: u64) -> Acc {
        match (self, spec) {
            (
                Self::Sum(total),
                AggSpec::Fold {
                    op: FoldOp::Sum,
                    signed: true,
                    ..
                },
            ) => {
                // Signed words are biased by 2^63. With at most u64::MAX
                // inputs, the raw total fits u128 and the decoded sum fits
                // i128. Subtract modulo 2^128 before interpreting the sign:
                // neither operand must individually fit i128.
                Acc::SumSigned(total.wrapping_sub(u128::from(count) << 63).cast_signed())
            }
            (
                Self::Sum(total),
                AggSpec::Fold {
                    op: FoldOp::Sum,
                    signed: false,
                    ..
                },
            ) => Acc::SumUnsigned(*total),
            (
                Self::Extrema { min, .. },
                AggSpec::Fold {
                    op: FoldOp::Min, ..
                },
            ) => Acc::Min(*min),
            (
                Self::Extrema { max, .. },
                AggSpec::Fold {
                    op: FoldOp::Max, ..
                }
                | AggSpec::Float {
                    op: FoldOp::Max, ..
                },
            ) => Acc::Max(*max),
            (
                Self::Extrema { min, max },
                AggSpec::Float {
                    op: FoldOp::Min, ..
                },
            ) => Acc::Min(if *max == super::NAN_KEY { 0 } else { *min }),
            _ => unreachable!("output operator matches its compiled scan kernel"),
        }
    }
}

impl AggregateSink {
    pub(super) fn prepare_fold_inputs(&mut self) {
        self.fold_sources.clear();
        self.fold_inputs.clear();
        for find in &self.finds {
            let SinkSpec::Agg(spec @ (AggSpec::Fold { slot, .. } | AggSpec::Float { slot, .. })) =
                find
            else {
                continue;
            };
            let source = match self.cached_leaf_words[*slot] {
                Some(word) => {
                    let word = word.get() as usize - 1;
                    let partial = Partial::seed(*spec);
                    let input = self
                        .fold_inputs
                        .iter()
                        .position(|input| input.word == word && input.partial.same_kernel(&partial))
                        .unwrap_or_else(|| {
                            let index = self.fold_inputs.len();
                            self.fold_inputs.push(FoldInput { word, partial });
                            index
                        });
                    FoldSource::Column(input)
                }
                None => FoldSource::Outer,
            };
            self.fold_sources.push(source);
        }
    }

    /// Merges this run's partials into group `group_idx`'s accumulators;
    /// outer-sourced inputs contribute `outer(slot)` `count` times. Records
    /// a cardinality overflow instead of merging past it.
    pub(super) fn merge_partials(
        &mut self,
        group_idx: usize,
        count: u64,
        outer: impl Fn(usize) -> u64,
    ) {
        let GroupState::Folds { accs, n_aggs } = &mut self.group_state else {
            unreachable!("partials merge into fold groups");
        };
        let mut accumulators = accs[group_idx * *n_aggs..(group_idx + 1) * *n_aggs].iter_mut();
        let mut sources = self.fold_sources.iter();
        for find in &self.finds {
            let SinkSpec::Agg(spec) = find else {
                continue;
            };
            let acc = accumulators.next().expect("one accumulator per aggregate");
            match spec {
                AggSpec::Count => {
                    let Acc::Count(n) = acc else {
                        unreachable!("accumulators are seeded per op");
                    };
                    *n = n.saturating_add(count);
                }
                AggSpec::Fold { slot, .. }
                | AggSpec::Float {
                    op: FoldOp::Min | FoldOp::Max,
                    slot,
                } => {
                    let source = sources.next().expect("one source per fold");
                    let output = match source {
                        FoldSource::Column(input) => {
                            self.fold_inputs[*input].partial.output(*spec, count)
                        }
                        FoldSource::Outer => Partial::seed(*spec)
                            .repeated(outer(*slot), count)
                            .output(*spec, count),
                    };
                    merge(acc, output);
                }
                AggSpec::Float { slot, .. } => {
                    let source = sources.next().expect("one source per fold");
                    let Acc::Float { index, primary } = acc else {
                        unreachable!("float accumulator handle")
                    };
                    if !*primary {
                        continue;
                    }
                    let bank = &mut self.float_accs[*index];
                    let merged = match source {
                        FoldSource::Column(input) => {
                            let Partial::Float(partial) = &self.fold_inputs[*input].partial else {
                                unreachable!("float inputs reduce exact totals")
                            };
                            bank.merge(partial)
                        }
                        FoldSource::Outer => bank.push_repeated(
                            F64::from_order_key(outer(*slot))
                                .expect("validated canonical F64 binding"),
                            count,
                        ),
                    };
                    if merged.is_err() {
                        self.cardinality_overflow = true;
                        return;
                    }
                }
            }
        }
    }
}

pub(super) fn merge(acc: &mut Acc, partial: Acc) {
    match (acc, partial) {
        (Acc::SumSigned(t), Acc::SumSigned(p)) => *t += p,
        (Acc::SumUnsigned(t), Acc::SumUnsigned(p)) => *t += p,
        (Acc::Min(t), Acc::Min(p)) => *t = (*t).min(p),
        (Acc::Max(t), Acc::Max(p)) => *t = (*t).max(p),
        (Acc::Count(t), Acc::Count(p)) => *t = t.saturating_add(p),
        _ => unreachable!("partials are seeded from the same finds"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_reductions_match_scalar_folds_across_strides_and_selections() {
        let spec = AggSpec::Fold {
            op: FoldOp::Sum,
            slot: 0,
            width: 1,
            signed: true,
        };
        let extremes = [i64::MIN, i64::MAX, -1, 0, 1, -999, 999];
        for len in [0, 1, 2, 3, 4, 7, 8, 9, 127, 128, 129, 257] {
            for stride in [1, 3, 5] {
                let words: Vec<_> = (0..len * stride)
                    .map(|i| crate::exec::sink::i64_to_word(extremes[i % extremes.len()]))
                    .collect();
                let mut positions: Vec<_> = (0..u32::try_from(len).unwrap()).rev().collect();
                if len > 2 {
                    positions.extend([1, 1]);
                }
                let runs = [
                    SuffixRun::Identity { start: 0, len },
                    SuffixRun::Positions(&positions),
                ];
                for run in runs {
                    let mut partial = Partial::seed(spec);
                    partial.fold(&words, stride, stride - 1, run);
                    let indices: Vec<usize> = match run {
                        SuffixRun::Identity { start, len } => (start..start + len).collect(),
                        SuffixRun::Positions(p) => p.iter().map(|&i| i as usize).collect(),
                    };
                    let expected: i128 = indices
                        .iter()
                        .map(|i| {
                            i128::from(crate::exec::sink::word_to_i64(
                                words[i * stride + stride - 1],
                            ))
                        })
                        .sum();
                    let Acc::SumSigned(actual) = partial.output(spec, run.len() as u64) else {
                        panic!("signed output")
                    };
                    assert_eq!(actual, expected, "len={len}, stride={stride}");
                }
            }
        }
    }

    #[test]
    fn signed_sum_decodes_after_accumulation_without_an_i128_intermediate_limit() {
        let spec = AggSpec::Fold {
            op: FoldOp::Sum,
            slot: 0,
            width: 1,
            signed: true,
        };
        for count in [1, u64::from(u32::MAX), u64::MAX] {
            for value in [i64::MIN, -1, 0, 1, i64::MAX] {
                let raw = u64::from_be_bytes(crate::encoding::encode_i64(value));
                let partial = Partial::Sum(u128::from(raw) * u128::from(count));
                let Acc::SumSigned(actual) = partial.output(spec, count) else {
                    panic!("signed output")
                };
                assert_eq!(actual, i128::from(value) * i128::from(count));
            }
        }
    }

    #[test]
    fn float_partials_equal_pushing_each_selected_value() {
        let spec = AggSpec::Float {
            op: FoldOp::Sum,
            slot: 0,
        };
        let values = [
            1e16,
            1.0,
            -1e16,
            0.5,
            f64::MAX,
            -f64::MAX,
            3.25,
            -0.0,
            5e-324,
        ];
        let words: Vec<u64> = values
            .iter()
            .flat_map(|&v| [7, F64::from(v).to_order_key()])
            .collect();
        let positions: Vec<u32> = vec![8, 0, 3, 3, 5, 1];
        for run in [
            SuffixRun::Identity {
                start: 1,
                len: values.len() - 1,
            },
            SuffixRun::Positions(&positions),
        ] {
            let mut partial = Partial::seed(spec);
            partial.fold(&words, 2, 1, run);
            let indices: Vec<usize> = match run {
                SuffixRun::Identity { start, len } => (start..start + len).collect(),
                SuffixRun::Positions(p) => p.iter().map(|&i| i as usize).collect(),
            };
            let mut expected = ExactF64Accumulator::default();
            for i in indices {
                expected.push(F64::from(values[i])).unwrap();
            }
            let Partial::Float(total) = partial else {
                panic!("float partial")
            };
            assert_eq!(total, expected);
        }
    }
}
