//! The `_data` key codec (`docs/architecture/50-storage.md` § Key layout):
//! first-byte namespaces, big-endian components.
//!
//! ```text
//! F | relation_id | row_id                                  facts
//! M | relation_id | fact_hash                               membership
//! U | relation_id | statement | determinant                      FD determinants
//! R | statement | key_bytes | source_rel | source_row      IND reverse edges
//! Q | relation_id | field_id                                fresh sequences
//! S | relation_id | stat                                    counters
//! ```
//!
//! `R` carries no target relation id: the statement id determines the
//! target relation, so storing it again would be transcription.
//!
//! Writers fill a caller-provided `[u8; MAX_KEY]` scratch and return the
//! written length — no oversized zeroing (post-mortem §25), and key types
//! never derive `Ord` (LMDB byte order is the only order).

use std::borrow::Borrow;
use std::ops::Deref;

use crate::encoding::{FactLayout, field_bytes};
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

/// LMDB's default key-size ceiling; every encoded key fits.
pub const MAX_KEY: usize = 511;

/// Fixed scratch buffer for key writers.
pub type KeyBuf = [u8; MAX_KEY];

/// Inline capacity of a [`DeterminantImage`]: one 8-byte scalar word
/// beside a whole 16-byte interval tail — the widest common determinant
/// shape — stays off the heap. Wider determinants (schema-bounded at
/// [`MAX_DETERMINANT_WIDTH`]) spill to an owned heap buffer.
const DETERMINANT_INLINE: usize = 24;

/// Owned canonical bytes of one functionality determinant.
///
/// The inner buffer is deliberately private: determinant images originate
/// only in this codec, while callers may retain, compare, and borrow the
/// resulting bytes without laundering arbitrary byte vectors into the type.
/// Identity is the byte string alone — `Eq`/`Ord` compare [`Self::as_bytes`]
/// whatever the representation (the `Borrow<[u8]>` consistency contract) —
/// and `Clone` re-inlines anything that fits, so retaining a typical
/// determinant never allocates.
#[derive(Debug)]
pub(crate) struct DeterminantImage(Image);

/// The two representations: a filled prefix of the fixed inline buffer,
/// or the spilled heap buffer. `clear` keeps a spilled buffer's capacity
/// (monotone high-water), so a wide-determinant scratch spills once, not
/// per fact.
#[derive(Debug)]
enum Image {
    Inline {
        len: u8,
        buf: [u8; DETERMINANT_INLINE],
    },
    Spilled(Vec<u8>),
}

impl DeterminantImage {
    /// Empty reusable output for the two determinant encoders below.
    #[must_use]
    pub(crate) fn scratch() -> Self {
        Self(Image::Inline {
            len: 0,
            buf: [0; DETERMINANT_INLINE],
        })
    }

    /// Empty reusable output with an expected determinant width.
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

    /// Codec-private: the two encoders below reset their output in place.
    fn clear(&mut self) {
        match &mut self.0 {
            Image::Inline { len, .. } => *len = 0,
            Image::Spilled(bytes) => bytes.clear(),
        }
    }

    /// Codec-private: appends canonical field bytes, spilling once past
    /// the inline capacity.
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
pub(crate) fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
    let mut buf = [0u8; MAX_KEY];
    let len = write(&mut buf);
    buf[..len].to_vec()
}

/// Byte overhead of a reverse-edge (`R`) key beyond its embedded key bytes:
/// `tag(1) + statement(2) + source_rel(4) + source_row(8)`.
const R_OVERHEAD: usize = 1 + 2 + 4 + 8;

/// Maximum determinant width a schema may declare.
///
/// Derivation: a determinant value must embed whole in every key shape that
/// carries one. The `U` key spends `tag(1) + relation(4) + statement(2)`
/// = 7 bytes beside its determinant; the `R` key embeds a whole target-key value
/// as its key-bytes segment and spends [`R_OVERHEAD`] = 15 beside it. The
/// `R` embedding is therefore the binding bound:
/// `MAX_DETERMINANT_WIDTH = MAX_KEY − R_OVERHEAD = 511 − 15 = 496`.
///
/// Schema-construction hook; rejection is `SchemaError::DeterminantKeyTooWide`
/// (the validator imports this constant — the bound has one owner).
pub const MAX_DETERMINANT_WIDTH: usize = MAX_KEY - R_OVERHEAD;

