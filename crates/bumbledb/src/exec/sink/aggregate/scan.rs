//! Shared column reductions for a fused leaf scan. Output aliases select
//! from one partial per input/kernel, so Min and Max of the same column
//! share its gather and comparisons. The persisted group state is unchanged.

use crate::exec::colt::SuffixRun;
use crate::exec::kernel;
use crate::exec::sink::{Acc, AggSpec, FoldOp};

#[derive(Debug)]
pub(in crate::exec::sink) struct ScanInput {
    pub word: usize,
    pub partial: ScanPartial,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::exec::sink) enum ScanPartial {
    Sum(u128),
    Extrema { min: u64, max: u64 },
}

impl ScanPartial {
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

    pub fn fold(&mut self, column: &[u64], run: SuffixRun<'_>) {
        match self {
            Self::Sum(total) => {
                *total += match run {
                    SuffixRun::Identity { start, len } => {
                        kernel::fold_sum_u64(column, 1, start, len)
                    }
                    SuffixRun::Positions(p) => kernel::fold_sum_u64_idx(column, 1, 0, p),
                };
            }
            Self::Extrema { min, max } => {
                let (lo, hi) = match run {
                    SuffixRun::Identity { start, len } => {
                        kernel::fold_min_max_u64(column, 1, start, len)
                    }
                    SuffixRun::Positions(p) => kernel::fold_min_max_u64_idx(column, 1, 0, p),
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

#[cfg(test)]
mod tests {
    use super::*;

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
                let partial = ScanPartial::Sum(u128::from(raw) * u128::from(count));
                let Acc::SumSigned(actual) = partial.output(spec, count) else {
                    panic!("signed output")
                };
                assert_eq!(actual, i128::from(value) * i128::from(count));
            }
        }
    }
}
