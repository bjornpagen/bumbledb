use super::output::Budget;
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, bytes, encoded_root, limits, name,
    ordering, polynomial, rational, root,
};
use bumbledb::event::AlgebraicRoot;

#[derive(Clone, Copy)]
pub(super) enum Op {
    Isolate,
    FromInterval,
    Validate,
    Describe,
    Compare,
    CompareRational,
    RationalBetween,
    Sign,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "isolate" => Self::Isolate,
            "fromInterval" => Self::FromInterval,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "compare" => Self::Compare,
            "compareRational" => Self::CompareRational,
            "rationalBetween" => Self::RationalBetween,
            "sign" => Self::Sign,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        count
            == match self {
                Self::Validate | Self::Describe => 1,
                Self::FromInterval => 4,
                _ => 2,
            }
            && argument == 0
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters.parameters;
    match op {
        Op::Isolate => {
            let parameter = name(&inputs[0])?;
            let polynomial = polynomial(&inputs[1], work)?;
            let roots = polynomial.isolate_roots(parameter, limits.region.roots, work)?;
            let mut budget = Budget::default();
            let mut values = Vec::new();
            values.try_reserve_exact(roots.len())?;
            for root in roots {
                values.push(budget.blob(root.to_bytes(limits.algebraic, work)?)?);
            }
            Ok(Output::Parameter(ParameterOutput::Roots(values)))
        }
        Op::FromInterval => {
            let parameter = name(&inputs[0])?;
            let polynomial = polynomial(&inputs[1], work)?;
            let lower = rational(&inputs[2], work)?;
            let upper = rational(&inputs[3], work)?;
            let value = AlgebraicRoot::from_interval(
                &polynomial,
                parameter,
                lower,
                upper,
                limits.region.roots,
                work,
            )?;
            encoded_root(&value, work)
        }
        _ => {
            let value = root(&inputs[0], work)?;
            match op {
                Op::Validate => encoded_root(&value, work),
                Op::Describe => {
                    let mut budget = Budget::default();
                    let polynomial = budget.blob(
                        value
                            .polynomial()
                            .to_bytes(limits.region.polynomial, work)?,
                    )?;
                    let (lower, upper) = value.interval();
                    Ok(Output::Parameter(ParameterOutput::Root {
                        parameter: value.parameter().0,
                        polynomial,
                        lower: budget.blob(lower.to_bytes(work)?)?,
                        upper: budget.blob(upper.to_bytes(work)?)?,
                    }))
                }
                Op::Compare => Ok(ordering(value.compare(
                    &root(&inputs[1], work)?,
                    limits.region.roots,
                    work,
                )?)),
                Op::CompareRational => Ok(ordering(value.compare_rational(
                    &rational(&inputs[1], work)?,
                    limits.region.roots,
                    work,
                )?)),
                Op::Sign => Ok(ordering(value.sign(
                    &polynomial(&inputs[1], work)?,
                    limits.region.roots,
                    work,
                )?)),
                Op::RationalBetween => {
                    let result = value.rational_between(
                        &root(&inputs[1], work)?,
                        limits.region.roots,
                        work,
                    )?;
                    bytes(result.to_bytes(work)?)
                }
                _ => Err(Error::InvalidEncoding),
            }
        }
    }
}
