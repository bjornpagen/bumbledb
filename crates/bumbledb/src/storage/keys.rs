//! The `_data` key codec:
//! first-byte namespaces, big-endian components.
//! ```text
//! F | relation_id | row_id                                  facts
//! M | relation_id | fact_hash                               membership
//! U | relation_id | statement | determinant                      FD determinants
//! R | statement | key_bytes | source_rel | source_row      IND reverse edges
//! Q | relation_id | field_id                                fresh sequences
//! S | relation_id | stat                                    counters
//! ```
//! `R` carries no target relation id: the statement id determines the
//! target relation, so storing it again would be transcription.
//! leftover: format-8 R/U statement slots stay [`StatementId`].
//! Fixed-width families (`F`/`M`/`Q`/`S`) return arrays by value. Variable-
//! width writers (`U`/`R`/prefixes) fill a caller-provided `[u8; MAX_KEY]`
//! (post-mortem §25), and key types never derive `Ord` (LMDB byte order

use std::borrow::Borrow;
use std::ops::Deref;

use crate::encoding::field_bytes;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

/// LMDB's default key-size ceiling; every encoded key fits.
pub const MAX_KEY: usize = 511;

/// Fixed scratch buffer for key writers.
pub type KeyBuf = [u8; MAX_KEY];

const DETERMINANT_INLINE: usize = 24;

#[derive(Debug)]
pub(crate) struct DeterminantImage(Image);

#[derive(Debug)]
enum Image {
    Inline {
        len: u8,
        buf: [u8; DETERMINANT_INLINE],
    },
    Spilled(Vec<u8>),
}

impl DeterminantImage {
    #[must_use]
    pub(crate) fn scratch() -> Self {
        Self(Image::Inline {
            len: 0,
            buf: [0; DETERMINANT_INLINE],
        })
    }

    #[must_use]
    pub(crate) fn scratch_with_capacity(capacity: usize) -> Self {
        if capacity <= DETERMINANT_INLINE {
            Self::scratch()
        } else {
            Self(Image::Spilled(Vec::with_capacity(capacity)))
        }
    }

    #[must_use]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Image::Inline { len, buf } => &buf[..usize::from(*len)],
            Image::Spilled(bytes) => bytes,
        }
    }

    fn clear(&mut self) {
        match &mut self.0 {
            Image::Inline { len, .. } => *len = 0,
            Image::Spilled(bytes) => bytes.clear(),
        }
    }

    fn extend(&mut self, bytes: &[u8]) {
        match &mut self.0 {
            Image::Inline { len, buf } => {
                let start = usize::from(*len);
                let end = start + bytes.len();
                if end <= DETERMINANT_INLINE {
                    buf[start..end].copy_from_slice(bytes);
                    *len = u8::try_from(end).expect("inline length fits u8");
                } else {
                    let mut spilled = Vec::with_capacity(end);
                    spilled.extend_from_slice(&buf[..start]);
                    spilled.extend_from_slice(bytes);
                    self.0 = Image::Spilled(spilled);
                }
            }
            Image::Spilled(spilled) => spilled.extend_from_slice(bytes),
        }
    }
}

impl Clone for DeterminantImage {
    fn clone(&self) -> Self {
        let bytes = self.as_bytes();
        if bytes.len() <= DETERMINANT_INLINE {
            let mut buf = [0; DETERMINANT_INLINE];
            buf[..bytes.len()].copy_from_slice(bytes);
            Self(Image::Inline {
                len: u8::try_from(bytes.len()).expect("inline length fits u8"),
                buf,
            })
        } else {
            Self(Image::Spilled(bytes.to_vec()))
        }
    }
}

impl Default for DeterminantImage {
    fn default() -> Self {
        Self::scratch()
    }
}

