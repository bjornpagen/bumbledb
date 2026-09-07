//! Shared column reductions for leaf scans and batches. Output aliases select
//! from one partial per input/kernel, so Min and Max of the same column
//! share its gather and comparisons. The persisted group state is unchanged.

use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::sink::{Acc, AggSpec, AggregateSink, FoldOp, FoldSource, SinkSpec};

#[derive(Debug)]
pub(in crate::exec::sink) struct FoldInput {
    pub word: usize,
    pub partial: Partial,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::exec::sink) enum Partial {
    Sum(u128),
    Extrema { min: u64, max: u64 },
}

impl Partial {
    pub fn seed(spec: AggSpec) -> Self {
        match spec {
            AggSpec::Fold {
                op: FoldOp::Sum, ..
            } => Self::Sum(0),
            AggSpec::Fold {
                op: FoldOp::Min | FoldOp::Max,
                ..
            } => Self::Extrema {
                min: u64::MAX,
                max: u64::MIN,
            },
            _ => unreachable!("only integer sums and word-order extrema use column scans"),
        }
    }

    pub fn same_kernel(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    pub fn reset(&mut self) {
        *self = match self {
            Self::Sum(_) => Self::Sum(0),
            Self::Extrema { .. } => Self::Extrema {
                min: u64::MAX,
                max: u64::MIN,
            },
        };
    }

    pub fn repeated(self, word: u64, count: u64) -> Self {
        match self {
            Self::Sum(_) => Self::Sum(u128::from(word) * u128::from(count)),
            Self::Extrema { .. } => Self::Extrema {
                min: word,
                max: word,
            },
        }
    }

    pub fn fold(&mut self, column: &[u64], stride: usize, word: usize, run: SuffixRun<'_>) {
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
        }
    }

    pub fn output(self, spec: AggSpec, count: u64) -> Acc {
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
            ) => Acc::SumUnsigned(total),
            (
                Self::Extrema { min, .. },
                AggSpec::Fold {
                    op: FoldOp::Min, ..
                },
            ) => Acc::Min(min),
            (
                Self::Extrema { max, .. },
                AggSpec::Fold {
                    op: FoldOp::Max, ..
                },
            ) => Acc::Max(max),
            _ => unreachable!("output operator matches its compiled scan kernel"),
        }
    }
}

impl AggregateSink {
    pub(super) fn prepare_fold_inputs(&mut self, key_slots: &[usize]) {
        self.fold_sources.clear();
        self.fold_inputs.clear();
        for find in &self.finds {
            let SinkSpec::Agg(spec @ AggSpec::Fold { slot, .. }) = find else {
                continue;
            };
            let source = match key_slots.iter().position(|k| k == slot) {
                Some(word) => {
                    let partial = Partial::seed(*spec);
                    let input = self
                        .fold_inputs
                        .iter()
                        .position(|input| input.word == word && input.partial.same_kernel(partial))
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
}
