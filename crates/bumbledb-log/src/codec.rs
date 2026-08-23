//! The command codec: one binary batch format, full parse before any
//! apply, every refusal a typed sum naming relation, row, and field
//! where one exists. Commands carry raw values, never intern ids, so
//! the footprint section is verifiable by pure recomputation (the
//! carried-and-checked law; the recompute lives in `crate::footprint`).
//! Illegal combinations are unencodable, not refused: value payloads
//! parse against the relation layout, footprint suffixes parse against
//! their class, and the one numeric field (the capacity child delta)
//! exists only where the class-and-mode pair admits it.

use bumbledb::schema::{IntervalElement, RelationId, SchemaDescriptor, StatementId, ValueType};
use bumbledb::{Interval, Value};

use crate::braids::{BraidId, Braids, braids};
use crate::footprint::{
    CapacityMode, ContainmentMode, Entry, FootprintError, Vocabulary, VocabularyError, footprint,
};

/// The four magic bytes opening every batch object.
pub const MAGIC: [u8; 4] = *b"BDBL";

/// The one accepted format version; consumers refuse anything else.
pub const VERSION: u16 = 2;

const TAG_BOOL: u8 = 0;
const TAG_U64: u8 = 1;
const TAG_I64: u8 = 2;
const TAG_STRING: u8 = 3;
const TAG_FIXED_BYTES: u8 = 4;
const TAG_INTERVAL: u8 = 5;
const TAG_FIXED_INTERVAL: u8 = 6;

pub(crate) const CLASS_FACT: u8 = 1;
pub(crate) const CLASS_KEY: u8 = 2;
pub(crate) const CLASS_CONTAINMENT: u8 = 3;
pub(crate) const CLASS_CAPACITY: u8 = 4;

const OP_INSERT: u8 = 1;
const OP_DELETE: u8 = 2;

/// A byte destination the one value-encoding function writes into: the
/// wire buffer, a blake3 hasher (footprint keys hash the identical
/// tagged bytes), or the discard sink used for shape checks alone.
pub(crate) trait ByteSink {
    fn put(&mut self, bytes: &[u8]);
}

