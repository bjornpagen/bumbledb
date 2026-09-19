//! Cumulatively bounded, owned Event/descriptor fields for worker inspection.
use crate::ingress::event_error;
use crate::marshal::output_vec;
use crate::runtime::RuntimeError;
use bumbledb::event::{AdmittedDescriptor, Capacity, Descriptor, DescriptorLimits, Error, Event};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

pub struct Inspection {
    pub(crate) kind: &'static str,
    pub(crate) fields: Vec<(&'static str, Vec<u8>)>,
    pub(crate) lists: Vec<(&'static str, Vec<Vec<u8>>)>,
}
pub(crate) struct Export<'a> {
    value: Inspection,
    limits: DescriptorLimits,
    bytes: usize,
    items: usize,
    work: &'a WorkContext,
}
impl<'a> Export<'a> {
    pub(crate) fn new(
        kind: &'static str,
        limits: DescriptorLimits,
        work: &'a WorkContext,
    ) -> Result<Self, RuntimeError> {
        let mut result = Self {
            value: Inspection {
                kind,
                fields: Vec::new(),
                lists: Vec::new(),
            },
            limits,
            bytes: limits.bytes,
            items: limits.items,
            work,
        };
        result.reserve(0)?;
        Ok(result)
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), RuntimeError> {
        self.work.checkpoint()?;
        self.items = self
            .items
            .checked_sub(1)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorItems)))?;
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
        Ok(())
    }
    fn field(&mut self, name: &'static str, bytes: Vec<u8>) -> Result<(), RuntimeError> {
        self.reserve(bytes.len())?;
        self.value
            .fields
            .try_reserve(1)
            .map_err(|_| RuntimeError::Work(bumbledb::work::WorkError::Allocation))?;
        self.value.fields.push((name, bytes));
        Ok(())
    }
    pub(crate) fn event(&mut self, name: &'static str, event: &Event) -> Result<(), RuntimeError> {
        self.field(name, event.to_bytes(self.work).map_err(event_error)?)
    }
    pub(crate) fn descriptor(
        &mut self,
        name: &'static str,
        value: &AdmittedDescriptor,
    ) -> Result<(), RuntimeError> {
        let bytes = Descriptor::capture(value, self.limits, self.work)
            .map_err(event_error)?
            .to_bytes(self.limits, self.work)
            .map_err(event_error)?;
        self.field(name, bytes)
    }
    pub(crate) fn list(
        &mut self,
        name: &'static str,
        len: usize,
        event: impl Fn(usize) -> bumbledb::event::Result<Event>,
    ) -> Result<(), RuntimeError> {
        self.reserve(0)?;
        if len > self.items {
            return Err(event_error(Error::Capacity(Capacity::DescriptorItems)));
        }
        let mut values = output_vec(len)?;
        for i in 0..len {
            let value = event(i)
                .map_err(event_error)?
                .to_bytes(self.work)
                .map_err(event_error)?;
            self.reserve(value.len())?;
            values.push(value);
        }
        self.value
            .lists
            .try_reserve(1)
            .map_err(|_| RuntimeError::Work(bumbledb::work::WorkError::Allocation))?;
        self.value.lists.push((name, values));
        Ok(())
    }
    pub(crate) fn finish(self) -> Result<Inspection, RuntimeError> {
        self.work.checkpoint()?;
        Ok(self.value)
    }
}
impl Inspection {
    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("kind", self.kind)?;
        for (name, bytes) in self.fields {
            object.set(name, Uint8Array::from(bytes))?;
        }
        for (name, list) in self.lists {
            let mut values =
                output_vec(list.len()).map_err(|e| crate::runtime_wire::thrown(*env, e))?;
            values.extend(list.into_iter().map(Uint8Array::from));
            object.set(name, values)?;
        }
        Ok(object)
    }
}
