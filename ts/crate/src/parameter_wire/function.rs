use super::output::FunctionPiece;
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, boolean, bytes, domain,
    encoded_region, limits, polynomial, rational, region, signs,
};
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, GuardedRationalFunction, ParameterFunction,
    SourceDescriptor,
};

#[derive(Clone, Copy)]
pub(super) enum Op {
    Ratio,
    Pieces,
    Validate,
    Describe,
    Domain,
    Defined,
    At,
    Sign,
    IsNowhere,
    Restrict,
    OnDomain,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equivalent,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "ratio" => Self::Ratio,
            "pieces" => Self::Pieces,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "domain" => Self::Domain,
            "defined" => Self::Defined,
            "at" => Self::At,
            "sign" => Self::Sign,
            "isNowhere" => Self::IsNowhere,
            "restrict" => Self::Restrict,
            "onDomain" => Self::OnDomain,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "divide" => Self::Divide,
            "equivalent" => Self::Equivalent,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        let arity = match self {
            Self::Pieces => count > 0 && (count - 1).is_multiple_of(3),
            Self::Ratio => count == 3,
            Self::At
            | Self::Restrict
            | Self::OnDomain
            | Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Equivalent => count == 2,
            _ => count == 1,
        };
        arity
            && if matches!(self, Self::Sign) {
                argument < 8
            } else {
                argument == 0
            }
    }
}
pub(super) fn function(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<ParameterFunction> {
    if let AdmittedSourceDescriptor::Family(value) =
        SourceDescriptor::import(bytes, limits(), work)?
        && let AdmittedFamilyDescriptor::Parameter(value) = *value
    {
        return Ok(value);
    }
    Err(Error::RoleMismatch)
}
pub(super) fn encoded(
    value: ParameterFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    let data = SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Parameter(value))),
        limits(),
        work,
    )?;
    data.to_bytes(limits(), control)
}
pub(super) fn encode(
    value: ParameterFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    bytes(encoded(value, control, work)?)
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters;
    let value = match op {
        Op::Ratio | Op::Pieces => {
            let domain = domain(&inputs[0], work)?;
            let mut pieces = Vec::new();
            let count = if matches!(op, Op::Ratio) {
                1
            } else {
                inputs.len() / 3
            };
            if count > super::limits().descriptors.items.saturating_sub(3) {
                return Err(Error::Capacity(bumbledb::event::Capacity::DescriptorItems));
            }
            pieces.try_reserve_exact(count)?;
            if matches!(op, Op::Ratio) {
                pieces.push(GuardedRationalFunction::new(
                    domain.clone(),
                    polynomial(&inputs[1], work)?,
                    polynomial(&inputs[2], work)?,
                    limits.parameters.region,
                    work,
                )?);
            } else {
                for part in inputs[1..].as_chunks::<3>().0 {
                    let numerator = polynomial(&part[0], work)?;
                    let denominator = polynomial(&part[1], work)?;
                    let defined = region(&part[2], work)?;
                    pieces.push(
                        GuardedRationalFunction::new(
                            domain.clone(),
                            numerator,
                            denominator,
                            limits.parameters.region,
                            work,
                        )?
                        .restrict(
                            &defined,
                            limits.parameters.region,
                            work,
                        )?,
                    );
                }
            }
            ParameterFunction::new(
                domain,
                &pieces,
                limits.parameters.region,
                limits.functions,
                work,
            )?
        }
        _ => return operate(op, inputs, argument, control, work),
    };
    encode(value, control, work)
}
fn operate(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = function(&inputs[0], work)?;
    let limits = limits().parameters;
    let (parameters, functions) = (limits.parameters.region, limits.functions);
    let result = match op {
        Op::Validate => value,
        Op::Describe => return describe(&value, work),
        Op::Domain => return encoded_region(value.ambient().region(), work),
        Op::Defined => return encoded_region(value.defined_on(), work),
        Op::IsNowhere => return Ok(boolean(value.is_nowhere_defined())),
        Op::Sign => {
            return encoded_region(
                &value.where_sign(signs(argument)?, parameters, functions, work)?,
                work,
            );
        }
        Op::At => {
            let point = rational(&inputs[1], work)?;
            let result = value
                .value_at(&point, parameters, functions, work)?
                .map(|v| v.to_bytes(work))
                .transpose()?;
            return Ok(Output::Parameter(ParameterOutput::Value(result)));
        }
        Op::Restrict => value.restrict(&region(&inputs[1], work)?, parameters, functions, work)?,
        Op::OnDomain => value.on_domain(&domain(&inputs[1], work)?, parameters, functions, work)?,
        Op::Add | Op::Subtract | Op::Multiply | Op::Divide | Op::Equivalent => {
            let other = function(&inputs[1], work)?;
            match op {
                Op::Add => value.add(&other, parameters, functions, work)?,
                Op::Subtract => value.sub(&other, parameters, functions, work)?,
                Op::Multiply => value.mul(&other, parameters, functions, work)?,
                Op::Divide => value.div(&other, parameters, functions, work)?,
                _ => {
                    return Ok(boolean(
                        value.equivalent(&other, parameters, functions, work)?,
                    ));
                }
            }
        }
        _ => return Err(Error::InvalidEncoding),
    };
    encode(result, control, work)
}
fn describe(value: &ParameterFunction, work: &mut ExactArithmetic<'_>) -> Result<Output> {
    let limits = limits().parameters.parameters;
    let mut budget = super::output::Budget::default();
    let ambient = budget.blob(value.ambient().to_bytes(limits, work)?)?;
    let defined = budget.blob(value.defined_on().to_bytes(limits, work)?)?;
    let mut pieces = Vec::new();
    pieces.try_reserve_exact(value.pieces().len())?;
    for piece in value.pieces() {
        pieces.push(FunctionPiece {
            numerator: budget.blob(piece.numerator().to_bytes(limits.region.polynomial, work)?)?,
            denominator: budget.blob(
                piece
                    .denominator()
                    .to_bytes(limits.region.polynomial, work)?,
            )?,
            defined: budget.blob(piece.defined_on().to_bytes(limits, work)?)?,
        });
    }
    Ok(Output::Parameter(ParameterOutput::Function {
        ambient,
        defined,
        pieces,
    }))
}
