#![allow(dead_code)]

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::ops::ControlFlow;

use crate::base_image::RelationBaseImage;
use crate::query::model::AtomOccurrenceId;

const INLINE_TUPLE_BYTES: usize = 128;
const TUPLE_BATCH_CAPACITY: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TupleField {
    pub(crate) variable: usize,
    pub(crate) field_id: Option<usize>,
    pub(crate) width: usize,
}

impl TupleField {
    pub(crate) fn new(
        variable: usize,
        field_id: Option<usize>,
        width: usize,
    ) -> Result<Self, TupleError> {
        validate_width(width)?;
        Ok(Self {
            variable,
            field_id,
            width,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TupleSchema {
    pub(crate) fields: Vec<TupleField>,
}

impl TupleSchema {
    pub(crate) fn new(fields: Vec<TupleField>) -> Self {
        Self { fields }
    }

    pub(crate) fn encoded_width(&self) -> usize {
        self.fields.iter().map(|field| field.width).sum()
    }

    pub(crate) fn vars(&self) -> Vec<usize> {
        self.fields.iter().map(|field| field.variable).collect()
    }

    pub(crate) fn tuple_from_bindings(
        &self,
        bindings: &BTreeMap<usize, Vec<u8>>,
    ) -> Result<EncodedTuple, TupleError> {
        let mut bytes = Vec::with_capacity(self.encoded_width());
        for field in &self.fields {
            let value = bindings
                .get(&field.variable)
                .ok_or(TupleError::MissingBinding {
                    variable: field.variable,
                })?;
            push_checked(&mut bytes, value, field.width)?;
        }
        Ok(EncodedTuple::from_bytes(bytes))
    }

    pub(crate) fn tuple_from_base_offset(
        &self,
        image: &RelationBaseImage,
        offset: usize,
    ) -> Result<EncodedTuple, TupleError> {
        let mut bytes = Vec::with_capacity(self.encoded_width());
        self.write_tuple_from_base_offset(image, offset, &mut bytes)?;
        Ok(EncodedTuple::from_bytes(bytes))
    }

    pub(crate) fn write_tuple_from_base_offset(
        &self,
        image: &RelationBaseImage,
        offset: usize,
        bytes: &mut Vec<u8>,
    ) -> Result<(), TupleError> {
        bytes.clear();
        for field in &self.fields {
            let field_id = field.field_id.ok_or(TupleError::MissingFieldId {
                variable: field.variable,
            })?;
            let column = image
                .columns
                .get(&field_id)
                .ok_or(TupleError::MissingColumn { field_id })?;
            let value = column
                .value_at(offset)
                .ok_or(TupleError::OffsetOutOfRange { offset })?;
            push_checked(bytes, value, field.width)?;
        }
        Ok(())
    }

    fn write_tuple_from_base_offset_inline(
        &self,
        image: &RelationBaseImage,
        offset: usize,
        out: &mut [u8],
    ) -> Result<usize, TupleError> {
        let mut written = 0;
        for field in &self.fields {
            let field_id = field.field_id.ok_or(TupleError::MissingFieldId {
                variable: field.variable,
            })?;
            let column = image
                .columns
                .get(&field_id)
                .ok_or(TupleError::MissingColumn { field_id })?;
            let value = column
                .value_at(offset)
                .ok_or(TupleError::OffsetOutOfRange { offset })?;
            if value.len() != field.width {
                return Err(TupleError::ComponentWidthMismatch {
                    expected: field.width,
                    actual: value.len(),
                });
            }
            let end = written + value.len();
            if end > out.len() {
                return Err(TupleError::TupleTooWide {
                    max: out.len(),
                    actual: end,
                });
            }
            out[written..end].copy_from_slice(value);
            written = end;
        }
        Ok(written)
    }

    pub(crate) fn inline_tuple_from_base_offset(
        &self,
        image: &RelationBaseImage,
        offset: usize,
        tuple: &mut InlineTuple,
    ) -> Result<(), TupleError> {
        let len = self.write_tuple_from_base_offset_inline(image, offset, &mut tuple.bytes)?;
        tuple.len = len as u16;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EncodedTuple {
    bytes: Vec<u8>,
}

impl EncodedTuple {
    pub(crate) fn new(schema: &TupleSchema, bytes: Vec<u8>) -> Result<Self, TupleError> {
        if bytes.len() != schema.encoded_width() {
            return Err(TupleError::TupleWidthMismatch {
                expected: schema.encoded_width(),
                actual: bytes.len(),
            });
        }
        Ok(Self::from_bytes(bytes))
    }

    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub(crate) fn as_ref(&self) -> EncodedTupleRef<'_> {
        EncodedTupleRef {
            bytes: self.bytes(),
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl Borrow<[u8]> for EncodedTuple {
    fn borrow(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EncodedTupleRef<'a> {
    bytes: &'a [u8],
}

impl<'a> EncodedTupleRef<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub(crate) fn to_owned_tuple(self) -> EncodedTuple {
        EncodedTuple::from_bytes(self.bytes.to_vec())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TupleCursor {
    pub(crate) position: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InlineTuple {
    len: u16,
    bytes: [u8; INLINE_TUPLE_BYTES],
}

impl InlineTuple {
    fn set(&mut self, bytes: &[u8]) -> Result<(), TupleError> {
        if bytes.len() > self.bytes.len() {
            return Err(TupleError::TupleTooWide {
                max: self.bytes.len(),
                actual: bytes.len(),
            });
        }
        self.len = bytes.len() as u16;
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn as_ref(&self) -> EncodedTupleRef<'_> {
        EncodedTupleRef::new(&self.bytes[..self.len as usize])
    }
}

impl Default for InlineTuple {
    fn default() -> Self {
        Self {
            len: 0,
            bytes: [0; INLINE_TUPLE_BYTES],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TupleBatch {
    pub(crate) tuples: [InlineTuple; TUPLE_BATCH_CAPACITY],
    len: usize,
    pub(crate) exhausted: bool,
}

impl TupleBatch {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn exhausted() -> Self {
        Self {
            exhausted: true,
            ..Self::default()
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<(), TupleError> {
        if self.len >= self.tuples.len() {
            return Ok(());
        }
        self.tuples[self.len].set(bytes)?;
        self.len += 1;
        Ok(())
    }

    pub(crate) fn push_from_base(
        &mut self,
        schema: &TupleSchema,
        image: &RelationBaseImage,
        offset: usize,
    ) -> Result<(), TupleError> {
        if self.len >= self.tuples.len() {
            return Ok(());
        }
        let tuple = &mut self.tuples[self.len];
        let len = schema.write_tuple_from_base_offset_inline(image, offset, &mut tuple.bytes)?;
        tuple.len = len as u16;
        self.len += 1;
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = EncodedTupleRef<'_>> {
        self.tuples[..self.len].iter().map(InlineTuple::as_ref)
    }
}

impl Default for TupleBatch {
    fn default() -> Self {
        Self {
            tuples: [InlineTuple::default(); TUPLE_BATCH_CAPACITY],
            len: 0,
            exhausted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyCountEstimate {
    Exact(usize),
    Estimate(usize),
}

pub(crate) trait GhtSource {
    type Child<'a>: GhtSource
    where
        Self: 'a;

    fn atom(&self) -> Option<AtomOccurrenceId>;
    fn vars(&self) -> &[usize];
    fn try_for_each_tuple<E, F>(&self, f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>;
    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch;
    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>>;
    fn key_count(&self) -> KeyCountEstimate;
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TupleError {
    #[error("unsupported tuple field width {width}")]
    UnsupportedWidth { width: usize },
    #[error("tuple width mismatch: expected {expected}, got {actual}")]
    TupleWidthMismatch { expected: usize, actual: usize },
    #[error("tuple width {actual} exceeds inline batch width {max}")]
    TupleTooWide { max: usize, actual: usize },
    #[error("component width mismatch: expected {expected}, got {actual}")]
    ComponentWidthMismatch { expected: usize, actual: usize },
    #[error("missing binding for variable {variable}")]
    MissingBinding { variable: usize },
    #[error("missing source field id for variable {variable}")]
    MissingFieldId { variable: usize },
    #[error("missing base image column {field_id}")]
    MissingColumn { field_id: usize },
    #[error("base image offset {offset} out of range")]
    OffsetOutOfRange { offset: usize },
}

fn validate_width(width: usize) -> Result<(), TupleError> {
    match width {
        1 | 8 | 16 => Ok(()),
        _ => Err(TupleError::UnsupportedWidth { width }),
    }
}

fn push_checked(out: &mut Vec<u8>, value: &[u8], expected: usize) -> Result<(), TupleError> {
    if value.len() != expected {
        return Err(TupleError::ComponentWidthMismatch {
            expected,
            actual: value.len(),
        });
    }
    out.extend_from_slice(value);
    Ok(())
}

#[cfg(test)]
#[path = "tuple_tests.rs"]
mod tests;
