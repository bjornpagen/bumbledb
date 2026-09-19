//! Structural descriptor construction and inspection. The bridge only owns
//! data; the shared descriptor constructors supply every mathematical check.
use bumbledb::event::{AdmittedDescriptor, Descriptor, DescriptorLimits};
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

pub enum DescriptorOutput {
    Description(Descriptor),
    Inspection(inspection::Inspection),
}

#[derive(Clone, Copy)]
enum Op {
    Admit,
    Describe,
    Inspect,
}
enum Input {
    Data(Descriptor),
    Bytes(Vec<u8>),
}

fn execute(op: Op, input: Input, work: &WorkContext) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    let limits = DescriptorLimits::default();
    let data = match input {
        Input::Data(data) => data,
        Input::Bytes(bytes) => Descriptor::from_bytes(&bytes, limits, work).map_err(event_error)?,
    };
    let admitted = data.admit(limits, work).map_err(event_error)?;
    drop(data);
    let result = match op {
        Op::Admit => {
            let bytes = encoded(&admitted, work)?;
            Output::Bytes(QueuedBytes { bytes })
        }
        Op::Describe => {
            let description = Descriptor::capture(&admitted, limits, work).map_err(event_error)?;
            // The full envelope is bounded too, not merely its embedded blobs.
            description.to_bytes(limits, work).map_err(event_error)?;
            Output::EventDescriptor(DescriptorOutput::Description(description))
        }
        Op::Inspect => Output::EventDescriptor(DescriptorOutput::Inspection(inspection::inspect(
            &admitted, work,
        )?)),
    };
    work.checkpoint()?;
    Ok(result)
}

fn encoded(value: &AdmittedDescriptor, work: &WorkContext) -> Result<Vec<u8>, RuntimeError> {
    let limits = DescriptorLimits::default();
    Descriptor::capture(value, limits, work)
        .map_err(event_error)?
        .to_bytes(limits, work)
        .map_err(event_error)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_descriptor(
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
pub fn runtime_event_descriptor_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<DescriptorOutput> {
    match take_output(env, handle)? {
        Output::EventDescriptor(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

impl napi::bindgen_prelude::ToNapiValue for DescriptorOutput {
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
        // SAFETY: the object was created in the live environment for this call.
        unsafe { Object::to_napi_value(env, object) }
    }
}

#[cfg(test)]
mod tests;