impl PartialEq for DeterminantImage {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for DeterminantImage {}

impl PartialOrd for DeterminantImage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeterminantImage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl Deref for DeterminantImage {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl AsRef<[u8]> for DeterminantImage {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<[u8]> for DeterminantImage {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(test)]
pub(crate) fn key(write: impl FnOnce(&mut KeyBuf) -> &[u8]) -> Vec<u8> {
    let mut buf = [0u8; MAX_KEY];
    write(&mut buf).to_vec()
}

const R_OVERHEAD: usize = 1 + 2 + 4 + 8;

/// Maximum determinant width a schema may declare.
/// Derivation: a determinant value must embed whole in every key shape that
/// carries one. The `U` key spends `tag(1) + relation(4) + statement(2)`
/// = 7 bytes beside its determinant; the `R` key embeds a whole target-key value
/// as its key-bytes segment and spends [`R_OVERHEAD`] = 15 beside it. The
/// `R` embedding is therefore the binding bound:
/// `MAX_DETERMINANT_WIDTH = MAX_KEY − R_OVERHEAD = 511 − 15 = 496`.
/// Schema-construction hook; rejection is `StatementErrorKind::DeterminantKeyTooWide`
pub const MAX_DETERMINANT_WIDTH: usize = MAX_KEY - R_OVERHEAD;

/// First byte of every `_data` key. `KeyWriter` takes this enum so a
/// typo cannot mint a tag the parsers do not name; each parser narrows
/// the same tag before it reads a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Namespace {
    Fact = b'F',
    Membership = b'M',
    Fresh = b'Q',
    Reverse = b'R',
    Stat = b'S',
    Determinant = b'U',
}

impl Namespace {
    #[must_use]
    pub const fn tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            b'F' => Some(Self::Fact),
            b'M' => Some(Self::Membership),
            b'Q' => Some(Self::Fresh),
            b'R' => Some(Self::Reverse),
            b'S' => Some(Self::Stat),
            b'U' => Some(Self::Determinant),
            _ => None,
        }
    }
}

impl TryFrom<u8> for Namespace {
    type Error = ();

    fn try_from(byte: u8) -> Result<Self, ()> {
        Self::from_byte(byte).ok_or(())
    }
}

fn split_namespace(key: &[u8]) -> Option<(Namespace, &[u8])> {
    let (&tag, rest) = key.split_first()?;
    Some((Namespace::from_byte(tag)?, rest))
}

fn rest_after(key: &[u8], expected: Namespace) -> Option<&[u8]> {
    let (ns, rest) = split_namespace(key)?;
    (ns == expected).then_some(rest)
}

/// Refusal hardening, debug builds: no `F`/`M`/`U`/`R` entry may name a closed
/// relation — the theory is its storage, and the store contains zero
/// vocabulary bytes. The commit plan asserts this at every fact-op
/// derivation (the one place all four namespaces' key bytes originate);
/// release builds rely on the write-surface refusal
/// ([`ClosedRelationWrite`]) and the offline sweeper's
/// `ClosedRelationEntry` conviction.
/// [`ClosedRelationWrite`]: crate::error::Error::ClosedRelationWrite
#[inline]
pub fn debug_assert_ordinary(schema: &crate::schema::Schema, relation: RelationId) {
    debug_assert!(
        schema.relation(relation).body().closed_rows().is_none(),
        "no F/M/U/R namespace entry may name closed relation {relation:?}"
    );
}

/// Which per-relation counter an `S` key addresses.
/// `RowCount` is the planner's statistic;
/// `RowIdHighWater` is the commit pipeline's row-id allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum StatKind {
    RowCount = 0,
    RowIdHighWater = 1,
}

impl StatKind {
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::RowCount),
            1 => Some(Self::RowIdHighWater),
            _ => None,
        }
    }
}

/// The `S` key's stat slot: a known kind, or a stored byte the enum does
/// not name. The sweeper needs the unknown arm; nobody needs the bare
/// discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatEntry {
    Known(StatKind),
    Unknown(u8),
}

impl StatEntry {
    const fn from_byte(byte: u8) -> Self {
        match StatKind::from_byte(byte) {
            Some(kind) => Self::Known(kind),
            None => Self::Unknown(byte),
        }
    }
}

