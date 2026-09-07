//! Portable logical rows, independent of LMDB keys, dictionary IDs, and hosts.
//!
//! The enclosing schema supplies field types; every field also has an explicit
//! scalar tag. Integers, lengths, and canonical F64 payloads use big endian.
//! There is no padding. Fixed integer intervals use their logical endpoints,
//! not the storage-only start compression. This is the core codec imported by
//! history, not another log-owned value vocabulary.
use crate::schema::compiled::{ExactScalarRef, exact_scalar_width, with_exact_scalar_bytes};
use crate::schema::{FieldDescriptor, ValueType, value_matches};
use crate::work::{ByteKind, ByteReservation};
use crate::{F64, Uuid, Value, WorkContext, WorkError};

/// The canonical bounded named-scalar record — the core codec the log's
/// declared `CommandResult` slot frames verbatim.
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
    bytes: Box<[u8]>,
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
        for (chunk, (descriptors, values)) in fields
            .chunks(FIELD_QUANTUM)
            .zip(values.chunks(FIELD_QUANTUM))
            .enumerate()
        {
            work.step(descriptors.len() as u64)?;
            for (offset, (descriptor, value)) in descriptors.iter().zip(values).enumerate() {
                let field = chunk * FIELD_QUANTUM + offset;
                let payload = match value {
                    Value::Bool(_) => 1,
                    Value::U64(_) | Value::I64(_) | Value::F64(_) => 8,
                    Value::Uuid(_) => 16,
                    Value::String(text) => {
                        text.len().checked_add(8).ok_or(RowError::LengthOverflow)?
                    }
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
                value_matches(value, &descriptor.value_type)
                    .map_err(|_| RowError::Type { field })?;
                size = size
                    .checked_add(1)
                    .and_then(|n| n.checked_add(payload))
                    .ok_or(RowError::LengthOverflow)?;
            }
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
        for values in values.chunks(FIELD_QUANTUM) {
            work.step(values.len() as u64)?;
            for value in values {
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
                    Value::Uuid(v) => {
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
        }
        debug_assert_eq!(bytes.len(), size);
        Ok(Self {
            bytes: bytes.into_boxed_slice(),
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
            bytes: owned.into_boxed_slice(),
            _reservation: reservation,
        })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Move immutable bytes only after their charge joins the receiving owner.
    /// This is an internal ownership transfer, never an uncharged extraction.
    pub(crate) fn transfer_to(self, owner: &mut ByteReservation) -> Box<[u8]> {
        let Self {
            bytes,
            _reservation: reservation,
        } = self;
        owner.join(reservation);
        bytes
    }
}

impl AsRef<[u8]> for CanonicalRow {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl std::ops::Deref for CanonicalRow {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

// Work polling granularity, not a database/row-size limit. At most this many
// bytes are copied/UTF-8 checked without returning to the operation ledger.
const COPY_QUANTUM: usize = 4096;

// Prepay one bounded scalar batch instead of reading the clock and updating
// the operation ledger per field. Successful work totals are unchanged;
// variable-width values retain their own COPY_QUANTUM polling inside a batch.
const FIELD_QUANTUM: usize = 64;

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
    walk(fields, bytes, work, None, |_, _| Ok(()))
}

/// Validate the whole canonical row while extracting only a bounded exact
/// route. Text is checked in chunks but never owned; fixed bytes stay borrowed.
/// `None` denotes an incompatible compiled projection, not missing row data.
pub(crate) fn exact_scalar_projection<'out>(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    projection: &crate::schema::CompiledProjection,
    work: &WorkContext,
    out: &'out mut [u8; crate::schema::MAX_EXACT_SCALAR_BYTES],
) -> Result<Option<&'out [u8]>, RowError> {
    use crate::schema::{KeyEncoding, MAX_EXACT_SCALAR_BYTES};

    work.checkpoint()?;
    let KeyEncoding::ExactBounded { scalar_width } = projection.encoding else {
        return Ok(None);
    };
    let count = projection.scalar_fields.len();
    if count > MAX_EXACT_SCALAR_BYTES || count != projection.scalar_positions.len() {
        return Ok(None);
    }
    // Every sealed scalar has positive width, so a 16-byte route contains at
    // most 16 fields. Store physical offsets, not row-order offsets: compiled
    // projections need not list their fields in canonical row order.
    let mut slots = [(0usize, 0usize); MAX_EXACT_SCALAR_BYTES];
    let mut width = 0;
    for (slot, (&position, descriptor)) in slots.iter_mut().zip(
        projection
            .scalar_positions
            .iter()
            .zip(&projection.scalar_fields),
    ) {
        let Some(field) = projection.projection.get(position) else {
            return Ok(None);
        };
        let field = usize::from(field.0);
        if fields.get(field).map(|field| field.value_type) != Some(descriptor.value_type) {
            return Ok(None);
        }
        let Some(field_width) = exact_scalar_width(&descriptor.value_type) else {
            return Ok(None);
        };
        if field_width == 0 || field_width > MAX_EXACT_SCALAR_BYTES - width {
            return Ok(None);
        }
        *slot = (field, width);
        width += field_width;
    }
    if width != usize::from(scalar_width) {
        return Ok(None);
    }
    let mut visited = 0;
    walk(fields, bytes, work, None, |field, value| {
        let Some((index, &(_, offset))) = slots[..count]
            .iter()
            .enumerate()
            .find(|(_, (selected, _))| *selected == field)
        else {
            return Ok(());
        };
        with_exact_scalar_bytes(
            value,
            &projection.scalar_fields[index].value_type,
            |bytes| {
                out.get_mut(offset..offset + bytes.len())?
                    .copy_from_slice(bytes);
                Some(())
            },
        )
        .ok_or(RowError::Type { field })?;
        visited += 1;
        Ok(())
    })?;
    Ok((visited == count).then_some(&out[..width]))
}

/// An owned decoded row, including the capacity reservation for its values.
///
/// The reservation covers the decoded values for as long as the owner
/// lives. Borrow [`DecodedRow::values`]; transfer the whole owner with
/// [`DecodedRow::into_owner`]. There is no owning `values` / `into_values`
/// / `into_parts` escape that refunds while the payload remains live.
#[derive(Debug)]
pub struct DecodedRow {
    values: Vec<Value>,
    reservation: ByteReservation,
}

impl AsRef<[Value]> for DecodedRow {
    fn as_ref(&self) -> &[Value] {
        self.values()
    }
}

impl std::ops::Deref for DecodedRow {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        self.values()
    }
}

impl PartialEq for DecodedRow {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl Eq for DecodedRow {}

impl<'a> IntoIterator for &'a DecodedRow {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
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

/// Operation-local decode storage. The vector stays charged between rows;
/// decoded payloads exist only during the visitor. Binding the work context
/// here prevents a reused allocation from silently changing charge ledgers.
pub(crate) struct DecodeScratch<'work> {
    // Drop payloads before their reservation, including during unwinding.
    values: Vec<Value>,
    reservation: Option<ByteReservation>,
    work: &'work WorkContext,
}

impl<'work> DecodeScratch<'work> {
    pub(crate) const fn new(work: &'work WorkContext) -> Self {
        Self {
            values: Vec::new(),
            reservation: None,
            work,
        }
    }

    pub(crate) const fn work(&self) -> &'work WorkContext {
        self.work
    }

    /// The visitor cannot extract the borrowed values. Normal errors clear
    /// payloads before refunding; a panic retains both payload and charge
    /// until this workspace is dropped or reused.
    pub(crate) fn with_decoded<T, E: From<RowError>>(
        &mut self,
        fields: &[FieldDescriptor],
        bytes: &[u8],
        visit: impl FnOnce(&[Value]) -> Result<T, E>,
    ) -> Result<T, E> {
        self.prepare(fields.len(), bytes.len()).map_err(E::from)?;
        let result = walk(fields, bytes, self.work, Some(&mut self.values), |_, _| {
            Ok(())
        })
        .map_err(E::from)
        .and_then(|()| visit(&self.values));
        self.clear_decoded();
        result
    }

    /// Decode a headerless tuple using the canonical row payload grammar.
    /// Descriptors may be a borrowed logical projection; no descriptor
    /// collection or second tag decoder is needed for grouped scratch.
    pub(crate) fn with_decoded_payload<'a, T, E: From<RowError>>(
        &mut self,
        fields: impl ExactSizeIterator<Item = &'a FieldDescriptor>,
        bytes: &[u8],
        visit: impl FnOnce(&[Value]) -> Result<T, E>,
    ) -> Result<T, E> {
        self.prepare(fields.len(), bytes.len()).map_err(E::from)?;
        let result = walk_payload(
            fields,
            Reader { bytes },
            self.work,
            Some(&mut self.values),
            |_, _| Ok(()),
        )
        .map_err(E::from)
        .and_then(|()| visit(&self.values));
        self.clear_decoded();
        result
    }

    fn prepare(&mut self, fields: usize, payload_bound: usize) -> Result<(), RowError> {
        self.clear_decoded();
        let requested_capacity = self.values.capacity().max(fields);
        let requested = Self::footprint(requested_capacity, payload_bound)?;
        if self
            .reservation
            .as_ref()
            .is_some_and(|reservation| reservation.bytes() >= requested)
        {
            // Growth checks cancellation itself. Even a zero-byte decode
            // must check when reusing an already sufficient reservation.
            self.work.checkpoint()?;
        }
        self.set_charge(requested)?;
        if fields > self.values.capacity() && self.values.try_reserve_exact(fields).is_err() {
            self.clear_decoded();
            return Err(RowError::Allocation);
        }
        // Exact reservation normally yields exactly the requested capacity.
        // If the allocator supplies more, account for it before exposing the
        // buffer; refusal drops the allocation before any refund.
        if self.values.capacity() != requested_capacity {
            let actual = Self::footprint(self.values.capacity(), payload_bound);
            let reconciled = actual.and_then(|actual| self.set_charge(actual));
            if let Err(error) = reconciled {
                self.values = Vec::new();
                self.clear_decoded();
                return Err(error);
            }
        }
        Ok(())
    }

    fn footprint(capacity: usize, payload_bound: usize) -> Result<u64, RowError> {
        capacity
            .checked_mul(std::mem::size_of::<Value>())
            .and_then(|size| size.checked_add(payload_bound))
            .and_then(|size| u64::try_from(size).ok())
            .ok_or(RowError::LengthOverflow)
    }

    fn set_charge(&mut self, bytes: u64) -> Result<(), RowError> {
        match &mut self.reservation {
            Some(reservation) => reservation.resize(bytes)?,
            None => self.reservation = Some(self.work.reserve(ByteKind::Working, bytes)?),
        }
        Ok(())
    }

    fn clear_decoded(&mut self) {
        self.values.clear();
        if let Some(reservation) = &mut self.reservation {
            let capacity = Self::footprint(self.values.capacity(), 0)
                .expect("a live vector's capacity fits the address space");
            reservation
                .resize(capacity)
                .expect("dropping decoded payloads only shrinks the reservation");
        }
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
    walk(fields, bytes, work, Some(&mut values), |_, _| Ok(()))?;
    Ok(DecodedRow {
        values,
        reservation,
    })
}

/// The schema's fixed-width closed extension enters the same charged row
/// representation as a stored canonical row. Only one row is decoded at a
/// time; a closed source need not acquire a resident relation image.
pub(crate) fn decode_sealed(
    relation: &crate::schema::Relation,
    bytes: &[u8],
    work: &WorkContext,
) -> crate::error::Result<DecodedRow> {
    let size = relation
        .fields()
        .len()
        .checked_mul(std::mem::size_of::<Value>())
        .and_then(|size| size.checked_add(bytes.len()))
        .ok_or_else(|| {
            crate::error::Error::from_store(crate::storage::store::StoreError::Allocation)
        })?;
    let reservation = work
        .reserve(ByteKind::Working, size as u64)
        .map_err(crate::api::prepared::source::work_error)?;
    work.step(relation.fields().len() as u64)
        .map_err(crate::api::prepared::source::work_error)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(relation.fields().len())
        .map_err(|_| {
            crate::error::Error::from_store(crate::storage::store::StoreError::Allocation)
        })?;
    crate::encoding::decode_values_keyed_into(
        relation.layout().encoded(bytes),
        &[],
        &[],
        |_| unreachable!("sealed closed extensions refuse text fields"),
        &mut values,
    )?;
    Ok(DecodedRow {
        values,
        reservation,
    })
}

fn walk(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    work: &WorkContext,
    output: Option<&mut Vec<Value>>,
    visit_scalar: impl FnMut(usize, ExactScalarRef<'_>) -> Result<(), RowError>,
) -> Result<(), RowError> {
    let mut reader = Reader { bytes };
    if usize::from(u16::from_be_bytes(reader.word()?)) != fields.len() {
        return Err(RowError::Arity);
    }
    walk_payload(fields.iter(), reader, work, output, visit_scalar)
}

/// The single typed payload parser shared by framed canonical rows and
/// exact headerless scratch tuples. Field iteration preserves logical order.
fn walk_payload<'a>(
    mut fields: impl ExactSizeIterator<Item = &'a FieldDescriptor>,
    mut reader: Reader<'_>,
    work: &WorkContext,
    mut output: Option<&mut Vec<Value>>,
    mut visit_scalar: impl FnMut(usize, ExactScalarRef<'_>) -> Result<(), RowError>,
) -> Result<(), RowError> {
    let count = fields.len();
    for first in (0..count).step_by(FIELD_QUANTUM) {
        let chunk = FIELD_QUANTUM.min(count - first);
        work.step(chunk as u64)?;
        for field in first..first + chunk {
            let descriptor = fields.next().expect("exact descriptor iterator");
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
                    visit_scalar(field, ExactScalarRef::FixedBytes(blob))?;
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
                8 => Value::Uuid(Uuid::from_bytes(reader.word()?)),
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
                value_matches(&value, &descriptor.value_type)
                    .map_err(|_| RowError::Type { field })?;
            }
            visit_scalar(field, (&value).into())?;
            if let Some(output) = &mut output {
                output.push(value);
            }
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
