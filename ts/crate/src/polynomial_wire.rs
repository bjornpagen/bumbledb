//! BEPL SDK marshaling. Polynomial normalization and arithmetic stay in the
//! shared Event core, under one worker-owned exact-arithmetic budget.
use crate::exact_wire::scalar;
use crate::ingress::{CopyContext, MAX_EVENT_BYTES, event_error};
use crate::marshal::{ValueOut, output_vec};
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, take_output, thrown,
};
use bumbledb::event::{
    ArithmeticLimits, Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational,
    ParameterId, PolynomialLimits, PolynomialTerm,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, BigInt, Env, External, Function, Object, Uint8Array};
use napi_derive::napi;

#[derive(Clone, Copy)]
enum Op {
    New,
    Parameter,
    Constant,
    Validate,
    Describe,
    Add,
    Subtract,
    Multiply,
    Pow,
    Substitute,
    Evaluate,
    IntegrateBeta,
    Equal,
    IsZero,
}
impl Op {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "parameter" => Self::Parameter,
            "constant" => Self::Constant,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "pow" => Self::Pow,
            "substitute" => Self::Substitute,
            "evaluate" => Self::Evaluate,
            "integrateBeta" => Self::IntegrateBeta,
            "equal" => Self::Equal,
            "isZero" => Self::IsZero,
            _ => return None,
        })
    }
    fn arity(self, count: usize) -> bool {
        match self {
            Self::New => count.is_multiple_of(2),
            Self::Substitute | Self::Evaluate => count > 0 && count % 2 == 1,
            Self::Add | Self::Subtract | Self::Multiply | Self::Equal => count == 2,
            Self::IntegrateBeta => count == 4,
            _ => count == 1,
        }
    }
}
fn name(bytes: &[u8]) -> bumbledb::event::Result<ParameterId> {
    Ok(ParameterId(
        bytes.try_into().map_err(|_| Error::ParameterBinding)?,
    ))
}
fn powers(
    bytes: &[u8],
    control: &WorkContext,
) -> bumbledb::event::Result<Box<[(ParameterId, u32)]>> {
    if !bytes.len().is_multiple_of(36) {
        return Err(Error::InvalidPolynomial);
    }
    if bytes.len() / 36 > PolynomialLimits::default().factors {
        return Err(Error::Capacity(Capacity::PolynomialFactors));
    }
    let mut result = Vec::new();
    result.try_reserve_exact(bytes.len() / 36)?;
    for part in bytes.as_chunks::<36>().0 {
        bumbledb::event::Control::checkpoint(control)?;
        result.push((
            name(&part[..32])?,
            u32::from_le_bytes(part[32..].try_into().expect("four bytes")),
        ));
    }
    Ok(result.into_boxed_slice())
}

pub struct Description {
    terms: Vec<Term>,
}
struct Term {
    coefficient: Vec<u8>,
    powers: Box<[(ParameterId, u32)]>,
}
fn describe(
    value: &ExactPolynomial,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    // The same canonical byte bound includes each coefficient/name/power before
    // exposing the corresponding owned, structured presentation.
    value.to_bytes(PolynomialLimits::default(), work)?;
    let mut terms = Vec::new();
    terms.try_reserve_exact(value.terms().len())?;
    for term in value.terms() {
        let mut powers = Vec::new();
        powers.try_reserve_exact(term.powers.len())?;
        powers.extend_from_slice(&term.powers);
        terms.push(Term {
            coefficient: term.coefficient.to_bytes(work)?,
            powers: powers.into_boxed_slice(),
        });
    }
    Ok(Output::Polynomial(Description { terms }))
}

fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u32,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    bumbledb::event::Control::checkpoint(control)?;
    let limits = PolynomialLimits::default();
    if !op.arity(inputs.len()) || (!matches!(op, Op::Pow) && argument != 0) {
        return Err(Error::InvalidEncoding);
    }
    let value = match op {
        Op::Parameter => ExactPolynomial::parameter(name(&inputs[0])?),
        Op::Constant => ExactPolynomial::constant(ExactRational::from_bytes(&inputs[0], work)?),
        Op::New => {
            if inputs.len() / 2 > limits.terms {
                return Err(Error::Capacity(Capacity::PolynomialTerms));
            }
            let mut terms = Vec::new();
            terms.try_reserve_exact(inputs.len() / 2)?;
            for pair in inputs.as_chunks::<2>().0 {
                terms.push(PolynomialTerm {
                    coefficient: ExactRational::from_bytes(&pair[0], work)?,
                    powers: powers(&pair[1], control)?,
                });
            }
            ExactPolynomial::from_terms(&terms, limits, work)?
        }
        _ => return operate(op, inputs, argument, work),
    };
    Ok(Output::Bytes(QueuedBytes {
        bytes: value.to_bytes(limits, work)?,
    }))
}
fn operate(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u32,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let limits = PolynomialLimits::default();
    let value = ExactPolynomial::from_bytes(&inputs[0], limits, work)?;
    let result = match op {
        Op::Validate => value,
        Op::Describe => return describe(&value, work),
        Op::IsZero => return Ok(scalar(ValueOut::Bool(value.is_zero()))),
        Op::Pow => value.pow(argument, limits, work)?,
        Op::Add | Op::Subtract | Op::Multiply | Op::Equal => {
            let other = ExactPolynomial::from_bytes(&inputs[1], limits, work)?;
            match op {
                Op::Add => value.add(&other, limits, work)?,
                Op::Subtract => value.sub(&other, limits, work)?,
                Op::Multiply => value.mul(&other, limits, work)?,
                _ => return Ok(scalar(ValueOut::Bool(value == other))),
            }
        }
        Op::Substitute => {
            let mut bindings = Vec::new();
            bindings.try_reserve_exact(inputs.len() / 2)?;
            for pair in inputs[1..].as_chunks::<2>().0 {
                bindings.push((
                    name(&pair[0])?,
                    ExactPolynomial::from_bytes(&pair[1], limits, work)?,
                ));
            }
            value.substitute(&bindings, limits, work)?
        }
        Op::Evaluate => {
            let mut bindings = Vec::new();
            bindings.try_reserve_exact(inputs.len() / 2)?;
            for pair in inputs[1..].as_chunks::<2>().0 {
                bindings.push((name(&pair[0])?, ExactRational::from_bytes(&pair[1], work)?));
            }
            let result = value.evaluate(&bindings, limits, work)?;
            return Ok(Output::Bytes(QueuedBytes {
                bytes: result.to_bytes(work)?,
            }));
        }
        Op::IntegrateBeta => {
            let parameter = name(&inputs[1])?;
            let alpha = ExactRational::from_bytes(&inputs[2], work)?;
            let beta = ExactRational::from_bytes(&inputs[3], work)?;
            value.integrate_beta(parameter, &alpha, &beta, limits, work)?
        }
        _ => return Err(Error::InvalidEncoding),
    };
    Ok(Output::Bytes(QueuedBytes {
        bytes: result.to_bytes(limits, work)?,
    }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_exact_polynomial(
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
        u32::try_from(argument).map_err(|_| thrown(env, RuntimeError::InvalidArgument))?;
    if negative
        || !lossless
        || !op.arity(inputs.len() as usize)
        || (!matches!(op, Op::Pow) && argument != 0)
    {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let inputs = copy.finish(copy.blobs(
                &inputs,
                MAX_EVENT_BYTES,
                2 * PolynomialLimits::default().terms + 1,
            ))?;
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
pub fn runtime_exact_polynomial_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Description> {
    match take_output(env, handle)? {
        Output::Polynomial(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
impl napi::bindgen_prelude::ToNapiValue for Description {
    #[expect(
        unsafe_code,
        reason = "N-API conversion transfers worker-owned polynomial terms"
    )]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        value: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let handle = Env::from_raw(env);
        let mut terms = output_vec(value.terms.len()).map_err(|e| thrown(handle, e))?;
        for term in value.terms {
            let mut object = Object::new(&handle)?;
            object.set("coefficient", Uint8Array::from(term.coefficient))?;
            let mut powers = output_vec(term.powers.len()).map_err(|e| thrown(handle, e))?;
            for (parameter, exponent) in term.powers {
                let mut power = Object::new(&handle)?;
                power.set("parameter", Uint8Array::from(parameter.0.to_vec()))?;
                power.set("exponent", BigInt::from(u64::from(exponent)))?;
                powers.push(power);
            }
            object.set("powers", powers)?;
            terms.push(object);
        }
        // SAFETY: the owned terms belong to the active N-API environment.
        unsafe { Vec::<Object>::to_napi_value(env, terms) }
    }
}

#[cfg(test)]
mod tests;
