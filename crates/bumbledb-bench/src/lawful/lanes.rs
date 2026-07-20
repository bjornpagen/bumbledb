//! The lawful family runners — one engine runner and one `SQLite`
//! runner per family, explicit [`Protocol`], `harness::measure`
//! throughout. Six families ([`super::families`]): two LEGAL commit
//! lanes (fsync-bound, both engines folding ONE precomputed
//! [`AttemptOp`] stream — the crud representation, so post-state
//! equality is structural) and four REJECTION lanes, where the refusal
//! IS the measured work.
//!
//! **The refusal contract rides in the closure's type** (the
//! `writebench::posting_swap` precedent): a rejection sample returns
//! `Ok(1)` ONLY on the expected refusal — ours on
//! `Error::CommitRejected` carrying the expected citation kind, theirs
//! on a constraint failure from the violating `INSERT` (then
//! `ROLLBACK`). An ACCEPTED commit, or a refusal of the wrong kind, is
//! an `Err` that aborts the whole run — the lane can never drift into
//! measuring accepted work.
//!
//! **The fresh-mint interaction (the never-reissue law, commit
//! d08651b4):** ours' rejected commits still BURN escaped fresh values
//! — an explicit fresh-field value above the high-water mark advances
//! it, and an abort flushes the advanced mark exactly as a commit does
//! — while `SQLite`'s host counter does not move on a refusal. The
//! rejection lanes therefore mint NOTHING through `tx.alloc`: the
//! violating rows carry EXPLICIT ids from [`REJECT_ID_BASE`] (legal —
//! the ETL explicit-fresh surface), mirrored verbatim on the `SQLite`
//! binding so a hypothetical acceptance would still be twin-identical.
//! Because every such commit refuses, no id is ever committed on
//! either side and the post-state fold stays green; the burned engine
//! mark is why the rejection lanes sit LAST in registry order — no
//! legal lane ever allocs after them.
//!
//! **Fresh minting on the legal lanes** threads a [`LawCursor`] through
//! ONE engine's pass (each engine pass gets its own cursor, constructed
//! at the shared base, so the two passes mint identical Attempt/Steer
//! id sequences by construction — the symmetry the post-state fold
//! certifies). The engine side asserts every `tx.alloc` mint equals its
//! cursor inside the closure: mint drift is a loud abort, never a
//! divergent measurement.

use bumbledb::{Db, StatementId};
use rusqlite::Connection;

use crate::harness::{self, Measurement, Protocol};

use super::{
    Attempt, LawAttemptId, LawSizes, LawSteerId, LawTaskId, LawfulWorld, Outcome, Steer,
    SteerKinds, SteerScope, Verdict, enforcement,
};

/// The `{0..8}` window's ceiling — the schema's `Task(id) <={0..8}
/// Attempt(task)` cap, transcribed once (the window setup fills task 0
/// exactly to it; the window rejection inserts one past it).
pub const WINDOW_CAP: u64 = 8;

/// The rejection lanes' explicit sacrificial id base: far beyond both
/// engines' counters (the engine's fresh high-water mark and the
/// `SQLite` host cursor both sit near the corpus mass), and within
/// `i64` so the mirrored `SQLite` binding is representable. Sample `i`
/// of a rejection lane binds `REJECT_ID_BASE + i` on BOTH engines — a
/// hypothetical acceptance would be twin-identical — and since every
/// such commit refuses, nothing is ever committed under these ids.
pub const REJECT_ID_BASE: u64 = 1 << 62;

/// The native `Attempt` insert (id, task, n — declaration order).
const ATTEMPT_INSERT: &str = "INSERT INTO \"Attempt\" VALUES (?1, ?2, ?3)";
/// The native `Verdict` insert (attempt, outcome).
const VERDICT_INSERT: &str = "INSERT INTO \"Verdict\" VALUES (?1, ?2)";
/// The native `Steer` insert (id, kind, task).
const STEER_INSERT: &str = "INSERT INTO \"Steer\" VALUES (?1, ?2, ?3)";
/// The native `SteerScope` insert (steer, grp).
const SCOPE_INSERT: &str = "INSERT INTO \"SteerScope\" VALUES (?1, ?2)";

/// One legal-lane op: the Attempt to commit — `(task, n)` fully
/// determines the row on both engines (the ids come from each engine's
/// own cursor, in lockstep by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptOp {
    pub task: u64,
    pub n: u64,
}

/// The legal lanes' shared op stream — a pure function of
/// `(sizes, count)`: round-robin over tasks `1..tasks` (task 0 is the
/// window lane's saturated cap target, never touched by a legal
/// stream), each task's `n` counter starting at `attempts_per_task`
/// (the seeded rows hold `0..attempts_per_task`), so every op commits
/// legally under the full roster. The orchestration generates ONE
/// stream covering both legal families and slices it in registry
/// order, so the per-task counters continue across families and no
/// `(task, n)` key is ever minted twice.
///
/// # Panics
///
/// On a degenerate size (fewer than two tasks — the round-robin needs
/// tasks `1..tasks` to be nonempty) or counts beyond the address space
/// (impossible for protocol counts).
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

