//! One immutable schema-bound final-state change, shared by the engine and the log.
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

use crate::canonical::{CanonicalRow, RowError};
use crate::schema::fingerprint::fingerprint;
use crate::{RelationId, Schema, SchemaFingerprint, Value, WorkContext, WorkError};

const MAGIC: &[u8; 8] = b"BDBCSET\0";
const VERSION: u16 = 1;
const HEADER: usize = 8 + 2 + 32 + 8;
const RECORD: usize = 1 + 4 + 8;

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

/// Whether a change adds rows to, and removes rows from, one relation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeltaShape {
    pub adds: bool,
    pub removes: bool,
}

impl DeltaShape {
    /// The change touches the relation at all.
    #[must_use]
    pub const fn touched(self) -> bool {
        self.adds || self.removes
    }
}

/// One relation's records: their byte range in the sealed stream.
#[derive(Debug)]
struct RelationRecords {
    relation: RelationId,
    shape: DeltaShape,
    bytes: Range<usize>,
}

/// Indexes the relations of a sealed record stream in one pass.
#[derive(Default)]
struct RelationIndex(Vec<RelationRecords>);

impl RelationIndex {
    /// Notes one record that starts at `start` and ends at `end`.
    fn note(&mut self, relation: RelationId, kind: ChangeKind, start: usize, end: usize) {
        if self.0.last().is_none_or(|last| last.relation != relation) {
            self.0.push(RelationRecords {
                relation,
                shape: DeltaShape::default(),
                bytes: start..start,
            });
        }
        let last = self.0.last_mut().expect("pushed above");
        last.bytes.end = end;
        match kind {
            ChangeKind::Add => last.shape.adds = true,
            ChangeKind::Remove => last.shape.removes = true,
        }
    }
}

#[derive(Debug)]
struct Payload {
    bytes: Vec<u8>,
    schema: SchemaFingerprint,
    added: u64,
    /// Ascending by relation; built once when the bytes are sealed or parsed.
    relations: Box<[RelationRecords]>,
}

/// Distinct requested actions, not net changes against a database state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeCounts {
    pub added: u64,
    pub removed: u64,
}

/// Clones share the same sealed native bytes.
#[derive(Debug, Clone)]
pub struct ChangeSet(Arc<Payload>);

impl ChangeSet {
    /// Callers supply a unique relation/full-row ordered traversal. Borrow
    /// those rows directly; only the final sealed payload is allocated.
    pub(crate) fn from_ordered_records<'a>(
        schema: &Schema,
        records: impl Iterator<Item = ChangeRef<'a>> + Clone,
        work: &WorkContext,
    ) -> Result<Self, ChangeError> {
        let (count, size) = records
            .clone()
            .try_fold((0u64, HEADER), |(count, size), record| {
                work.checkpoint()?;
                let count = count.checked_add(1).ok_or(ChangeError::LengthOverflow)?;
                size.checked_add(RECORD)
                    .and_then(|size| size.checked_add(record.row.len()))
                    .map(|size| (count, size))
                    .ok_or(ChangeError::LengthOverflow)
            })?;
        work.checkpoint()?;
        for record in records.clone() {
            work.checkpoint()?;
            crate::canonical::validate(
                writable_fields(schema, record.relation)?,
                record.row,
                work,
            )?;
        }
        seal_records(fingerprint(schema), count, size, records.map(Ok), work)
    }

    #[must_use]
    pub fn builder(schema: &Schema, work: WorkContext) -> ChangeSetBuilder<'_> {
        ChangeSetBuilder {
            schema,
            work,
            pending: Ok(Vec::new()),
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

    #[must_use]
    pub fn counts(&self) -> ChangeCounts {
        ChangeCounts {
            added: self.0.added,
            removed: self.len() - self.0.added,
        }
    }

    /// Combine two commands as one final-state change. Exact-fact additions
    /// win over removals; distinct rows sharing a key remain distinct and
    /// must still pass database admission. This operation is commutative,
    /// associative and idempotent, not sequential replay.
    ///
    /// Merges canonical records in linear time without decoding row values.
    /// # Errors
    /// Rejects a different schema, cancellation or allocation failure.
    pub fn compose(&self, other: &Self, work: &WorkContext) -> Result<Self, ChangeError> {
        work.checkpoint()?;
        if self.schema() != other.schema() {
            return Err(ChangeError::WrongSchema);
        }
        if Arc::ptr_eq(&self.0, &other.0) || other.is_empty() {
            return Ok(self.clone());
        }
        if self.is_empty() {
            return Ok(other.clone());
        }
        let records = merge_records(self, other, work);
        let (count, size) = records
            .clone()
            .try_fold((0u64, HEADER), |(count, size), record| {
                let record = record?;
                let count = count.checked_add(1).ok_or(ChangeError::LengthOverflow)?;
                let size = size
                    .checked_add(RECORD)
                    .and_then(|size| size.checked_add(record.row.len()))
                    .ok_or(ChangeError::LengthOverflow)?;
                Ok::<_, ChangeError>((count, size))
            })?;
        seal_records(self.schema(), count, size, records, work)
    }

    /// Accepts exactly one normalization: unique rows ordered by relation/full
    /// canonical bytes, at most one action per row. Signed/hashed input is
    /// rejected rather than silently normalized.
    /// # Errors
    /// Rejects malformed, foreign-schema or noncanonical data, cancellation,
    /// or an unallocatable capacity.
    pub fn parse(schema: &Schema, bytes: &[u8], work: &WorkContext) -> Result<Self, ChangeError> {
        let (identity, added, relations) = validate_bytes(schema, bytes, work)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| ChangeError::Allocation)?;
        owned.extend_from_slice(bytes);
        Ok(Self(Arc::new(Payload {
            bytes: owned,
            schema: identity,
            added,
            relations,
        })))
    }

    /// Check and retain an owned payload without a second byte copy.
    /// Uses exactly the same canonical validator as `parse`.
    /// # Errors
    /// Rejects malformed, foreign-schema or noncanonical data, or cancellation.
    pub fn from_bytes(
        schema: &Schema,
        bytes: Vec<u8>,
        work: &WorkContext,
    ) -> Result<Self, ChangeError> {
        let (identity, added, relations) = validate_bytes(schema, &bytes, work)?;
        Ok(Self(Arc::new(Payload {
            bytes,
            schema: identity,
            added,
            relations,
        })))
    }
}

