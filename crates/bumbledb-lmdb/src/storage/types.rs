use super::*;

/// A logical fact for the generic storage layer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    pub(super) relation: String,
    pub(super) values: BTreeMap<String, Value>,
}

impl Fact {
    /// Creates a fact for `relation`.
    pub fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        Self {
            relation: relation.into(),
            values: values
                .into_iter()
                .map(|(field, value)| (field.into(), value))
                .collect(),
        }
    }

    /// Returns this fact's relation name.
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Returns a field value.
    pub fn value(&self, field: &str) -> Option<&Value> {
        self.values.get(field)
    }

    /// Returns all fact values keyed by field name.
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}

/// Field values used to build an index prefix.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FieldValues {
    pub(super) relation: String,
    pub(super) values: BTreeMap<String, Value>,
}

#[cfg(test)]
impl FieldValues {
    /// Creates index-prefix field values for `relation`.
    pub(super) fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        Self {
            relation: relation.into(),
            values: values
                .into_iter()
                .map(|(field, value)| (field.into(), value))
                .collect(),
        }
    }
}

/// Logical storage value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 64-bit integer.
    I64(i64),
    /// Typed nominal serial.
    Serial(u64),
    /// UTC timestamp micros.
    Timestamp(TimestampMicros),
    /// Fixed-scale decimal raw value.
    Decimal(DecimalRaw),
    /// Closed enum represented as a stable one-byte code.
    Enum(u8),
    /// String to intern.
    String(String),
    /// Bytes to intern.
    Bytes(Vec<u8>),
}

impl Value {
    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::U64(_) => "u64",
            Value::I64(_) => "i64",
            Value::Serial(_) => "serial",
            Value::Timestamp(_) => "timestamp",
            Value::Decimal(_) => "decimal",
            Value::Enum(_) => "enum",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
        }
    }
}

/// Encoded component from an access key.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedComponent {
    /// Field name.
    pub field_name: String,
    /// Encoded bytes for this field in the index key.
    pub bytes: Vec<u8>,
}

/// A fact yielded from an index scan.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FactCursorRecord {
    /// Decoded logical fact.
    pub fact: Fact,
    /// Encoded components in index-key order.
    pub encoded_components: Vec<EncodedComponent>,
}

/// Result of inserting a fact into a relation-as-set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    /// The fact was newly inserted.
    Inserted,
    /// The exact fact was already present and no storage state changed.
    AlreadyPresent,
}

/// Result of deleting an exact fact from a relation-as-set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// The fact was present and deleted.
    Deleted,
    /// The exact fact was absent and no storage state changed.
    Absent,
}

#[cfg(test)]
impl FactCursorRecord {
    /// Returns an encoded component by field name.
    pub(super) fn encoded_component(&self, field: &str) -> Option<&[u8]> {
        self.encoded_components
            .iter()
            .find(|component| component.field_name == field)
            .map(|component| component.bytes.as_slice())
    }
}

/// Encoded fact component view yielded from an access scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedAccessItem {
    pub(super) key: Vec<u8>,
    pub(super) prefix_len: usize,
}

impl EncodedAccessItem {
    /// Returns the encoded index key bytes.
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// Returns an encoded component by ordinal.
    pub fn component(&self, components: &[AccessComponent], index: usize) -> Option<&[u8]> {
        let mut offset = self.prefix_len;
        for component in components.get(..index)? {
            offset += component.encoded_width;
        }
        let width = components.get(index)?.encoded_width;
        self.key.get(offset..offset + width)
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(super) struct EncodedRange {
    pub(super) offset: usize,
    pub(super) width: usize,
    pub(super) start: Option<Vec<u8>>,
    pub(super) end: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EncodedFact {
    pub(super) relation: RelationId,
    pub(super) bytes: Vec<u8>,
}

impl EncodedFact {
    pub(super) fn field(&self, relation: &RelationDescriptor, name: &str) -> Result<&[u8]> {
        let (offset, width) = field_layout(relation, name)?;
        self.bytes
            .get(offset..offset + width)
            .ok_or_else(|| Error::corrupt("encoded fact width does not match schema"))
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

pub(super) enum InternMode {
    Create,
    Existing,
}