/// The shared fresh-mint cursor: one per engine pass, both constructed
/// at [`LawCursor::at_base`], both advanced by the same events in the
/// same registry order (window setup, then the legal families), so the
/// two engines' Attempt/Steer id sequences are identical by
/// construction — the symmetry the post-state fold certifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawCursor {
    /// The next `Attempt.id` either engine mints.
    pub attempt: u64,
    /// The next `Steer.id` either engine mints.
    pub steer: u64,
}

impl LawCursor {
    /// The mint base after load: the seeded corpus is dense from 0, so
    /// both the engine's fresh high-water mark and the `SQLite` host
    /// counter sit at the seeded row counts.
    #[must_use]
    pub fn at_base(sizes: LawSizes) -> Self {
        Self {
            attempt: sizes.tasks * sizes.attempts_per_task,
            steer: sizes.steers,
        }
    }
}

/// The ψ-selected containment's materialized [`StatementId`], derived
/// from the enforcement map's position (the map is in materialized
/// statement order — its totality test pins the length, the naive
/// parity test pins the citations) — the scope rejection lane matches
/// its Containment citation against exactly this statement.
///
/// # Panics
///
/// Never in practice: the map carries exactly one ψ row.
#[must_use]
pub fn psi_statement() -> StatementId {
    let index = enforcement::MAP
        .iter()
        .position(|row| row.law == "ψ-selected containment")
        .expect("the enforcement map carries the ψ row");
    StatementId(u16::try_from(index).expect("the map is tiny"))
}

/// The in-closure refusal sentinel (the `posting_swap` precedent):
/// returning this from a write closure drops the delta whole, so a
/// refused sample commits nothing.
fn refuse(what: &str) -> bumbledb::Error {
    bumbledb::Error::Io(std::io::Error::other(what.to_owned()))
}

/// The protocol's total closure invocations — every stream's required
/// length.
fn invocations(proto: Protocol) -> usize {
    usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small")
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
    i64::try_from(value).expect("lawful ids and values stay below 2^63")
}

/// One judged Attempt, engine side, inside an open transaction: alloc,
/// assert the mint equals the cursor (drift aborts the transaction
/// whole), insert, advance.
fn mint_attempt(
    tx: &mut bumbledb::WriteTx<'_, LawfulWorld>,
    op: AttemptOp,
    cursor: &mut LawCursor,
) -> bumbledb::Result<LawAttemptId> {
    let id: LawAttemptId = tx.alloc()?;
    if id.0 != cursor.attempt {
        return Err(refuse(&format!(
            "the Attempt mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.attempt
        )));
    }
    tx.insert(&Attempt {
        id,
        task: LawTaskId(op.task),
        n: op.n,
    })?;
    cursor.attempt += 1;
    Ok(id)
}

/// One Repartition Steer, engine side, inside an open transaction —
/// the cluster's ψ-selected target ([`mint_attempt`]'s sibling).
fn mint_steer(
    tx: &mut bumbledb::WriteTx<'_, LawfulWorld>,
    task: u64,
    cursor: &mut LawCursor,
) -> bumbledb::Result<LawSteerId> {
    let id: LawSteerId = tx.alloc()?;
    if id.0 != cursor.steer {
        return Err(refuse(&format!(
            "the Steer mint drifted from the shared cursor: minted {}, expected {}",
            id.0, cursor.steer
        )));
    }
    tx.insert(&Steer {
        id,
        kind: SteerKinds::Repartition.id(),
        task: LawTaskId(task),
    })?;
    cursor.steer += 1;
    Ok(id)
}

/// One judged Attempt, `SQLite` side, inside an open transaction: the
/// cursor binds directly (the `sqlite_run/commits.rs` host-counter
/// precedent) — with FKs ON, the UNIQUE live, and the window trigger
/// firing its COUNT probe on every insert.
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
/// commit filling task 0 from its seeded `attempts_per_task` rows to
/// the window's cap of [`WINDOW_CAP`] — every fill row legal, minted
/// through the cursor so both engines stay in lockstep.
///
/// # Errors
///
/// Engine errors, stringified; a cursor-drifted mint, named.
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
    .map_err(|e| format!("law_reject_window setup (engine): {e:?}"))
}

/// The window setup, `SQLite` side: the same fill rows in one
/// `BEGIN IMMEDIATE … COMMIT`, ids from this engine pass's own cursor.
///
/// # Errors
///
/// `SQLite` errors, stringified (the transaction rolls back).
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

