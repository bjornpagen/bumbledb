//! Owned family dynamics. All admission and replay use one exact worker budget.
use super::{Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, bytes, limits};
use bumbledb::event::{
    AdmittedDescriptor, AdmittedFamilyDescriptor, AdmittedSourceDescriptor, CoordinateMap,
    Descriptor, SourceDescriptor,
};
mod kernel;
mod output;
mod restriction;
mod revision;
use output::Budget;
pub use output::Details;

#[derive(Clone, Copy)]
pub(super) enum Op {
    Kernel(kernel::Op),
    Restriction(restriction::Op),
    Revision(revision::Op),
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        if let Some(name) = name.strip_prefix("kernel.") {
            return kernel::Op::parse(name).map(Self::Kernel);
        }
        if let Some(name) = name.strip_prefix("restriction.") {
            return restriction::Op::parse(name).map(Self::Restriction);
        }
        name.strip_prefix("revision.")
            .and_then(revision::Op::parse)
            .map(Self::Revision)
    }
    pub(super) fn valid(self, count: usize) -> bool {
        match self {
            Self::Kernel(op) => op.valid(count),
            Self::Restriction(op) => op.valid(count),
            Self::Revision(op) => op.valid(count),
        }
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    match op {
        Op::Kernel(op) => kernel::execute(op, inputs, control, work),
        Op::Restriction(op) => restriction::execute(op, inputs, control, work),
        Op::Revision(op) => revision::execute(op, inputs, control, work),
    }
}
pub(super) fn encoded(
    value: AdmittedFamilyDescriptor,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Family(Box::new(value)),
        limits(),
        work,
    )?
    .to_bytes(limits(), control)
}
fn import(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<AdmittedFamilyDescriptor> {
    match SourceDescriptor::import(bytes, limits(), work)? {
        AdmittedSourceDescriptor::Family(value) => Ok(*value),
        _ => Err(Error::RoleMismatch),
    }
}
fn details(value: Details) -> Output {
    Output::Parameter(ParameterOutput::Dynamics(value))
}
fn encoded_map(value: &CoordinateMap, onto: bool, control: &WorkContext) -> Result<Vec<u8>> {
    let value = if onto {
        AdmittedDescriptor::Surjective(value.certify_surjective(control)?)
    } else {
        AdmittedDescriptor::Map(value.clone())
    };
    Descriptor::capture(&value, limits().descriptors, control)?
        .to_bytes(limits().descriptors, control)
}
