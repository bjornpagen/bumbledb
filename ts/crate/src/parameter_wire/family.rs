use super::output::{Budget, FunctionPiece};
use super::source::{encode as encode_event, event, finite, observation, outcomes, space};
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, boolean, bytes, function,
    limits, polynomial, rational, region, signs,
};
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, CoordinateMap, Descriptor, FamilyFunction,
    FamilyFunctionPiece, GuardedRationalFunction, ParameterDomain, SourceDescriptor,
};

#[derive(Clone, Copy)]
pub(super) enum Op {
    New,
    FromParameter,
    FromFinite,
    Density,
    Designate,
    Validate,
    Describe,
    At,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equivalent,
    IsNonnegative,
    Sign,
    Align,
    Pullback,
    Pushforward,
    Descend,
    Refine,
    Sum,
    Expectation,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "fromParameter" => Self::FromParameter,
            "fromFinite" => Self::FromFinite,
            "density" => Self::Density,
            "designate" => Self::Designate,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "at" => Self::At,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "divide" => Self::Divide,
            "equivalent" => Self::Equivalent,
            "isNonnegative" => Self::IsNonnegative,
            "sign" => Self::Sign,
            "align" => Self::Align,
            "pullback" => Self::Pullback,
            "pushforward" => Self::Pushforward,
            "descend" => Self::Descend,
            "refine" => Self::Refine,
            "sum" => Self::Sum,
            "expectation" => Self::Expectation,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        let arity = match self {
            Self::New => count > 0 && (count - 1).is_multiple_of(4),
            Self::At => count == 3,
            Self::FromFinite
            | Self::Density
            | Self::Designate
            | Self::Validate
            | Self::Describe
            | Self::IsNonnegative
            | Self::Sign => count == 1,
            _ => count == 2,
        };
        arity
            && if matches!(self, Self::Sign) {
                argument < 8
            } else {
                argument == 0
            }
    }
}
pub(super) fn family(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<FamilyFunction> {
    if let AdmittedSourceDescriptor::Family(value) =
        SourceDescriptor::import(bytes, limits(), work)?
        && let AdmittedFamilyDescriptor::Function(value) = *value
    {
        return Ok(value);
    }
    Err(Error::RoleMismatch)
}
fn encoded(
    value: FamilyFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Function(value))),
        limits(),
        work,
    )?
    .to_bytes(limits(), control)
}
fn map(
    bytes: &[u8],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<CoordinateMap> {
    match Descriptor::from_bytes(bytes, limits().descriptors, control)? {
        Descriptor::Map(value) => value.admit_with_parameters(limits(), work),
        Descriptor::Surjective(value) => Ok(value
            .admit_with_parameters(limits(), work)?
            .certify_surjective(control)?
            .map()
            .clone()),
        _ => Err(Error::RoleMismatch),
    }
}
fn guarded(
    domain: &ParameterDomain,
    inputs: &[Vec<u8>; 3],
    work: &mut ExactArithmetic<'_>,
) -> Result<GuardedRationalFunction> {
    let numerator = polynomial(&inputs[0], work)?;
    let denominator = polynomial(&inputs[1], work)?;
    let defined = region(&inputs[2], work)?;
    GuardedRationalFunction::new(
        domain.clone(),
        numerator,
        denominator,
        limits().parameters.parameters.region,
        work,
    )?
    .restrict(&defined, limits().parameters.parameters.region, work)
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters;
    let result = match op {
        Op::New => {
            let space = space(&inputs[0], work)?;
            let domain = space.parameter_domain().ok_or(Error::MissingParameter)?;
            let count = (inputs.len() - 1) / 4;
            if count > super::limits().descriptors.items.saturating_sub(3) / 2 {
                return Err(Error::Capacity(bumbledb::event::Capacity::DescriptorItems));
            }
            let mut pieces = Vec::new();
            pieces.try_reserve_exact(count)?;
            for part in inputs[1..].as_chunks::<4>().0 {
                let region = event(&part[0], work)?;
                let value = guarded(
                    domain,
                    part[1..].try_into().expect("three function components"),
                    work,
                )?;
                pieces.push(FamilyFunctionPiece { region, value });
            }
            FamilyFunction::new(&space, &pieces, limits, work)?
        }
        Op::FromParameter => FamilyFunction::from_parameter(
            &space(&inputs[0], work)?,
            &function::function(&inputs[1], work)?,
            limits,
            work,
        )?,
        Op::FromFinite => FamilyFunction::from_finite(&finite(&inputs[0], work)?, limits, work)?,
        Op::Density => FamilyFunction::density(&space(&inputs[0], work)?, limits, work)?,
        _ => return operate(op, inputs, argument, control, work),
    };
    bytes(encoded(result, control, work)?)
}
fn operate(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = family(&inputs[0], work)?;
    let limits = limits().parameters;
    let result = match op {
        Op::Validate => value,
        Op::Describe => return describe(&value, control, work),
        Op::Designate => return encode_event(&value.designate(limits, work)?.full(), control),
        Op::At => {
            let point = rational(&inputs[1], work)?;
            let result = value
                .value_at(&point, outcomes(&inputs[2])?, limits, work)?
                .map(|v| v.to_bytes(work))
                .transpose()?;
            return Ok(Output::Parameter(ParameterOutput::Value(result)));
        }
        Op::IsNonnegative => return Ok(boolean(value.is_nonnegative(limits, work)?)),
        Op::Sign => {
            return encode_event(&value.where_sign(signs(argument)?, limits, work)?, control);
        }
        Op::Align => value.align_to(&space(&inputs[1], work)?, limits, work)?,
        Op::Refine => value.refine(
            &super::refinement::refinement(&inputs[1], work)?,
            limits,
            work,
        )?,
        Op::Pullback | Op::Pushforward | Op::Descend => {
            let map = map(&inputs[1], control, work)?;
            match op {
                Op::Pullback => value.pullback(&map, limits, work)?,
                Op::Pushforward => value.pushforward(&map, limits, work)?,
                _ => value.descend(&map.certify_surjective(control)?, limits, work)?,
            }
        }
        Op::Sum => {
            return function::encode(
                value.outcome_sum(&event(&inputs[1], work)?, limits, work)?,
                control,
                work,
            );
        }
        Op::Expectation => {
            let evidence = event(&inputs[1], work)?;
            let result = value.expectation(&evidence, limits, work)?;
            return observation(
                "familyExpectation",
                encoded(result.function().clone(), control, work)?,
                result.evidence(),
                result.numerator(),
                result.evidence_mass(),
                result.conditional(),
                control,
                work,
            );
        }
        Op::Add | Op::Subtract | Op::Multiply | Op::Divide | Op::Equivalent => {
            let other = family(&inputs[1], work)?;
            match op {
                Op::Add => value.add(&other, limits, work)?,
                Op::Subtract => value.sub(&other, limits, work)?,
                Op::Multiply => value.multiply(&other, limits, work)?,
                Op::Divide => value.divide(&other, limits, work)?,
                _ => return Ok(boolean(value.equivalent(&other, limits, work)?)),
            }
        }
        _ => return Err(Error::InvalidEncoding),
    };
    bytes(encoded(result, control, work)?)
}
fn describe(
    value: &FamilyFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let mut budget = Budget::default();
    let limits = limits().parameters.parameters;
    let space = budget.blob(value.space().full().to_bytes(control)?)?;
    let mut pieces = Vec::new();
    pieces.try_reserve_exact(value.pieces().len())?;
    for (region, part) in value.pieces() {
        pieces.push((
            budget.blob(region.to_bytes(control)?)?,
            FunctionPiece {
                numerator: budget
                    .blob(part.numerator().to_bytes(limits.region.polynomial, work)?)?,
                denominator: budget.blob(
                    part.denominator()
                        .to_bytes(limits.region.polynomial, work)?,
                )?,
                defined: budget.blob(part.defined_on().to_bytes(limits, work)?)?,
            },
        ));
    }
    Ok(Output::Parameter(ParameterOutput::Family { space, pieces }))
}
