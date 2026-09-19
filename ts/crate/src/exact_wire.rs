//! Exact scalar operations use native BERA admission and one arithmetic budget.
use crate::ingress::{CopyContext, MAX_EVENT_BYTES, event_error};
use crate::marshal::ValueOut;
use crate::runtime::{Output, QueuedBytes, QueuedOutput, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, thrown,
};
use bumbledb::event::{ArithmeticLimits, Error, ExactArithmetic, ExactRational};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, Env, External, Function};
use napi_derive::napi;

#[derive(Clone, Copy)]
enum Op {
    Fraction,
    Decimal,
    Binary64,
    Validate,
    Text,
    Add,
    Subtract,
    Multiply,
    Divide,
    Compare,
    Equal,
    IsZero,
    IsNegative,
    IsProbability,
}
impl Op {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "fraction" => Self::Fraction,
            "decimal" => Self::Decimal,
            "binary64" => Self::Binary64,
            "validate" => Self::Validate,
            "text" => Self::Text,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "divide" => Self::Divide,
            "compare" => Self::Compare,
            "equal" => Self::Equal,
            "isZero" => Self::IsZero,
            "isNegative" => Self::IsNegative,
            "isProbability" => Self::IsProbability,
            _ => return None,
        })
    }
    const fn arity(self) -> usize {
        match self {
            Self::Binary64 => 0,
            Self::Fraction
            | Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Compare
            | Self::Equal => 2,
            _ => 1,
        }
    }
}
pub(crate) fn scalar(value: ValueOut) -> Output {
    Output::Rows(QueuedOutput {
        rows: vec![vec![value]],
    })
}
fn text(bytes: &[u8]) -> bumbledb::event::Result<&str> {
    std::str::from_utf8(bytes).map_err(|_| Error::InvalidRational)
}
fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: f64,
    control: &WorkContext,
) -> Result<Output, RuntimeError> {
    control.checkpoint()?;
    if inputs.len() != op.arity() || (!matches!(op, Op::Binary64) && argument != 0.0) {
        return Err(RuntimeError::InvalidArgument);
    }
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), control);
    let result = match op {
        Op::Fraction => ExactRational::fraction(
            text(&inputs[0]).map_err(event_error)?,
            text(&inputs[1]).map_err(event_error)?,
            &mut work,
        ),
        Op::Decimal => ExactRational::decimal(text(&inputs[0]).map_err(event_error)?, &mut work),
        Op::Binary64 => ExactRational::binary64(argument, &mut work),
        _ => {
            let a = ExactRational::from_bytes(&inputs[0], &mut work).map_err(event_error)?;
            let b = inputs
                .get(1)
                .map(|bytes| ExactRational::from_bytes(bytes, &mut work))
                .transpose()
                .map_err(event_error)?;
            match op {
                Op::Validate => Ok(a),
                Op::Text => return Ok(scalar(ValueOut::Text(a.to_string()))),
                Op::IsZero => return Ok(scalar(ValueOut::Bool(a.is_zero()))),
                Op::IsNegative => return Ok(scalar(ValueOut::Bool(a.is_negative()))),
                Op::IsProbability => return Ok(scalar(ValueOut::Bool(a.is_probability()))),
                _ => {
                    let b = b.ok_or(RuntimeError::InvalidArgument)?;
                    match op {
                        Op::Add => a.add(&b, &mut work),
                        Op::Subtract => a.sub(&b, &mut work),
                        Op::Multiply => a.mul(&b, &mut work),
                        Op::Divide => a.div(&b, &mut work),
                        Op::Equal => return Ok(scalar(ValueOut::Bool(a == b))),
                        Op::Compare => {
                            let difference = a.sub(&b, &mut work).map_err(event_error)?;
                            return Ok(scalar(ValueOut::I64(if difference.is_zero() {
                                0
                            } else if difference.is_negative() {
                                -1
                            } else {
                                1
                            })));
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
    .map_err(event_error)?;
    let bytes = result.to_bytes(&mut work).map_err(event_error)?;
    control.checkpoint()?;
    Ok(Output::Bytes(QueuedBytes { bytes }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_exact_rational(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    inputs: Array,
    argument: f64,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = Op::parse(&operation).ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
    if inputs.len() as usize != op.arity() {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let inputs = copy.finish(copy.blobs(&inputs, MAX_EVENT_BYTES, 2))?;
            Ok(Box::new(move |work: &WorkContext| {
                let result = execute(op, &inputs, argument, work)?;
                work.checkpoint()?;
                Ok(result)
            }) as crate::runtime::Work)
        })
        .map_err(|e| thrown(env, e))?;
    Ok(operation_handle(runtime, operation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::work::WorkError;
    #[test]
    fn canonical_scalars_keep_exactness_and_refuse_invalid_or_cancelled_work() {
        let work = WorkContext::new();
        let Output::Bytes(a) = execute(Op::Decimal, &[b"0.1".to_vec()], 0.0, &work).unwrap() else {
            panic!()
        };
        let Output::Bytes(b) = execute(Op::Binary64, &[], 0.1, &work).unwrap() else {
            panic!()
        };
        assert_ne!(a.bytes, b.bytes);
        assert!(matches!(
            execute(Op::Fraction, &[b"1".to_vec(), b"0".to_vec()], 0.0, &work),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        work.cancel();
        assert!(matches!(
            execute(Op::Validate, &[a.bytes], 0.0, &work),
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
    }
}
