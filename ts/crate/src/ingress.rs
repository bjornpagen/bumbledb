//! Owned wire data awaiting worker-side semantic admission. No N-API value,
//! borrowed backing, unchecked Event or forged native import crosses this stage.
use std::cell::Cell;

use bumbledb::event::{Capacity, Error as EventError};
use bumbledb::work::{WorkContext, WorkError};
use bumbledb::{Value, schema::ValueType};
use napi::bindgen_prelude::{Array, Env, Object, Unknown};

use crate::marshal::{self, output_vec};
use crate::runtime::{QueuedBytes, RuntimeError};

pub(crate) mod query;
#[cfg(test)]
mod tests;

pub(crate) const MAX_EVENT_BYTES: usize = 16 * 1024 * 1024;

pub(crate) fn event_error(error: EventError) -> RuntimeError {
    match error {
        EventError::Cancelled => WorkError::Cancelled.into(),
        EventError::Allocation => WorkError::Allocation.into(),
        other => crate::db_wire::engine_error(&bumbledb::Error::Event(other)),
    }
}

/// Preserve work/resource errors across the N-API shape parser. Shape errors
/// remain `InvalidArgument`; a cancelled copy never becomes a malformed value.
pub(crate) struct CopyContext<'a> {
    pub env: Env,
    pub work: &'a WorkContext,
    error: Cell<Option<RuntimeError>>,
}
impl<'a> CopyContext<'a> {
    pub fn new(env: Env, work: &'a WorkContext) -> Self {
        Self {
            env,
            work,
            error: Cell::new(None),
        }
    }
    pub fn finish<T>(&self, result: napi::Result<T>) -> Result<T, RuntimeError> {
        result.map_err(|_| self.error.take().unwrap_or(RuntimeError::InvalidArgument))
    }
    pub(crate) fn checked<T>(&self, result: Result<T, RuntimeError>) -> napi::Result<T> {
        result.map_err(|error| {
            self.error.set(Some(error));
            marshal::err("wire input copy refused".into())
        })
    }
    pub fn checkpoint(&self) -> napi::Result<()> {
        self.checked(self.work.checkpoint().map_err(Into::into))
    }
    pub fn bytes(&self, input: Unknown, maximum: usize) -> napi::Result<Vec<u8>> {
        self.checkpoint()?;
        let bytes = crate::runtime_wire::unshared_input(self.env, input)?;
        if bytes.len() > maximum {
            return self.checked(Err(event_error(EventError::Capacity(
                Capacity::DescriptorBytes,
            ))));
        }
        Ok(self
            .checked(QueuedBytes::copy_from(self.work, &bytes))?
            .bytes)
    }
    /// A bounded operand roster, owned before registration of worker algebra.
    pub fn blobs(
        &self,
        values: &Array,
        maximum: usize,
        items: usize,
    ) -> napi::Result<Vec<Vec<u8>>> {
        self.checkpoint()?;
        if values.len() as usize > items {
            return self.checked(Err(event_error(EventError::Capacity(
                Capacity::DescriptorItems,
            ))));
        }
        let mut result = self.checked(output_vec(values.len() as usize))?;
        let mut remaining = maximum;
        for index in 0..values.len() {
            let bytes = self.bytes(marshal::req_at(values, index, "Event operands")?, remaining)?;
            remaining -= bytes.len();
            result.push(bytes);
        }
        Ok(result)
    }
    pub fn schema_value(
        &self,
        expected: &ValueType,
        input: Unknown,
        relation: &str,
        field: &str,
    ) -> napi::Result<ValueInput> {
        self.checkpoint()?;
        if *expected == ValueType::Event {
            return Ok(ValueInput::Event(self.bytes(input, MAX_EVENT_BYTES)?));
        }
        marshal::schema_value_in(expected, &input, relation, field).map(ValueInput::Scalar)
    }
    pub fn tagged_value(&self, input: &Object) -> napi::Result<ValueInput> {
        self.checkpoint()?;
        if marshal::req_text(input, "kind", "value")? == crate::tags::value::EVENT {
            return Ok(ValueInput::Event(self.bytes(
                marshal::req(input, "value", "Event value")?,
                MAX_EVENT_BYTES,
            )?));
        }
        marshal::tagged_value(input).map(ValueInput::Scalar)
    }
}

#[derive(Debug)]
pub(crate) enum ValueInput {
    Scalar(Value),
    Event(Vec<u8>),
}

#[derive(Debug)]
pub(crate) struct ImportInput(pub Vec<u8>);

#[derive(Debug)]
pub(crate) enum ParamInput {
    Scalar(ValueInput),
    Set(Vec<ValueInput>),
}

pub(crate) trait Admit: Sized {
    type Output;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError>;
}
impl Admit for ValueInput {
    type Output = Value;
    fn admit(self, work: &WorkContext) -> Result<Value, RuntimeError> {
        work.checkpoint()?;
        match self {
            Self::Scalar(value) => Ok(value),
            Self::Event(bytes) => bumbledb::Event::from_bytes(&bytes, work)
                .map(Value::Event)
                .map_err(event_error),
        }
    }
}
impl Admit for ImportInput {
    type Output = bumbledb::EventImport;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        bumbledb::EventImport::from_bytes(
            &self.0,
            bumbledb::event::DescriptorLimits::default(),
            work,
        )
        .map_err(event_error)
    }
}
impl Admit for ParamInput {
    type Output = marshal::OwnedParam;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(match self {
            Self::Scalar(value) => marshal::OwnedParam::Scalar(value.admit(work)?),
            Self::Set(values) => marshal::OwnedParam::Set(values.admit(work)?),
        })
    }
}
impl<T: Admit> Admit for Vec<T> {
    type Output = Vec<T::Output>;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        let mut result = output_vec(self.len())?;
        for value in self {
            result.push(value.admit(work)?);
        }
        Ok(result)
    }
}
impl<T: Admit> Admit for Box<T> {
    type Output = Box<T::Output>;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        Ok(Box::new((*self).admit(work)?))
    }
}
impl<T: Admit> Admit for Option<T> {
    type Output = Option<T::Output>;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        self.map(|value| value.admit(work)).transpose()
    }
}
impl<T: Admit> Admit for bumbledb::NonEmpty<T> {
    type Output = bumbledb::NonEmpty<T::Output>;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        Ok(bumbledb::NonEmpty::new(
            self.first.admit(work)?,
            self.rest.admit(work)?,
        ))
    }
}
impl Admit for bumbledb::FieldId {
    type Output = Self;
    fn admit(self, work: &WorkContext) -> Result<Self, RuntimeError> {
        work.checkpoint()?;
        Ok(self)
    }
}
impl<A: Admit, B: Admit> Admit for (A, B) {
    type Output = (A::Output, B::Output);
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        Ok((self.0.admit(work)?, self.1.admit(work)?))
    }
}

/// Install the admission phase inside a worker-affine snapshot operation.
pub(crate) fn snapshot_work<T: Admit + Send + 'static>(
    input: T,
    next: impl FnOnce(T::Output) -> crate::runtime::session::SnapshotWork + Send + 'static,
) -> crate::runtime::session::SnapshotWork {
    Box::new(move |work, access| next(input.admit(work)?)(work, access))
}