/// `law_commit_attempt` on bumbledb: one sample = one `db.write`
/// allocating and inserting one stream Attempt under the FULL roster —
/// the fresh key, the declared `(task, n)` key, the `Attempt(task) <=
/// Task(id)` containment, and the `{0..8}` window all judged per
/// commit. work = 1 per sample.
///
/// # Errors
///
/// Engine errors, stringified; a stream/protocol length mismatch or a
/// cursor-drifted mint, named.
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
            .map(|()| 1)
            .map_err(|e| format!("law_commit_attempt: {e:?}"))
    })
}

/// `law_commit_attempt` on `SQLite`: one sample = one bound `INSERT` on
/// a reused prepared statement inside `BEGIN IMMEDIATE … COMMIT` — FKs
/// ON, the UNIQUE index live, the window trigger firing its COUNT
/// probe (the equivalent enforcement bill, per the map).
///
/// # Errors
///
/// `SQLite` errors, stringified (a failed sample rolls back).
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

/// `law_commit_cluster` on bumbledb: one sample = ONE judged commit of
/// the 4-row cluster — a fresh Attempt (fresh key, declared key, plain
/// containment, window), its Verdict `{ outcome: Accepted }` (declared
/// key, plain containment, closed containment), a fresh Repartition
/// Steer (fresh key, closed containment, plain containment), and its
/// `SteerScope` (declared key, the ψ-selected containment) — every
/// statement family exercised in one final-state judgment. work = 4.
///
/// # Errors
///
/// Engine errors, stringified; a stream/protocol length mismatch or a
/// cursor-drifted mint, named.
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
            tx.insert(&Verdict {
                attempt,
                outcome: Outcome::Accepted.id(),
            })?;
            let steer = mint_steer(tx, op.task, cursor)?;
            tx.insert(&SteerScope {
                steer,
                grp: op.task,
            })?;
            Ok(())
        })
        .map(|()| 4)
        .map_err(|e| format!("law_commit_cluster: {e:?}"))
    })
}

/// `law_commit_cluster` on `SQLite`: the same 4 rows as 4 bound
/// `INSERT`s in one `BEGIN IMMEDIATE … COMMIT`, insert-ordered so the
/// immediate FK and trigger checks pass (Attempt before its Verdict,
/// Steer before its `SteerScope` — `SQLite` checks per statement where
/// the engine judges the final state; for this insert-ordered shape the
/// two disciplines render the same verdict, the module-doc honesty
/// note).
///
/// # Errors
///
/// `SQLite` errors, stringified (a failed sample rolls back).
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

/// Whether the sealed violation set carries a Functionality citation.
fn cites_functionality(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Functionality { .. }))
}

/// Whether the sealed violation set carries a Containment citation.
fn cites_containment(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Containment { .. }))
}

/// Whether the sealed violation set carries a Cardinality citation.
fn cites_cardinality(violations: &bumbledb::Violations) -> bool {
    violations
        .iter()
        .any(|violation| matches!(violation, bumbledb::Violation::Cardinality { .. }))
}

/// Whether the sealed violation set cites the ψ statement itself — a
/// Containment citation on exactly [`psi_statement`].
fn cites_psi(violations: &bumbledb::Violations) -> bool {
    violations.iter().any(|violation| {
        matches!(violation, bumbledb::Violation::Containment { statement, .. }
            if *statement == psi_statement())
    })
}

/// One refused engine commit — the rejection lanes' shared spine. The
/// sample is `Ok(1)` ONLY when the commit comes back
/// `Error::CommitRejected` AND the sealed set carries the expected
/// citation kind; an accepted commit, a rejection citing something
/// else, or any other error kind (a `FreshExhausted` or a shape error
/// would be a lane bug) is an `Err` aborting the run — the wrong fork
/// is unrepresentable as a measurement.
fn refused_commit(
    db: &Db<LawfulWorld>,
    family: &'static str,
    expected: &'static str,
    cites: fn(&bumbledb::Violations) -> bool,
    violate: impl FnOnce(&mut bumbledb::WriteTx<'_, LawfulWorld>) -> bumbledb::Result<()>,
) -> Result<u64, String> {
    match db.write(violate) {
        Ok(()) => Err(format!(
            "{family}: the violating commit was ACCEPTED — the refusal contract is broken"
        )),
        Err(bumbledb::Error::CommitRejected { violations }) => {
            if cites(&violations) {
                Ok(1)
            } else {
                Err(format!(
                    "{family}: rejected without the expected {expected} citation: {:?}",
                    crate::differential::cited(&violations)
                ))
            }
        }
        Err(other) => Err(format!(
            "{family}: expected Error::CommitRejected, the engine said {other:?}"
        )),
    }
}

/// One refused `SQLite` insert — the mirror spine: `BEGIN IMMEDIATE`,
/// the violating `INSERT` on a reused prepared statement (expected to
/// fail with a constraint violation — UNIQUE, FK, or a trigger's
/// `RAISE(ABORT)`), then `ROLLBACK`; the whole round trip, rollback
/// included, is the sample. An accepted insert or a non-constraint
/// error is an `Err` aborting the run.
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

/// `law_reject_key` on bumbledb: every sample offers an Attempt
/// duplicating the seeded `(task 1, n 0)` determinant under an explicit
/// sacrificial id — the commit MUST come back `Error::CommitRejected`
/// with a Functionality citation; anything else aborts the run.
///
/// # Errors
///
/// An accepted commit, a wrong citation, or a non-`CommitRejected`
/// error kind, named.
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
                tx.insert(&Attempt {
                    id,
                    task: LawTaskId(1),
                    n: 0,
                })
                .map(|_| ())
            },
        )
    })
}

