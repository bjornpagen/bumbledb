//! The crud family runners — one engine runner and one `SQLite` runner
//! per family, both folding the SAME precomputed op stream
//! ([`super::ops`]) over their own store, so post-state equality is a
//! consequence of the representation, not a hope ([`crate::poststate`]
//! is the judge). Every runner takes its [`Protocol`] explicitly —
//! tests pass tiny protocols; no protocol is baked into a runner.
//!
//! **The `SQLite` twins play their own best game**: their writes are
//! the NATIVE SQL for each shape — bound `INSERT`s, `UPDATE … WHERE`,
//! the conflict-target upsert, `DELETE … WHERE` — inside
//! `BEGIN IMMEDIATE … COMMIT` on reused prepared statements (the
//! `sqlite_run::commits` shape), never a transliteration of our
//! delete+insert revision idiom.
//!
//! **Refusal contracts ride inside the write closures** (the
//! `writebench::posting_swap` precedent): a delete lane that stops
//! deleting, an update whose previous value is gone, or an upsert that
//! stops matching its stream aborts the transaction whole — engine
//! side by an in-closure `Err` (the delta drops, nothing commits),
//! `SQLite` side by `ROLLBACK` — instead of silently measuring the
//! wrong fork.
//!
//! **Fresh minting** threads a [`FreshCursor`] through the
//! insert-bearing runners of ONE engine's pass; each engine gets its
//! own cursor, so the two passes mint identical id/key/val/payload
//! sequences by construction. The engine side asserts the minted fresh
//! id equals the cursor inside the closure — mint drift is a loud
//! abort, never a divergent measurement.

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

/// The mixed lane's read fan: 9 point reads per single-row insert — the
/// 90/10 shape.
const MIXED_READS: u32 = 9;

/// The native `Doc` insert (id, key, val, payload — declaration order).
const DOC_INSERT: &str = "INSERT INTO \"Doc\" VALUES (?1, ?2, ?3, ?4)";
/// The native counter update.
const COUNTER_UPDATE: &str = "UPDATE \"Counter\" SET \"val\" = ?1 WHERE \"key\" = ?2";
/// The native upsert — the conflict target is the UNIQUE index the
/// `Counter(key) -> Counter` statement renders on the mirror.
const COUNTER_UPSERT: &str = "INSERT INTO \"Counter\" VALUES (?1, ?2) \
                              ON CONFLICT(\"key\") DO UPDATE SET \"val\" = excluded.\"val\"";
/// The rmw round trip's read half.
const COUNTER_SELECT: &str = "SELECT \"val\" FROM \"Counter\" WHERE \"key\" = ?1";
/// The native pool delete.
const DOC_DELETE: &str = "DELETE FROM \"Doc\" WHERE \"id\" = ?1";

/// The shared fresh-mint cursor: starts at `docs + delete_pool` (the
/// fresh mint base both engines share after load) and advances one per
/// minted `Doc` row. One cursor per engine pass — the inserted row is
/// `{ id: cursor, key: cursor, val: cursor, payload: f(seed, cursor) }`
/// on BOTH sides by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshCursor(pub u64);

impl FreshCursor {
    /// The mint base: the first id above the loaded corpus (standing
    /// docs plus the delete pool).
    #[must_use]
    pub fn at_base(sizes: CrudSizes) -> Self {
        Self(sizes.docs + sizes.delete_pool)
    }
}

/// The `crud_read_point` query (timed in the crud orchestration; the
/// query LIVES here): a single rule finding `(id, val)` of the one
/// `Doc` whose `key` binds the parameter — the `points.rs` `by_key`
/// shape over the crud world.
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

/// The protocol's total closure invocations — every stream's required
/// length.
fn invocations(proto: Protocol) -> usize {
    usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small")
}

/// The in-closure refusal sentinel (the `posting_swap` precedent):
/// returning this from a write closure drops the delta whole, so a
/// refused sample commits nothing.
fn refuse(what: &str) -> bumbledb::Error {
    bumbledb::Error::Io(std::io::Error::other(what.to_owned()))
}

/// A stream whose length disagrees with the protocol is refused at
/// runner entry — the count contract is part of the representation.
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

/// One corpus-axiom u64 as the `SQLite` INTEGER it maps to.
fn sql_u64(value: u64) -> i64 {
    i64::try_from(value).expect("corpus ids and keys stay below 2^63")
}

