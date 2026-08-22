//! Six families ([`super::families`]): two LEGAL commit lanes (fsync-bound,
//! both engines folding ONE precomputed [`AttemptOp`] stream — the crud
//! representation, so post-state equality is structural) and four REJECTION
//! lanes, where the refusal IS the measured work. **The refusal contract rides
//! in the closure's type** (the runner per family, explicit [`Protocol`],
//! `harness::measure` legal lane ever mints after them.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Schema, StatementId, Theory};
use rusqlite::Connection;

use crate::harness::{self, Measurement, Protocol};

use super::{
    Attempt, LawAttemptId, LawSizes, LawSteerId, LawTaskId, LawfulWorld, Outcome, Steer,
    SteerKinds, SteerScope, Verdict, enforcement,
};

pub const WINDOW_CAP: u64 = 8;

pub const REJECT_ID_BASE: u64 = 1 << 62;

const ATTEMPT_INSERT: &str = "INSERT INTO \"Attempt\" VALUES (?1, ?2, ?3)";

const VERDICT_INSERT: &str = "INSERT INTO \"Verdict\" VALUES (?1, ?2)";

const STEER_INSERT: &str = "INSERT INTO \"Steer\" VALUES (?1, ?2, ?3)";

const SCOPE_INSERT: &str = "INSERT INTO \"SteerScope\" VALUES (?1, ?2)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptOp {
    pub task: u64,
    pub n: u64,
}