fn validate_bytes(
    schema: &Schema,
    bytes: &[u8],
    work: &WorkContext,
) -> Result<(SchemaFingerprint, u64, Box<[RelationRecords]>), ChangeError> {
    work.checkpoint()?;
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
    let mut added = 0;
    let mut index = RelationIndex::default();
    for _ in 0..count {
        work.checkpoint()?;
        let start = bytes.len() - rest.len();
        let record = take(&mut rest, RECORD)?;
        if record[0] > 1 {
            return Err(ChangeError::InvalidKind);
        }
        added += u64::from(record[0] == 1);
        let relation = RelationId(u32::from_be_bytes(record[1..5].try_into().unwrap()));
        let len = usize::try_from(u64::from_be_bytes(record[5..13].try_into().unwrap()))
            .map_err(|_| ChangeError::LengthOverflow)?;
        let row = take(&mut rest, len)?;
        crate::canonical::validate(writable_fields(schema, relation)?, row, work)?;
        if previous.is_some_and(|prior| prior >= (relation, row)) {
            return Err(ChangeError::NonCanonicalOrder);
        }
        previous = Some((relation, row));
        let kind = if record[0] == 1 {
            ChangeKind::Add
        } else {
            ChangeKind::Remove
        };
        index.note(relation, kind, start, bytes.len() - rest.len());
    }
    if !rest.is_empty() {
        return Err(ChangeError::TrailingBytes);
    }
    work.checkpoint()?;
    Ok((identity, added, index.0.into_boxed_slice()))
}

/// Bridge-facing view of one accepted change record. Not embedding API.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ChangeRef<'a> {
    pub relation: RelationId,
    pub kind: ChangeKind,
    pub row: &'a [u8],
}

impl ChangeSet {
    /// Bridge-facing record walk (the one canonical decoder rides it).
    /// Not embedding API.
    #[doc(hidden)]
    pub fn records(&self) -> impl Iterator<Item = ChangeRef<'_>> + Clone {
        let mut rest = &self.0.bytes[HEADER..];
        std::iter::from_fn(move || next_record(&mut rest))
    }

    /// The change's shape over one relation; the default for one it leaves
    /// untouched.
    #[must_use]
    pub fn shape(&self, relation: RelationId) -> DeltaShape {
        self.relation_records(relation)
            .map_or_else(DeltaShape::default, |records| records.shape)
    }

    /// One relation's records in canonical order.
    pub(crate) fn records_of(&self, relation: RelationId) -> impl Iterator<Item = ChangeRef<'_>> {
        let mut rest = self
            .relation_records(relation)
            .map_or(&[][..], |records| &self.0.bytes[records.bytes.clone()]);
        std::iter::from_fn(move || next_record(&mut rest))
    }

    fn relation_records(&self, relation: RelationId) -> Option<&RelationRecords> {
        let relations = &self.0.relations;
        relations
            .binary_search_by_key(&relation, |records| records.relation)
            .ok()
            .map(|at| &relations[at])
    }

    /// Bridge-facing owned traversal. Cloning retains the same bytes at an
    /// independent position, allowing preview/commit without copying rows.
    #[doc(hidden)]
    #[must_use]
    pub fn cursor(&self) -> ChangeCursor {
        ChangeCursor {
            changes: self.clone(),
            offset: HEADER,
        }
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct ChangeCursor {
    changes: ChangeSet,
    offset: usize,
}

impl ChangeCursor {
    /// Borrow the next checked record in O(1) framing work. No rescan or
    /// decoded row cache; callers choose their own bounded delivery quantum.
    pub fn next_record(&mut self) -> Option<ChangeRef<'_>> {
        let mut rest = &self.changes.0.bytes[self.offset..];
        let record = next_record(&mut rest)?;
        self.offset = self.changes.0.bytes.len() - rest.len();
        Some(record)
    }
}