/// One freshly minted `Doc`, engine side, inside an open transaction:
/// alloc, assert the mint equals the cursor (drift aborts the
/// transaction whole), insert, advance.
fn mint_doc(
    tx: &mut bumbledb::WriteTx<'_, CrudWorld>,
    seed: u64,
    cursor: &mut FreshCursor,
) -> bumbledb::Result<()> {
    let id: CrudDocId = tx.alloc()?;
    if id.0 != cursor.0 {
        return Err(refuse(&format!(
            "the fresh mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.0
        )));
    }
    tx.insert(&Doc {
        id,
        key: cursor.0,
        val: i64::try_from(cursor.0).expect("mints stay below 2^63"),
        payload: ops::fresh_payload(seed, cursor.0),
    })?;
    cursor.0 += 1;
    Ok(())
}

/// One freshly minted `Doc`, `SQLite` side, inside an open transaction:
/// the cursor binds directly (the `commits.rs` host-counter precedent).
fn mint_doc_sqlite(conn: &Connection, seed: u64, cursor: &mut FreshCursor) -> Result<(), String> {
    let id = sql_u64(cursor.0);
    conn.prepare_cached(DOC_INSERT)
        .map_err(|e| format!("prepare: {e}"))?
        .execute((id, id, id, ops::fresh_payload(seed, cursor.0).to_vec()))
        .map_err(|e| format!("insert: {e}"))?;
    cursor.0 += 1;
    Ok(())
}

/// `crud_insert` and its batch siblings on bumbledb: one sample = one
/// `db.write` minting `per_commit` fresh `Doc` rows through the typed
/// path (`per_commit` = 1/10/100/1000 for the four registered
/// families). work = `per_commit` per sample.
///
/// # Errors
///
/// Engine errors, stringified; a cursor-drifted mint, named.
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
        .map(|()| per_commit)
        .map_err(|e| format!("crud_insert x{per_commit}: {e:?}"))
    })
}

/// `crud_insert` and its batch siblings on `SQLite`: one sample =
/// `per_commit` bound executions of the native `INSERT` on a reused
/// prepared statement inside `BEGIN IMMEDIATE … COMMIT`.
///
/// # Errors
///
/// `SQLite` errors, stringified.
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

/// `crud_update` / `crud_update_hot` on bumbledb (the two families are
/// one runner over two streams): one sample = one `db.write` replacing
/// `Counter{key, prev}` with `Counter{key, next}` — delete-bearing by
/// contract: a no-op delete (the stream's `prev` absent) refuses inside
/// the closure and the transaction aborts whole.
///
/// # Errors
///
/// Engine errors, stringified; a stream/protocol length mismatch or a
/// non-delete-bearing update, named.
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
            if !tx.delete(&Counter {
                key: op.key,
                val: op.prev,
            })? {
                return Err(refuse(
                    "the update must be delete-bearing: the stream's prev value was absent",
                ));
            }
            tx.insert(&Counter {
                key: op.key,
                val: op.next,
            })?;
            Ok(())
        })
        .map(|()| 1)
        .map_err(|e| format!("crud_update: {e:?}"))
    })
}

/// `crud_update` / `crud_update_hot` on `SQLite`: the native
/// `UPDATE "Counter" SET "val" = ? WHERE "key" = ?` inside
/// `BEGIN IMMEDIATE … COMMIT`, with `changes() == 1` asserted inside
/// the closure — a missed update rolls back and errs.
///
/// # Errors
///
/// `SQLite` errors, stringified; a stream/protocol length mismatch or
/// an update that changed anything but exactly one row, named.
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

/// `crud_upsert` on bumbledb — the blessed idiom (the `WriteTx::get`
/// doc example): keyed point read inside the write, delete+insert on a
/// hit, plain insert on a miss. The store is checked against the
/// stream's `prev` inside the closure — drift aborts the transaction
/// whole instead of measuring a fork the `SQLite` twin never took.
///
/// # Errors
///
/// Engine errors, stringified; a stream/protocol length mismatch or a
/// stream-drifted upsert, named.
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
                    tx.delete(&old)?;
                    tx.insert(&Counter {
                        key: op.key,
                        val: op.next,
                    })?;
                }
                None => {
                    tx.insert(&Counter {
                        key: op.key,
                        val: op.next,
                    })?;
                }
            }
            Ok(())
        })
        .map(|()| 1)
        .map_err(|e| format!("crud_upsert: {e:?}"))
    })
}

