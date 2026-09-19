//! Fixed-law SDK operations. One worker and arithmetic budget admit every
//! operand, perform native algebra, and encode owned outputs before delivery.
use crate::exact_wire::scalar;
use crate::ingress::{CopyContext, event_error};
use crate::marshal::{ValueOut, output_vec};
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, take_output, thrown,
};
use bumbledb::event::{
    AdmittedSourceDescriptor, ArithmeticLimits, Capacity, CoordinateMap, Descriptor, Error, Event,
    ExactArithmetic, ExactRational, FiniteFunction, FunctionDescriptor, FunctionPiece,
    SourceDescriptor, SourceDescriptorLimits, Space,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, BigInt, Env, External, Function, Object, Uint8Array};
use napi_derive::napi;

pub enum SourceOutput {
    Function(FunctionDescriptor),
    Observation {
        probability: bool,
        input: Vec<u8>,
        given: Vec<u8>,
        numerator: Vec<u8>,
        evidence_mass: Vec<u8>,
        value: Option<Vec<u8>>,
    },
}
#[derive(Clone, Copy)]
enum Op {
    New,
    Constant,
    Density,
    Validate,
    Describe,
    Add,
    Multiply,
    Align,
    Pullback,
    Pushforward,
    IsZero,
    IsNonnegative,
    Equivalent,
    At,
    Designate,
    Mass,
    Probability,
    Expectation,
}
impl Op {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "constant" => Self::Constant,
            "density" => Self::Density,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "add" => Self::Add,
            "multiply" => Self::Multiply,
            "align" => Self::Align,
            "pullback" => Self::Pullback,
            "pushforward" => Self::Pushforward,
            "isZero" => Self::IsZero,
            "isNonnegative" => Self::IsNonnegative,
            "equivalent" => Self::Equivalent,
            "at" => Self::At,
            "designate" => Self::Designate,
            "mass" => Self::Mass,
            "probability" => Self::Probability,
            "expectation" => Self::Expectation,
            _ => return None,
        })
    }
    fn arity(self, n: usize) -> bool {
        match self {
            Self::New => n > 0 && n % 2 == 1,
            Self::Constant
            | Self::Add
            | Self::Multiply
            | Self::Align
            | Self::Pullback
            | Self::Pushforward
            | Self::Equivalent
            | Self::Probability
            | Self::Expectation => n == 2,
            _ => n == 1,
        }
    }
}