/// Namespace tags, one byte, first in every key.
pub const NS_FACT: u8 = b'F';
pub const NS_MEMBERSHIP: u8 = b'M';
pub const NS_DETERMINANT: u8 = b'U';
pub const NS_REVERSE: u8 = b'R';
pub const NS_FRESH: u8 = b'Q';
pub const NS_STAT: u8 = b'S';

/// Refusal hardening, debug builds (`docs/architecture/50-storage.md`
/// § virtual relations): no `F`/`M`/`U`/`R` entry may name a closed
/// relation — the theory is its storage, and the store contains zero
/// vocabulary bytes. The commit plan asserts this at every fact-op
/// derivation (the one place all four namespaces' key bytes originate);
/// release builds rely on the write-surface refusal
/// ([`ClosedRelationWrite`]) and the offline sweeper's
/// `ClosedRelationEntry` conviction.
///
/// [`ClosedRelationWrite`]: crate::error::Error::ClosedRelationWrite
#[inline]
pub fn debug_assert_ordinary(schema: &crate::schema::Schema, relation: RelationId) {
    debug_assert!(
        !schema.relation(relation).is_closed(),
        "no F/M/U/R namespace entry may name closed relation {relation:?}"
    );
}

/// Which per-relation counter an `S` key addresses.
///
/// `RowCount` is the planner's statistic (`docs/architecture/50-storage.md`);
/// `RowIdHighWater` is the commit pipeline's row-id allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatKind {
    RowCount = 0,
    RowIdHighWater = 1,
}