/// `crud_upsert` on `SQLite`: the native conflict-target upsert
/// ([`COUNTER_UPSERT`] — the UNIQUE index from the key statement is the
/// target) inside `BEGIN IMMEDIATE … COMMIT`.
///
/// # Errors
///
/// `SQLite` errors, stringified; a stream/protocol length mismatch,
/// named.
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

/// `crud_rmw` on bumbledb — the read-modify-write round trip: keyed
/// point read inside the write, `val + 1` computed by the host, delete
/// the read fact, insert the successor. A missing key refuses (the
/// stream draws over the standing mass, which never shrinks).
///
/// # Errors
///
/// Engine errors, stringified; a stream/protocol length mismatch or a
/// missing counter row, named.
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
            tx.delete(&old)?;
            tx.insert(&Counter { key, val: next })?;
            Ok(())
        })
        .map(|()| 1)
        .map_err(|e| format!("crud_rmw: {e:?}"))
    })
}

/// `crud_rmw` on `SQLite`: `BEGIN IMMEDIATE`, `SELECT "val"` by key,
/// the host computes `val + 1`, `UPDATE` binds it back, `changes() == 1`
/// asserted, `COMMIT` — the value genuinely round-trips through the
/// host on both sides.
///
/// # Errors
///
/// `SQLite` errors, stringified; a stream/protocol length mismatch, a
/// missing counter row, or a missed update, named.
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

/// `crud_delete` on bumbledb: sample `i` deletes pool row `docs + i` —
/// the full fact re-derived through the one corpus row function
/// ([`ops::delete_rows`], which asserts the pool-size ≥
/// warmups+samples invariant at derivation). Delete-bearing by
/// contract: an absent pool row refuses inside the closure.
///
/// # Errors
///
/// Engine errors, stringified; a non-delete-bearing sample, named.
///
/// # Panics
///
/// When the protocol outruns the delete pool (the [`ops::delete_rows`]
/// invariant — a misregistered protocol, loud at entry).
pub fn delete_bumbledb(
    db: &Db<CrudWorld>,
    proto: Protocol,
    seed: u64,
    sizes: CrudSizes,
) -> Result<Measurement, String> {
    let rows = ops::delete_rows(seed, sizes, invocations(proto));
    let mut iter = rows.iter();
    harness::measure(proto, || {
        let row = iter.next().expect("delete_rows covers the protocol");
        db.write(|tx| {
            if !tx.delete_dyn(ids::DOC, row)? {
                return Err(refuse(
                    "the delete must be delete-bearing: the pool row was absent",
                ));
            }
            Ok(())
        })
        .map(|()| 1)
        .map_err(|e| format!("crud_delete: {e:?}"))
    })
}

/// `crud_delete` on `SQLite`: the native `DELETE … WHERE "id" = ?` on
/// the same pool id sequence (`docs + i`, in order), `changes() == 1`
/// asserted inside the closure.
///
/// # Errors
///
/// `SQLite` errors, stringified; a delete that changed anything but
/// exactly one row, named.
///
/// # Panics
///
/// When the protocol outruns the delete pool (the shared invariant,
/// asserted at entry).
pub fn delete_sqlite(
    conn: &Connection,
    proto: Protocol,
    sizes: CrudSizes,
) -> Result<Measurement, String> {
    let count = u64::try_from(invocations(proto)).expect("protocol counts are small");
    assert!(
        count <= sizes.delete_pool,
        "the delete pool ({}) must cover every invocation ({count})",
        sizes.delete_pool
    );
    let mut next = sizes.docs;
    harness::measure(proto, || {
        let id = sql_u64(next);
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
            Ok(()) => {
                conn.execute_batch("COMMIT")
                    .map_err(|e| format!("commit: {e}"))?;
                next += 1;
                Ok(1)
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("crud_delete sqlite: {e}"))
            }
        }
    })
}

/// `crud_mixed_90_10` on bumbledb: one sample = 9 point reads (the
/// prepared [`read_query`] over the rotating [`ops::read_keys`] sets)
/// plus one single-row insert commit. work = answers drained + 1.
///
/// # Errors
///
/// Engine errors, stringified; a cursor-drifted mint, named.
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
            .map_err(|e| format!("crud_mixed_90_10 insert: {e:?}"))?;
        Ok(drained + 1)
    })
}

/// `crud_mixed_90_10` on `SQLite`: 9 executions of the CANONICAL
/// translation of [`read_query`] on one reused prepared statement
/// (rotating the IDENTICAL key sequence), then one native single-row
/// `INSERT` transaction. work = rows drained + 1.
///
/// # Errors
///
/// Translation and `SQLite` errors, stringified.
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