struct KeyWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> KeyWriter<'a> {
    fn new(buf: &'a mut [u8], namespace: Namespace) -> Self {
        buf[0] = namespace.tag();
        Self { buf, len: 1 }
    }

    fn put(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        self
    }

    fn relation(&mut self, id: RelationId) -> &mut Self {
        self.put(&id.0.to_be_bytes())
    }

    fn statement(&mut self, id: StatementId) -> &mut Self {
        self.put(&id.0.to_be_bytes())
    }

    fn finish(self) -> &'a [u8] {
        &self.buf[..self.len]
    }
}

fn write_fixed<const N: usize>(
    namespace: Namespace,
    fill: impl FnOnce(&mut KeyWriter<'_>),
) -> [u8; N] {
    let mut buf = [0u8; N];
    let mut writer = KeyWriter::new(&mut buf, namespace);
    fill(&mut writer);
    debug_assert_eq!(writer.len, N);
    buf
}

fn write_var(buf: &mut [u8], namespace: Namespace, fill: impl FnOnce(&mut KeyWriter<'_>)) -> &[u8] {
    let mut writer = KeyWriter::new(buf, namespace);
    fill(&mut writer);
    writer.finish()
}

/// `F | relation | row_id` — a stored fact's key.
/// `F` key width: tag + relation + row id.
pub const FACT_KEY_LEN: usize = 1 + 4 + 8;
/// `M` key width: tag + relation + 32-byte hash.
pub const MEMBERSHIP_KEY_LEN: usize = 1 + 4 + 32;
/// `Q` key width: tag + relation + field.
pub const FRESH_KEY_LEN: usize = 1 + 4 + 2;
/// `S` key width: tag + relation + stat kind.
pub const STAT_KEY_LEN: usize = 1 + 4 + 1;

#[must_use]
pub fn fact_key(relation: RelationId, row_id: u64) -> [u8; FACT_KEY_LEN] {
    write_fixed(Namespace::Fact, |w| {
        w.relation(relation).put(&row_id.to_be_bytes());
    })
}

/// `F | relation` — the prefix every fact of a relation shares (scan reader).
pub fn fact_prefix(buf: &mut KeyBuf, relation: RelationId) -> &[u8] {
    write_var(buf, Namespace::Fact, |w| {
        w.relation(relation);
    })
}

/// `M | relation | fact_hash` — the membership key.
#[must_use]
pub fn membership_key(relation: RelationId, fact_hash: &[u8; 32]) -> [u8; MEMBERSHIP_KEY_LEN] {
    write_fixed(Namespace::Membership, |w| {
        w.relation(relation).put(fact_hash);
    })
}

/// `U | relation | statement | determinant` — an FD determinant key. `determinant` is the
/// concatenated canonical encodings of the statement's projected fields in
/// statement projection order ([`determinant_image`]; width-bounded at schema
/// construction).
pub fn determinant_key<'a>(
    buf: &'a mut KeyBuf,
    relation: RelationId,
    statement: StatementId,
    determinant: &[u8],
) -> &'a [u8] {
    debug_assert!(determinant.len() <= MAX_DETERMINANT_WIDTH);
    write_var(buf, Namespace::Determinant, |w| {
        w.relation(relation).statement(statement).put(determinant);
    })
}

/// `R | statement | key_bytes | source_rel | source_row` — one reverse-edge
/// entry (target-side containment reader). Statement-scoped: the statement
/// id determines the target relation, so none is stored.
pub fn reverse_key<'a>(
    buf: &'a mut KeyBuf,
    statement: StatementId,
    key_bytes: &[u8],
    source_relation: RelationId,
    source_row: u64,
) -> &'a [u8] {
    debug_assert!(key_bytes.len() <= MAX_DETERMINANT_WIDTH);
    write_var(buf, Namespace::Reverse, |w| {
        w.statement(statement)
            .put(key_bytes)
            .relation(source_relation)
            .put(&source_row.to_be_bytes());
    })
}