/// Cursor over a key scratch buffer; every writer is a straight-line
/// sequence of `put_*` calls returning the final length.
struct KeyWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> KeyWriter<'a> {
    fn new(buf: &'a mut [u8], namespace: u8) -> Self {
        buf[0] = namespace;
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

    fn finish(&self) -> usize {
        self.len
    }
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

pub fn fact_key(buf: &mut [u8], relation: RelationId, row_id: u64) -> usize {
    KeyWriter::new(buf, NS_FACT)
        .relation(relation)
        .put(&row_id.to_be_bytes())
        .finish()
}

/// `F | relation` — the prefix every fact of a relation shares (scan reader).
pub fn fact_prefix(buf: &mut KeyBuf, relation: RelationId) -> usize {
    KeyWriter::new(buf, NS_FACT).relation(relation).finish()
}

/// `M | relation | fact_hash` — the membership key.
pub fn membership_key(buf: &mut [u8], relation: RelationId, fact_hash: &[u8; 32]) -> usize {
    KeyWriter::new(buf, NS_MEMBERSHIP)
        .relation(relation)
        .put(fact_hash)
        .finish()
}

/// `U | relation | statement | determinant` — an FD determinant key. `determinant` is the
/// concatenated canonical encodings of the statement's projected fields in
/// statement projection order ([`determinant_image`]; width-bounded at schema
/// construction).
pub fn determinant_key(
    buf: &mut KeyBuf,
    relation: RelationId,
    statement: StatementId,
    determinant: &[u8],
) -> usize {
    debug_assert!(determinant.len() <= MAX_DETERMINANT_WIDTH);
    KeyWriter::new(buf, NS_DETERMINANT)
        .relation(relation)
        .statement(statement)
        .put(determinant)
        .finish()
}

/// `R | statement | key_bytes | source_rel | source_row` — one reverse-edge
/// entry (target-side containment reader). Statement-scoped: the statement
/// id determines the target relation, so none is stored.
pub fn reverse_key(
    buf: &mut KeyBuf,
    statement: StatementId,
    key_bytes: &[u8],
    source_relation: RelationId,
    source_row: u64,
) -> usize {
    debug_assert!(key_bytes.len() <= MAX_DETERMINANT_WIDTH);
    KeyWriter::new(buf, NS_REVERSE)
        .statement(statement)
        .put(key_bytes)
        .relation(source_relation)
        .put(&source_row.to_be_bytes())
        .finish()
}

/// `R | statement | key_bytes` — the prefix shared by every source fact
/// requiring one target key value (reverse-edge prefix-scan reader).
pub fn reverse_prefix(buf: &mut KeyBuf, statement: StatementId, key_bytes: &[u8]) -> usize {
    debug_assert!(key_bytes.len() <= MAX_DETERMINANT_WIDTH);
    KeyWriter::new(buf, NS_REVERSE)
        .statement(statement)
        .put(key_bytes)
        .finish()
}

/// `Q | relation | field` — a fresh sequence's key.
pub fn fresh_key(buf: &mut [u8], relation: RelationId, field: FieldId) -> usize {
    KeyWriter::new(buf, NS_FRESH)
        .relation(relation)
        .put(&field.0.to_be_bytes())
        .finish()
}

/// `S | relation | stat` — a per-relation counter's key.
pub fn stat_key(buf: &mut [u8], relation: RelationId, stat: StatKind) -> usize {
    KeyWriter::new(buf, NS_STAT)
        .relation(relation)
        .put(&[stat as u8])
        .finish()
}

/// Splits a full `R` key into `(statement, key_bytes, source_rel,
/// source_row)`. The key bytes are everything between the statement id and
/// the fixed 12-byte source tail — self-delimiting, no width table needed.
/// `None` on anything not shaped like a reverse-edge key (corrupt data).
#[must_use]
pub fn parse_reverse_key(key: &[u8]) -> Option<(StatementId, &[u8], RelationId, u64)> {
    // The split chain is the length check: tag(1) + statement(2) off the
    // front, source_row(8) then source_rel(4) off the back — anything
    // shorter than R_OVERHEAD fails a split.
    let (&namespace, rest) = key.split_first()?;
    if namespace != NS_REVERSE {
        return None;
    }
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
    let (_, rest) = key.split_first()?;
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
    let (_, rest) = key.split_first()?;
    let (&relation, rest) = rest.split_first_chunk()?;
    let hash = <&[u8; 32]>::try_from(rest).ok()?;
    Some((RelationId(u32::from_be_bytes(relation)), hash))
}

/// Splits a full `U` key into `(relation, statement, determinant)`. `None` when
/// the header is short or the determinant empty (projections are non-empty by
/// validation, so an empty determinant is corrupt data).
#[must_use]
pub fn parse_determinant_key(key: &[u8]) -> Option<(RelationId, StatementId, &[u8])> {
    let (_, rest) = key.split_first()?;
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

/// Splits a full `S` key into `(relation, stat byte)`. `None` on anything
/// not exactly the codec's fixed 6-byte stat-key shape.
#[must_use]
pub fn parse_stat_key(key: &[u8]) -> Option<(RelationId, u8)> {
    let (_, rest) = key.split_first()?;
    let (&relation, rest) = rest.split_first_chunk()?;
    match rest {
        &[stat] => Some((RelationId(u32::from_be_bytes(relation)), stat)),
        _ => None,
    }
}

/// Concatenates the canonical encodings of `projection`'s fields, sliced
/// out of `fact_bytes`, in statement projection order, into `out` — the
/// determinant segment of a `U` key, re-derived per fact, never a scan.
///
/// An interval field copies its whole encoded tail in one piece — 16-byte
/// `start ‖ end` general, the 8-byte start for a fixed-width
/// `interval<E, w>` position (the slice width comes from the layout —
/// never split here): the contiguity is what keeps the determinant B-tree
/// ordered by interval start within one scalar-prefix group (the fixed
/// family's one word is the start itself).
///
/// MEASURED-LAW GRAVESTONE (cleanup-0.5.0 ruling 8, reversed by its own
/// clause 2026-07-24, finding 034): the split's recorded cost — the
/// identity-permuted route 1.23–1.25× slower per fact (13 vs 17
/// ns/fact, commit-shaped 3-field interval projection, warm DRAM,
/// interleaved min-of-7 × 200k facts, two process runs; pre-stated bar
/// 1.09) — was entirely the permuted arm's per-fact O(k²) inverse
/// search. The reversal condition ("precomputes its inverse, the search
/// hoisted out of the per-fact loop") is now the representation:
/// validation mints the permutation in inverse form and
/// [`permuted_determinant_image`] is a straight indexed gather — this
/// direct arm IS that gather under the identity, so the pair stays
/// split only for the identity case's spelled clarity. Re-measure rides
/// the end-of-campaign bench night (R20/R21).
pub fn determinant_image<'a>(
    layout: &FactLayout,
    projection: &[FieldId],
    fact_bytes: &[u8],
    out: &'a mut DeterminantImage,
) -> &'a DeterminantImage {
    out.clear();
    for &field in projection {
        out.extend(field_bytes(fact_bytes, layout, usize::from(field.0)));
    }
    out
}

