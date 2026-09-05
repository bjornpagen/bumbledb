//! Portable logical rows, independent of LMDB keys, dictionary IDs, and hosts.
//!
//! The enclosing schema supplies field types; every field also has an explicit
//! scalar tag. Integers, lengths, and canonical F64 payloads use big endian.
//! There is no padding. Fixed integer intervals use their logical endpoints,
//! not the storage-only start compression. This is the core codec imported by
//! history, not another log-owned value vocabulary.
use crate::schema::{FieldDescriptor, ValueType, value_matches};
use crate::work::{ByteKind, ByteReservation};
use crate::{F64, Id128, Interval, Value, WorkContext, WorkError};

/// The canonical bounded named-scalar record — the core codec the log's
/// declared `CommandResult` slot frames verbatim (C01; chapter 30).
pub mod result;

pub(crate) mod field;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowError {
    Work(WorkError),
    Arity,
    Type { field: usize },
    Truncated,
    TrailingBytes,
    InvalidTag { field: usize },
    InvalidBool { field: usize },
    NonCanonicalFloat { field: usize },
    InvalidInterval { field: usize },
    InvalidUtf8 { field: usize },
    LengthOverflow,
    Allocation,
}

impl From<WorkError> for RowError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl std::fmt::Display for RowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical row: {self:?}")
    }
}
impl std::error::Error for RowError {}

/// Canonical, owned, schema-checked bytes. No unvalidated constructor or raw
/// mutable view exists. The reservation lives exactly as long as its bytes.
#[derive(Debug)]
pub struct CanonicalRow {
    bytes: Vec<u8>,
    _reservation: ByteReservation,
}

impl CanonicalRow {
    /// Checks and owns caller values before they can enter a draft.
    /// # Errors
    /// Rejects wrong shape or insufficient input/working allowance.
    #[expect(
        clippy::too_many_lines,
        reason = "the per-type encode arms are one linear wire table"
    )]
    pub fn encode(
        fields: &[FieldDescriptor],
        values: &[Value],
        work: &WorkContext,
    ) -> Result<Self, RowError> {
        work.rows(1)?;
        if fields.len() != values.len() || fields.len() > usize::from(u16::MAX) {
            return Err(RowError::Arity);
        }
        let mut size = 2usize;
        for (field, (descriptor, value)) in fields.iter().zip(values).enumerate() {
            work.step(1)?;
            let payload = match value {
                Value::Bool(_) => 1,
                Value::U64(_) | Value::I64(_) | Value::F64(_) => 8,
                Value::Id128(_) => 16,
                Value::String(text) => text.len().checked_add(8).ok_or(RowError::LengthOverflow)?,
                Value::FixedBytes(bytes) => {
                    bytes.len().checked_add(8).ok_or(RowError::LengthOverflow)?
                }
                Value::IntervalU64(v) => {
                    if v.start() >= v.end() {
                        return Err(RowError::InvalidInterval { field });
                    }
                    16
                }
                Value::IntervalI64(v) => {
                    if v.start() >= v.end() {
                        return Err(RowError::InvalidInterval { field });
                    }
                    16
                }
                Value::IntervalF64(v) => {
                    if v.start().is_nan() || v.end().is_nan() || v.start() >= v.end() {
                        return Err(RowError::InvalidInterval { field });
                    }
                    16
                }
            };
            value_matches(value, &descriptor.value_type).map_err(|_| RowError::Type { field })?;
            size = size
                .checked_add(1)
                .and_then(|n| n.checked_add(payload))
                .ok_or(RowError::LengthOverflow)?;
        }
        work.input(size as u64)?;
        let reservation = work.reserve(ByteKind::Working, size as u64)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| RowError::Allocation)?;
        bytes.extend_from_slice(
            &u16::try_from(values.len())
                .map_err(|_| RowError::Arity)?
                .to_be_bytes(),
        );
        for value in values {
            work.step(1)?;
            match value {
                Value::Bool(v) => bytes.extend_from_slice(&[0, u8::from(*v)]),
                Value::U64(v) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&v.to_be_bytes());
                }
                Value::I64(v) => {
                    bytes.push(2);
                    bytes.extend_from_slice(&v.to_be_bytes());
                }
                Value::F64(v) => {
                    bytes.push(3);
                    bytes.extend_from_slice(&v.to_be_bytes());
                }
                Value::String(v) => {
                    bytes.push(4);
                    append_bytes(&mut bytes, v.as_bytes(), work)?;
                }
                Value::FixedBytes(v) => {
                    bytes.push(5);
                    append_bytes(&mut bytes, v, work)?;
                }
                Value::IntervalU64(v) => {
                    bytes.push(6);
                    bytes.extend_from_slice(&v.start().to_be_bytes());
                    bytes.extend_from_slice(&v.end().to_be_bytes());
                }
                Value::IntervalI64(v) => {
                    bytes.push(7);
                    bytes.extend_from_slice(&v.start().to_be_bytes());
                    bytes.extend_from_slice(&v.end().to_be_bytes());
                }
                Value::Id128(v) => {
                    bytes.push(8);
                    bytes.extend_from_slice(v.as_bytes());
                }
                Value::IntervalF64(v) => {
                    // Wire endpoints are the canonical binary64 payload bits,
                    // big endian — never the index order keys.
                    bytes.push(9);
                    bytes.extend_from_slice(&v.start().to_be_bytes());
                    bytes.extend_from_slice(&v.end().to_be_bytes());
                }
            }
        }
        debug_assert_eq!(bytes.len(), size);
        Ok(Self {
            bytes,
            _reservation: reservation,
        })
    }

    /// Strict wire parsing: alternative NaNs/negative zero, malformed scalar
    /// widths, trailing bytes and schema disagreement all refuse.
    /// # Errors
    /// Returns the first malformed field or resource failure, before owning bytes.
    pub fn parse(
        fields: &[FieldDescriptor],
        bytes: &[u8],
        work: &WorkContext,
    ) -> Result<Self, RowError> {
        work.rows(1)?;
        work.input(bytes.len() as u64)?;
        validate(fields, bytes, work)?;
        let reservation = work.reserve(ByteKind::Working, bytes.len() as u64)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| RowError::Allocation)?;
        for chunk in bytes.chunks(COPY_QUANTUM) {
            work.step(chunk.len() as u64)?;
            owned.extend_from_slice(chunk);
        }
        Ok(Self {
            bytes: owned,
            _reservation: reservation,
        })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl AsRef<[u8]> for CanonicalRow {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

// Work polling granularity, not a database/row-size limit. At most this many
// bytes are copied/UTF-8 checked without returning to the operation ledger.
const COPY_QUANTUM: usize = 4096;

fn append_bytes(out: &mut Vec<u8>, input: &[u8], work: &WorkContext) -> Result<(), RowError> {
    out.extend_from_slice(&(input.len() as u64).to_be_bytes());
    for chunk in input.chunks(COPY_QUANTUM) {
        work.step(chunk.len() as u64)?;
        out.extend_from_slice(chunk);
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
}
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], RowError> {
        let (head, rest) = self
            .bytes
            .split_at_checked(len)
            .ok_or(RowError::Truncated)?;
        self.bytes = rest;
        Ok(head)
    }
    fn word<const N: usize>(&mut self) -> Result<[u8; N], RowError> {
        self.take(N)?.try_into().map_err(|_| RowError::Truncated)
    }
    fn blob(&mut self) -> Result<&'a [u8], RowError> {
        let len = usize::try_from(u64::from_be_bytes(self.word()?))
            .map_err(|_| RowError::LengthOverflow)?;
        self.take(len)
    }
}