impl ByteSink for Vec<u8> {
    fn put(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

impl ByteSink for blake3::Hasher {
    fn put(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

/// Shape checking without bytes: `append_value` into nothing.
pub(crate) struct NullSink;

impl ByteSink for NullSink {
    fn put(&mut self, _bytes: &[u8]) {}
}

/// Why a raw value refused to encode against its declared type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    /// Wrong structural kind, wrong width, an empty or inverted
    /// interval, or a fixed-width interval that is a ray.
    Kind { expected: ValueType },

    /// A string longer than the u32 length prefix can carry.
    Oversize,
}

/// The one value-encoding function: `tag u8` + payload, the exact bytes
/// the wire carries and the exact bytes footprint keys hash. Both
/// consumers go through here so the two encodings cannot drift apart.
pub(crate) fn append_value<S: ByteSink>(
    sink: &mut S,
    value: &Value,
    expected: ValueType,
) -> Result<(), ValueShape> {
    let kind = || ValueShape::Kind { expected };
    match (value, expected) {
        (Value::Bool(b), ValueType::Bool) => {
            sink.put(&[TAG_BOOL, u8::from(*b)]);
            Ok(())
        }
        (Value::U64(v), ValueType::U64) => {
            sink.put(&[TAG_U64]);
            sink.put(&v.to_le_bytes());
            Ok(())
        }
        (Value::I64(v), ValueType::I64) => {
            sink.put(&[TAG_I64]);
            sink.put(&v.to_le_bytes());
            Ok(())
        }
        (Value::String(s), ValueType::String) => {
            let len = u32::try_from(s.len()).map_err(|_| ValueShape::Oversize)?;
            sink.put(&[TAG_STRING]);
            sink.put(&len.to_le_bytes());
            sink.put(s.as_bytes());
            Ok(())
        }
        (Value::FixedBytes(raw), ValueType::FixedBytes { len }) => {
            if raw.len() == usize::from(len) {
                sink.put(&[TAG_FIXED_BYTES]);
                sink.put(raw);
                Ok(())
            } else {
                Err(kind())
            }
        }
        (
            Value::IntervalU64(interval),
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        ) => {
            sink.put(&[TAG_INTERVAL]);
            sink.put(&interval.start().to_le_bytes());
            sink.put(&interval.end().to_le_bytes());
            Ok(())
        }
        (
            Value::IntervalI64(interval),
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ) => {
            sink.put(&[TAG_INTERVAL]);
            sink.put(&interval.start().to_le_bytes());
            sink.put(&interval.end().to_le_bytes());
            Ok(())
        }
        (
            Value::IntervalU64(interval),
            ValueType::FixedInterval {
                element: IntervalElement::U64,
                width,
            },
        ) => {
            if interval.end() - interval.start() == width && !interval.is_ray() {
                sink.put(&[TAG_FIXED_INTERVAL]);
                sink.put(&interval.start().to_le_bytes());
                Ok(())
            } else {
                Err(kind())
            }
        }
        (
            Value::IntervalI64(interval),
            ValueType::FixedInterval {
                element: IntervalElement::I64,
                width,
            },
        ) => {
            if interval.end().abs_diff(interval.start()) == width && !interval.is_ray() {
                sink.put(&[TAG_FIXED_INTERVAL]);
                sink.put(&interval.start().to_le_bytes());
                Ok(())
            } else {
                Err(kind())
            }
        }
        _ => Err(kind()),
    }
}

const fn expected_tag(ty: ValueType) -> u8 {
    match ty {
        ValueType::Bool => TAG_BOOL,
        ValueType::U64 => TAG_U64,
        ValueType::I64 => TAG_I64,
        ValueType::String => TAG_STRING,
        ValueType::FixedBytes { .. } => TAG_FIXED_BYTES,
        ValueType::Interval { .. } => TAG_INTERVAL,
        ValueType::FixedInterval { .. } => TAG_FIXED_INTERVAL,
    }
}

/// The op verb, and equally the F-class entry mode: an op's net
/// survivor keeps the verb as its fact disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpKind {
    Insert,
    Delete,
}

impl OpKind {
    #[must_use]
    pub(crate) const fn wire(self) -> u8 {
        match self {
            Self::Insert => OP_INSERT,
            Self::Delete => OP_DELETE,
        }
    }
}

/// One op: a verb, a relation, and its rows (row-major, fields in
/// declaration order, raw values only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub kind: OpKind,
    pub relation: RelationId,
    pub rows: Vec<Box<[Value]>>,
}

/// The batch header: everything apply-time chain checks read before
/// touching an op — the slot identity (`braid`, `braid_gen`), the
/// backlink (`prev`), the provenance (`writer`), and the clamped
/// timestamp whose monotony apply refuses violations of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchHeader {
    pub fingerprint: [u8; 32],
    pub braid: BraidId,
    pub braid_gen: u64,
    pub prev: [u8; 32],
    pub writer: u64,
    pub timestamp: u64,
}

/// A fully parsed batch. `footprint` is the published section as
/// carried; consumers recompute it from `ops` and refuse mismatches —
/// no consumer ever acts on the carried copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub header: BatchHeader,
    pub ops: Vec<Op>,
    pub footprint: Vec<Entry>,
}

/// Encode-side refusals: typed, named by op, relation, row, and field
/// where one exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    FingerprintMismatch,
    UnknownBraid {
        braid: u32,
    },
    UnknownRelation {
        op: usize,
        relation: RelationId,
    },
    ClosedRelation {
        op: usize,
        relation: RelationId,
    },
    OpRelationOutsideBraid {
        op: usize,
        relation: RelationId,
        braid: BraidId,
    },
    Arity {
        op: usize,
        relation: RelationId,
        row: usize,
    },
    Value {
        op: usize,
        relation: RelationId,
        row: usize,
        field: u16,
        cause: ValueShape,
    },
    TooManyOps,
    TooManyRows {
        op: usize,
    },
    TooManyFootprintEntries,
    Footprint(FootprintError),
}

impl From<FootprintError> for EncodeError {
    fn from(error: FootprintError) -> Self {
        Self::Footprint(error)
    }
}

