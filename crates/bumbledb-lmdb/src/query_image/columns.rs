use std::collections::BTreeMap;

#[cfg(test)]
use crate::EncodedOwned;
use crate::query_image::{EncodedRef, FactId, FieldId, FieldImage};
use crate::{Error, Result};

/// Typed fixed-width column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedColumn<T> {
    field: FieldId,
    values: Vec<T>,
}

impl<T> FixedColumn<T> {
    fn new(field: FieldId, values: Vec<T>) -> Self {
        Self { field, values }
    }

    /// Field ID stored by this column.
    #[cfg(test)]
    pub fn field(&self) -> FieldId {
        self.field
    }

    /// Number of encoded values in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl<T: Copy> FixedColumn<T> {
    /// Returns a copied value by fact ID.
    #[cfg(test)]
    #[inline]
    pub fn get(&self, fact: FactId) -> Option<T> {
        self.values.get(fact.0 as usize).copied()
    }

    /// Returns a borrowed value by fact ID.
    #[inline]
    pub fn get_ref(&self, fact: FactId) -> Option<&T> {
        self.values.get(fact.0 as usize)
    }
}

/// Builder for fixed-width encoded column images.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EncodedColumnBuilder {
    Bool {
        field: FieldId,
        values: Vec<[u8; 1]>,
    },
    Fixed8 {
        field: FieldId,
        values: Vec<[u8; 8]>,
    },
    Fixed16 {
        field: FieldId,
        values: Vec<[u8; 16]>,
    },
}

impl EncodedColumnBuilder {
    pub(crate) fn with_capacity(field: FieldId, width: usize, capacity: usize) -> Result<Self> {
        Ok(match width {
            1 => Self::Bool {
                field,
                values: Vec::with_capacity(capacity),
            },
            8 => Self::Fixed8 {
                field,
                values: Vec::with_capacity(capacity),
            },
            16 => Self::Fixed16 {
                field,
                values: Vec::with_capacity(capacity),
            },
            _ => return Err(Error::internal(format!("unsupported column width {width}"))),
        })
    }

    pub(crate) fn append_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        match self {
            Self::Bool { values, .. } => values.push(exact_array::<1>(bytes)?),
            Self::Fixed8 { values, .. } => values.push(exact_array::<8>(bytes)?),
            Self::Fixed16 { values, .. } => values.push(exact_array::<16>(bytes)?),
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn append_encoded_owned(&mut self, value: &EncodedOwned) -> Result<()> {
        self.append_bytes(value.as_bytes())
    }

    #[cfg(test)]
    pub(crate) fn extend_flat_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let width = self.width();
        if width == 0 || !bytes.len().is_multiple_of(width) {
            return Err(Error::corrupt("column byte width mismatch"));
        }
        for chunk in bytes.chunks_exact(width) {
            self.append_bytes(chunk)?;
        }
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Bool { values, .. } => values.len(),
            Self::Fixed8 { values, .. } => values.len(),
            Self::Fixed16 { values, .. } => values.len(),
        }
    }

    #[cfg(test)]
    pub(crate) fn width(&self) -> usize {
        match self {
            Self::Bool { .. } => 1,
            Self::Fixed8 { .. } => 8,
            Self::Fixed16 { .. } => 16,
        }
    }

    pub(crate) fn finish(self) -> ColumnImage {
        match self {
            Self::Bool { field, values } => ColumnImage::Bool(FixedColumn::new(field, values)),
            Self::Fixed8 { field, values } => ColumnImage::Fixed8(FixedColumn::new(field, values)),
            Self::Fixed16 { field, values } => {
                ColumnImage::Fixed16(FixedColumn::new(field, values))
            }
        }
    }
}

pub(super) fn encoded_column_builders(
    fields: &[FieldImage],
    capacity: usize,
) -> Result<BTreeMap<FieldId, EncodedColumnBuilder>> {
    fields
        .iter()
        .map(|field| {
            Ok((
                field.id,
                EncodedColumnBuilder::with_capacity(field.id, field.width, capacity)?,
            ))
        })
        .collect()
}

pub(super) fn finish_column_builders(
    builders: BTreeMap<FieldId, EncodedColumnBuilder>,
) -> BTreeMap<FieldId, ColumnImage> {
    builders
        .into_iter()
        .map(|(field, builder)| (field, builder.finish()))
        .collect()
}

/// Encoded fixed-width column image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnImage {
    /// Boolean/one-byte fixed-width column.
    Bool(FixedColumn<[u8; 1]>),
    /// Eight-byte fixed-width column.
    Fixed8(FixedColumn<[u8; 8]>),
    /// Sixteen-byte fixed-width column.
    Fixed16(FixedColumn<[u8; 16]>),
}

impl ColumnImage {
    #[cfg(test)]
    pub(crate) fn from_flat_bytes(field: FieldId, width: usize, bytes: &[u8]) -> Result<Self> {
        let mut builder = EncodedColumnBuilder::with_capacity(field, width, bytes.len() / width)?;
        builder.extend_flat_bytes(bytes)?;
        Ok(builder.finish())
    }

    pub(super) fn encoded(&self, fact: FactId) -> Option<EncodedRef<'_>> {
        match self {
            ColumnImage::Bool(column) => column.get_ref(fact).map(EncodedRef::One),
            ColumnImage::Fixed8(column) => column.get_ref(fact).map(EncodedRef::Eight),
            ColumnImage::Fixed16(column) => column.get_ref(fact).map(EncodedRef::Sixteen),
        }
    }

    /// Field ID stored by this column.
    #[cfg(test)]
    pub fn field(&self) -> FieldId {
        match self {
            ColumnImage::Bool(column) => column.field(),
            ColumnImage::Fixed8(column) => column.field(),
            ColumnImage::Fixed16(column) => column.field(),
        }
    }

    /// Number of values in this column.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        match self {
            ColumnImage::Bool(column) => column.len(),
            ColumnImage::Fixed8(column) => column.len(),
            ColumnImage::Fixed16(column) => column.len(),
        }
    }

    /// True when this column has no values.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fixed encoded width of values in this column.
    #[cfg(test)]
    pub fn width(&self) -> usize {
        match self {
            ColumnImage::Bool(_) => 1,
            ColumnImage::Fixed8(_) => 8,
            ColumnImage::Fixed16(_) => 16,
        }
    }

    pub(super) fn byte_len(&self) -> usize {
        match self {
            ColumnImage::Bool(column) => column.len(),
            ColumnImage::Fixed8(column) => column.len() * 8,
            ColumnImage::Fixed16(column) => column.len() * 16,
        }
    }

    #[cfg(test)]
    pub(super) fn hash_into(&self, hasher: &mut blake3::Hasher) {
        match self {
            ColumnImage::Bool(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed8(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed16(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
        }
    }
}

fn exact_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::corrupt("query image column width mismatch"))
}