pub(crate) fn validate(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    work: &WorkContext,
) -> Result<(), RowError> {
    walk(fields, bytes, work, None)
}

/// Bridge-facing decoded row. Not embedding API.
///
/// The reservation covers the decoded values for as long as the owner
/// lives. Borrow [`DecodedRow::values`]; transfer the whole owner with
/// [`DecodedRow::into_owner`]. There is no owning `values` / `into_values`
/// / `into_parts` escape that refunds while the payload remains live.
#[doc(hidden)]
#[derive(Debug)]
pub struct DecodedRow {
    values: Vec<Value>,
    reservation: ByteReservation,
}

impl DecodedRow {
    /// Borrow the decoded values; the row's reservation covers them.
    /// There is no owning extraction: transfer the whole [`DecodedRow`].
    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    #[must_use]
    pub fn charged_bytes(&self) -> u64 {
        self.reservation.bytes()
    }

    /// Transfer the charged owner. Charge and payload stay together (C2).
    #[must_use]
    pub fn into_owner(self) -> Self {
        self
    }
}

/// Bridge-facing strict canonical decode (the ONE row decoder).
/// Not embedding API.
#[doc(hidden)]
pub fn decode(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    work: &WorkContext,
) -> Result<DecodedRow, RowError> {
    let size = fields
        .len()
        .checked_mul(std::mem::size_of::<Value>())
        .and_then(|n| n.checked_add(bytes.len()))
        .ok_or(RowError::LengthOverflow)?;
    let reservation = work.reserve(ByteKind::Working, size as u64)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(fields.len())
        .map_err(|_| RowError::Allocation)?;
    walk(fields, bytes, work, Some(&mut values))?;
    Ok(DecodedRow {
        values,
        reservation,
    })
}

