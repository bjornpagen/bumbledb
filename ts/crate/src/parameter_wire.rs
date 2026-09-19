//! Exact parameter SDK marshaling; the shared core owns solving, domains and
//! partial-function arithmetic. One worker budget covers every nested object.
use crate::exact_wire::scalar;
use crate::ingress::{CopyContext, MAX_EVENT_BYTES, event_error};
use crate::marshal::ValueOut;
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, thrown,
};
use bumbledb::event::{
    AlgebraicRoot, ArithmeticLimits, Capacity, Error, ExactArithmetic, ExactPolynomial,
    ExactRational, ParameterDomain, ParameterId, ParameterRegion, PolynomialSigns,
    SourceDescriptorLimits,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, BigInt, Env, External, Function};
use napi_derive::napi;

mod dynamics;
mod family;
mod function;
mod output;
mod prior;
mod refinement;
mod region;
mod root;
mod source;
pub use output::{ParameterOutput, runtime_event_parameter_take};
type Result<T> = bumbledb::event::Result<T>;

#[derive(Clone, Copy)]
enum Op {
    Prior(prior::Op),
    Region(region::Op),
    Root(root::Op),
    Function(function::Op),
    Family(family::Op),
    Source(source::Op),
    Refinement(refinement::Op),
    Dynamics(dynamics::Op),
}
impl Op {
    fn parse(name: &str) -> Option<Self> {
        if let Some(name) = name.strip_prefix("prior.") {
            return prior::Op::parse(name).map(Self::Prior);
        }
        if let Some(name) = name.strip_prefix("region.") {
            return region::Op::parse(name).map(Self::Region);
        }
        if let Some(name) = name.strip_prefix("root.") {
            return root::Op::parse(name).map(Self::Root);
        }
        if let Some(name) = name.strip_prefix("function.") {
            return function::Op::parse(name).map(Self::Function);
        }
        if let Some(name) = name.strip_prefix("family.") {
            return family::Op::parse(name).map(Self::Family);
        }
        if let Some(name) = name.strip_prefix("source.") {
            return source::Op::parse(name).map(Self::Source);
        }
        if let Some(name) = name.strip_prefix("refinement.") {
            return refinement::Op::parse(name).map(Self::Refinement);
        }
        dynamics::Op::parse(name).map(Self::Dynamics)
    }
    fn valid(self, count: usize, argument: u8) -> bool {
        match self {
            Self::Prior(op) => op.valid(count, argument),
            Self::Region(op) => op.valid(count, argument),
            Self::Root(op) => op.valid(count, argument),
            Self::Function(op) => op.valid(count, argument),
            Self::Family(op) => op.valid(count, argument),
            Self::Source(op) => op.valid(count, argument),
            Self::Refinement(op) => op.valid(count, argument),
            Self::Dynamics(op) => argument == 0 && op.valid(count),
        }
    }
}
fn limits() -> SourceDescriptorLimits {
    SourceDescriptorLimits::default()
}
fn name(bytes: &[u8]) -> Result<ParameterId> {
    Ok(ParameterId(
        bytes.try_into().map_err(|_| Error::ParameterBinding)?,
    ))
}
fn polynomial(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<ExactPolynomial> {
    ExactPolynomial::from_bytes(
        bytes,
        limits().parameters.parameters.region.polynomial,
        work,
    )
}
fn rational(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<ExactRational> {
    ExactRational::from_bytes(bytes, work)
}
fn region(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<ParameterRegion> {
    ParameterRegion::from_bytes(bytes, limits().parameters.parameters, work)
}
fn domain(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<ParameterDomain> {
    ParameterDomain::from_bytes(bytes, limits().parameters.parameters, work)
}
fn root(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<AlgebraicRoot> {
    AlgebraicRoot::from_bytes(bytes, limits().parameters.parameters.algebraic, work)
}
fn bytes(bytes: Vec<u8>) -> Result<Output> {
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    Ok(Output::Bytes(QueuedBytes { bytes }))
}
fn encoded_region(value: &ParameterRegion, work: &mut ExactArithmetic<'_>) -> Result<Output> {
    bytes(value.to_bytes(limits().parameters.parameters, work)?)
}
fn encoded_root(value: &AlgebraicRoot, work: &mut ExactArithmetic<'_>) -> Result<Output> {
    bytes(value.to_bytes(limits().parameters.parameters.algebraic, work)?)
}
fn signs(argument: u8) -> Result<PolynomialSigns> {
    PolynomialSigns::new(argument).ok_or(Error::InvalidEncoding)
}
fn boolean(value: bool) -> Output {
    scalar(ValueOut::Bool(value))
}
fn ordering(value: std::cmp::Ordering) -> Output {
    scalar(ValueOut::I64(match value {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}
fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u8,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    bumbledb::event::Control::checkpoint(control)?;
    if !op.valid(inputs.len(), argument) {
        return Err(Error::InvalidEncoding);
    }
    match op {
        Op::Prior(op) => prior::execute(op, inputs, control, work),
        Op::Region(op) => region::execute(op, inputs, argument, work),
        Op::Root(op) => root::execute(op, inputs, work),
        Op::Function(op) => function::execute(op, inputs, argument, control, work),
        Op::Family(op) => family::execute(op, inputs, argument, control, work),
        Op::Source(op) => source::execute(op, inputs, control, work),
        Op::Refinement(op) => refinement::execute(op, inputs, control, work),
        Op::Dynamics(op) => dynamics::execute(op, inputs, control, work),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_parameter(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    inputs: Array,
    argument: BigInt,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = Op::parse(&operation).ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
    let (negative, argument, lossless) = argument.get_u64();
    let argument =
        u8::try_from(argument).map_err(|_| thrown(env, RuntimeError::InvalidArgument))?;
    if negative || !lossless || !op.valid(inputs.len() as usize, argument) {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let inputs = copy.finish(copy.blobs(&inputs, MAX_EVENT_BYTES, 16_384))?;
            Ok(Box::new(move |control: &WorkContext| {
                let mut work = ExactArithmetic::new(ArithmeticLimits::default(), control);
                let output =
                    execute(op, &inputs, argument, control, &mut work).map_err(event_error)?;
                control.checkpoint()?;
                Ok(output)
            }) as crate::runtime::Work)
        })
        .map_err(|e| thrown(env, e))?;
    Ok(operation_handle(runtime, operation))
}

#[cfg(test)]
mod tests;