/// `law_reject_key` on `SQLite`: the same duplicate `(task 1, n 0)`
/// binding (the identical sacrificial id) expecting the UNIQUE
/// violation, then `ROLLBACK` — the refused round trip is the sample.
///
/// # Errors
///
/// An accepted insert or a non-constraint failure, named.
pub fn reject_key_sqlite(conn: &Connection, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = sql_u64(REJECT_ID_BASE + sample);
        sample += 1;
        refused_insert_sqlite(conn, "law_reject_key", ATTEMPT_INSERT, (id, 1i64, 0i64))
    })
}

/// `law_reject_containment` on bumbledb: every sample offers an Attempt
/// under an ABSENT task (`tasks + 1_000_000`) — `Error::CommitRejected`
/// with a Containment citation, or the run aborts.
///
/// # Errors
///
/// An accepted commit, a wrong citation, or a non-`CommitRejected`
/// error kind, named.
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
                tx.insert(&Attempt {
                    id,
                    task: LawTaskId(absent),
                    n: 0,
                })
                .map(|_| ())
            },
        )
    })
}

/// `law_reject_containment` on `SQLite`: the same absent-task binding
/// expecting the FK violation, then `ROLLBACK`.
///
/// # Errors
///
/// An accepted insert or a non-constraint failure, named.
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
/// 0 to the cap): every sample offers a 9th attempt on task 0 —
/// `Error::CommitRejected` with a Cardinality citation, or the run
/// aborts.
///
/// # Errors
///
/// An accepted commit, a wrong citation, or a non-`CommitRejected`
/// error kind, named.
pub fn reject_window_engine(db: &Db<LawfulWorld>, proto: Protocol) -> Result<Measurement, String> {
    let mut sample = 0u64;
    harness::measure(proto, || {
        let id = LawAttemptId(REJECT_ID_BASE + sample);
        sample += 1;
        refused_commit(
            db,
            "law_reject_window",
            "Cardinality",
            cites_cardinality,
            |tx| {
                tx.insert(&Attempt {
                    id,
                    task: LawTaskId(0),
                    n: WINDOW_CAP,
                })
                .map(|_| ())
            },
        )
    })
}

/// `law_reject_window` on `SQLite`: the same 9th-attempt binding
/// expecting the window trigger's `RAISE(ABORT)`, then `ROLLBACK`.
///
/// # Errors
///
/// An accepted insert or a non-constraint failure, named.
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

/// `law_reject_scope` on bumbledb: every sample offers a `SteerScope`
/// under the seeded steer 0 — an EVEN, Observe-kind steer the ψ
/// selection excludes (`SteerScope` carries no fresh field, so nothing
/// burns here) — `Error::CommitRejected` with a Containment citation on
/// the ψ statement itself ([`psi_statement`]), or the run aborts.
///
/// # Errors
///
/// An accepted commit, a wrong citation, or a non-`CommitRejected`
/// error kind, named.
pub fn reject_scope_engine(db: &Db<LawfulWorld>, proto: Protocol) -> Result<Measurement, String> {
    harness::measure(proto, || {
        refused_commit(
            db,
            "law_reject_scope",
            "ψ-statement Containment",
            cites_psi,
            |tx| {
                tx.insert(&SteerScope {
                    steer: LawSteerId(0),
                    grp: 0,
                })
                .map(|_| ())
            },
        )
    })
}

/// `law_reject_scope` on `SQLite`: the same Observe-steer scope binding
/// expecting the ψ trigger's `RAISE(ABORT)`, then `ROLLBACK`.
///
/// # Errors
///
/// An accepted insert or a non-constraint failure, named.
pub fn reject_scope_sqlite(conn: &Connection, proto: Protocol) -> Result<Measurement, String> {
    harness::measure(proto, || {
        refused_insert_sqlite(conn, "law_reject_scope", SCOPE_INSERT, (0i64, 0i64))
    })
}