fn walk(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    work: &WorkContext,
    mut output: Option<&mut Vec<Value>>,
) -> Result<(), RowError> {
    let mut reader = Reader { bytes };
    if usize::from(u16::from_be_bytes(reader.word()?)) != fields.len() {
        return Err(RowError::Arity);
    }
    for (field, descriptor) in fields.iter().enumerate() {
        work.step(1)?;
        let tag = reader.word::<1>()?[0];
        let value = match tag {
            0 => match reader.word::<1>()?[0] {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(RowError::InvalidBool { field }),
            },
            1 => Value::U64(u64::from_be_bytes(reader.word()?)),
            2 => Value::I64(i64::from_be_bytes(reader.word()?)),
            3 => Value::F64(
                F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| RowError::NonCanonicalFloat { field })?,
            ),
            4 => {
                let blob = reader.blob()?;
                if descriptor.value_type != ValueType::String {
                    return Err(RowError::Type { field });
                }
                let owned = utf8(blob, field, work, output.is_some())?;
                if let (Some(output), Some(text)) = (&mut output, owned) {
                    output.push(Value::String(text.into_boxed_str()));
                }
                continue;
            }
            5 => {
                let blob = reader.blob()?;
                if !matches!(descriptor.value_type, ValueType::FixedBytes {len} if usize::from(len) == blob.len())
                {
                    return Err(RowError::Type { field });
                }
                if let Some(output) = &mut output {
                    let mut owned = Vec::new();
                    owned
                        .try_reserve_exact(blob.len())
                        .map_err(|_| RowError::Allocation)?;
                    for chunk in blob.chunks(COPY_QUANTUM) {
                        work.step(chunk.len() as u64)?;
                        owned.extend_from_slice(chunk);
                    }
                    output.push(Value::FixedBytes(owned.into_boxed_slice()));
                }
                continue;
            }
            6 => field::decode_interval_u64(
                u64::from_be_bytes(reader.word()?),
                u64::from_be_bytes(reader.word()?),
                descriptor,
                field,
            )?,
            7 => field::decode_interval_i64(
                i64::from_be_bytes(reader.word()?),
                i64::from_be_bytes(reader.word()?),
                descriptor,
                field,
            )?,
            8 => Value::Id128(Id128::from_bytes(reader.word()?)),
            9 => field::decode_interval_f64(
                F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| RowError::NonCanonicalFloat { field })?,
                F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| RowError::NonCanonicalFloat { field })?,
                descriptor,
                field,
            )?,
            _ => return Err(RowError::InvalidTag { field }),
        };
        if !matches!(tag, 6 | 7 | 9) {
            value_matches(&value, &descriptor.value_type).map_err(|_| RowError::Type { field })?;
        }
        if let Some(output) = &mut output {
            output.push(value);
        }
    }
    if !reader.bytes.is_empty() {
        return Err(RowError::TrailingBytes);
    }
    Ok(())
}

// Validate UTF-8 in bounded chunks; only at most three trailing code-point
// bytes cross a polling boundary. Materialization uses those same checked
// chunks, with no second unbounded scan or unsafe string constructor.
fn utf8(
    mut remaining: &[u8],
    field: usize,
    work: &WorkContext,
    own: bool,
) -> Result<Option<String>, RowError> {
    let mut owned = if own {
        let mut text = String::new();
        text.try_reserve_exact(remaining.len())
            .map_err(|_| RowError::Allocation)?;
        Some(text)
    } else {
        None
    };
    while !remaining.is_empty() {
        let end = remaining.len().min(COPY_QUANTUM);
        work.step(end as u64)?;
        let (text, consumed) = match std::str::from_utf8(&remaining[..end]) {
            Ok(text) => (text, end),
            Err(error) if error.error_len().is_none() && end < remaining.len() => {
                let valid = error.valid_up_to();
                (
                    std::str::from_utf8(&remaining[..valid])
                        .map_err(|_| RowError::InvalidUtf8 { field })?,
                    valid,
                )
            }
            Err(_) => return Err(RowError::InvalidUtf8 { field }),
        };
        if let Some(owned) = &mut owned {
            owned.push_str(text);
        }
        remaining = &remaining[consumed..];
    }
    Ok(owned)
}

/// Canonical row owner for stable logical fact ordering (C4). Independent
/// of local row ids, cursor order and reminting — the sort key for bounded
/// diagnostic selection before truncation. Compare through
/// [`CanonicalRow::as_bytes`]; retain this owner. Do not copy the bytes
/// out from under the charge.
/// # Errors
/// Rejects wrong shape or insufficient work allowance.
pub fn fact_sort_key(
    fields: &[FieldDescriptor],
    values: &[Value],
    work: &WorkContext,
) -> Result<CanonicalRow, RowError> {
    CanonicalRow::encode(fields, values, work)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod f3c_accounting;