/// # Panics
/// On a degenerate size (fewer than two tasks — the round-robin needs tasks
/// `1..tasks` to be nonempty) or counts beyond the address space (impossible
/// for protocol counts).
#[must_use]
pub fn attempt_ops(sizes: LawSizes, count: usize) -> Vec<AttemptOp> {
    assert!(
        sizes.tasks > 1,
        "the legal streams round-robin tasks 1..tasks"
    );
    let span = usize::try_from(sizes.tasks - 1).expect("sizes fit usize");
    let mut next_n = vec![sizes.attempts_per_task; span];
    (0..count)
        .map(|i| {
            let slot = i % span;
            let n = next_n[slot];
            next_n[slot] += 1;
            AttemptOp {
                task: 1 + u64::try_from(slot).expect("span fits u64"),
                n,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawCursor {
    pub attempt: u64,

    pub steer: u64,
}

impl LawCursor {
    /// The mint base after load: the seeded corpus is dense from 0, so
    #[must_use]
    pub fn at_base(sizes: LawSizes) -> Self {
        Self {
            attempt: sizes.tasks * sizes.attempts_per_task,
            steer: sizes.steers,
        }
    }
}

fn lawful_schema() -> Schema {
    LawfulWorld
        .descriptor()
        .validate()
        .expect("LawfulWorld is a valid theory")
}

/// # Panics
#[must_use]
pub fn psi_statement() -> StatementId {
    let index = enforcement::MAP
        .iter()
        .position(|row| row.law == "ψ-selected containment")
        .expect("the enforcement map carries the ψ row");
    StatementId(u16::try_from(index).expect("the map is tiny"))
}

/// The in-closure refusal sentinel (the `posting_swap` precedent): returning
/// this from a write closure drops the delta whole, so a refused sample commits
/// nothing.
fn refuse(what: &str) -> bumbledb::Error {
    bumbledb::Error::from(std::io::Error::other(what.to_owned()))
}

/// The protocol's total closure invocations — every stream's required length.
fn invocations(proto: Protocol) -> usize {
    usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small")
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
    i64::try_from(value).expect("lawful ids and values stay below 2^63")
}

fn mint_attempt(
    tx: &mut bumbledb::WriteTx<'_, LawfulWorld>,
    op: AttemptOp,
    cursor: &mut LawCursor,
) -> bumbledb::Result<LawAttemptId> {
    let id: LawAttemptId = tx.reserve(1)?.start().expect("nonempty");
    if id.0 != cursor.attempt {
        return Err(refuse(&format!(
            "the Attempt mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.attempt
        )));
    }
    tx.insert([&Attempt {
        id,
        task: LawTaskId(op.task),
        n: op.n,
    }])?;
    cursor.attempt += 1;
    Ok(id)
}

fn mint_steer(
    tx: &mut bumbledb::WriteTx<'_, LawfulWorld>,
    task: u64,
    cursor: &mut LawCursor,
) -> bumbledb::Result<LawSteerId> {
    let id: LawSteerId = tx.reserve(1)?.start().expect("nonempty");
    if id.0 != cursor.steer {
        return Err(refuse(&format!(
            "the Steer mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.steer
        )));
    }
    tx.insert([&Steer {
        id,
        kind: SteerKinds::Repartition.id(),
        task: LawTaskId(task),
    }])?;
    cursor.steer += 1;
    Ok(id)
}

fn insert_attempt_sqlite(
    conn: &Connection,
    op: AttemptOp,
    cursor: &mut LawCursor,
) -> rusqlite::Result<()> {
    conn.prepare_cached(ATTEMPT_INSERT)?.execute((
        sql_u64(cursor.attempt),
        sql_u64(op.task),
        sql_u64(op.n),
    ))?;
    cursor.attempt += 1;
    Ok(())
}

/// The window setup, engine side (untimed, before any measuring): one
/// # Errors
pub fn fill_window_target_engine(
    db: &Db<LawfulWorld>,
    sizes: LawSizes,
    cursor: &mut LawCursor,
) -> Result<(), String> {
    db.write(|tx| {
        for n in sizes.attempts_per_task..WINDOW_CAP {
            mint_attempt(tx, AttemptOp { task: 0, n }, cursor)?;
        }
        Ok(())
    })
    .map_err(|e| format!("law_reject_window setup (engine): {e:?}"))?
    .unwrap();
    Ok(())
}

/// # Errors
pub fn fill_window_target_sqlite(
    conn: &Connection,
    sizes: LawSizes,
    cursor: &mut LawCursor,
) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("law_reject_window setup begin: {e}"))?;
    let mut step = || -> rusqlite::Result<()> {
        for n in sizes.attempts_per_task..WINDOW_CAP {
            insert_attempt_sqlite(conn, AttemptOp { task: 0, n }, cursor)?;
        }
        Ok(())
    };
    match step() {
        Ok(()) => conn
            .execute_batch("COMMIT")
            .map_err(|e| format!("law_reject_window setup commit: {e}")),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(format!("law_reject_window setup (sqlite): {e}"))
        }
    }
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn commit_attempt_engine(
    db: &Db<LawfulWorld>,
    proto: Protocol,
    stream: &[AttemptOp],
    cursor: &mut LawCursor,
) -> Result<Measurement, String> {
    check_stream("law_commit_attempt", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = *iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| mint_attempt(tx, op, cursor).map(|_| ()))
            .map(|admission| {
                admission.unwrap();
                1
            })
            .map_err(|e| format!("law_commit_attempt: {e:?}"))
    })
}

/// # Errors
pub fn commit_attempt_sqlite(
    conn: &Connection,
    proto: Protocol,
    stream: &[AttemptOp],
    cursor: &mut LawCursor,
) -> Result<Measurement, String> {
    check_stream("law_commit_attempt", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = *iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        match insert_attempt_sqlite(conn, op, cursor) {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| 1)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("law_commit_attempt sqlite: {e}"))
            }
        }
    })
}

/// # Errors
/// Engine errors, stringified; a stream/protocol length mismatch or a
pub fn commit_cluster_engine(
    db: &Db<LawfulWorld>,
    proto: Protocol,
    stream: &[AttemptOp],
    cursor: &mut LawCursor,
) -> Result<Measurement, String> {
    check_stream("law_commit_cluster", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = *iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        db.write(|tx| {
            let attempt = mint_attempt(tx, op, cursor)?;
            tx.insert([&Verdict {
                attempt,
                outcome: Outcome::Accepted.id(),
            }])?;
            let steer = mint_steer(tx, op.task, cursor)?;
            tx.insert([&SteerScope {
                steer,
                grp: op.task,
            }])?;
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            4
        })
        .map_err(|e| format!("law_commit_cluster: {e:?}"))
    })
}

