//! The `_data` key codec (`docs/architecture/50-storage.md` § Key layout):
//! first-byte namespaces, big-endian components.
//!
//! ```text
//! F | relation_id | row_id                                  facts
//! M | relation_id | fact_hash                               membership
//! U | relation_id | statement | guard                      FD guards
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

use crate::encoding::{FactLayout, field_bytes};
use crate::schema::{FieldId, RelationId, StatementId};

/// LMDB's default key-size ceiling; every encoded key fits.
pub const MAX_KEY: usize = 511;

/// Fixed scratch buffer for key writers.
pub type KeyBuf = [u8; MAX_KEY];

#[cfg(test)]
pub(crate) fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
    let mut buf = [0u8; MAX_KEY];
    let len = write(&mut buf);
    buf[..len].to_vec()
}

/// Byte overhead of a reverse-edge (`R`) key beyond its embedded key bytes:
/// `tag(1) + statement(2) + source_rel(4) + source_row(8)`.
const R_OVERHEAD: usize = 1 + 2 + 4 + 8;

/// Maximum guard width a schema may declare.
///
/// Derivation: a guard value must embed whole in every key shape that
/// carries one. The `U` key spends `tag(1) + relation(4) + statement(2)`
/// = 7 bytes beside its guard; the `R` key embeds a whole target-key value
/// as its key-bytes segment and spends [`R_OVERHEAD`] = 15 beside it. The
/// `R` embedding is therefore the binding bound:
/// `MAX_GUARD_WIDTH = MAX_KEY − R_OVERHEAD = 511 − 15 = 496`.
///
/// Schema-construction hook; rejection is `SchemaError::GuardKeyTooWide`
/// (the validator imports this constant — the bound has one owner).
pub const MAX_GUARD_WIDTH: usize = MAX_KEY - R_OVERHEAD;

/// Namespace tags, one byte, first in every key.
pub const NS_FACT: u8 = b'F';
pub const NS_MEMBERSHIP: u8 = b'M';
pub const NS_GUARD: u8 = b'U';
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
/// `R` key width after the key bytes: `source_rel` + `source_row`.
pub const REVERSE_KEY_TAIL_LEN: usize = 4 + 8;

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

/// `U | relation | statement | guard` — an FD guard key. `guard` is the
/// concatenated canonical encodings of the statement's projected fields in
/// statement projection order ([`guard_bytes`]; width-bounded at schema
/// construction).
pub fn guard_key(
    buf: &mut KeyBuf,
    relation: RelationId,
    statement: StatementId,
    guard: &[u8],
) -> usize {
    debug_assert!(guard.len() <= MAX_GUARD_WIDTH);
    KeyWriter::new(buf, NS_GUARD)
        .relation(relation)
        .statement(statement)
        .put(guard)
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
    debug_assert!(key_bytes.len() <= MAX_GUARD_WIDTH);
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
    debug_assert!(key_bytes.len() <= MAX_GUARD_WIDTH);
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
    if key.len() < R_OVERHEAD || key[0] != NS_REVERSE {
        return None;
    }
    let statement = StatementId(u16::from_be_bytes(
        key[1..3].try_into().expect("fixed-width slice"),
    ));
    let tail = key.len() - REVERSE_KEY_TAIL_LEN;
    let key_bytes = &key[3..tail];
    let source_relation = RelationId(u32::from_be_bytes(
        key[tail..tail + 4].try_into().expect("fixed-width slice"),
    ));
    let source_row = u64::from_be_bytes(key[tail + 4..].try_into().expect("fixed-width slice"));
    Some((statement, key_bytes, source_relation, source_row))
}

/// Concatenates the canonical encodings of `projection`'s fields, sliced
/// out of `fact_bytes`, in statement projection order, into `out` — the
/// guard segment of a `U` key, re-derived per fact, never a scan.
///
/// An interval field copies its whole 16-byte `start ‖ end` encoding in
/// one piece (the slice width comes from the layout — never split here):
/// the contiguity is what keeps the guard B-tree ordered by interval start
/// within one scalar-prefix group.
pub fn guard_bytes(
    layout: &FactLayout,
    projection: &[FieldId],
    fact_bytes: &[u8],
    out: &mut Vec<u8>,
) {
    out.clear();
    for &field in projection {
        out.extend_from_slice(field_bytes(fact_bytes, layout, usize::from(field.0)));
    }
}

