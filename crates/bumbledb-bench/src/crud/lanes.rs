//! Every runner takes its [`Protocol`] explicitly — tests pass tiny protocols;
//! no protocol is baked into a runner.
use bumbledb::schema::ValueType;
use bumbledb::{
    Answers, Atom, AtomSource, Db, FieldId, FindTerm, ParamId, Query, Rule, Term, VarId,
};
use rusqlite::Connection;

use crate::families;
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::sqlite_run::{self, PreparedFamily};
use crate::translate;

use super::ops::{self, UpdateOp, UpsertOp};
use super::{Counter, CounterByKey, CrudDocId, CrudSizes, CrudWorld, Doc, ids, schema};

const MIXED_READS: u32 = 9;

const DOC_INSERT: &str = "INSERT INTO \"Doc\" VALUES (?1, ?2, ?3, ?4)";

const COUNTER_UPDATE: &str = "UPDATE \"Counter\" SET \"val\" = ?1 WHERE \"key\" = ?2";

const COUNTER_UPSERT: &str = "INSERT INTO \"Counter\" VALUES (?1, ?2) \
                              ON CONFLICT(\"key\") DO UPDATE SET \"val\" = excluded.\"val\"";

const COUNTER_SELECT: &str = "SELECT \"val\" FROM \"Counter\" WHERE \"key\" = ?1";

const DOC_DELETE: &str = "DELETE FROM \"Doc\" WHERE \"id\" = ?1";

/// fresh mint base both engines share after load) and advances one per
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshCursor(pub u64);

impl FreshCursor {
    #[must_use]
    pub fn at_base(sizes: CrudSizes) -> Self {
        Self(sizes.docs + sizes.delete_pool)
    }
}

#[must_use]
pub fn read_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::DOC),
            bindings: vec![
                (FieldId(1), Term::Param(ParamId(0))),
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// The protocol's total closure invocations — every stream's required length.
fn invocations(proto: Protocol) -> usize {
    usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small")
}

/// The in-closure refusal sentinel (the `posting_swap` precedent): returning
/// this from a write closure drops the delta whole, so a refused sample commits
/// nothing.
fn refuse(what: &str) -> bumbledb::Error {
    bumbledb::Error::from(std::io::Error::other(what.to_owned()))
}

/// A stream whose length disagrees with the protocol is refused at runner entry
/// — the count contract is part of the representation.
fn check_stream(family: &str, len: usize, proto: Protocol) -> Result<(), String> {
    let want = invocations(proto);
    if len == want {
        Ok(())
    } else {
        Err(format!(
            "{family}: the stream carries {len} ops but the protocol makes {want} invocations"
        ))
    }
}

fn sql_u64(value: u64) -> i64 {
    i64::try_from(value).expect("corpus ids and keys stay below 2^63")
}

fn mint_doc(
    tx: &mut bumbledb::WriteTx<'_, CrudWorld>,
    seed: u64,
    cursor: &mut FreshCursor,
) -> bumbledb::Result<()> {
    let id: CrudDocId = tx.reserve(1)?.start().expect("nonempty");
    if id.0 != cursor.0 {
        return Err(refuse(&format!(
            "the fresh mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.0
        )));
    }
    tx.insert([&Doc {
        id,
        key: cursor.0,
        val: i64::try_from(cursor.0).expect("mints stay below 2^63"),
        payload: ops::fresh_payload(seed, cursor.0),
    }])?;
    cursor.0 += 1;
    Ok(())
}

fn mint_doc_sqlite(conn: &Connection, seed: u64, cursor: &mut FreshCursor) -> Result<(), String> {
    let id = sql_u64(cursor.0);
    conn.prepare_cached(DOC_INSERT)
        .map_err(|e| format!("prepare: {e}"))?
        .execute((id, id, id, ops::fresh_payload(seed, cursor.0).to_vec()))
        .map_err(|e| format!("insert: {e}"))?;
    cursor.0 += 1;
    Ok(())
}

/// # Errors
pub fn insert_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    seed: u64,
    per_commit: u64,
    cursor: &mut FreshCursor,
) -> Result<Measurement, String> {
    harness::measure(proto, || {
        db.write(|tx| {
            for _ in 0..per_commit {
                mint_doc(tx, seed, cursor)?;
            }
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            per_commit
        })
        .map_err(|e| format!("crud_insert x{per_commit}: {e:?}"))
    })
}

