//! The `_data` key codec (PRD 04): first-byte namespaces, big-endian
//! components (`docs/architecture/40-storage.md`).
//!
//! ```text
//! F | relation_id | row_id                                       facts
//! M | relation_id | fact_hash                                    membership
//! U | relation_id | constraint | guard_key                       unique guards
//! R | target_rel  | constraint | guard_key | source_rel | source_row
//! Q | relation_id | field_id                                     serial sequences
//! S | relation_id | stat                                         counters
//! ```
//!
//! Writers fill a caller-provided `[u8; MAX_KEY]` scratch and return the
//! written length — no oversized zeroing (post-mortem §25), and key types
//! never derive `Ord` (LMDB byte order is the only order).

use crate::schema::{ConstraintId, FieldId, RelationId};

/// LMDB's default key-size ceiling; every encoded key fits.
pub const MAX_KEY: usize = 511;

/// Fixed scratch buffer for key writers.
pub type KeyBuf = [u8; MAX_KEY];

/// Byte overhead of a Restrict key beyond the guard bytes:
/// `tag(1) + target_rel(4) + constraint(2) + source_rel(4) + source_row(8)`.
const RESTRICT_OVERHEAD: usize = 1 + 4 + 2 + 4 + 8;

/// Maximum guard-key width a schema may declare: the widest key embedding a
/// guard is the Restrict key, so its overhead bounds every guard
/// (schema-construction hook; rejection is `SchemaError::GuardKeyTooWide`).
pub const MAX_GUARD_WIDTH: usize = MAX_KEY - RESTRICT_OVERHEAD;

/// Namespace tags, one byte, first in every key.
pub const NS_FACT: u8 = b'F';
pub const NS_MEMBERSHIP: u8 = b'M';
pub const NS_UNIQUE: u8 = b'U';
pub const NS_RESTRICT: u8 = b'R';
pub const NS_SERIAL: u8 = b'Q';
pub const NS_STAT: u8 = b'S';

/// Which per-relation counter an `S` key addresses.
///
/// `RowCount` is the planner's statistic (`40-storage.md`); `RowIdHighWater`
/// is the delta core's row-id allocator (PRD 06 extends the `S` codec, noted
/// in the doc).
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

    fn constraint(&mut self, id: ConstraintId) -> &mut Self {
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
pub const SERIAL_KEY_LEN: usize = 1 + 4 + 2;
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

/// `U | relation | constraint | guard` — a unique-guard key. `guard` is the
/// concatenated canonical encodings of the constrained fields in constraint
/// field order (width-bounded at schema construction).
pub fn unique_key(
    buf: &mut KeyBuf,
    relation: RelationId,
    constraint: ConstraintId,
    guard: &[u8],
) -> usize {
    debug_assert!(guard.len() <= MAX_GUARD_WIDTH);
    KeyWriter::new(buf, NS_UNIQUE)
        .relation(relation)
        .constraint(constraint)
        .put(guard)
        .finish()
}

/// `R | target_rel | constraint | guard | source_rel | source_row` — one
/// reverse-reference entry (the Restrict check's reader).
pub fn restrict_key(
    buf: &mut KeyBuf,
    target_relation: RelationId,
    target_constraint: ConstraintId,
    guard: &[u8],
    source_relation: RelationId,
    source_row: u64,
) -> usize {
    debug_assert!(guard.len() <= MAX_GUARD_WIDTH);
    KeyWriter::new(buf, NS_RESTRICT)
        .relation(target_relation)
        .constraint(target_constraint)
        .put(guard)
        .relation(source_relation)
        .put(&source_row.to_be_bytes())
        .finish()
}

/// `R | target_rel | constraint | guard` — the prefix shared by every
/// referrer of one unique key (Restrict prefix-scan reader).
pub fn restrict_prefix(
    buf: &mut KeyBuf,
    target_relation: RelationId,
    target_constraint: ConstraintId,
    guard: &[u8],
) -> usize {
    debug_assert!(guard.len() <= MAX_GUARD_WIDTH);
    KeyWriter::new(buf, NS_RESTRICT)
        .relation(target_relation)
        .constraint(target_constraint)
        .put(guard)
        .finish()
}

/// `Q | relation | field` — a serial sequence's key.
pub fn serial_key(buf: &mut [u8], relation: RelationId, field: FieldId) -> usize {
    KeyWriter::new(buf, NS_SERIAL)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
        let mut buf = [0u8; MAX_KEY];
        let len = write(&mut buf);
        buf[..len].to_vec()
    }

    #[test]
    fn fact_key_round_trips_components() {
        let k = key(|b| fact_key(b, RelationId(7), 0x0102_0304_0506_0708));
        assert_eq!(k.len(), 13);
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
        assert_eq!(k.len(), 37);
        assert_eq!(k[0], NS_MEMBERSHIP);
        assert_eq!(&k[5..], &hash);
    }

    #[test]
    fn unique_and_restrict_keys_embed_guard_bytes() {
        let guard = [1u8, 2, 3];
        let u = key(|b| unique_key(b, RelationId(2), ConstraintId(5), &guard));
        assert_eq!(u.len(), 1 + 4 + 2 + 3);
        assert_eq!(u[0], NS_UNIQUE);
        assert_eq!(&u[7..], &guard);

        let r = key(|b| restrict_key(b, RelationId(2), ConstraintId(5), &guard, RelationId(9), 11));
        assert_eq!(r.len(), RESTRICT_OVERHEAD + guard.len());
        assert_eq!(r[0], NS_RESTRICT);
        let prefix = key(|b| restrict_prefix(b, RelationId(2), ConstraintId(5), &guard));
        assert!(r.starts_with(&prefix));
    }

    #[test]
    fn serial_and_stat_keys() {
        let q = key(|b| serial_key(b, RelationId(3), FieldId(4)));
        assert_eq!(q, vec![NS_SERIAL, 0, 0, 0, 3, 0, 4]);
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
            key(|b| serial_key(b, RelationId(1), FieldId(0))),
            key(|b| serial_key(b, RelationId(1), FieldId(1))),
            key(|b| restrict_key(b, RelationId(1), ConstraintId(0), &[9], RelationId(0), 0)),
            key(|b| stat_key(b, RelationId(1), StatKind::RowCount)),
            key(|b| stat_key(b, RelationId(1), StatKind::RowIdHighWater)),
            key(|b| unique_key(b, RelationId(1), ConstraintId(0), &[0])),
            key(|b| unique_key(b, RelationId(1), ConstraintId(0), &[1])),
            key(|b| unique_key(b, RelationId(1), ConstraintId(1), &[0])),
        ];
        let mut sorted = ordered.clone();
        sorted.sort();
        assert_eq!(ordered, sorted);
    }

    #[test]
    fn guard_width_bound_matches_restrict_overhead() {
        assert_eq!(MAX_GUARD_WIDTH, 511 - 19);
        // A maximal guard still fits both key shapes inside MAX_KEY.
        let guard = vec![0xEE; MAX_GUARD_WIDTH];
        let r = key(|b| restrict_key(b, RelationId(0), ConstraintId(0), &guard, RelationId(0), 0));
        assert_eq!(r.len(), MAX_KEY);
    }
}
