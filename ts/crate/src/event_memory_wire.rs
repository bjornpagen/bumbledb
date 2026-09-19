//! Owned possibility-memory recipes. All mathematical checks and graph
//! reconstruction run in the Event core on a cancellable worker.
use bumbledb::event::{
    ArithmeticLimits, BeliefDescriptor, BeliefDescriptorLimits, DescriptorLimits, ExactArithmetic,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, External, Function, Object, Unknown};
use napi_derive::napi;

use crate::ingress::{CopyContext, event_error};
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, take_output, thrown,
};

mod data;
mod inspection;

pub enum MemoryOutput {
    Description(BeliefDescriptor),
    Inspection(inspection::Inspection),
}
#[derive(Clone, Copy)]
enum Op {
    Admit,
    Describe,
    Inspect,
}
enum Input {
    Data(BeliefDescriptor),
    Bytes(Vec<u8>),
}

fn execute(op: Op, input: Input, work: &WorkContext) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    let limits = BeliefDescriptorLimits::default();
    let data = match input {
        Input::Data(data) => data,
        Input::Bytes(bytes) => {
            BeliefDescriptor::from_bytes(&bytes, limits.descriptors, work).map_err(event_error)?
        }
    };
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), work);
    let memory = data.admit(limits, &mut arithmetic).map_err(event_error)?;
    drop(data);
    let output = match op {
        Op::Admit | Op::Describe => {
            let data = BeliefDescriptor::capture(&memory, limits.descriptors, work)
                .map_err(event_error)?;
            let bytes = data
                .to_bytes(limits.descriptors, work)
                .map_err(event_error)?;
            if matches!(op, Op::Admit) {
                Output::Bytes(QueuedBytes { bytes })
            } else {
                Output::EventMemory(MemoryOutput::Description(data))
            }
        }
        Op::Inspect => Output::EventMemory(MemoryOutput::Inspection(inspection::inspect(
            &memory,
            limits.descriptors,
            work,
        )?)),
    };
    work.checkpoint()?;
    Ok(output)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_memory(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    input: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let op = match operation.as_str() {
        "admit" => Op::Admit,
        "describe" => Op::Describe,
        "inspect" => Op::Inspect,
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    };
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let input = match op {
                Op::Admit => Input::Data(copy.finish(data::parse(input, &copy))?),
                Op::Describe | Op::Inspect => {
                    Input::Bytes(copy.finish(copy.bytes(input, DescriptorLimits::default().bytes))?)
                }
            };
            Ok(Box::new(move |work: &WorkContext| execute(op, input, work))
                as crate::runtime::Work)
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_event_memory_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<MemoryOutput> {
    match take_output(env, handle)? {
        Output::EventMemory(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

impl napi::bindgen_prelude::ToNapiValue for MemoryOutput {
    #[expect(
        unsafe_code,
        reason = "N-API requires an unsafe conversion entry; this transfers owned data through its object API"
    )]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        value: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let handle = Env::from_raw(env);
        let object = match value {
            Self::Description(value) => data::object(&handle, value)?,
            Self::Inspection(value) => value.object(&handle)?,
        };
        // SAFETY: this object was created in the live environment for this call.
        unsafe { Object::to_napi_value(env, object) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{
        BeliefLimits, BeliefMemory, EventPartition, PartitionLimits, Space, SpaceId,
    };
    use bumbledb::work::WorkError;

    #[test]
    fn worker_replays_recipe_and_never_publishes_cancelled_or_malformed_memory() {
        let work = WorkContext::new();
        let source = Space::new(SpaceId([90; 32]), 1, &work).unwrap();
        let observations = EventPartition::on(
            &source.full(),
            &[source.full(), source.empty()],
            PartitionLimits::default(),
            &work,
        )
        .unwrap();
        let memory = BeliefMemory::new(
            &[],
            &observations,
            &source.full(),
            BeliefLimits::default(),
            &work,
        )
        .unwrap();
        let data = BeliefDescriptor::capture(&memory, DescriptorLimits::default(), &work).unwrap();
        let bytes = data.to_bytes(DescriptorLimits::default(), &work).unwrap();
        let Output::Bytes(encoded) = execute(Op::Admit, Input::Data(data.clone()), &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(encoded.bytes, bytes);
        let Output::EventMemory(MemoryOutput::Description(restored)) =
            execute(Op::Describe, Input::Bytes(bytes.clone()), &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(restored, data);
        drop(memory);
        drop(source);
        assert!(matches!(
            execute(Op::Inspect, Input::Bytes(bytes.clone()), &work),
            Ok(Output::EventMemory(MemoryOutput::Inspection(_)))
        ));
        assert!(matches!(
            execute(Op::Inspect, Input::Bytes(b"BEBM\x01".to_vec()), &work),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        let mut invalid = data;
        invalid.source = invalid.observations[1].clone();
        assert!(matches!(
            execute(Op::Admit, Input::Data(invalid), &work),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        work.cancel();
        for op in [Op::Admit, Op::Describe, Op::Inspect] {
            assert!(matches!(
                execute(op, Input::Bytes(bytes.clone()), &work),
                Err(RuntimeError::Work(WorkError::Cancelled))
            ));
        }
    }
}
