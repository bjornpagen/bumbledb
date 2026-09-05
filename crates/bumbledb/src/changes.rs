//! One immutable schema-bound final-state change, shared by core and history.
use std::cmp::Ordering;
use std::sync::Arc;

use crate::canonical::{CanonicalRow, RowError};
use crate::schema::fingerprint::fingerprint;
use crate::work::{ByteKind, ByteReservation};
use crate::{RelationId, Schema, SchemaFingerprint, Value, WorkContext, WorkError};

const MAGIC: &[u8; 8] = b"BDBCSET\0";
const VERSION: u16 = 1;
const HEADER: usize = 8 + 2 + 32 + 8;
const RECORD: usize = 1 + 4 + 8;
const BYTE_QUANTUM: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Remove,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeError {
    Row(RowError),
    Work(WorkError),
    UnknownRelation(RelationId),
    ClosedRelation(RelationId),
    WrongFamily,
    WrongVersion,
    WrongSchema,
    Truncated,
    TrailingBytes,
    NonCanonicalOrder,
    InvalidKind,
    LengthOverflow,
    Allocation,
}
impl From<RowError> for ChangeError {
    fn from(error: RowError) -> Self {
        Self::Row(error)
    }
}
impl From<WorkError> for ChangeError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}
impl std::fmt::Display for ChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "change set: {self:?}")
    }
}
impl std::error::Error for ChangeError {}

#[derive(Debug)]
struct Payload {
    bytes: Vec<u8>,
    schema: SchemaFingerprint,
    _reservation: ByteReservation,
}

/// Clones retain the same sealed native bytes and their original memory charge.
#[derive(Debug, Clone)]
pub struct ChangeSet(Arc<Payload>);

impl ChangeSet {
    #[must_use]
    pub fn builder(schema: &Schema, work: WorkContext) -> ChangeSetBuilder<'_> {
        ChangeSetBuilder {
            schema,
            work,
            pending: Ok(Vec::new()),
            capacity_charge: None,
        }
    }
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0.bytes
    }
    #[must_use]
    pub fn schema(&self) -> SchemaFingerprint {
        self.0.schema
    }
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "private checked construction proves header width"
    )]
    pub fn len(&self) -> u64 {
        u64::from_be_bytes(
            self.0.bytes[HEADER - 8..HEADER]
                .try_into()
                .expect("checked header"),
        )
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Accepts exactly one normalization: unique rows ordered by relation/full
    /// canonical bytes, at most one action per row. Signed/hashed input is
    /// rejected rather than silently normalized.
    /// # Errors
    /// Rejects malformed, foreign-schema, noncanonical or over-budget data.
    #[expect(
        clippy::missing_panics_doc,
        reason = "header and record widths are checked before fixed-array conversions"
    )]
    pub fn parse(schema: &Schema, bytes: &[u8], work: &WorkContext) -> Result<Self, ChangeError> {
        work.input(bytes.len() as u64)?;
        if bytes.len() < HEADER {
            return Err(ChangeError::Truncated);
        }
        if &bytes[..8] != MAGIC {
            return Err(ChangeError::WrongFamily);
        }
        if u16::from_be_bytes(bytes[8..10].try_into().unwrap()) != VERSION {
            return Err(ChangeError::WrongVersion);
        }
        let identity = fingerprint(schema);
        if bytes[10..42] != identity.0 {
            return Err(ChangeError::WrongSchema);
        }
        let count = u64::from_be_bytes(bytes[42..50].try_into().unwrap());
        let mut rest = &bytes[HEADER..];
        if count > (rest.len() / (RECORD + 2)) as u64 {
            return Err(ChangeError::Truncated);
        }
        let mut previous: Option<(RelationId, &[u8])> = None;
        for _ in 0..count {
            work.rows(1)?;
            work.step(1)?;
            let record = take(&mut rest, RECORD)?;
            if record[0] > 1 {
                return Err(ChangeError::InvalidKind);
            }
            let relation = RelationId(u32::from_be_bytes(record[1..5].try_into().unwrap()));
            let len = usize::try_from(u64::from_be_bytes(record[5..13].try_into().unwrap()))
                .map_err(|_| ChangeError::LengthOverflow)?;
            let row = take(&mut rest, len)?;
            crate::canonical::validate(writable_fields(schema, relation)?, row, work)?;
            if let Some((prior_relation, prior_row)) = previous {
                let order = if prior_relation == relation {
                    compare_bytes(prior_row, row, work)?
                } else {
                    prior_relation.cmp(&relation)
                };
                if order != Ordering::Less {
                    return Err(ChangeError::NonCanonicalOrder);
                }
            }
            previous = Some((relation, row));
        }
        if !rest.is_empty() {
            return Err(ChangeError::TrailingBytes);
        }
        let reservation = reserve_payload(work, bytes.len())?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| ChangeError::Allocation)?;
        copy(&mut owned, bytes, work)?;
        Ok(Self(Arc::new(Payload {
            bytes: owned,
            schema: identity,
            _reservation: reservation,
        })))
    }
}

