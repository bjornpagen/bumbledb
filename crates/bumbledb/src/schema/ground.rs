//! Owned ground axioms. Portable identity never contains resolver keys. Scalar
//! rows keep their existing dense bytes; Event rows retain checked values and
//! field boundaries alongside the portable bytes, then bind at execution.
use std::borrow::Cow;

use super::{FactLayout, FieldDescriptor};
use crate::Value;
use crate::event::{Capacity, Control, Error, Registry};
use crate::image::intern::InternerHandle;

#[derive(Debug, Clone)]
enum GroundData {
    Fixed,
    Owned {
        ends: Box<[usize]>,
        values: Box<[Value]>,
    },
}

/// A named ground axiom. `fact` is portable schema identity, not a resident
/// query row. Scalar encodings are unchanged. An Event cell is a u32
/// little-endian byte length followed by its complete canonical BEVT value.
#[derive(Debug, Clone)]
pub struct SealedRow {
    pub handle: Box<str>,
    pub fact: Box<[u8]>,
    data: GroundData,
}

impl PartialEq for SealedRow {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.fact == other.fact
    }
}
impl Eq for SealedRow {}

impl SealedRow {
    pub(super) fn from_values(
        handle: Box<str>,
        fields: &[FieldDescriptor],
        mut values: Vec<Value>,
        control: &dyn Control,
    ) -> Result<Self, Error> {
        control.checkpoint()?;
        let has_events = values.iter().any(|value| matches!(value, Value::Event(_)));
        let mut registry = Registry::default();
        let mut fact = Vec::new();
        let mut ends = Vec::new();
        if has_events {
            ends.try_reserve_exact(values.len())
                .map_err(|_| Error::Allocation)?;
        }
        for (field, value) in fields.iter().zip(&mut values) {
            control.checkpoint()?;
            if let Value::Event(event) = value {
                *event = registry.intern(event, control)?;
                let bytes = event.to_bytes(control)?;
                let size = u32::try_from(bytes.len())
                    .map_err(|_| Error::Capacity(Capacity::DescriptorBytes))?;
                fact.try_reserve(4 + bytes.len())
                    .map_err(|_| Error::Allocation)?;
                fact.extend_from_slice(&size.to_le_bytes());
                fact.extend_from_slice(&bytes);
            } else {
                fact.try_reserve(64).map_err(|_| Error::Allocation)?;
                crate::encoding::encode_literal(value, field.value_type, &mut fact);
            }
            if has_events {
                ends.push(fact.len());
            }
        }
        Ok(Self {
            handle,
            fact: fact.into_boxed_slice(),
            data: if has_events {
                GroundData::Owned {
                    ends: ends.into_boxed_slice(),
                    values: values.into_boxed_slice(),
                }
            } else {
                GroundData::Fixed
            },
        })
    }

    pub(crate) fn field_bytes(&self, layout: &FactLayout, field: usize) -> &[u8] {
        match &self.data {
            GroundData::Fixed => crate::encoding::field_bytes(layout.encoded(&self.fact), field),
            GroundData::Owned { ends, .. } => {
                let start = field.checked_sub(1).map_or(0, |previous| ends[previous]);
                &self.fact[start..ends[field]]
            }
        }
    }

    pub(crate) fn event(&self, field: usize) -> &crate::Event {
        let GroundData::Owned { values, .. } = &self.data else {
            unreachable!("an Event column has owned ground values")
        };
        let Value::Event(value) = &values[field] else {
            unreachable!("sealed Event column")
        };
        value
    }

    pub(crate) fn values(&self, layout: &FactLayout) -> crate::error::Result<Cow<'_, [Value]>> {
        match &self.data {
            GroundData::Fixed => {
                let mut values = Vec::new();
                values
                    .try_reserve_exact(layout.field_count())
                    .map_err(|_| Error::Allocation)?;
                crate::encoding::decode_values_keyed_into(
                    layout.encoded(&self.fact),
                    &[],
                    &[],
                    |_| unreachable!("closed relations refuse text columns"),
                    &mut values,
                )?;
                Ok(Cow::Owned(values))
            }
            GroundData::Owned { values, .. } => Ok(Cow::Borrowed(values)),
        }
    }

    pub(crate) fn resident(
        &self,
        layout: &FactLayout,
        interner: &InternerHandle<'_>,
    ) -> crate::error::Result<Cow<'_, [u8]>> {
        match &self.data {
            GroundData::Fixed => Ok(Cow::Borrowed(&self.fact)),
            GroundData::Owned { values, .. } => {
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(layout.fact_width())
                    .map_err(|_| Error::Allocation)?;
                for (field, value) in values.iter().enumerate() {
                    if let Value::Event(event) = value {
                        let value = interner.intern_event(event)?;
                        crate::encoding::append_field(
                            crate::encoding::ValueRef::Event(value.key().words()),
                            layout.field_type(field),
                            &mut bytes,
                        );
                    } else {
                        bytes.extend_from_slice(self.field_bytes(layout, field));
                    }
                }
                Ok(Cow::Owned(bytes))
            }
        }
    }
}
