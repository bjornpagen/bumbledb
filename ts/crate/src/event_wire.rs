//! Event value operations on the existing owned, cancellable executor. All
//! mathematical operations and canonical admission remain in the shared core.
use bumbledb::event::{BoolOp4, Capacity, Error as EventError, Event, Space, SpaceId};
use bumbledb::work::{WorkContext, WorkError};
use napi::bindgen_prelude::{Array, BigInt, Env, External, Function, Unknown};
use napi_derive::napi;

use crate::ingress::{MAX_EVENT_BYTES, event_error};
use crate::marshal::ValueOut;
use crate::runtime::{Output, QueuedOutput, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, thrown, unshared_input,
};

#[derive(Clone, Copy)]
enum Op {
    Space,
    Validate,
    Coordinate,
    Full,
    Empty,
    Complement,
    Restrict,
    Apply,
    Ite,
    Count,
    Signature,
    IsEmpty,
    IsFull,
    Subset,
    Equal,
    Disjoint,
    Contains,
}
impl Op {
    fn parse(op: &str) -> Option<Self> {
        Some(match op {
            "space" => Self::Space,
            "validate" => Self::Validate,
            "coordinate" => Self::Coordinate,
            "full" => Self::Full,
            "empty" => Self::Empty,
            "complement" => Self::Complement,
            "restrict" => Self::Restrict,
            "apply" => Self::Apply,
            "ite" => Self::Ite,
            "count" => Self::Count,
            "signature" => Self::Signature,
            "isEmpty" => Self::IsEmpty,
            "isFull" => Self::IsFull,
            "subset" => Self::Subset,
            "equal" => Self::Equal,
            "disjoint" => Self::Disjoint,
            "contains" => Self::Contains,
            _ => return None,
        })
    }
    const fn arity(self) -> usize {
        match self {
            Self::Restrict
            | Self::Apply
            | Self::Signature
            | Self::Subset
            | Self::Equal
            | Self::Disjoint => 2,
            Self::Ite => 3,
            _ => 1,
        }
    }
    fn argument(self, value: u64) -> Result<(), RuntimeError> {
        let valid = match self {
            Self::Space => value <= 62,
            Self::Coordinate => value < 62,
            Self::Apply => value < 16,
            Self::Contains => true,
            _ => value == 0,
        };
        if valid {
            Ok(())
        } else {
            Err(RuntimeError::InvalidArgument)
        }
    }
}
fn output(value: &Event, control: &WorkContext) -> Result<ValueOut, RuntimeError> {
    let bytes = value.to_bytes(control).map_err(event_error)?;
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(event_error(EventError::Capacity(Capacity::DescriptorBytes)));
    }
    Ok(ValueOut::Event(bytes))
}
fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    argument: u64,
    control: &WorkContext,
) -> Result<ValueOut, RuntimeError> {
    control.checkpoint()?;
    op.argument(argument)?;
    if inputs.len() != op.arity() {
        return Err(RuntimeError::InvalidArgument);
    }
    if inputs.iter().any(|bytes| bytes.len() > MAX_EVENT_BYTES) {
        return Err(event_error(EventError::Capacity(Capacity::DescriptorBytes)));
    }
    if let Op::Space = op {
        let name: [u8; 32] = inputs[0]
            .as_slice()
            .try_into()
            .map_err(|_| RuntimeError::InvalidArgument)?;
        return output(
            &Space::new(
                SpaceId(name),
                u8::try_from(argument).map_err(|_| RuntimeError::InvalidArgument)?,
                control,
            )
            .map_err(event_error)?
            .full(),
            control,
        );
    }
    let mut events = Vec::new();
    events
        .try_reserve_exact(inputs.len())
        .map_err(|_| WorkError::Allocation)?;
    for bytes in inputs {
        events.push(Event::from_bytes(bytes, control).map_err(event_error)?);
    }
    let first = &events[0];
    let space = first.space();
    // Check alignment even for constant functions and empty/full predicates.
    for event in &mut events[1..] {
        *event = event.align_to(&space, control).map_err(event_error)?;
    }
    let first = &events[0];
    let result = match op {
        Op::Space => unreachable!("handled before Event decoding"),
        Op::Validate => first.clone(),
        Op::Full => space.full(),
        Op::Empty => space.empty(),
        Op::Complement => first.complement(),
        Op::Coordinate => space
            .coordinate(
                u8::try_from(argument).map_err(|_| RuntimeError::InvalidArgument)?,
                control,
            )
            .map_err(event_error)?,
        Op::Restrict => space
            .restrict(&events[1], control)
            .map_err(event_error)?
            .full(),
        Op::Apply => first
            .apply(
                BoolOp4::new(u8::try_from(argument).map_err(|_| RuntimeError::InvalidArgument)?)
                    .ok_or(RuntimeError::InvalidArgument)?,
                &events[1],
                control,
            )
            .map_err(event_error)?,
        Op::Ite => first
            .ite(&events[1], &events[2], control)
            .map_err(event_error)?,
        Op::Count => return first.count(control).map(ValueOut::U64).map_err(event_error),
        Op::Contains => {
            return first
                .contains(argument)
                .map(ValueOut::Bool)
                .map_err(event_error);
        }
        Op::IsEmpty => return Ok(ValueOut::Bool(first.is_empty())),
        Op::IsFull => return Ok(ValueOut::Bool(first.is_full())),
        Op::Signature | Op::Subset | Op::Equal | Op::Disjoint => {
            let signature = first.signature(&events[1], control).map_err(event_error)?;
            return Ok(match op {
                Op::Signature => ValueOut::U64(u64::from(signature.bits())),
                Op::Subset => ValueOut::Bool(signature.included()),
                Op::Equal => ValueOut::Bool(signature.equal()),
                Op::Disjoint => ValueOut::Bool(signature.disjoint()),
                _ => unreachable!(),
            });
        }
    };
    output(&result, control)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    inputs: Array,
    argument: BigInt,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = Op::parse(&operation).ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
    let argument = crate::marshal::u64_in(&argument, "Event argument")?;
    op.argument(argument).map_err(|e| thrown(env, e))?;
    if inputs.len() as usize != op.arity() {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let mut marshal_error = None;
    let operation = runtime.submit(WorkContext::new(), notification(callback)?, |context| {
        let prepared = (|| -> napi::Result<Vec<Vec<u8>>> {
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(op.arity())
                .map_err(|_| thrown(env, WorkError::Allocation.into()))?;
            for i in 0..inputs.len() {
                context.checkpoint().map_err(|e| thrown(env, e.into()))?;
                let value: Unknown = inputs
                    .get(i)?
                    .ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
                let bytes = unshared_input(env, value)?;
                if bytes.len() > MAX_EVENT_BYTES {
                    return Err(thrown(
                        env,
                        event_error(EventError::Capacity(Capacity::DescriptorBytes)),
                    ));
                }
                owned.push(
                    crate::runtime::QueuedBytes::copy_from(context, &bytes)
                        .map_err(|e| thrown(env, e))?
                        .bytes,
                );
            }
            Ok(owned)
        })();
        match prepared {
            Ok(owned) => Ok(Box::new(move |context: &WorkContext| {
                let value = execute(op, &owned, argument, context)?;
                context.checkpoint()?;
                Ok(Output::Rows(QueuedOutput {
                    rows: vec![vec![value]],
                }))
            }) as crate::runtime::Work),
            Err(error) => {
                marshal_error = Some(error);
                Err(RuntimeError::InvalidArgument)
            }
        }
    });
    if let Some(error) = marshal_error {
        return Err(error);
    }
    Ok(operation_handle(
        runtime,
        operation.map_err(|e| thrown(env, e))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_operations_check_independent_contexts_and_all_boolean_functions() {
        let context = WorkContext::new();
        let space = Space::new(SpaceId([180; 32]), 2, &context).unwrap();
        let a = space.coordinate(0, &context).unwrap();
        let b = space.coordinate(1, &context).unwrap();
        let inputs = [a.to_bytes(&context).unwrap(), b.to_bytes(&context).unwrap()];
        for mask in 0..16 {
            let ValueOut::Event(bytes) = execute(Op::Apply, &inputs, mask, &context).unwrap()
            else {
                panic!()
            };
            let out = Event::from_bytes(&bytes, &context).unwrap();
            for world in 0..4 {
                assert_eq!(
                    out.contains(world).unwrap(),
                    mask & (1 << (((world & 1) << 1) | (world >> 1))) != 0
                );
            }
        }
        let foreign = Space::new(SpaceId([181; 32]), 2, &context).unwrap();
        for op in [
            Op::Apply,
            Op::Subset,
            Op::Equal,
            Op::Disjoint,
            Op::Signature,
            Op::Restrict,
        ] {
            assert!(matches!(
                execute(
                    op,
                    &[
                        space.empty().to_bytes(&context).unwrap(),
                        foreign.empty().to_bytes(&context).unwrap()
                    ],
                    0,
                    &context
                ),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        context.cancel();
        assert!(matches!(
            execute(Op::Validate, &inputs[..1], 0, &context),
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
    }
}