/// # Errors
pub fn insert_sqlite(
    conn: &Connection,
    proto: Protocol,
    seed: u64,
    per_commit: u64,
    cursor: &mut FreshCursor,
) -> Result<Measurement, String> {
    harness::measure(proto, || {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        let mut step = || -> Result<(), String> {
            for _ in 0..per_commit {
                mint_doc_sqlite(conn, seed, cursor)?;
            }
            Ok(())
        };
        match step() {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| per_commit)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_insert x{per_commit} sqlite: {e}"))
            }
        }
    })
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn update_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    stream: &[UpdateOp],
) -> Result<Measurement, String> {
    check_stream("crud_update", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| {
            if tx
                .delete([&Counter {
                    key: op.key,
                    val: op.prev,
                }])?
                .changed()
                == 0
            {
                return Err(refuse(
                    "the update must be delete-bearing: the stream's prev value was absent",
                ));
            }
            tx.insert([&Counter {
                key: op.key,
                val: op.next,
            }])?;
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            1
        })
        .map_err(|e| format!("crud_update: {e:?}"))
    })
}

/// # Errors
/// `SQLite` errors, stringified; a stream/protocol length mismatch or
pub fn update_sqlite(
    conn: &Connection,
    proto: Protocol,
    stream: &[UpdateOp],
) -> Result<Measurement, String> {
    check_stream("crud_update", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        let step = || -> Result<(), String> {
            let changed = conn
                .prepare_cached(COUNTER_UPDATE)
                .map_err(|e| format!("prepare: {e}"))?
                .execute((op.next, sql_u64(op.key)))
                .map_err(|e| format!("update: {e}"))?;
            if changed == 1 {
                Ok(())
            } else {
                Err(format!(
                    "the update must change exactly one row, changed {changed}"
                ))
            }
        };
        match step() {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| 1)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_update sqlite: {e}"))
            }
        }
    })
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn upsert_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    stream: &[UpsertOp],
) -> Result<Measurement, String> {
    check_stream("crud_upsert", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| {
            let old = tx.get(CounterByKey { key: op.key })?;
            if old.as_ref().map(|o| o.val) != op.prev {
                return Err(refuse(
                    "the upsert drifted from its stream: the stored value is not the stream's prev",
                ));
            }
            match old {
                Some(old) => {
                    tx.delete([&old])?;
                    tx.insert([&Counter {
                        key: op.key,
                        val: op.next,
                    }])?;
                }
                None => {
                    tx.insert([&Counter {
                        key: op.key,
                        val: op.next,
                    }])?;
                }
            }
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            1
        })
        .map_err(|e| format!("crud_upsert: {e:?}"))
    })
}

/// # Errors
/// `SQLite` errors, stringified; a stream/protocol length mismatch,
pub fn upsert_sqlite(
    conn: &Connection,
    proto: Protocol,
    stream: &[UpsertOp],
) -> Result<Measurement, String> {
    check_stream("crud_upsert", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        let run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            conn.prepare_cached(COUNTER_UPSERT)?
                .execute((sql_u64(op.key), op.next))?;
            conn.execute_batch("COMMIT")
        };
        run()
            .map(|()| 1)
            .map_err(|e| format!("crud_upsert sqlite: {e}"))
    })
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn rmw_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    keys: &[u64],
) -> Result<Measurement, String> {
    check_stream("crud_rmw", keys.len(), proto)?;
    let mut iter = keys.iter();
    harness::measure(proto, || {
        let key = *iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| {
            let Some(old) = tx.get(CounterByKey { key })? else {
                return Err(refuse("the rmw round trip needs an existing counter row"));
            };
            let next = old.val + 1;
            tx.delete([&old])?;
            tx.insert([&Counter { key, val: next }])?;
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            1
        })
        .map_err(|e| format!("crud_rmw: {e:?}"))
    })
}