/// requiring one target key value (reverse-edge prefix-scan reader).
pub fn reverse_prefix<'a>(
    buf: &'a mut KeyBuf,
    statement: StatementId,
    key_bytes: &[u8],
) -> &'a [u8] {
    debug_assert!(key_bytes.len() <= MAX_DETERMINANT_WIDTH);
    write_var(buf, Namespace::Reverse, |w| {
        w.statement(statement).put(key_bytes);
    })
}

/// `Q | relation | field` — a fresh sequence's key.
#[must_use]
pub fn fresh_key(relation: RelationId, field: FieldId) -> [u8; FRESH_KEY_LEN] {
    write_fixed(Namespace::Fresh, |w| {
        w.relation(relation).put(&field.0.to_be_bytes());
    })
}

/// `S | relation | stat` — a per-relation counter's key.
#[must_use]
pub fn stat_key(relation: RelationId, stat: StatKind) -> [u8; STAT_KEY_LEN] {
    write_fixed(Namespace::Stat, |w| {
        w.relation(relation).put(&[stat as u8]);
    })
}

/// Splits a full `R` key into `(statement, key_bytes, source_rel,
/// source_row)`. The key bytes are everything between the statement id and
/// the fixed 12-byte source tail — self-delimiting, no width table needed.
/// `None` on anything not shaped like a reverse-edge key (corrupt data).
#[must_use]
pub fn parse_reverse_key(key: &[u8]) -> Option<(StatementId, &[u8], RelationId, u64)> {
    let rest = rest_after(key, Namespace::Reverse)?;
    let (&statement, rest) = rest.split_first_chunk()?;
    let (rest, &source_row) = rest.split_last_chunk()?;
    let (key_bytes, &source_relation) = rest.split_last_chunk()?;
    Some((
        StatementId(u16::from_be_bytes(statement)),
        key_bytes,
        RelationId(u32::from_be_bytes(source_relation)),
        u64::from_be_bytes(source_row),
    ))
}

/// Splits a full `F` key into `(relation, row_id)`. `None` on anything
/// not exactly the codec's fixed 13-byte fact-key shape (corrupt data) —
/// the split chain is the length check.
#[must_use]
pub fn parse_fact_key(key: &[u8]) -> Option<(RelationId, u64)> {
    let rest = rest_after(key, Namespace::Fact)?;
    let (&relation, rest) = rest.split_first_chunk()?;
    let &row_id = <&[u8; 8]>::try_from(rest).ok()?;
    Some((
        RelationId(u32::from_be_bytes(relation)),
        u64::from_be_bytes(row_id),
    ))
}

/// Splits a full `M` key into `(relation, fact_hash)`. `None` on anything
/// not exactly the codec's fixed 37-byte membership-key shape.
#[must_use]
pub fn parse_membership_key(key: &[u8]) -> Option<(RelationId, &[u8; 32])> {
    let rest = rest_after(key, Namespace::Membership)?;
    let (&relation, rest) = rest.split_first_chunk()?;
    let hash = <&[u8; 32]>::try_from(rest).ok()?;
    Some((RelationId(u32::from_be_bytes(relation)), hash))
}

/// Splits a full `U` key into `(relation, statement, determinant)`. `None` when
/// the header is short or the determinant empty (projections are non-empty by
/// validation, so an empty determinant is corrupt data).
#[must_use]
pub fn parse_determinant_key(key: &[u8]) -> Option<(RelationId, StatementId, &[u8])> {
    let rest = rest_after(key, Namespace::Determinant)?;
    let (&relation, rest) = rest.split_first_chunk()?;
    let (&statement, determinant) = rest.split_first_chunk()?;
    if determinant.is_empty() {
        return None;
    }
    Some((
        RelationId(u32::from_be_bytes(relation)),
        StatementId(u16::from_be_bytes(statement)),
        determinant,
    ))
}

/// Splits a full `Q` key into `(relation, field)`. `None` on anything
/// not exactly the codec's fixed 7-byte fresh-key shape.
#[must_use]
pub fn parse_fresh_key(key: &[u8]) -> Option<(RelationId, FieldId)> {
    let rest = rest_after(key, Namespace::Fresh)?;
    let (&relation, rest) = rest.split_first_chunk()?;
    let &field = <&[u8; 2]>::try_from(rest).ok()?;
    Some((
        RelationId(u32::from_be_bytes(relation)),
        FieldId(u16::from_be_bytes(field)),
    ))
}

