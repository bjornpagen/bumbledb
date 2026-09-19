use super::output::Budget;
use super::source::{encode, event, space};
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, bytes, limits, name,
    region,
};
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, ParameterRefinement, SourceDescriptor,
    SpaceId,
};

#[derive(Clone, Copy)]
pub(super) enum Op {
    New,
    Validate,
    Describe,
    Lift,
    Descend,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "lift" => Self::Lift,
            "descend" => Self::Descend,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        argument == 0
            && match self {
                Self::New => count >= 2,
                Self::Lift | Self::Descend => count == 2,
                _ => count == 1,
            }
    }
}
pub(super) fn refinement(
    bytes: &[u8],
    work: &mut ExactArithmetic<'_>,
) -> Result<ParameterRefinement> {
    if let AdmittedSourceDescriptor::Family(value) =
        SourceDescriptor::import(bytes, limits(), work)?
        && let AdmittedFamilyDescriptor::Refinement(value) = *value
    {
        return Ok(value);
    }
    Err(Error::RoleMismatch)
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = if matches!(op, Op::New) {
        let identity = SpaceId(name(&inputs[0])?.0);
        let source = space(&inputs[1], work)?;
        if inputs.len() - 2 > 62 {
            return Err(Error::Capacity(bumbledb::event::Capacity::Coordinates));
        }
        let mut predicates = Vec::new();
        predicates.try_reserve_exact(inputs.len() - 2)?;
        for bytes in &inputs[2..] {
            predicates.push(region(bytes, work)?);
        }
        ParameterRefinement::new(identity, &source, &predicates, limits().parameters, work)?
    } else {
        refinement(&inputs[0], work)?
    };
    match op {
        Op::Lift | Op::Descend => {
            let event = event(&inputs[1], work)?;
            encode(
                &if matches!(op, Op::Lift) {
                    value.lift(&event, control)?
                } else {
                    value.descend(&event, control)?
                },
                control,
            )
        }
        Op::Describe => {
            let mut budget = Budget::default();
            Ok(Output::Parameter(ParameterOutput::Refinement {
                source: budget.blob(value.source().full().to_bytes(control)?)?,
                refined: budget.blob(value.refined().full().to_bytes(control)?)?,
            }))
        }
        _ => bytes(
            SourceDescriptor::capture(
                &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Refinement(
                    value,
                ))),
                limits(),
                work,
            )?
            .to_bytes(limits(), control)?,
        ),
    }
}
