use super::output::Witness;
use super::{
    ExactArithmetic, Output, ParameterOutput, ParameterRegion, Result, boolean, domain,
    encoded_region, limits, name, polynomial, rational, region, root, signs,
};
use bumbledb::event::{BoolOp4, Error, ParameterCell, RealWitness};

#[derive(Clone, Copy)]
pub(super) enum Op {
    Full,
    Empty,
    Sign,
    Validate,
    Domain,
    Apply,
    Complement,
    Equivalent,
    Included,
    IsEmpty,
    IsFull,
    ContainsRational,
    ContainsRoot,
    Witness,
    Describe,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "full" => Self::Full,
            "empty" => Self::Empty,
            "sign" => Self::Sign,
            "validate" => Self::Validate,
            "domain" => Self::Domain,
            "apply" => Self::Apply,
            "complement" => Self::Complement,
            "equivalent" => Self::Equivalent,
            "included" => Self::Included,
            "isEmpty" => Self::IsEmpty,
            "isFull" => Self::IsFull,
            "containsRational" => Self::ContainsRational,
            "containsRoot" => Self::ContainsRoot,
            "witness" => Self::Witness,
            "describe" => Self::Describe,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        let arity = match self {
            Self::Sign
            | Self::Apply
            | Self::Equivalent
            | Self::Included
            | Self::ContainsRational
            | Self::ContainsRoot => 2,
            _ => 1,
        };
        count == arity
            && match self {
                Self::Sign => argument < 8,
                Self::Apply => argument < 16,
                _ => argument == 0,
            }
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters.parameters;
    let value = match op {
        Op::Full => ParameterRegion::full(name(&inputs[0])?),
        Op::Empty => ParameterRegion::empty(name(&inputs[0])?),
        Op::Sign => ParameterRegion::from_polynomial(
            name(&inputs[0])?,
            &polynomial(&inputs[1], work)?,
            signs(argument)?,
            limits.region,
            work,
        )?,
        Op::Domain => domain(&inputs[0], work)?.region().clone(),
        _ => {
            let value = region(&inputs[0], work)?;
            match op {
                Op::Validate => value,
                Op::Complement => value.complement(),
                Op::Apply => value.apply(
                    BoolOp4::new(argument).ok_or(Error::InvalidEncoding)?,
                    &region(&inputs[1], work)?,
                    limits.region,
                    work,
                )?,
                Op::Equivalent => {
                    return Ok(boolean(value.equivalent(
                        &region(&inputs[1], work)?,
                        limits.region,
                        work,
                    )?));
                }
                Op::Included => {
                    return Ok(boolean(value.included(
                        &region(&inputs[1], work)?,
                        limits.region,
                        work,
                    )?));
                }
                Op::IsEmpty => return Ok(boolean(value.is_empty())),
                Op::IsFull => return Ok(boolean(value.is_full())),
                Op::ContainsRational | Op::ContainsRoot => {
                    let point = if matches!(op, Op::ContainsRational) {
                        RealWitness::Rational(rational(&inputs[1], work)?)
                    } else {
                        RealWitness::Algebraic(root(&inputs[1], work)?)
                    };
                    return Ok(boolean(value.contains(&point, limits.region, work)?));
                }
                Op::Witness => {
                    let witness = match value.witness(limits.region, work)? {
                        None => None,
                        Some(RealWitness::Rational(value)) => {
                            Some(Witness::Rational(value.to_bytes(work)?))
                        }
                        Some(RealWitness::Algebraic(value)) => {
                            Some(Witness::Algebraic(value.to_bytes(limits.algebraic, work)?))
                        }
                    };
                    return Ok(Output::Parameter(ParameterOutput::Witness(witness)));
                }
                Op::Describe => return describe(&value, work),
                _ => return Err(Error::InvalidEncoding),
            }
        }
    };
    encoded_region(&value, work)
}
fn describe(value: &ParameterRegion, work: &mut ExactArithmetic<'_>) -> Result<Output> {
    let limits = limits().parameters.parameters;
    value.to_bytes(limits, work)?;
    let mut boundaries = Vec::new();
    let mut membership = Vec::new();
    boundaries.try_reserve_exact(value.cells().len() / 2)?;
    membership.try_reserve_exact(value.cells().len())?;
    for (cell, inside) in value.cells() {
        membership.push(inside);
        if let ParameterCell::Point(root) = cell {
            boundaries.push(root.to_bytes(limits.algebraic, work)?);
        }
    }
    Ok(Output::Parameter(ParameterOutput::Region {
        parameter: value.parameter().0,
        boundaries,
        membership,
    }))
}