/// Decode-side refusals. `identity` is the cross-implementation name
/// the conformance sidecars pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated {
        offset: usize,
    },
    BadMagic {
        got: [u8; 4],
    },
    Version {
        got: u16,
    },
    Flags {
        got: u16,
    },
    FingerprintMismatch {
        got: [u8; 32],
    },
    UnknownBraid {
        got: u32,
    },
    UnknownOpKind {
        op: usize,
        got: u8,
    },
    UnknownRelation {
        op: usize,
        relation: RelationId,
    },
    ClosedRelation {
        op: usize,
        relation: RelationId,
    },
    OpRelationOutsideBraid {
        op: usize,
        relation: RelationId,
        braid: BraidId,
    },
    TagMismatch {
        relation: RelationId,
        row: usize,
        field: u16,
        expected: ValueType,
        got: u8,
    },
    BoolByte {
        relation: RelationId,
        row: usize,
        field: u16,
        got: u8,
    },
    InvalidUtf8 {
        relation: RelationId,
        row: usize,
        field: u16,
    },
    EmptyInterval {
        relation: RelationId,
        row: usize,
        field: u16,
    },
    IntervalOverflow {
        relation: RelationId,
        row: usize,
        field: u16,
    },
    UnknownFootprintClass {
        index: usize,
        got: u8,
    },
    UnknownFootprintMode {
        index: usize,
        class: u8,
        got: u8,
    },
    UnsortedFootprint {
        index: usize,
    },
    DuplicateFootprintEntry {
        index: usize,
    },
    TrailingBytes {
        at: usize,
    },
}

impl DecodeError {
    /// The refusal's stable cross-implementation name.
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Truncated { .. } => "Truncated",
            Self::BadMagic { .. } => "BadMagic",
            Self::Version { .. } => "Version",
            Self::Flags { .. } => "Flags",
            Self::FingerprintMismatch { .. } => "FingerprintMismatch",
            Self::UnknownBraid { .. } => "UnknownBraid",
            Self::UnknownOpKind { .. } => "UnknownOpKind",
            Self::UnknownRelation { .. } => "UnknownRelation",
            Self::ClosedRelation { .. } => "ClosedRelation",
            Self::OpRelationOutsideBraid { .. } => "OpRelationOutsideBraid",
            Self::TagMismatch { .. } => "TagMismatch",
            Self::BoolByte { .. } => "BoolByte",
            Self::InvalidUtf8 { .. } => "InvalidUtf8",
            Self::EmptyInterval { .. } => "EmptyInterval",
            Self::IntervalOverflow { .. } => "IntervalOverflow",
            Self::UnknownFootprintClass { .. } => "UnknownFootprintClass",
            Self::UnknownFootprintMode { .. } => "UnknownFootprintMode",
            Self::UnsortedFootprint { .. } => "UnsortedFootprint",
            Self::DuplicateFootprintEntry { .. } => "DuplicateFootprintEntry",
            Self::TrailingBytes { .. } => "TrailingBytes",
        }
    }
}

struct Cursor<'bytes> {
    bytes: &'bytes [u8],
    at: usize,
}

