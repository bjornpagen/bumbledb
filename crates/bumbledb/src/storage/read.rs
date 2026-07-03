//! Storage read primitives (docs/architecture/40-storage.md): membership probe, unique-guard probe,
//! fact fetch, the sequential relation scan that feeds images and export,
//! and the planner's row count. All allocation-free with borrowed returns.
//!
//! Namespace readers per `docs/architecture/40-storage.md`: `M` serves
//! idempotence and point lookups, `U` constraint checks and guard-probe
//! lookups, `F` image builds / point-lookup fetch / export scan, `S` the
//! planner.

use crate::encoding::fact_hash;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{ConstraintId, RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

/// `M` probe: the row id of a fact, if it is live.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn fact_row(txn: &ReadTxn<'_>, rel: RelationId, fact_bytes: &[u8]) -> Result<Option<u64>> {
    fact_row_by_hash(txn, rel, &fact_hash(fact_bytes))
}

/// `M` probe by a caller-computed hash — the delta already hashed the fact
/// for its own map key; blake3 is the record path's most expensive CPU
/// step and must not run twice.
///
/// # Errors
///
/// As [`fact_row`].
pub fn fact_row_by_hash(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    hash: &[u8; 32],
) -> Result<Option<u64>> {
    // Right-sized stack buffer: this probe runs once per user write
    // operation — zeroing 511 bytes for a 37-byte key was measurable
    // waste (the codec header promises no oversized zeroing).
    let mut key = [0u8; keys::MEMBERSHIP_KEY_LEN];
    let len = keys::membership_key(&mut key, rel, hash);
    debug_assert_eq!(len, key.len());
    row_id_value(txn.env().data().get(txn.raw(), &key)?)
}

/// `U` probe: the row id holding a unique key, if any. `key_bytes` is the
/// concatenated canonical encodings of the constrained fields in constraint
/// field order.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn unique_row(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    constraint: ConstraintId,
    key_bytes: &[u8],
) -> Result<Option<u64>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::unique_key(&mut key, rel, constraint, key_bytes);
    row_id_value(txn.env().data().get(txn.raw(), &key[..len])?)
}

/// `F` get: the canonical bytes of the fact at `row_id`, borrowed from the
/// LMDB page.
///
/// # Errors
///
/// `Corruption(MissingFact)` when the row is absent — a row id obtained
/// from `M`/`U` in the same snapshot must resolve; `Corruption
/// (WrongFactWidth)` when the stored value does not match the schema's
/// fact width. Never a skip.
pub fn fetch<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    row_id: u64,
) -> Result<&'txn [u8]> {
    let mut key = [0u8; keys::FACT_KEY_LEN];
    let len = keys::fact_key(&mut key, rel, row_id);
    debug_assert_eq!(len, key.len());
    let bytes = txn
        .env()
        .data()
        .get(txn.raw(), &key[..len])?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation: rel,
            row_id,
        }))?;
    check_width(schema, rel, row_id, bytes)?;
    Ok(bytes)
}

/// One `F`-prefix cursor over a relation's live facts in `row_id` order.
/// Holes from deletes are absent keys, not tombstones — they simply do not
/// appear. A wrong-width fact yields `Err(Corruption)`; the caller is
/// expected to stop at the first error (hard error, never a skip).
///
/// # Errors
///
/// `Lmdb` on cursor-open failure.
///
/// # Panics
///
/// Only on a programmer-invariant violation: an `F` key shorter than its
/// fixed 13-byte shape (the codec writes every one).
pub fn scan<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &'txn Schema,
    rel: RelationId,
) -> Result<impl Iterator<Item = Result<(u64, &'txn [u8])>>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::fact_prefix(&mut key, rel);
    let iter = txn.env().data().prefix_iter(txn.raw(), &key[..len])?;
    // Fused on error: after the first corruption the iterator yields
    // nothing more — "never a skip" is structural, not a caller
    // obligation (a caller ignoring an Err cannot resume past it).
    let mut dead = false;
    Ok(iter.map_while(move |entry| {
        if dead {
            return None;
        }
        let item = (|| {
            let (raw_key, bytes) = entry?;
            // F | relation(4) | row_id(8): the row id is the last 8 bytes.
            let row_id = u64::from_be_bytes(
                raw_key[raw_key.len() - 8..]
                    .try_into()
                    .expect("F keys end in an 8-byte row id"),
            );
            check_width(schema, rel, row_id, bytes)?;
            Ok((row_id, bytes))
        })();
        dead = item.is_err();
        Some(item)
    }))
}