/// Like [`guard_bytes`], but lays the sliced fields down in the *target
/// key's* guard order: `key_permutation[i]` is the guard position of
/// projection element `i` (statement projection order → target key order,
/// `Enforcement::Probe::key_permutation`) — the key-bytes segment of an
/// `R` key. Interval fields copy their whole 16 bytes, exactly as in
/// [`guard_bytes`].
pub fn permuted_guard_bytes(
    layout: &FactLayout,
    projection: &[FieldId],
    key_permutation: &[u16],
    fact_bytes: &[u8],
    out: &mut Vec<u8>,
) {
    debug_assert_eq!(projection.len(), key_permutation.len());
    out.clear();
    for guard_pos in 0..key_permutation.len() {
        let source_pos = key_permutation
            .iter()
            .position(|&p| usize::from(p) == guard_pos)
            .expect("key permutation contains every guard position");
        out.extend_from_slice(field_bytes(
            fact_bytes,
            layout,
            usize::from(projection[source_pos].0),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{TypeDesc, ValueRef, encode_fact, encode_interval_u64, encode_u64};
    use crate::schema::IntervalElement;

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
    fn guard_key_golden_bytes() {
        // U | relation(u32) | statement(u16) | guard — exact byte sequence.
        let guard = [1u8, 2, 3];
        let u = key(|b| guard_key(b, RelationId(2), StatementId(5), &guard));
        assert_eq!(u, vec![NS_GUARD, 0, 0, 0, 2, 0, 5, 1, 2, 3]);
    }

    #[test]
    fn guard_key_keeps_a_16_byte_interval_guard_contiguous() {
        // Guard = scalar u64 ‖ whole 16-byte interval, contiguous. The
        // 7-byte header is tag + relation(u32) + statement(u16).
        let mut guard = Vec::new();
        guard.extend_from_slice(&encode_u64(0xAAAA_BBBB_CCCC_DDDD));
        guard.extend_from_slice(&encode_interval_u64(10, 20));
        assert_eq!(guard.len(), 24);

        let k = key(|b| guard_key(b, RelationId(3), StatementId(9), &guard));
        assert_eq!(k.len(), 7 + 24);
        // The interval's 16 bytes sit unsplit at the guard's tail.
        assert_eq!(&k[7 + 8..], encode_interval_u64(10, 20));
        assert_eq!(&k[7..], &guard[..]);
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
        key_bytes.extend_from_slice(&encode_interval_u64(100, 200));

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
        let guard = key(|b| guard_key(b, RelationId(1), StatementId(1), &[9]));
        assert!(parse_reverse_key(&guard).is_none());
        let reverse = key(|b| reverse_key(b, StatementId(1), &[9], RelationId(1), 1));
        assert!(parse_reverse_key(&reverse[..R_OVERHEAD - 1]).is_none());
    }

    /// Layout for the slicing tests: `f0` u64, `f1` interval, `f2` u64.
    fn interval_layout() -> FactLayout {
        FactLayout::new(&[
            TypeDesc::U64,
            TypeDesc::Interval {
                element: IntervalElement::U64,
            },
            TypeDesc::U64,
        ])
    }

    fn interval_fact() -> Vec<u8> {
        let mut fact = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(0x1111_1111_1111_1111),
                ValueRef::IntervalU64(3, 9),
                ValueRef::U64(0x2222_2222_2222_2222),
            ],
            &interval_layout(),
            &mut fact,
        );
        fact
    }

    #[test]
    fn guard_bytes_slices_projection_order_and_copies_intervals_whole() {
        let layout = interval_layout();
        let fact = interval_fact();
        let mut guard = Vec::new();
        // Projection (f2, f1): scalar first, interval last, statement order.
        guard_bytes(&layout, &[FieldId(2), FieldId(1)], &fact, &mut guard);

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222));
        expected.extend_from_slice(&encode_interval_u64(3, 9));
        assert_eq!(guard, expected);
    }

    #[test]
    fn permuted_guard_bytes_lay_fields_in_target_key_order() {
        let layout = interval_layout();
        let fact = interval_fact();
        // Source projection order (f2, f0, f1); the target key's guard
        // order is (f0, f2, interval): permutation maps projection
        // position -> guard position.
        let projection = [FieldId(2), FieldId(0), FieldId(1)];
        let key_permutation = [1u16, 0, 2];
        let mut key_bytes = Vec::new();
        permuted_guard_bytes(
            &layout,
            &projection,
            &key_permutation,
            &fact,
            &mut key_bytes,
        );

        let mut expected = Vec::new();
        expected.extend_from_slice(&encode_u64(0x1111_1111_1111_1111)); // f0
        expected.extend_from_slice(&encode_u64(0x2222_2222_2222_2222)); // f2
        expected.extend_from_slice(&encode_interval_u64(3, 9)); // f1, whole
        assert_eq!(key_bytes, expected);

        // The permutation-ordered R key round-trips through the parser.
        let r = key(|b| reverse_key(b, StatementId(4), &key_bytes, RelationId(1), 5));
        let (stmt, parsed, src_rel, src_row) =
            parse_reverse_key(&r).expect("well-formed reverse key");
        assert_eq!(
            (stmt, parsed, src_rel, src_row),
            (StatementId(4), &key_bytes[..], RelationId(1), 5)
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
            key(|b| guard_key(b, RelationId(1), StatementId(0), &[0])),
            key(|b| guard_key(b, RelationId(1), StatementId(0), &[1])),
            key(|b| guard_key(b, RelationId(1), StatementId(1), &[0])),
        ];
        let mut sorted = ordered.clone();
        sorted.sort();
        assert_eq!(ordered, sorted);
    }

    #[test]
    fn guard_width_bound_matches_reverse_overhead() {
        // MAX_KEY − (tag + statement + source_rel + source_row) = 511 − 15.
        // schema::validate imports this same constant for its
        // declaration-time rejection — the bound is never duplicated.
        assert_eq!(MAX_GUARD_WIDTH, 511 - 15);
        // A guard exactly at the limit builds, and the widest key shape —
        // the R embedding — lands exactly on MAX_KEY.
        let guard = vec![0xEE; MAX_GUARD_WIDTH];
        let r = key(|b| reverse_key(b, StatementId(0), &guard, RelationId(0), 0));
        assert_eq!(r.len(), MAX_KEY);
        let u = key(|b| guard_key(b, RelationId(0), StatementId(0), &guard));
        assert!(u.len() <= MAX_KEY);
    }
}