/// Splits a full `S` key into `(relation, stat)`. `None` on anything
/// not exactly the codec's fixed 6-byte stat-key shape.
#[must_use]
pub fn parse_stat_key(key: &[u8]) -> Option<(RelationId, StatEntry)> {
    let rest = rest_after(key, Namespace::Stat)?;
    let (&relation, rest) = rest.split_first_chunk()?;
    match rest {
        &[stat] => Some((
            RelationId(u32::from_be_bytes(relation)),
            StatEntry::from_byte(stat),
        )),
        _ => None,
    }
}

/// Concatenates the canonical encodings of `projection`'s fields, sliced
/// out of `fact_bytes`, in statement projection order, into `out` — the
/// determinant segment of a `U` key, re-derived per fact, never a scan.
/// An interval field copies its whole encoded tail in one piece — 16-byte
/// `start ‖ end` general, the 8-byte start for a fixed-width
/// `interval<E, w>` position (the slice width comes from the layout —
/// never split here): the contiguity is what keeps the determinant B-tree
/// ordered by interval start within one scalar-prefix group (the fixed
pub fn determinant_image<'a>(
    fact: crate::encoding::FactView<'_, '_>,
    projection: &[FieldId],
    out: &'a mut DeterminantImage,
) -> &'a DeterminantImage {
    out.clear();
    for &field in projection {
        out.extend(field_bytes(fact, usize::from(field.0)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{
        FactLayout, ValueRef, ValueType, encode_fact, encode_interval_u64, encode_u64,
    };
    use bumbledb_theory::schema::IntervalElement;

    #[test]
    fn fact_key_round_trips_components() {
        let k = fact_key(RelationId(7), 0x0102_0304_0506_0708).to_vec();
        assert_eq!(k.len(), FACT_KEY_LEN);
        assert_eq!(k[0], Namespace::Fact.tag());
        assert_eq!(&k[1..5], &7u32.to_be_bytes());
        assert_eq!(&k[5..], &0x0102_0304_0506_0708u64.to_be_bytes());
    }

    #[test]
    fn fact_prefix_is_a_prefix_of_every_fact_key() {
        let prefix = key(|b| fact_prefix(b, RelationId(7)));
        let k = fact_key(RelationId(7), 42).to_vec();
        assert!(k.starts_with(&prefix));
        let other = fact_key(RelationId(8), 42).to_vec();
        assert!(!other.starts_with(&prefix));
    }

    #[test]
    fn membership_key_embeds_full_hash() {
        let hash = [0xABu8; 32];
        let k = membership_key(RelationId(1), &hash).to_vec();
        assert_eq!(k.len(), MEMBERSHIP_KEY_LEN);
        assert_eq!(k[0], Namespace::Membership.tag());
        assert_eq!(&k[5..], &hash);
    }

    #[test]
    fn determinant_key_golden_bytes() {
        let determinant = [1u8, 2, 3];
        let u = key(|b| determinant_key(b, RelationId(2), StatementId(5), &determinant));
        assert_eq!(
            u,
            vec![Namespace::Determinant.tag(), 0, 0, 0, 2, 0, 5, 1, 2, 3]
        );
    }

    #[test]
    fn determinant_key_keeps_a_16_byte_interval_determinant_contiguous() {
        let mut determinant = Vec::new();
        determinant.extend_from_slice(&encode_u64(0xAAAA_BBBB_CCCC_DDDD));
        determinant.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
        ));
        assert_eq!(determinant.len(), 24);

        let k = key(|b| determinant_key(b, RelationId(3), StatementId(9), &determinant));
        assert_eq!(k.len(), 7 + 24);

        assert_eq!(
            &k[7 + 8..],
            encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval")
            )
        );
        assert_eq!(&k[7..], &determinant[..]);
    }

    #[test]
    fn reverse_key_golden_bytes_are_statement_scoped() {
        let key_bytes = [7u8, 8];
        let r = key(|b| reverse_key(b, StatementId(5), &key_bytes, RelationId(9), 11));
        assert_eq!(
            r,
            vec![
                Namespace::Reverse.tag(),
                0,
                5,
                7,
                8,
                0,
                0,
                0,
                9,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                11
            ]
        );
        assert_eq!(r.len(), R_OVERHEAD + key_bytes.len());
        let prefix = key(|b| reverse_prefix(b, StatementId(5), &key_bytes));
        assert!(r.starts_with(&prefix));
    }

    #[test]
    fn reverse_key_with_interval_bearing_key_bytes_parses_back() {
        let mut key_bytes = Vec::new();
        key_bytes.extend_from_slice(&encode_u64(4));
        key_bytes.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(100, 200).expect("nonempty interval"),
        ));

        let r = key(|b| reverse_key(b, StatementId(2), &key_bytes, RelationId(6), 77));
        let (stmt, parsed, src_rel, src_row) =
            parse_reverse_key(&r).expect("well-formed reverse key");
        assert_eq!(stmt, StatementId(2));
        assert_eq!(parsed, &key_bytes[..]);
        assert_eq!(src_rel, RelationId(6));
        assert_eq!(src_row, 77);
    }

    #[test]
    fn parsers_reject_other_namespace_and_truncated_keys() {
        let membership = membership_key(RelationId(1), &[0u8; 32]).to_vec();
        assert!(parse_fact_key(&membership).is_none());
        let mut thirteen_byte_membership = membership[..FACT_KEY_LEN].to_vec();
        thirteen_byte_membership[0] = Namespace::Membership.tag();
        assert!(
            parse_fact_key(&thirteen_byte_membership).is_none(),
            "an M-tagged 13-byte key is not a fact key"
        );
        let determinant = key(|b| determinant_key(b, RelationId(1), StatementId(1), &[9]));
        assert!(parse_reverse_key(&determinant).is_none());
        let reverse = key(|b| reverse_key(b, StatementId(1), &[9], RelationId(1), 1));
        assert!(parse_reverse_key(&reverse[..R_OVERHEAD - 1]).is_none());
    }

    fn interval_layout() -> FactLayout {
        FactLayout::new(&[
            ValueType::U64,
            ValueType::Interval {
                element: IntervalElement::U64,
            },
            ValueType::U64,
        ])
    }

    fn interval_fact() -> Vec<u8> {
        let mut fact = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(0x1111_1111_1111_1111),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval"),
                ),
                ValueRef::U64(0x2222_2222_2222_2222),
            ],
            &interval_layout(),
            &mut fact,
        );
        fact
    }

    #[test]
    fn determinant_image_slices_projection_order_and_copies_intervals_whole() {
        let layout = interval_layout();
        let fact = interval_fact();
        let mut determinant = DeterminantImage::scratch();

        determinant_image(
            layout.encoded(&fact),
            &[FieldId(2), FieldId(1)],
            &mut determinant,
        );

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222));
        expected.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval"),
        ));
        assert_eq!(determinant.as_bytes(), expected);
    }

    #[test]
    fn sealed_key_projection_lays_fields_in_target_key_order() {
        let layout = interval_layout();
        let fact = interval_fact();

        let key_projection = [FieldId(0), FieldId(2), FieldId(1)];
        let mut key_bytes = DeterminantImage::scratch();
        determinant_image(layout.encoded(&fact), &key_projection, &mut key_bytes);

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x1111_1111_1111_1111));
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222));
        expected.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval"),
        ));
        assert_eq!(key_bytes.as_bytes(), expected);

        let r = key(|b| reverse_key(b, StatementId(4), &key_bytes, RelationId(1), 5));
        let (stmt, parsed, src_rel, src_row) =
            parse_reverse_key(&r).expect("well-formed reverse key");
        assert_eq!(
            (stmt, parsed, src_rel, src_row),
            (StatementId(4), key_bytes.as_bytes(), RelationId(1), 5)
        );
    }

    #[test]
    fn fresh_and_stat_keys() {
        let q = fresh_key(RelationId(3), FieldId(4)).to_vec();
        assert_eq!(q, vec![Namespace::Fresh.tag(), 0, 0, 0, 3, 0, 4]);
        let s = stat_key(RelationId(3), StatKind::RowCount).to_vec();
        assert_eq!(s, vec![Namespace::Stat.tag(), 0, 0, 0, 3, 0]);
        assert_eq!(
            parse_stat_key(&s),
            Some((RelationId(3), StatEntry::Known(StatKind::RowCount)))
        );
        let hw = stat_key(RelationId(3), StatKind::RowIdHighWater).to_vec();
        assert_eq!(hw, vec![Namespace::Stat.tag(), 0, 0, 0, 3, 1]);
        assert_eq!(
            parse_stat_key(&hw),
            Some((RelationId(3), StatEntry::Known(StatKind::RowIdHighWater)))
        );
        let mut unknown = s.clone();
        *unknown.last_mut().expect("stat byte") = 0xAB;
        assert_eq!(
            parse_stat_key(&unknown),
            Some((RelationId(3), StatEntry::Unknown(0xAB)))
        );
    }

    #[test]
    fn keys_sort_by_namespace_then_components() {
        let ordered = vec![
            fact_key(RelationId(1), 5).to_vec(),
            fact_key(RelationId(1), 6).to_vec(),
            fact_key(RelationId(2), 0).to_vec(),
            membership_key(RelationId(1), &[0u8; 32]).to_vec(),
            membership_key(RelationId(1), &[1u8; 32]).to_vec(),
            fresh_key(RelationId(1), FieldId(0)).to_vec(),
            fresh_key(RelationId(1), FieldId(1)).to_vec(),
            key(|b| reverse_key(b, StatementId(0), &[9], RelationId(0), 0)),
            key(|b| reverse_key(b, StatementId(1), &[0], RelationId(0), 0)),
            stat_key(RelationId(1), StatKind::RowCount).to_vec(),
            stat_key(RelationId(1), StatKind::RowIdHighWater).to_vec(),
            key(|b| determinant_key(b, RelationId(1), StatementId(0), &[0])),
            key(|b| determinant_key(b, RelationId(1), StatementId(0), &[1])),
            key(|b| determinant_key(b, RelationId(1), StatementId(1), &[0])),
        ];
        let mut sorted = ordered.clone();
        sorted.sort();
        assert_eq!(ordered, sorted);
    }

    #[test]
    fn determinant_image_identity_is_bytes_across_representations() {
        let wide = vec![0xCD; DETERMINANT_INLINE + 8];
        let mut spilled = DeterminantImage::scratch();
        spilled.extend(&wide);
        assert_eq!(spilled.as_bytes(), &wide[..]);
        spilled.clear();
        spilled.extend(&[1, 2, 3]);
        let mut inline = DeterminantImage::scratch();
        inline.extend(&[1, 2, 3]);
        assert_eq!(spilled, inline);
        assert_eq!(spilled.cmp(&inline), std::cmp::Ordering::Equal);
        assert_eq!(
            <DeterminantImage as std::borrow::Borrow<[u8]>>::borrow(&spilled),
            &[1, 2, 3]
        );

        let mut crossing = DeterminantImage::scratch_with_capacity(8);
        crossing.extend(&[9; DETERMINANT_INLINE - 1]);
        crossing.extend(&[7, 7]);
        let mut expected = vec![9u8; DETERMINANT_INLINE - 1];
        expected.extend_from_slice(&[7, 7]);
        assert_eq!(crossing.as_bytes(), &expected[..]);

        assert_eq!(spilled.clone(), spilled);
        assert_eq!(crossing.clone().as_bytes(), crossing.as_bytes());
    }

    #[test]
    fn determinant_image_spill_boundary_is_exactly_inline_capacity() {
        let mut at_cap = DeterminantImage::scratch();
        at_cap.extend(&encode_u64(0xDEAD_BEEF_0000_0001));
        at_cap.extend(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(5, 6).expect("nonempty interval"),
        ));
        assert_eq!(at_cap.as_bytes().len(), DETERMINANT_INLINE);
        assert!(
            matches!(at_cap.0, Image::Inline { .. }),
            "24 bytes must not spill"
        );

        let mut crossing = DeterminantImage::scratch();
        crossing.extend(&[0xAB; 9]);
        crossing.extend(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(7, 9).expect("nonempty interval"),
        ));
        assert_eq!(crossing.as_bytes().len(), 25);
        assert!(matches!(crossing.0, Image::Spilled(_)), "25 bytes spill");
        let mut expected = vec![0xABu8; 9];
        expected.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(7, 9).expect("nonempty interval"),
        ));
        assert_eq!(crossing.as_bytes(), &expected[..]);

        let mut plus_one = DeterminantImage::scratch();
        plus_one.extend(&[0x11; DETERMINANT_INLINE]);
        assert!(matches!(plus_one.0, Image::Inline { .. }));
        plus_one.extend(&[0x22]);
        assert!(matches!(plus_one.0, Image::Spilled(_)));
        let mut expected = vec![0x11u8; DETERMINANT_INLINE];
        expected.push(0x22);
        assert_eq!(plus_one.as_bytes(), &expected[..]);

        assert!(matches!(at_cap.clone().0, Image::Inline { .. }));
        assert!(matches!(plus_one.clone().0, Image::Spilled(_)));
        assert_eq!(at_cap.clone(), at_cap);
        assert_eq!(plus_one.clone(), plus_one);
    }

    /// would diverge from LMDB's byte order.
    #[test]
    fn determinant_image_order_is_byte_order_across_representations() {
        let of = |bytes: &[u8]| {
            let mut image = DeterminantImage::scratch();
            image.extend(bytes);
            image
        };

        let spilled_of = |bytes: &[u8]| {
            let mut image = DeterminantImage::scratch();
            image.extend(&[0u8; DETERMINANT_INLINE + 1]);
            image.clear();
            image.extend(bytes);
            assert!(matches!(image.0, Image::Spilled(_)));
            image
        };
        let prefix24 = vec![0x7Fu8; DETERMINANT_INLINE];
        let mut extended25 = prefix24.clone();
        extended25.push(0x00);
        let corpus: Vec<Vec<u8>> = vec![
            vec![],
            vec![0x00],
            vec![0x00, 0xFF],
            vec![0x01],
            prefix24.clone(),
            extended25.clone(),
            vec![0x80; DETERMINANT_INLINE + 8],
            vec![0xFF],
        ];
        let mut images: Vec<DeterminantImage> = Vec::new();
        for bytes in &corpus {
            images.push(of(bytes));
            if bytes.len() <= DETERMINANT_INLINE {
                images.push(spilled_of(bytes));
            }
        }
        let mut sorted_images = images;
        sorted_images.sort();
        let sorted_bytes: Vec<Vec<u8>> = sorted_images
            .iter()
            .map(|i| i.as_bytes().to_vec())
            .collect();
        let mut expected = sorted_bytes.clone();
        expected.sort();
        assert_eq!(sorted_bytes, expected, "Ord must equal byte order");

        assert!(of(&prefix24) < of(&extended25));

        assert_eq!(
            of(&prefix24).cmp(&spilled_of(&prefix24)),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            <DeterminantImage as Borrow<[u8]>>::borrow(&spilled_of(&prefix24)),
            &prefix24[..]
        );
    }

    #[test]
    fn determinant_width_bound_matches_reverse_overhead() {
        assert_eq!(MAX_DETERMINANT_WIDTH, 511 - 15);

        let determinant = vec![0xEE; MAX_DETERMINANT_WIDTH];
        let r = key(|b| reverse_key(b, StatementId(0), &determinant, RelationId(0), 0));
        assert_eq!(r.len(), MAX_KEY);
        let u = key(|b| determinant_key(b, RelationId(0), StatementId(0), &determinant));
        assert!(u.len() <= MAX_KEY);
    }
}