impl<'bytes> Cursor<'bytes> {
    const fn new(bytes: &'bytes [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'bytes [u8], DecodeError> {
        let end = self
            .at
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(DecodeError::Truncated { offset: self.at })?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        let bytes = self.take(8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(raw))
    }

    fn i64(&mut self) -> Result<i64, DecodeError> {
        let bytes = self.take(8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(bytes);
        Ok(i64::from_le_bytes(raw))
    }

    fn array32(&mut self) -> Result<[u8; 32], DecodeError> {
        let bytes = self.take(32)?;
        let mut raw = [0u8; 32];
        raw.copy_from_slice(bytes);
        Ok(raw)
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
}

/// The codec context: the descriptor parsed once (vocabulary + braid
/// map) beside the schema fingerprint the wire pins. Encode and decode
/// allocate output buffers only; everything derived lives here.
#[derive(Debug, Clone)]
pub struct Codec {
    fingerprint: [u8; 32],
    vocabulary: Vocabulary,
    braids: Braids,
}

impl Codec {
    /// Parses the descriptor into the derived views the codec reads.
    pub fn new(
        descriptor: &SchemaDescriptor,
        fingerprint: [u8; 32],
    ) -> Result<Self, VocabularyError> {
        Ok(Self {
            fingerprint,
            vocabulary: Vocabulary::new(descriptor)?,
            braids: braids(descriptor),
        })
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &[u8; 32] {
        &self.fingerprint
    }

    #[must_use]
    pub const fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }

    #[must_use]
    pub const fn braids(&self) -> &Braids {
        &self.braids
    }

    /// Encodes a batch: header, ops, then the derived footprint section
    /// (`crate::footprint::footprint` is the one producer of it).
    pub fn encode(&self, header: &BatchHeader, ops: &[Op]) -> Result<Vec<u8>, EncodeError> {
        if header.fingerprint != self.fingerprint {
            return Err(EncodeError::FingerprintMismatch);
        }
        if self.braids.parse(header.braid.raw()).is_none() {
            return Err(EncodeError::UnknownBraid {
                braid: header.braid.raw(),
            });
        }
        let op_count = u32::try_from(ops.len()).map_err(|_| EncodeError::TooManyOps)?;

        let mut out: Vec<u8> = Vec::new();
        out.put(&MAGIC);
        out.put(&VERSION.to_le_bytes());
        out.put(&0u16.to_le_bytes());
        out.put(&self.fingerprint);
        out.put(&header.braid.raw().to_le_bytes());
        out.put(&header.braid_gen.to_le_bytes());
        out.put(&header.prev);
        out.put(&header.writer.to_le_bytes());
        out.put(&header.timestamp.to_le_bytes());
        out.put(&op_count.to_le_bytes());

        for (op_index, op) in ops.iter().enumerate() {
            let info = match self.vocabulary.relation(op.relation) {
                Some(info) if info.is_ordinary() => info,
                Some(_) => {
                    return Err(EncodeError::ClosedRelation {
                        op: op_index,
                        relation: op.relation,
                    });
                }
                None => {
                    return Err(EncodeError::UnknownRelation {
                        op: op_index,
                        relation: op.relation,
                    });
                }
            };
            if self.braids.braid_of(op.relation) != Some(header.braid) {
                return Err(EncodeError::OpRelationOutsideBraid {
                    op: op_index,
                    relation: op.relation,
                    braid: header.braid,
                });
            }
            let row_count = u32::try_from(op.rows.len())
                .map_err(|_| EncodeError::TooManyRows { op: op_index })?;
            out.put(&[op.kind.wire()]);
            out.put(&op.relation.0.to_le_bytes());
            out.put(&row_count.to_le_bytes());
            let layout = info.layout();
            for (row_index, row) in op.rows.iter().enumerate() {
                if row.len() != layout.len() {
                    return Err(EncodeError::Arity {
                        op: op_index,
                        relation: op.relation,
                        row: row_index,
                    });
                }
                for (field_index, (value, ty)) in row.iter().zip(layout.iter()).enumerate() {
                    append_value(&mut out, value, *ty).map_err(|cause| EncodeError::Value {
                        op: op_index,
                        relation: op.relation,
                        row: row_index,
                        field: field_index_u16(field_index),
                        cause,
                    })?;
                }
            }
        }

        let entries = footprint(&self.vocabulary, ops)?;
        let fp_count =
            u32::try_from(entries.len()).map_err(|_| EncodeError::TooManyFootprintEntries)?;
        out.put(&fp_count.to_le_bytes());
        for entry in &entries {
            append_entry(&mut out, entry);
        }
        Ok(out)
    }

    /// Decodes a batch: a full sequential parse of every byte before
    /// any apply, refusing version, flags, fingerprint, braid
    /// membership, malformed values, and footprint order violations.
    pub fn decode(&self, bytes: &[u8]) -> Result<Batch, DecodeError> {
        let mut cur = Cursor::new(bytes);

        let magic = cur.take(4)?;
        if magic != MAGIC {
            let mut got = [0u8; 4];
            got.copy_from_slice(magic);
            return Err(DecodeError::BadMagic { got });
        }
        let version = cur.u16()?;
        if version != VERSION {
            return Err(DecodeError::Version { got: version });
        }
        let flags = cur.u16()?;
        if flags != 0 {
            return Err(DecodeError::Flags { got: flags });
        }
        let fingerprint = cur.array32()?;
        if fingerprint != self.fingerprint {
            return Err(DecodeError::FingerprintMismatch { got: fingerprint });
        }
        let braid_raw = cur.u32()?;
        let braid = self
            .braids
            .parse(braid_raw)
            .ok_or(DecodeError::UnknownBraid { got: braid_raw })?;
        let braid_gen = cur.u64()?;
        let prev = cur.array32()?;
        let writer = cur.u64()?;
        let timestamp = cur.u64()?;

        let op_count = cur.u32()?;
        let mut ops: Vec<Op> = Vec::with_capacity(capped(op_count, cur.remaining(), 9));
        for op_index in 0..op_count {
            let op_index = usize::try_from(op_index).expect("op index fits usize");
            ops.push(self.decode_op(&mut cur, op_index, braid)?);
        }

        let fp_count = cur.u32()?;
        let mut entries: Vec<Entry> = Vec::with_capacity(capped(fp_count, cur.remaining(), 34));
        for index in 0..fp_count {
            let index = usize::try_from(index).expect("entry index fits usize");
            let entry = decode_entry(&mut cur, index)?;
            if let Some(last) = entries.last() {
                match last.sort_key().cmp(&entry.sort_key()) {
                    std::cmp::Ordering::Less => {}
                    std::cmp::Ordering::Equal => {
                        return Err(DecodeError::DuplicateFootprintEntry { index });
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(DecodeError::UnsortedFootprint { index });
                    }
                }
            }
            entries.push(entry);
        }

        if cur.remaining() != 0 {
            return Err(DecodeError::TrailingBytes { at: cur.at });
        }

        Ok(Batch {
            header: BatchHeader {
                fingerprint,
                braid,
                braid_gen,
                prev,
                writer,
                timestamp,
            },
            ops,
            footprint: entries,
        })
    }

    fn decode_op(
        &self,
        cur: &mut Cursor<'_>,
        op_index: usize,
        braid: BraidId,
    ) -> Result<Op, DecodeError> {
        let kind = match cur.u8()? {
            OP_INSERT => OpKind::Insert,
            OP_DELETE => OpKind::Delete,
            got => return Err(DecodeError::UnknownOpKind { op: op_index, got }),
        };
        let relation = RelationId(cur.u32()?);
        let info = match self.vocabulary.relation(relation) {
            Some(info) if info.is_ordinary() => info,
            Some(_) => {
                return Err(DecodeError::ClosedRelation {
                    op: op_index,
                    relation,
                });
            }
            None => {
                return Err(DecodeError::UnknownRelation {
                    op: op_index,
                    relation,
                });
            }
        };
        if self.braids.braid_of(relation) != Some(braid) {
            return Err(DecodeError::OpRelationOutsideBraid {
                op: op_index,
                relation,
                braid,
            });
        }
        let layout = info.layout();
        let row_count = cur.u32()?;
        let min_row = layout.len().max(1);
        let mut rows: Vec<Box<[Value]>> =
            Vec::with_capacity(capped(row_count, cur.remaining(), min_row));
        for row_index in 0..row_count {
            let row_index = usize::try_from(row_index).expect("row index fits usize");
            let mut row: Vec<Value> = Vec::with_capacity(layout.len());
            for (field_index, ty) in layout.iter().enumerate() {
                let value =
                    decode_value(cur, *ty, relation, row_index, field_index_u16(field_index))?;
                row.push(value);
            }
            rows.push(row.into_boxed_slice());
        }
        Ok(Op {
            kind,
            relation,
            rows,
        })
    }
}

fn field_index_u16(index: usize) -> u16 {
    u16::try_from(index).expect("field count fits u16")
}

/// Caps a wire-declared count by what the remaining bytes could hold,
/// so a hostile count cannot force an allocation the input cannot back.
fn capped(count: u32, remaining: usize, min_item: usize) -> usize {
    let declared = usize::try_from(count).expect("u32 fits usize");
    declared.min(remaining / min_item.max(1) + 1)
}

fn decode_value(
    cur: &mut Cursor<'_>,
    ty: ValueType,
    relation: RelationId,
    row: usize,
    field: u16,
) -> Result<Value, DecodeError> {
    let tag = cur.u8()?;
    if tag != expected_tag(ty) {
        return Err(DecodeError::TagMismatch {
            relation,
            row,
            field,
            expected: ty,
            got: tag,
        });
    }
    match ty {
        ValueType::Bool => match cur.u8()? {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            got => Err(DecodeError::BoolByte {
                relation,
                row,
                field,
                got,
            }),
        },
        ValueType::U64 => Ok(Value::U64(cur.u64()?)),
        ValueType::I64 => Ok(Value::I64(cur.i64()?)),
        ValueType::String => {
            let len = cur.u32()?;
            let bytes = cur.take(usize::try_from(len).expect("u32 fits usize"))?;
            let text = std::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 {
                relation,
                row,
                field,
            })?;
            Ok(Value::String(Box::from(text)))
        }
        ValueType::FixedBytes { len } => {
            let bytes = cur.take(usize::from(len))?;
            Ok(Value::FixedBytes(Box::from(bytes)))
        }
        ValueType::Interval { element } => match element {
            IntervalElement::U64 => {
                let start = cur.u64()?;
                let end = cur.u64()?;
                let interval =
                    Interval::<u64>::new(start, end).ok_or(DecodeError::EmptyInterval {
                        relation,
                        row,
                        field,
                    })?;
                Ok(Value::IntervalU64(interval))
            }
            IntervalElement::I64 => {
                let start = cur.i64()?;
                let end = cur.i64()?;
                let interval =
                    Interval::<i64>::new(start, end).ok_or(DecodeError::EmptyInterval {
                        relation,
                        row,
                        field,
                    })?;
                Ok(Value::IntervalI64(interval))
            }
        },
        ValueType::FixedInterval { element, width } => match element {
            IntervalElement::U64 => {
                let start = cur.u64()?;
                let interval =
                    Interval::<u64>::fixed(start, width).ok_or(DecodeError::IntervalOverflow {
                        relation,
                        row,
                        field,
                    })?;
                Ok(Value::IntervalU64(interval))
            }
            IntervalElement::I64 => {
                let start = cur.i64()?;
                let interval =
                    Interval::<i64>::fixed(start, width).ok_or(DecodeError::IntervalOverflow {
                        relation,
                        row,
                        field,
                    })?;
                Ok(Value::IntervalI64(interval))
            }
        },
    }
}

fn append_entry(out: &mut Vec<u8>, entry: &Entry) {
    match entry {
        Entry::Fact { fid, mode } => {
            out.put(&[CLASS_FACT]);
            out.put(fid);
            out.put(&[mode.wire()]);
        }
        Entry::Key { statement, key } => {
            out.put(&[CLASS_KEY]);
            out.put(&statement.0.to_le_bytes());
            out.put(key);
        }
        Entry::Containment {
            statement,
            key,
            mode,
        } => {
            out.put(&[CLASS_CONTAINMENT]);
            out.put(&statement.0.to_le_bytes());
            out.put(key);
            out.put(&[mode.wire()]);
        }
        Entry::Capacity {
            statement,
            key,
            mode,
        } => {
            out.put(&[CLASS_CAPACITY]);
            out.put(&statement.0.to_le_bytes());
            out.put(key);
            match mode {
                CapacityMode::ChildDelta(delta) => {
                    out.put(&[mode.wire()]);
                    out.put(&delta.to_le_bytes());
                }
                CapacityMode::ParentAdd | CapacityMode::ParentRemove => {
                    out.put(&[mode.wire()]);
                }
            }
        }
    }
}

fn decode_entry(cur: &mut Cursor<'_>, index: usize) -> Result<Entry, DecodeError> {
    let class = cur.u8()?;
    match class {
        CLASS_FACT => {
            let fid = cur.array32()?;
            let mode = match cur.u8()? {
                OP_INSERT => OpKind::Insert,
                OP_DELETE => OpKind::Delete,
                got => {
                    return Err(DecodeError::UnknownFootprintMode { index, class, got });
                }
            };
            Ok(Entry::Fact { fid, mode })
        }
        CLASS_KEY => {
            let statement = StatementId(cur.u16()?);
            let key = cur.array32()?;
            Ok(Entry::Key { statement, key })
        }
        CLASS_CONTAINMENT => {
            let statement = StatementId(cur.u16()?);
            let key = cur.array32()?;
            let mode = match cur.u8()? {
                1 => ContainmentMode::Need,
                2 => ContainmentMode::SupportAdd,
                3 => ContainmentMode::SupportRemove,
                got => {
                    return Err(DecodeError::UnknownFootprintMode { index, class, got });
                }
            };
            Ok(Entry::Containment {
                statement,
                key,
                mode,
            })
        }
        CLASS_CAPACITY => {
            let statement = StatementId(cur.u16()?);
            let key = cur.array32()?;
            let mode = match cur.u8()? {
                1 => CapacityMode::ChildDelta(cur.i64()?),
                2 => CapacityMode::ParentAdd,
                3 => CapacityMode::ParentRemove,
                got => {
                    return Err(DecodeError::UnknownFootprintMode { index, class, got });
                }
            };
            Ok(Entry::Capacity {
                statement,
                key,
                mode,
            })
        }
        got => Err(DecodeError::UnknownFootprintClass { index, got }),
    }
}