/// `S` get: the relation's exact row count — the planner's statistic.
/// Missing means no state-changing commit ever touched the relation: 0.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed counter value.
pub fn row_count(txn: &ReadTxn<'_>, rel: RelationId) -> Result<u64> {
    let mut key = [0u8; keys::STAT_KEY_LEN];
    let len = keys::stat_key(&mut key, rel, StatKind::RowCount);
    debug_assert_eq!(len, key.len());
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            Error::Corruption(CorruptionError::MalformedValue("S row count"))
        })?)),
        None => Ok(0),
    }
}

fn row_id_value(value: Option<&[u8]>) -> Result<Option<u64>> {
    match value {
        None => Ok(None),
        Some(bytes) => Ok(Some(u64::from_le_bytes(bytes.try_into().map_err(
            |_| Error::Corruption(CorruptionError::MalformedValue("M/U row id")),
        )?))),
    }
}

fn check_width(schema: &Schema, rel: RelationId, row_id: u64, bytes: &[u8]) -> Result<()> {
    let expected = schema.relation(rel).layout().fact_width();
    if bytes.len() == expected {
        Ok(())
    } else {
        Err(Error::Corruption(CorruptionError::WrongFactWidth {
            relation: rel,
            row_id,
            expected,
            actual: bytes.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, encode_u64, ValueRef};
    use crate::schema::{
        ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor,
        SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::storage::keys;
    use crate::testutil::TempDir;

    /// R(id serial, amount i64) with a declared unique on amount too.
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "amount".into(),
                    fields: Box::new([FieldId(1)]),
                }],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);

    fn fact(schema: &Schema, id: u64, amount: i64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::I64(amount)],
            schema.relation(R).layout(),
            &mut b,
        );
        b
    }

    /// Committed fixture: facts (id, amount) = (0,10), (1,20), (2,30), then
    /// (1,20) deleted — leaving a row-id hole at 1.
    fn fixture(dir: &TempDir, schema: &Schema) -> Environment {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (id, amount) in [(0, 10), (1, 20), (2, 30)] {
            delta
                .insert(&view, R, &fact(schema, id, amount))
                .expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");

        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        delta
            .delete(&view, R, &fact(schema, 1, 20))
            .expect("delete");
        drop(view);
        commit(delta, &env).expect("commit");
        env
    }

    #[test]
    fn membership_probe_hit_and_miss() {
        let dir = TempDir::new("read-membership");
        let schema = schema();
        let env = fixture(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        // Row ids are assigned in fact-hash order at commit (the delta's
        // deterministic iteration), so tests derive them rather than assume
        // insertion order.
        let row = fact_row(&txn, R, &fact(&schema, 0, 10)).expect("probe");
        assert!(row.is_some());
        // The deleted fact and a never-inserted fact both miss.
        assert_eq!(
            fact_row(&txn, R, &fact(&schema, 1, 20)).expect("probe"),
            None
        );
        assert_eq!(
            fact_row(&txn, R, &fact(&schema, 9, 90)).expect("probe"),
            None
        );
    }

    #[test]
    fn unique_probe_hit_and_miss() {
        let dir = TempDir::new("read-unique");
        let schema = schema();
        let env = fixture(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let row = fact_row(&txn, R, &fact(&schema, 2, 30))
            .expect("probe")
            .expect("present");
        // The serial auto-unique (constraint 0) on id and the declared
        // unique (constraint 1) on amount both resolve to the same row.
        assert_eq!(
            unique_row(&txn, R, ConstraintId(0), &encode_u64(2)).expect("probe"),
            Some(row)
        );
        assert_eq!(
            unique_row(&txn, R, ConstraintId(1), &crate::encoding::encode_i64(30)).expect("probe"),
            Some(row)
        );
        // The deleted fact's keys are gone.
        assert_eq!(
            unique_row(&txn, R, ConstraintId(0), &encode_u64(1)).expect("probe"),
            None
        );
    }

    #[test]
    fn fetch_round_trips_inserted_bytes() {
        let dir = TempDir::new("read-fetch");
        let schema = schema();
        let env = fixture(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let row = fact_row(&txn, R, &fact(&schema, 2, 30))
            .expect("probe")
            .expect("present");
        assert_eq!(
            fetch(&txn, &schema, R, row).expect("fetch"),
            fact(&schema, 2, 30)
        );
        // The deleted fact left a row-id hole: fetching it is corruption
        // (a row id reaching fetch must have come from M/U in-snapshot).
        let live: Vec<u64> = scan(&txn, &schema, R)
            .expect("scan")
            .map(|r| r.expect("ok").0)
            .collect();
        let hole = (0..3).find(|id| !live.contains(id)).expect("one hole");
        let err = fetch(&txn, &schema, R, hole).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::MissingFact {
                    relation: R,
                    row_id
                }) if row_id == hole
            ),
            "{err:?}"
        );
    }

    #[test]
    fn scan_yields_live_facts_in_row_id_order_skipping_holes() {
        let dir = TempDir::new("read-scan");
        let schema = schema();
        let env = fixture(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let rows: Vec<(u64, Vec<u8>)> = scan(&txn, &schema, R)
            .expect("scan")
            .map(|r| r.map(|(id, b)| (id, b.to_vec())))
            .collect::<Result<_>>()
            .expect("no corruption");
        // Exactly the two live facts, in strictly increasing row-id order,
        // with the deleted fact's row id absent.
        assert_eq!(rows.len(), 2);
        assert!(rows[0].0 < rows[1].0);
        let live_bytes: Vec<&[u8]> = rows.iter().map(|(_, b)| b.as_slice()).collect();
        assert!(live_bytes.contains(&fact(&schema, 0, 10).as_slice()));
        assert!(live_bytes.contains(&fact(&schema, 2, 30).as_slice()));
        for (row_id, bytes) in &rows {
            assert_eq!(
                fact_row(&txn, R, bytes).expect("probe"),
                Some(*row_id),
                "scan and membership agree"
            );
        }
    }

    #[test]
    fn corrupted_fact_width_is_an_error_never_a_skip() {
        let dir = TempDir::new("read-corrupt");
        let schema = schema();
        let env = fixture(&dir, &schema);
        // Truncate the *last* live F value behind the schema's back.
        let victim = {
            let txn = env.read_txn().expect("txn");
            scan(&txn, &schema, R)
                .expect("scan")
                .map(|r| r.expect("ok").0)
                .max()
                .expect("nonempty")
        };
        {
            let mut wtxn = env.write_txn().expect("txn");
            let mut key: KeyBuf = [0; MAX_KEY];
            let len = keys::fact_key(&mut key, R, victim);
            env.data()
                .put(wtxn.raw_mut(), &key[..len], &[0xAB, 0xCD])
                .expect("put");
            wtxn.commit().expect("commit");
        }
        let txn = env.read_txn().expect("txn");
        let results: Vec<Result<(u64, &[u8])>> = scan(&txn, &schema, R).expect("scan").collect();
        assert!(results[0].is_ok()); // the first live row is intact
        let err = results[1].as_ref().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::WrongFactWidth {
                    relation: R,
                    row_id,
                    expected: 16,
                    actual: 2
                }) if *row_id == victim
            ),
            "{err:?}"
        );
        // fetch reports the same corruption.
        assert!(fetch(&txn, &schema, R, victim).is_err());
    }

    #[test]
    fn row_count_equals_scan_count_after_mixed_commits() {
        let dir = TempDir::new("read-row-count");
        let schema = schema();
        let env = fixture(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let scanned = scan(&txn, &schema, R).expect("scan").count() as u64;
        assert_eq!(row_count(&txn, R).expect("count"), scanned);
        assert_eq!(scanned, 2);
    }
}
