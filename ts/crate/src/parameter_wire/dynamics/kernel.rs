use super::super::{boolean, family, source};
use super::{
    AdmittedFamilyDescriptor, Budget, Details, Error, ExactArithmetic, Output, Result, WorkContext,
    bytes, details, encoded, encoded_map, import, limits,
};
use bumbledb::event::FamilyKernel;

#[derive(Clone, Copy)]
pub(crate) enum Op {
    New,
    Validate,
    Describe,
    Close,
    Factors,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "close" => Self::Close,
            "factorsThrough" => Self::Factors,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize) -> bool {
        count
            == if matches!(self, Self::Validate | Self::Describe) {
                1
            } else {
                2
            }
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = if matches!(op, Op::New) {
        let parent = family::map(&inputs[0], control, work)?;
        let density = family::family(&inputs[1], work)?;
        FamilyKernel::new(&parent, &density, limits().parameters, work)?
    } else if let AdmittedFamilyDescriptor::Kernel(value) = import(&inputs[0], work)? {
        value
    } else {
        return Err(Error::RoleMismatch);
    };
    let mut budget = Budget::new();
    match op {
        Op::Describe => Ok(details(Details::Kernel {
            parent: budget.blob(encoded_map(value.parent().map(), true, control)?)?,
            density: budget.blob(family::encoded(value.density().clone(), control, work)?)?,
        })),
        Op::Close => {
            let prior = source::space(&inputs[1], work)?;
            let extension = value.close(&prior, limits().parameters, work)?;
            Ok(details(Details::Extension {
                space: budget.blob(extension.space().full().to_bytes(control)?)?,
                parent: budget.blob(encoded_map(extension.parent(), true, control)?)?,
            }))
        }
        Op::Factors => {
            let readout = family::map(&inputs[1], control, work)?.certify_surjective(control)?;
            Ok(boolean(value.factors_through(
                &readout,
                limits().parameters,
                work,
            )?))
        }
        _ => bytes(encoded(
            AdmittedFamilyDescriptor::Kernel(value),
            control,
            work,
        )?),
    }
}