/// immediate FK and trigger checks pass (Attempt before its Verdict,
/// # Errors
pub fn commit_cluster_sqlite(
    conn: &Connection,
    proto: Protocol,
    stream: &[AttemptOp],
    cursor: &mut LawCursor,
) -> Result<Measurement, String> {
    check_stream("law_commit_cluster", stream.len(), proto)?;
    let mut iter = stream.iter();
    harness::measure(proto, || {
        let op = *iter
            .next()
            .ok_or("the stream ended before the protocol did")?;
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("begin: {e}"))?;
        let step = |cursor: &mut LawCursor| -> rusqlite::Result<()> {
            let attempt = cursor.attempt;
            insert_attempt_sqlite(conn, op, cursor)?;
            conn.prepare_cached(VERDICT_INSERT)?
                .execute((sql_u64(attempt), sql_u64(Outcome::Accepted.id().0)))?;
            conn.prepare_cached(STEER_INSERT)?.execute((
                sql_u64(cursor.steer),
                sql_u64(SteerKinds::Repartition.id().0),
                sql_u64(op.task),
            ))?;
            conn.prepare_cached(SCOPE_INSERT)?
                .execute((sql_u64(cursor.steer), sql_u64(op.task)))?;
            cursor.steer += 1;
            Ok(())
        };
        match step(cursor) {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map(|()| 4)
                .map_err(|e| format!("commit: {e}")),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(format!("law_commit_cluster sqlite: {e}"))
            }
        }
    })
}

fn cites_functionality(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Functionality { .. }))
}

fn cites_containment(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Containment { .. }))
}

fn cites_capacity(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Capacity { .. }))
}

fn cites_psi(violations: &bumbledb::Violations) -> bool {
    violations.iter().any(|violation| {
        matches!(violation, bumbledb::Violation::Containment { .. })
            && violation.statement_id(&lawful_schema()) == psi_statement()
    })
}

/// One refused engine commit — the rejection lanes' shared spine.
fn refused_commit(
    db: &Db<LawfulWorld>,
    family: &'static str,
    expected: &'static str,
    cites: fn(&bumbledb::Violations) -> bool,
    violate: impl FnOnce(&mut bumbledb::WriteTx<'_, LawfulWorld>) -> bumbledb::Result<()>,
) -> Result<u64, String> {
    match db.write(violate) {
        Ok(bumbledb::Admission::Rejected(violations)) => {
            if cites(&violations) {
                Ok(1)
            } else {
                Err(format!(
                    "{family}: rejected without the expected {expected} citation: {:?}",
                    crate::differential::cited(&violations, db.schema())
                ))
            }
        }
        Ok(bumbledb::Admission::Accepted(_)) => Err(format!(
            "{family}: the violating commit was ACCEPTED — the refusal contract is broken"
        )),
        Err(other) => Err(format!(
            "{family}: expected admission rejection, the engine said {other:?}"
        )),
    }
}

/// One refused `SQLite` insert — the mirror spine: `BEGIN IMMEDIATE`, the
/// violating `INSERT` on a reused prepared statement (expected to fail with a
/// constraint violation — UNIQUE, FK, or a trigger's `RAISE(ABORT)`), then
/// `ROLLBACK`; the whole round trip, rollback included, is the sample.
fn refused_insert_sqlite<P: rusqlite::Params>(
    conn: &Connection,
    family: &'static str,
    sql: &str,
    params: P,
) -> Result<u64, String> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("{family}: begin: {e}"))?;
    let outcome = conn
        .prepare_cached(sql)
        .and_then(|mut stmt| stmt.execute(params));
    conn.execute_batch("ROLLBACK")
        .map_err(|e| format!("{family}: rollback: {e}"))?;
    match outcome {
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Ok(1)
        }
        Err(other) => Err(format!(
            "{family}: expected a constraint refusal, sqlite said: {other}"
        )),
        Ok(changed) => Err(format!(
            "{family}: the violating INSERT was ACCEPTED ({changed} row changed) — \
             the refusal contract is broken"
        )),
    }
}

