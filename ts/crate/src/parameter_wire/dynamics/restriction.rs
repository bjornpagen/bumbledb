use super::super::{name, refinement, region, source};
use super::{
    AdmittedFamilyDescriptor, Budget, Details, Error, ExactArithmetic, Output, Result, WorkContext,
    bytes, details, encoded, encoded_map, import, limits,
};
use bumbledb::event::{Capacity, ParameterRestriction, SpaceId};

#[derive(Clone, Copy)]
pub(crate) enum Op {
    New,
    FromRefinement,
    Validate,
    Describe,
    Pullback,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "fromRefinement" => Self::FromRefinement,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "pullback" => Self::Pullback,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize) -> bool {
        match self {
            Self::New => (3..=65).contains(&count),
            Self::FromRefinement | Self::Pullback => count == 2,
            _ => count == 1,
        }
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = match op {
        Op::New => {
            let identity = SpaceId(name(&inputs[0])?.0);
            let source = source::space(&inputs[1], work)?;
            let predicate = region(&inputs[2], work)?;
            if inputs.len() - 3 > 62 {
                return Err(Error::Capacity(Capacity::Coordinates));
            }
            let mut additional = Vec::new();
            additional.try_reserve_exact(inputs.len() - 3)?;
            for bytes in &inputs[3..] {
                additional.push(region(bytes, work)?);
            }
            ParameterRestriction::with_predicates(
                identity,
                &source,
                &predicate,
                &additional,
                limits().parameters,
                work,
            )?
        }
        Op::FromRefinement => ParameterRestriction::from_refinement(
            &refinement::refinement(&inputs[0], work)?,
            &region(&inputs[1], work)?,
            limits().parameters,
            work,
        )?,
        _ => {
            if let AdmittedFamilyDescriptor::Restriction(value) = import(&inputs[0], work)? {
                value
            } else {
                return Err(Error::RoleMismatch);
            }
        }
    };
    match op {
        Op::Describe => {
            let mut budget = Budget::new();
            Ok(details(Details::Restriction {
                prior: budget.blob(value.prior().full().to_bytes(control)?)?,
                refined: budget.blob(value.refined_prior().full().to_bytes(control)?)?,
                space: budget.blob(value.space().full().to_bytes(control)?)?,
                refinement: budget.blob(encoded(
                    AdmittedFamilyDescriptor::Refinement(value.refinement().clone()),
                    control,
                    work,
                )?)?,
                inclusion: budget.blob(encoded_map(value.inclusion(), false, control)?)?,
            }))
        }
        Op::Pullback => source::encode(
            &value.pullback(&source::event(&inputs[1], work)?, control)?,
            control,
        ),
        _ => bytes(encoded(
            AdmittedFamilyDescriptor::Restriction(value),
            control,
            work,
        )?),
    }
}