pub(crate) struct ChangeRef<'a> {
    pub relation: RelationId,
    pub kind: ChangeKind,
    pub row: &'a [u8],
}

impl ChangeSet {
    pub(crate) fn records(&self) -> impl Iterator<Item = ChangeRef<'_>> {
        let mut rest = &self.0.bytes[HEADER..];
        std::iter::from_fn(move || {
            if rest.is_empty() {
                return None;
            }
            let header = take(&mut rest, RECORD).expect("sealed record");
            let relation = RelationId(u32::from_be_bytes(
                header[1..5].try_into().expect("sealed relation"),
            ));
            let kind = if header[0] == 1 {
                ChangeKind::Add
            } else {
                ChangeKind::Remove
            };
            let length = usize::try_from(u64::from_be_bytes(
                header[5..13].try_into().expect("sealed length"),
            ))
            .expect("sealed row fits memory");
            let row = take(&mut rest, length).expect("sealed row");
            Some(ChangeRef {
                relation,
                kind,
                row,
            })
        })
    }
}

struct Pending {
    relation: RelationId,
    kind: ChangeKind,
    row: CanonicalRow,
}

/// Database-free staging; failures spend the draft. Finish consumes it. No
/// user callback/iterator is retained for transaction or network replay.
pub struct ChangeSetBuilder<'s> {
    schema: &'s Schema,
    work: WorkContext,
    pending: Result<Vec<Pending>, ChangeError>,
    capacity_charge: Option<ByteReservation>,
}

impl ChangeSetBuilder<'_> {
    /// # Errors
    /// Rejects unknown/closed relations, wrong shapes and exhausted work.
    pub fn insert(&mut self, relation: RelationId, values: &[Value]) -> Result<(), ChangeError> {
        self.ingest(relation, ChangeKind::Add, values)
    }
    /// # Errors
    /// Rejects unknown/closed relations, wrong shapes and exhausted work.
    pub fn delete(&mut self, relation: RelationId, values: &[Value]) -> Result<(), ChangeError> {
        self.ingest(relation, ChangeKind::Remove, values)
    }

    fn ingest(
        &mut self,
        relation: RelationId,
        kind: ChangeKind,
        values: &[Value],
    ) -> Result<(), ChangeError> {
        let result = self.push(relation, kind, values);
        if let Err(error) = result {
            self.pending = Err(error);
            self.capacity_charge = None;
        }
        result
    }
    fn push(
        &mut self,
        relation: RelationId,
        kind: ChangeKind,
        values: &[Value],
    ) -> Result<(), ChangeError> {
        let pending = self.pending.as_mut().map_err(|error| *error)?;
        let row =
            CanonicalRow::encode(writable_fields(self.schema, relation)?, values, &self.work)?;
        if pending.len() == pending.capacity() {
            let capacity = pending
                .capacity()
                .max(1)
                .checked_mul(2)
                .ok_or(ChangeError::LengthOverflow)?;
            let bytes = capacity
                .checked_mul(std::mem::size_of::<Pending>())
                .ok_or(ChangeError::LengthOverflow)?;
            // Old capacity stays charged during the allocation/transfer.
            let charge = self.work.reserve(ByteKind::Working, bytes as u64)?;
            pending
                .try_reserve_exact(capacity - pending.len())
                .map_err(|_| ChangeError::Allocation)?;
            self.capacity_charge = Some(charge);
        }
        pending.push(Pending {
            relation,
            kind,
            row,
        });
        Ok(())
    }

    /// Add wins over remove for the identical fact in this one command. Across
    /// separately ordered commands this is ordinary set mutation, not a CRDT.
    /// # Errors
    /// Refuses exhausted work/memory without returning a partial change set.
    pub fn finish(self) -> Result<ChangeSet, ChangeError> {
        let mut pending = self.pending?;
        sort(&mut pending, &self.work)?;
        let mut unique = 0;
        for read in 0..pending.len() {
            self.work.step(1)?;
            let duplicate = unique > 0
                && compare_fact(&pending[unique - 1], &pending[read], &self.work)?
                    == Ordering::Equal;
            if !duplicate {
                pending.swap(unique, read);
                unique += 1;
            }
        }
        pending.truncate(unique);
        let size = pending.iter().try_fold(HEADER, |size, entry| {
            size.checked_add(RECORD)
                .and_then(|n| n.checked_add(entry.row.as_bytes().len()))
                .ok_or(ChangeError::LengthOverflow)
        })?;
        let reservation = reserve_payload(&self.work, size)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| ChangeError::Allocation)?;
        let identity = fingerprint(self.schema);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_be_bytes());
        bytes.extend_from_slice(&identity.0);
        bytes.extend_from_slice(&(unique as u64).to_be_bytes());
        for entry in &pending {
            self.work.step(1)?;
            bytes.push(u8::from(entry.kind == ChangeKind::Add));
            bytes.extend_from_slice(&entry.relation.0.to_be_bytes());
            bytes.extend_from_slice(&(entry.row.as_bytes().len() as u64).to_be_bytes());
            copy(&mut bytes, entry.row.as_bytes(), &self.work)?;
        }
        Ok(ChangeSet(Arc::new(Payload {
            bytes,
            schema: identity,
            _reservation: reservation,
        })))
    }
}