/// # Errors
pub fn reject_key_engine(db: &Db<LawfulWorld>, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = LawAttemptId(REJECT_ID_BASE + sample);
        sample += 1;
        refused_commit(
            db,
            "law_reject_key",
            "Functionality",
            cites_functionality,
            |tx| {
                tx.insert([&Attempt {
                    id,
                    task: LawTaskId(1),
                    n: 0,
                }])
                .map(|_| ())
            },
        )
    })
}

/// `law_reject_key` on `SQLite`: the same duplicate `(task 1, n 0)` binding
/// (the identical sacrificial id) expecting the UNIQUE violation, then
/// `ROLLBACK` — the refused round trip is the sample.
/// # Errors
pub fn reject_key_sqlite(conn: &Connection, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = sql_u64(REJECT_ID_BASE + sample);
        sample += 1;
        refused_insert_sqlite(conn, "law_reject_key", ATTEMPT_INSERT, (id, 1i64, 0i64))
    })
}

/// # Errors
pub fn reject_containment_engine(
    db: &Db<LawfulWorld>,
    proto: Protocol,
    sizes: LawSizes,
) -> Result<Measurement, String> {
    let absent = sizes.tasks + 1_000_000;
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = LawAttemptId(REJECT_ID_BASE + sample);
        sample += 1;
        refused_commit(
            db,
            "law_reject_containment",
            "Containment",
            cites_containment,
            |tx| {
                tx.insert([&Attempt {
                    id,
                    task: LawTaskId(absent),
                    n: 0,
                }])
                .map(|_| ())
            },
        )
    })
}

/// # Errors
pub fn reject_containment_sqlite(
    conn: &Connection,
    proto: Protocol,
    sizes: LawSizes,
) -> Result<Measurement, String> {
    let absent = sql_u64(sizes.tasks + 1_000_000);
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = sql_u64(REJECT_ID_BASE + sample);
        sample += 1;
        refused_insert_sqlite(
            conn,
            "law_reject_containment",
            ATTEMPT_INSERT,
            (id, absent, 0i64),
        )
    })
}

/// `law_reject_window` on bumbledb (after the untimed setup filled task
/// # Errors
pub fn reject_window_engine(db: &Db<LawfulWorld>, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = LawAttemptId(REJECT_ID_BASE + sample);
        sample += 1;
        refused_commit(db, "law_reject_window", "Capacity", cites_capacity, |tx| {
            tx.insert([&Attempt {
                id,
                task: LawTaskId(0),
                n: WINDOW_CAP,
            }])
            .map(|_| ())
        })
    })
}

/// # Errors
pub fn reject_window_sqlite(conn: &Connection, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = sql_u64(REJECT_ID_BASE + sample);
        sample += 1;
        refused_insert_sqlite(
            conn,
            "law_reject_window",
            ATTEMPT_INSERT,
            (id, 0i64, sql_u64(WINDOW_CAP)),
        )
    })
}

/// # Errors
pub fn reject_scope_engine(db: &Db<LawfulWorld>, proto: Protocol) -> Result<Measurement, String> {
    harness::measure(proto, || {
        refused_commit(
            db,
            "law_reject_scope",
            "ψ-statement Containment",
            cites_psi,
            |tx| {
                tx.insert([&SteerScope {
                    steer: LawSteerId(0),
                    grp: 0,
                }])
                .map(|_| ())
            },
        )
    })
}

/// # Errors
pub fn reject_scope_sqlite(conn: &Connection, proto: Protocol) -> Result<Measurement, String> {
    harness::measure(proto, || {
        refused_insert_sqlite(conn, "law_reject_scope", SCOPE_INSERT, (0i64, 0i64))
    })
}