fn event(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Event> {
    Event::from_bytes_with_arithmetic(bytes, None, limits.descriptors.events, limits.laws, work)
}
fn space(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Space> {
    let event = event(bytes, limits, work)?;
    if !event.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(event.space())
}
fn function(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<FiniteFunction> {
    match SourceDescriptor::import(bytes, limits, work)? {
        AdmittedSourceDescriptor::Function(value) => Ok(value),
        _ => Err(Error::RoleMismatch),
    }
}
fn map(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<CoordinateMap> {
    match Descriptor::from_bytes(bytes, limits.descriptors, control)? {
        Descriptor::Map(value) => value.admit_with_arithmetic(limits, work),
        Descriptor::Surjective(value) => Ok(value
            .admit_with_arithmetic(limits, work)?
            .certify_surjective(control)?
            .map()
            .clone()),
        _ => Err(Error::RoleMismatch),
    }
}
fn capture(
    value: FiniteFunction,
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<SourceDescriptor> {
    SourceDescriptor::capture(&AdmittedSourceDescriptor::Function(value), limits, work)
}
fn encoded(
    value: FiniteFunction,
    limits: SourceDescriptorLimits,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Vec<u8>> {
    capture(value, limits, work)?.to_bytes(limits, control)
}
fn bytes(bytes: Vec<u8>) -> bumbledb::event::Result<Output> {
    if bytes.len() > SourceDescriptorLimits::default().descriptors.bytes {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    Ok(Output::Bytes(QueuedBytes { bytes }))
}
fn observation(
    probability: bool,
    input: Vec<u8>,
    given: Vec<u8>,
    numerator: &ExactRational,
    evidence_mass: &ExactRational,
    value: Option<ExactRational>,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let numerator = numerator.to_bytes(work)?;
    let evidence_mass = evidence_mass.to_bytes(work)?;
    let value = value.map(|v| v.to_bytes(work)).transpose()?;
    let size = [&input, &given, &numerator, &evidence_mass]
        .into_iter()
        .chain(value.iter())
        .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()))
        .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
    if size > SourceDescriptorLimits::default().descriptors.bytes {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    Ok(Output::EventSource(SourceOutput::Observation {
        probability,
        input,
        given,
        numerator,
        evidence_mass,
        value,
    }))
}

fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u64,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    bumbledb::event::Control::checkpoint(control)?;
    let limits = SourceDescriptorLimits::default();
    let functions = limits.functions;
    match op {
        Op::New => {
            let space = space(&inputs[0], limits, work)?;
            let mut pieces = Vec::new();
            pieces.try_reserve_exact(inputs.len() / 2)?;
            for pair in inputs[1..].as_chunks::<2>().0 {
                pieces.push(FunctionPiece {
                    region: event(&pair[0], limits, work)?,
                    value: ExactRational::from_bytes(&pair[1], work)?,
                });
            }
            bytes(encoded(
                FiniteFunction::new(&space, &pieces, functions, work)?,
                limits,
                control,
                work,
            )?)
        }
        Op::Constant => {
            let space = space(&inputs[0], limits, work)?;
            let value = ExactRational::from_bytes(&inputs[1], work)?;
            bytes(encoded(
                FiniteFunction::constant(&space, value, functions, work)?,
                limits,
                control,
                work,
            )?)
        }
        Op::Density => {
            let space = space(&inputs[0], limits, work)?;
            bytes(encoded(
                FiniteFunction::density(&space, functions, work)?,
                limits,
                control,
                work,
            )?)
        }
        Op::Mass => bytes(
            event(&inputs[0], limits, work)?
                .mass(work)?
                .to_bytes(work)?,
        ),
        Op::Probability => {
            let a = event(&inputs[0], limits, work)?;
            let given = event(&inputs[1], limits, work)?.align_to(&a.space(), control)?;
            let result = a.probability(&given, work)?;
            observation(
                true,
                a.to_bytes(control)?,
                given.to_bytes(control)?,
                result.numerator(),
                result.evidence_mass(),
                result.value(work)?,
                work,
            )
        }
        _ => function_operation(op, inputs, argument, control, work),
    }
}

fn function_operation(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u64,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let limits = SourceDescriptorLimits::default();
    let functions = limits.functions;
    let a = function(&inputs[0], limits, work)?;
    let result = match op {
        Op::Validate => a,
        Op::Describe => {
            let description = capture(a, limits, work)?;
            description.to_bytes(limits, control)?;
            let SourceDescriptor::Function(data) = description else {
                unreachable!()
            };
            return Ok(Output::EventSource(SourceOutput::Function(data)));
        }
        Op::IsZero => return Ok(scalar(ValueOut::Bool(a.is_zero()))),
        Op::IsNonnegative => return Ok(scalar(ValueOut::Bool(a.is_nonnegative()))),
        Op::At => return bytes(a.at(argument, control)?.to_bytes(work)?),
        Op::Designate => {
            return bytes(a.designate(limits.laws, work)?.full().to_bytes(control)?);
        }
        Op::Add | Op::Multiply | Op::Equivalent => {
            let b = function(&inputs[1], limits, work)?;
            match op {
                Op::Add => a.add(&b, functions, work)?,
                Op::Multiply => a.multiply(&b, functions, work)?,
                _ => return Ok(scalar(ValueOut::Bool(a.equivalent(&b, control)?))),
            }
        }
        Op::Align => a.align_to(&space(&inputs[1], limits, work)?, functions, work)?,
        Op::Pullback | Op::Pushforward => {
            let map = map(&inputs[1], limits, control, work)?;
            if matches!(op, Op::Pullback) {
                a.pullback(&map, functions, work)?
            } else {
                a.pushforward(&map, functions, work)?
            }
        }
        Op::Expectation => {
            let given = event(&inputs[1], limits, work)?;
            let result = a.expectation(&given, work)?;
            return observation(
                false,
                encoded(a, limits, control, work)?,
                result.evidence().to_bytes(control)?,
                result.numerator(),
                result.evidence_mass(),
                result.value(work)?,
                work,
            );
        }
        _ => unreachable!(),
    };
    bytes(encoded(result, limits, control, work)?)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_source(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    inputs: Array,
    argument: BigInt,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = Op::parse(&operation).ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
    let argument = crate::marshal::u64_in(&argument, "Event source argument")?;
    if !op.arity(inputs.len() as usize) || (!matches!(op, Op::At) && argument != 0) {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |control| {
            let copy = CopyContext::new(env, control);
            let limits = SourceDescriptorLimits::default().descriptors;
            let inputs = copy.finish(copy.blobs(&inputs, limits.bytes, limits.items))?;
            Ok(Box::new(move |control: &WorkContext| {
                let mut work = ExactArithmetic::new(ArithmeticLimits::default(), control);
                let result =
                    execute(op, &inputs, argument, control, &mut work).map_err(event_error)?;
                control.checkpoint()?;
                Ok(result)
            }) as crate::runtime::Work)
        })
        .map_err(|e| thrown(env, e))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_event_source_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<SourceOutput> {
    match take_output(env, handle)? {
        Output::EventSource(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
impl napi::bindgen_prelude::ToNapiValue for SourceOutput {
    #[expect(
        unsafe_code,
        reason = "N-API conversion transfers worker-owned data through the object API"
    )]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        value: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let handle = Env::from_raw(env);
        let mut object = Object::new(&handle)?;
        match value {
            Self::Function(data) => {
                object.set("space", Uint8Array::from(data.space))?;
                let mut pieces = output_vec(data.pieces.len()).map_err(|e| thrown(handle, e))?;
                for piece in data.pieces {
                    let mut value = Object::new(&handle)?;
                    value.set("region", Uint8Array::from(piece.region))?;
                    value.set("value", Uint8Array::from(piece.value))?;
                    pieces.push(value);
                }
                object.set("pieces", pieces)?;
            }
            Self::Observation {
                probability,
                input,
                given,
                numerator,
                evidence_mass,
                value,
            } => {
                object.set(
                    "kind",
                    if probability {
                        "probability"
                    } else {
                        "expectation"
                    },
                )?;
                object.set(
                    if probability { "event" } else { "function" },
                    Uint8Array::from(input),
                )?;
                object.set("given", Uint8Array::from(given))?;
                object.set("numerator", Uint8Array::from(numerator))?;
                object.set("evidenceMass", Uint8Array::from(evidence_mass))?;
                object.set("value", value.map(Uint8Array::from))?;
            }
        }
        // SAFETY: object belongs to the live N-API environment for this call.
        unsafe { Object::to_napi_value(env, object) }
    }
}

#[cfg(test)]
mod tests;