/// # Errors
/// `SQLite` errors, stringified; a stream/protocol length mismatch, a
pub fn rmw_sqlite(conn: &Connection, proto: Protocol, keys: &[u64]) -> Result<Measurement, String> {
    check_stream("crud_rmw", keys.len(), proto)?;
    let mut iter = keys.iter();
    harness::measure(proto, || {
        let key = sql_u64(
            *iter
                .next()
                .ok_or("the stream ended before the protocol did")?,
        );
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        let step = || -> Result<(), String> {
            let val: i64 = conn
                .prepare_cached(COUNTER_SELECT)
                .map_err(|e| format!("prepare select: {e}"))?
                .query_row([key], |row| row.get(0))
                .map_err(|e| format!("select: {e}"))?;
            let changed = conn
                .prepare_cached(COUNTER_UPDATE)
                .map_err(|e| format!("prepare update: {e}"))?
                .execute((val + 1, key))
                .map_err(|e| format!("update: {e}"))?;
            if changed == 1 {
                Ok(())
            } else {
                Err(format!(
                    "the rmw update must change exactly one row, changed {changed}"
                ))
            }
        };
        match step() {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| 1)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_rmw sqlite: {e}"))
            }
        }
    })
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn delete_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    rows: &[Vec<bumbledb::Value>],
) -> Result<Measurement, String> {
    check_stream("crud_delete", rows.len(), proto)?;
    let mut iter = rows.iter();
    harness::measure(proto, || {
        let row = iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| {
            if tx.delete_dyn(ids::DOC, [row])?.changed() == 0 {
                return Err(refuse(
                    "the delete must be delete-bearing: the pool row was absent",
                ));
            }
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            1
        })
        .map_err(|e| format!("crud_delete: {e:?}"))
    })
}

/// # Errors
/// `SQLite` errors, stringified; a stream/protocol length mismatch or a
pub fn delete_sqlite(
    conn: &Connection,
    proto: Protocol,
    pool_ids: &[u64],
) -> Result<Measurement, String> {
    check_stream("crud_delete", pool_ids.len(), proto)?;
    let mut iter = pool_ids.iter();
    harness::measure(proto, || {
        let id = sql_u64(
            *iter
                .next()
                .ok_or("the stream ended before the protocol did")?,
        );
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        let step = || -> Result<(), String> {
            let changed = conn
                .prepare_cached(DOC_DELETE)
                .map_err(|e| format!("prepare: {e}"))?
                .execute([id])
                .map_err(|e| format!("delete: {e}"))?;
            if changed == 1 {
                Ok(())
            } else {
                Err(format!(
                    "the delete must change exactly one row, changed {changed}"
                ))
            }
        };
        match step() {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| 1)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_delete sqlite: {e}"))
            }
        }
    })
}

/// # Errors
pub fn mixed_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    seed: u64,
    sizes: CrudSizes,
    cursor: &mut FreshCursor,
) -> Result<Measurement, String> {
    let query = read_query();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new(ops::read_keys(seed, sizes));
    let mut buffer = Answers::new();
    harness::measure(proto, || {
        let mut drained = 0u64;
        for _ in 0..MIXED_READS {
            let binds = families::bind_values(rotation.next_set());
            db.read(|snap| snap.execute(&mut prepared, &binds, &mut buffer))
                .map_err(|e| format!("crud_mixed_90_10 read: {e:?}"))?;
            drained += buffer.len() as u64;
        }
        db.write(|tx| mint_doc(tx, seed, cursor))
            .map_err(|e| format!("crud_mixed_90_10 insert: {e:?}"))?
            .unwrap();
        Ok(drained + 1)
    })
}

/// # Errors
pub fn mixed_sqlite(
    conn: &Connection,
    proto: Protocol,
    seed: u64,
    sizes: CrudSizes,
    cursor: &mut FreshCursor,
) -> Result<Measurement, String> {
    let translated = translate::translate(&read_query(), schema(), &[])
        .map_err(|e| format!("translate: {e}"))?;
    let mut prepared =
        PreparedFamily::new(conn, &translated, vec![ValueType::U64, ValueType::I64])?;
    let mut rotation = Rotation::new(ops::read_keys(seed, sizes));
    harness::measure(proto, || {
        let mut drained = 0u64;
        for _ in 0..MIXED_READS {
            drained += sqlite_run::sample(&mut prepared, rotation.next_set())?;
        }
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        match mint_doc_sqlite(conn, seed, cursor) {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| drained + 1)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_mixed_90_10 sqlite: {e}"))
            }
        }
    })
}