/// Like [`determinant_image`], but lays the sliced fields down in the *target
/// key's* determinant order: `key_permutation[d]` is the projection index
/// whose field lands at determinant position `d` (the INVERSE form,
/// minted at validate —
/// `Enforcement::{ScalarProbe, IntervalCoverage}::key_permutation`) — the
/// key-bytes segment of an `R` key, one indexed gather per position.
/// Interval fields copy their whole encoded tail (16 bytes general, the
/// 8-byte fixed start), exactly as in [`determinant_image`].
pub fn permuted_determinant_image<'a>(
    layout: &FactLayout,
    projection: &[FieldId],
    key_permutation: &[u16],
    fact_bytes: &[u8],
    out: &'a mut DeterminantImage,
) -> &'a DeterminantImage {
    debug_assert_eq!(projection.len(), key_permutation.len());
    out.clear();
    for &source_pos in key_permutation {
        out.extend(field_bytes(
            fact_bytes,
            layout,
            usize::from(projection[usize::from(source_pos)].0),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{TypeDesc, ValueRef, encode_fact, encode_interval_u64, encode_u64};
    use bumbledb_theory::schema::IntervalElement;

    #[test]
    fn fact_key_round_trips_components() {
        let k = key(|b| fact_key(b, RelationId(7), 0x0102_0304_0506_0708));
        assert_eq!(k.len(), FACT_KEY_LEN);
        assert_eq!(k[0], NS_FACT);
        assert_eq!(&k[1..5], &7u32.to_be_bytes());
        assert_eq!(&k[5..], &0x0102_0304_0506_0708u64.to_be_bytes());
    }

    #[test]
    fn fact_prefix_is_a_prefix_of_every_fact_key() {
        let prefix = key(|b| fact_prefix(b, RelationId(7)));
        let k = key(|b| fact_key(b, RelationId(7), 42));
        assert!(k.starts_with(&prefix));
        let other = key(|b| fact_key(b, RelationId(8), 42));
        assert!(!other.starts_with(&prefix));
    }

    #[test]
    fn membership_key_embeds_full_hash() {
        let hash = [0xABu8; 32];
        let k = key(|b| membership_key(b, RelationId(1), &hash));
        assert_eq!(k.len(), MEMBERSHIP_KEY_LEN);
        assert_eq!(k[0], NS_MEMBERSHIP);
        assert_eq!(&k[5..], &hash);
    }

    #[test]
    fn determinant_key_golden_bytes() {
        // U | relation(u32) | statement(u16) | determinant — exact byte sequence.
        let determinant = [1u8, 2, 3];
        let u = key(|b| determinant_key(b, RelationId(2), StatementId(5), &determinant));
        assert_eq!(u, vec![NS_DETERMINANT, 0, 0, 0, 2, 0, 5, 1, 2, 3]);
    }

    #[test]
    fn determinant_key_keeps_a_16_byte_interval_determinant_contiguous() {
        // Determinant = scalar u64 ‖ whole 16-byte interval, contiguous. The
        // 7-byte header is tag + relation(u32) + statement(u16).
        let mut determinant = Vec::new();
        determinant.extend_from_slice(&encode_u64(0xAAAA_BBBB_CCCC_DDDD));
        determinant.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
        ));
        assert_eq!(determinant.len(), 24);

        let k = key(|b| determinant_key(b, RelationId(3), StatementId(9), &determinant));
        assert_eq!(k.len(), 7 + 24);
        // The interval's 16 bytes sit unsplit at the determinant's tail.
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
        // R | statement(u16) | key_bytes | source_rel(u32) | source_row(u64)
        // — statement-scoped; the target relation id appears nowhere.
        let key_bytes = [7u8, 8];
        let r = key(|b| reverse_key(b, StatementId(5), &key_bytes, RelationId(9), 11));
        assert_eq!(
            r,
            vec![NS_REVERSE, 0, 5, 7, 8, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 11]
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
        let determinant = key(|b| determinant_key(b, RelationId(1), StatementId(1), &[9]));
        assert!(parse_reverse_key(&determinant).is_none());
        let reverse = key(|b| reverse_key(b, StatementId(1), &[9], RelationId(1), 1));
        assert!(parse_reverse_key(&reverse[..R_OVERHEAD - 1]).is_none());
    }

    /// Layout for the slicing tests: `f0` u64, `f1` interval, `f2` u64.
    fn interval_layout() -> FactLayout {
        FactLayout::new(&[
            TypeDesc::U64,
            TypeDesc::Interval {
                element: IntervalElement::U64,
                width: None,
            },
            TypeDesc::U64,
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
        // Projection (f2, f1): scalar first, interval last, statement order.
        determinant_image(&layout, &[FieldId(2), FieldId(1)], &fact, &mut determinant);

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222));
        expected.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval"),
        ));
        assert_eq!(determinant.as_bytes(), expected);
    }

    #[test]
    fn permuted_determinant_image_lay_fields_in_target_key_order() {
        let layout = interval_layout();
        let fact = interval_fact();
        // Source projection order (f2, f1, f0); the target key's determinant
        // order is (f0, f2, interval f1): the stored permutation is the
        // INVERSE — determinant position -> projection index — and the
        // 3-cycle pins the direction (an involution would pass either way).
        let projection = [FieldId(2), FieldId(1), FieldId(0)];
        let key_permutation = [2u16, 0, 1];
        let mut key_bytes = DeterminantImage::scratch();
        permuted_determinant_image(
            &layout,
            &projection,
            &key_permutation,
            &fact,
            &mut key_bytes,
        );

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x1111_1111_1111_1111)); // f0
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222)); // f2
        expected.extend_from_slice(&encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval"),
        )); // f1, whole
        assert_eq!(key_bytes.as_bytes(), expected);

        // The permutation-ordered R key round-trips through the parser.
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
        let q = key(|b| fresh_key(b, RelationId(3), FieldId(4)));
        assert_eq!(q, vec![NS_FRESH, 0, 0, 0, 3, 0, 4]);
        let s = key(|b| stat_key(b, RelationId(3), StatKind::RowCount));
        assert_eq!(s, vec![NS_STAT, 0, 0, 0, 3, 0]);
        let hw = key(|b| stat_key(b, RelationId(3), StatKind::RowIdHighWater));
        assert_eq!(hw, vec![NS_STAT, 0, 0, 0, 3, 1]);
    }

    #[test]
    fn keys_sort_by_namespace_then_components() {
        // Byte order must equal (namespace, components) order: everything is
        // big-endian and namespace tags F < M < Q < R < S < U in ASCII.
        let ordered = vec![
            key(|b| fact_key(b, RelationId(1), 5)),
            key(|b| fact_key(b, RelationId(1), 6)),
            key(|b| fact_key(b, RelationId(2), 0)),
            key(|b| membership_key(b, RelationId(1), &[0u8; 32])),
            key(|b| membership_key(b, RelationId(1), &[1u8; 32])),
            key(|b| fresh_key(b, RelationId(1), FieldId(0))),
            key(|b| fresh_key(b, RelationId(1), FieldId(1))),
            key(|b| reverse_key(b, StatementId(0), &[9], RelationId(0), 0)),
            key(|b| reverse_key(b, StatementId(1), &[0], RelationId(0), 0)),
            key(|b| stat_key(b, RelationId(1), StatKind::RowCount)),
            key(|b| stat_key(b, RelationId(1), StatKind::RowIdHighWater)),
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
        // A once-spilled scratch cleared back to a small determinant must
        // equal (and order with) the inline form carrying the same bytes,
        // and Borrow<[u8]> must agree — the BTreeMap-lookup contract.
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
        // Cross-boundary spill mid-extend keeps the whole byte string.
        let mut crossing = DeterminantImage::scratch_with_capacity(8);
        crossing.extend(&[9; DETERMINANT_INLINE - 1]);
        crossing.extend(&[7, 7]);
        let mut expected = vec![9u8; DETERMINANT_INLINE - 1];
        expected.extend_from_slice(&[7, 7]);
        assert_eq!(crossing.as_bytes(), &expected[..]);
        // The clone of anything that fits inline compares equal and
        // round-trips its bytes (re-inlined; no heap retained).
        assert_eq!(spilled.clone(), spilled);
        assert_eq!(crossing.clone().as_bytes(), crossing.as_bytes());
    }

    /// The exact spill boundary: 24 bytes (one u64 scalar beside one
    /// whole 16-byte interval tail — the constant's stated shape) stays
    /// inline; the 25th byte spills, whether it arrives alone or inside
    /// a 16-byte interval-tail extend that crosses the boundary
    /// mid-piece. Identity (Eq/Ord/Borrow) is bytes on both sides of
    /// the boundary.
    #[test]
    fn determinant_image_spill_boundary_is_exactly_inline_capacity() {
        // 8 + 16 = exactly DETERMINANT_INLINE: the widest inline shape.
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
        // A 16-byte interval tail landing on a 9-byte prefix crosses the
        // boundary INSIDE one extend: bytes must survive the spill copy.
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
        // A single byte past capacity also spills (the off-by-one twin).
        let mut plus_one = DeterminantImage::scratch();
        plus_one.extend(&[0x11; DETERMINANT_INLINE]);
        assert!(matches!(plus_one.0, Image::Inline { .. }));
        plus_one.extend(&[0x22]);
        assert!(matches!(plus_one.0, Image::Spilled(_)));
        let mut expected = vec![0x11u8; DETERMINANT_INLINE];
        expected.push(0x22);
        assert_eq!(plus_one.as_bytes(), &expected[..]);
        // Clone canonicalizes: at-capacity re-inlines, past-capacity stays
        // spilled, and both compare equal to their originals.
        assert!(matches!(at_cap.clone().0, Image::Inline { .. }));
        assert!(matches!(plus_one.clone().0, Image::Spilled(_)));
        assert_eq!(at_cap.clone(), at_cap);
        assert_eq!(plus_one.clone(), plus_one);
    }

    /// `Ord` is the byte order whatever the representation: a sorted
    /// mixed-representation set must land in exactly the order of its
    /// byte strings — including the prefix rule ACROSS the spill
    /// boundary (a 24-byte inline value against its own 25-byte spilled
    /// extension), where a representation-tag or length-first compare
    /// would diverge from LMDB's byte order.
    #[test]
    fn determinant_image_order_is_byte_order_across_representations() {
        let of = |bytes: &[u8]| {
            let mut image = DeterminantImage::scratch();
            image.extend(bytes);
            image
        };
        // Same small bytes, forced into the SPILLED representation via a
        // once-wide scratch cleared back down.
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
        // The 24-inline value orders strictly below its 25-byte spilled
        // extension (the prefix rule across the boundary).
        assert!(of(&prefix24) < of(&extended25));
        // Eq agrees with Ord's Equal across representations at the
        // boundary width itself, and Borrow<[u8]> sees the same bytes.
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
        // MAX_KEY − (tag + statement + source_rel + source_row) = 511 − 15.
        // schema::validate imports this same constant for its
        // declaration-time rejection — the bound is never duplicated.
        assert_eq!(MAX_DETERMINANT_WIDTH, 511 - 15);
        // A determinant exactly at the limit builds, and the widest key shape —
        // the R embedding — lands exactly on MAX_KEY.
        let determinant = vec![0xEE; MAX_DETERMINANT_WIDTH];
        let r = key(|b| reverse_key(b, StatementId(0), &determinant, RelationId(0), 0));
        assert_eq!(r.len(), MAX_KEY);
        let u = key(|b| determinant_key(b, RelationId(0), StatementId(0), &determinant));
        assert!(u.len() <= MAX_KEY);
    }
}
