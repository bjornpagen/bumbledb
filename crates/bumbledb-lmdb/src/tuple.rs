#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::base_image::RelationBaseImage;
use crate::query::model::AtomOccurrenceId;

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
        Ok(EncodedTuple { bytes })
    }

    pub(crate) fn tuple_from_base_offset(
        &self,
        image: &RelationBaseImage,
        offset: usize,
    ) -> Result<EncodedTuple, TupleError> {
        let mut bytes = Vec::with_capacity(self.encoded_width());
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
            push_checked(&mut bytes, value, field.width)?;
        }
        Ok(EncodedTuple { bytes })
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
        Ok(Self { bytes })
    }

    pub(crate) fn as_ref(&self) -> EncodedTupleRef<'_> {
        EncodedTupleRef { bytes: &self.bytes }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EncodedTupleRef<'a> {
    bytes: &'a [u8],
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
    fn iter(&self) -> Vec<EncodedTuple>;
    fn iter_batch(&self, batch_size: usize) -> Vec<Vec<EncodedTuple>> {
        let batch_size = batch_size.max(1);
        let tuples = self.iter();
        tuples
            .chunks(batch_size)
            .map(<[EncodedTuple]>::to_vec)
            .collect()
    }
    fn get(&self, tuple: &EncodedTuple) -> Option<Self::Child<'_>>;
    fn key_count(&self) -> KeyCountEstimate;
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TupleError {
    #[error("unsupported tuple field width {width}")]
    UnsupportedWidth { width: usize },
    #[error("tuple width mismatch: expected {expected}, got {actual}")]
    TupleWidthMismatch { expected: usize, actual: usize },
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