fn reserve_payload(work: &WorkContext, bytes: usize) -> Result<ByteReservation, ChangeError> {
    let bytes = bytes
        .checked_add(std::mem::size_of::<Payload>())
        .ok_or(ChangeError::LengthOverflow)?;
    Ok(work.reserve(ByteKind::Working, bytes as u64)?)
}
fn writable_fields(
    schema: &Schema,
    relation: RelationId,
) -> Result<&[crate::schema::FieldDescriptor], ChangeError> {
    let view = schema
        .relation_checked(relation)
        .ok_or(ChangeError::UnknownRelation(relation))?;
    if view.body().closed_rows().is_some() {
        return Err(ChangeError::ClosedRelation(relation));
    }
    Ok(view.fields())
}
fn take<'a>(rest: &mut &'a [u8], len: usize) -> Result<&'a [u8], ChangeError> {
    let (head, tail) = rest.split_at_checked(len).ok_or(ChangeError::Truncated)?;
    *rest = tail;
    Ok(head)
}
fn copy(out: &mut Vec<u8>, bytes: &[u8], work: &WorkContext) -> Result<(), ChangeError> {
    for chunk in bytes.chunks(BYTE_QUANTUM) {
        work.step(chunk.len() as u64)?;
        out.extend_from_slice(chunk);
    }
    Ok(())
}
fn compare_bytes(left: &[u8], right: &[u8], work: &WorkContext) -> Result<Ordering, ChangeError> {
    for (a, b) in left.chunks(BYTE_QUANTUM).zip(right.chunks(BYTE_QUANTUM)) {
        work.step(a.len().min(b.len()) as u64)?;
        let order = a.cmp(b);
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(left.len().cmp(&right.len()))
}
fn compare_fact(a: &Pending, b: &Pending, work: &WorkContext) -> Result<Ordering, ChangeError> {
    let relation = a.relation.cmp(&b.relation);
    if relation != Ordering::Equal {
        work.step(1)?;
        return Ok(relation);
    }
    compare_bytes(a.row.as_bytes(), b.row.as_bytes(), work)
}

// In-place heapsort permits fallible comparisons and bounded polling. An
// infallible std sort callback cannot propagate exhaustion without unwinding.
fn sort(rows: &mut [Pending], work: &WorkContext) -> Result<(), ChangeError> {
    fn greater(a: &Pending, b: &Pending, work: &WorkContext) -> Result<bool, ChangeError> {
        Ok(compare_fact(a, b, work)?.then_with(|| {
            u8::from(a.kind == ChangeKind::Remove).cmp(&u8::from(b.kind == ChangeKind::Remove))
        }) == Ordering::Greater)
    }
    fn sift(rows: &mut [Pending], mut root: usize, work: &WorkContext) -> Result<(), ChangeError> {
        while root < rows.len() / 2 {
            let mut child = 2 * root + 1;
            if child + 1 < rows.len() && greater(&rows[child + 1], &rows[child], work)? {
                child += 1;
            }
            if !greater(&rows[child], &rows[root], work)? {
                break;
            }
            rows.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..rows.len() / 2).rev() {
        sift(rows, root, work)?;
    }
    for end in (1..rows.len()).rev() {
        rows.swap(0, end);
        sift(&mut rows[..end], 0, work)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