fn next_record<'a>(rest: &mut &'a [u8]) -> Option<ChangeRef<'a>> {
    if rest.is_empty() {
        return None;
    }
    let header = take(rest, RECORD).expect("sealed record");
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
    let row = take(rest, length).expect("sealed row");
    Some(ChangeRef {
        relation,
        kind,
        row,
    })
}

fn merge_records<'a>(
    left: &'a ChangeSet,
    right: &'a ChangeSet,
    work: &'a WorkContext,
) -> impl Iterator<Item = Result<ChangeRef<'a>, ChangeError>> + Clone {
    let mut left = left.records().peekable();
    let mut right = right.records().peekable();
    let mut failed = false;
    std::iter::from_fn(move || {
        if failed {
            return None;
        }
        let next = (|| {
            work.checkpoint()?;
            let (Some(a), Some(b)) = (left.peek(), right.peek()) else {
                return Ok(left.next().or_else(|| right.next()));
            };
            Ok(match (a.relation, a.row).cmp(&(b.relation, b.row)) {
                Ordering::Less => left.next(),
                Ordering::Greater => right.next(),
                Ordering::Equal => {
                    let kind = if a.kind == ChangeKind::Add || b.kind == ChangeKind::Add {
                        ChangeKind::Add
                    } else {
                        ChangeKind::Remove
                    };
                    let record = ChangeRef { kind, ..*a };
                    left.next();
                    right.next();
                    Some(record)
                }
            })
        })();
        match next {
            Ok(record) => record.map(Ok),
            Err(error) => {
                failed = true;
                Some(Err(error))
            }
        }
    })
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
}

impl ChangeSetBuilder<'_> {
    /// # Errors
    /// Rejects unknown/closed relations, wrong shapes, or cancellation.
    pub fn insert(&mut self, relation: RelationId, values: &[Value]) -> Result<(), ChangeError> {
        self.ingest(relation, ChangeKind::Add, values)
    }
    /// # Errors
    /// Rejects unknown/closed relations, wrong shapes, or cancellation.
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
        pending
            .try_reserve(1)
            .map_err(|_| ChangeError::Allocation)?;
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
    /// Refuses cancellation or allocation failure without returning a partial change set.
    pub fn finish(self) -> Result<ChangeSet, ChangeError> {
        self.work.checkpoint()?;
        let mut pending = self.pending?;
        // Facts ascending; for one fact the add sorts first and survives.
        pending.sort_unstable_by(|a, b| {
            fact(a)
                .cmp(&fact(b))
                .then_with(|| (a.kind == ChangeKind::Remove).cmp(&(b.kind == ChangeKind::Remove)))
        });
        pending.dedup_by(|next, kept| fact(next) == fact(kept));
        self.work.checkpoint()?;
        let unique = pending.len();
        let size = pending.iter().try_fold(HEADER, |size, entry| {
            size.checked_add(RECORD)
                .and_then(|n| n.checked_add(entry.row.as_bytes().len()))
                .ok_or(ChangeError::LengthOverflow)
        })?;
        seal_records(
            fingerprint(self.schema),
            unique as u64,
            size,
            pending.iter().map(|entry| {
                Ok(ChangeRef {
                    relation: entry.relation,
                    kind: entry.kind,
                    row: entry.row.as_bytes(),
                })
            }),
            &self.work,
        )
    }
}

/// The only writer of the sealed header and record framing. Callers prove
/// row validity, ordering, uniqueness and exact payload size before entry.
fn seal_records<'a>(
    identity: SchemaFingerprint,
    count: u64,
    size: usize,
    records: impl Iterator<Item = Result<ChangeRef<'a>, ChangeError>>,
    work: &WorkContext,
) -> Result<ChangeSet, ChangeError> {
    work.checkpoint()?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| ChangeError::Allocation)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_be_bytes());
    bytes.extend_from_slice(&identity.0);
    bytes.extend_from_slice(&count.to_be_bytes());
    let mut added = 0;
    let mut index = RelationIndex::default();
    for record in records {
        work.checkpoint()?;
        let record = record?;
        let start = bytes.len();
        added += u64::from(record.kind == ChangeKind::Add);
        bytes.push(u8::from(record.kind == ChangeKind::Add));
        bytes.extend_from_slice(&record.relation.0.to_be_bytes());
        bytes.extend_from_slice(&(record.row.len() as u64).to_be_bytes());
        bytes.extend_from_slice(record.row);
        index.note(record.relation, record.kind, start, bytes.len());
    }
    debug_assert_eq!(bytes.len(), size);
    Ok(ChangeSet(Arc::new(Payload {
        bytes,
        schema: identity,
        added,
        relations: index.0.into_boxed_slice(),
    })))
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
/// A pending change's identity: its relation, then its canonical row.
fn fact(pending: &Pending) -> (RelationId, &[u8]) {
    (pending.relation, pending.row.as_bytes())
}

#[cfg(test)]
mod tests;
