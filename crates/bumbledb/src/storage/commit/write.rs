use std::collections::BTreeMap;

use crate::error::{
    Admission, CitedCitations, CorruptionError, Error, Result, Violation, Violations,
};
use crate::obs;
use crate::storage::delta::WriteDelta;
use crate::storage::env::{Environment, FreshMarks, WriteTxn};
use crate::storage::keys::{self, StatKind};
use bumbledb_theory::schema::RelationId;

use super::plan::plan_commit;
use super::{CommitReport, apply, crashpoint, judgment};

/// The bound on [`commit_bounded`]'s retries of the transient
/// commit-sync class — a decision, not a knob. With the 10 ms-doubling
/// backoff the worst case adds 70 ms before the typed error escapes.
const COMMIT_SYNC_RETRIES: u32 = 3;

/// Bounded, observable retry of the durability boundary (PRD 22 ruling).
/// `mdb_txn_commit` aborts its transaction on failure — nothing
/// persisted — so `attempt` rebuilds and re-commits the whole
/// transaction; its inputs are immutable (the plan, the delta) and
/// committed state is stable under the single-writer mutex, so every
/// re-run writes the same bytes. Only the transient sync class retries
/// ([`Error::CommitSync`]: a raw errno out of the commit's write/sync
/// syscalls — on macOS `fcntl(F_FULLFSYNC)` has been observed failing
/// transiently under I/O pressure, and `mdb.c` surfaces the errno raw
/// with no fallback sync); every other error escapes on the first
/// throw. Each retry is an obs event (`COMMIT_SYNC_RETRY`), never
/// silent, and the escaping error carries the count. The durability
/// contract is untouched: a retry re-runs the full write-and-sync, so
/// every commit that reports success fsynced — no mode was born.
///
/// Dead end, recorded per PRD 22: `mdb_env_set_mapsize` racing readers
/// is eliminated — the map size (`MAP_SIZE`, one constant for both
/// kinds, still not a knob) is set once at open and no resize call
/// exists to race.
pub(super) fn commit_bounded<T>(mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
    let mut retries = 0u32;
    loop {
        match attempt() {
            Err(Error::CommitSync { error, .. }) => {
                if retries == COMMIT_SYNC_RETRIES {
                    return Err(Error::CommitSync { retries, error });
                }
                retries += 1;
                obs::event(
                    obs::names::COMMIT_SYNC_RETRY,
                    obs::TraceArgs::Pair(
                        u64::from(retries),
                        error
                            .raw_os_error()
                            .map_or(0, |code| u64::from(code.unsigned_abs())),
                    ),
                );
                std::thread::sleep(std::time::Duration::from_millis(10 << (retries - 1)));
            }
            other => return other,
        }
    }
}

/// The full commit (docs/architecture/50-storage.md): plan derivation
/// (the pure function of the delta), apply (phases 1-2), the judgment
/// phase (phase 3 — containment source and target sides), counter flush
/// (phase 4), LMDB commit (phase 5). Any error anywhere aborts — nothing
/// persists. Phases 1-5 run under [`commit_bounded`]: a transient
/// commit-sync failure rebuilds the transaction and retries, bounded and
/// observable.
///
/// # Errors
///
/// `Admission::Rejected` on a final state violating the theory, carrying the
/// COMPLETE violation set in materialized statement order: every
/// violated key statement (phase 2, which preempts the judgment), or
/// every violated containment statement — a source left without its
/// target, or a deleted target key a surviving source still requires
/// (`docs/architecture/30-dependencies.md` § judged on final states).
/// `CommitSync` on a durability-boundary failure that survived the
/// bounded retry; `Lmdb`/`Corruption` on storage failure.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema shapes).
#[expect(
    clippy::needless_pass_by_value,
    reason = "consuming the delta is the commit boundary contract"
)] // consuming the delta IS the contract: a commit ends it
pub fn commit(delta: WriteDelta<'_>, env: &Environment) -> Result<Admission<CommitReport>> {
    // The empty delta is the *only* no-op commit shape — net dispositions
    // make every recorded entry a genuine state change. It commits without
    // touching query-visible state: the tx id does not advance and no
    // cached image is invalidated. But every commit persists the fresh
    // values it issued — the closure may have returned those ids to the
    // host — so dirty `Q` marks flush even here (`flush_escaped_fresh_ids`),
    // exactly as an aborted commit now burns its escaped ids: a fresh
    // value, once issued, is never re-issued, the transaction's fate
    // irrelevant (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
    // Pending interns are still dropped — intern ids never escape (hosts
    // see values, not words), so recycling an unflushed provisional intern
    // id is invisible.
    if delta.is_empty() {
        obs::event(obs::names::COMMIT_NOOP, obs::TraceArgs::None);
        // Incremental empty-delta no-op. Sound iff the base already
        // satisfies the theory (`State.models`). This is not complete
        // admission: an empty [`IncrementalObligations`] roster would
        // also accept, which misses closed-source containments, floored
        // empty capacity groups, and every other untouched obligation.
        // Format 8 create complete-admits first, so this shortcut is
        // legal on that admitted base. Ordinary open refuses every
        // earlier format; this arm never blesses a format-7 store.
        // InstanceBuilder / create use
        // [`crate::schema::CompleteObligations`], never this path.
        //
        // The burn precedes the generation read: once the delta reaches
        // `commit()`, the write region's drop guard has disarmed and
        // commit owns the flush on EVERY termination — a generation read
        // failing transiently (readers full, any Lmdb error) must not
        // skip past the escaped marks, or the next transaction re-issues
        // an id the host already holds
        // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
        flush_escaped_fresh_ids(env, &delta)?;
        let generation = {
            let rtxn = env.read_txn()?;
            rtxn.generation()?
        };
        return Ok(Admission::Accepted(CommitReport::Noop { generation }));
    }

    crashpoint!("after-staging");
    let mut commit_span = obs::span(obs::names::COMMIT);
    // The plan: every derivable key byte and check set, computed as a
    // pure function of (delta, schema) before the write lock. Selection
    // literals encode once per commit here — the resolution reads only
    // the committed dictionary (frozen for the single writer) plus the
    // delta's pending interns. The plan block runs INSIDE `outcome` so
    // its infra errors (the snapshot read, a corrupt dictionary forward
    // id) share the burn exit below with `commit_bounded`'s — commit
    // owns the flush on every termination it owns
    // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
    let schema = delta.schema();
    let outcome = (|| {
        let plan = {
            let view = env.read_txn()?;
            let selections = judgment::Selections::encode(&delta, &view)?;
            plan_commit(&delta, selections)?
        };
        commit_bounded(|| {
            let applied = match apply(&plan, env)? {
                Admission::Rejected(violations) => {
                    return Ok(Admission::Rejected(violations));
                }
                Admission::Accepted(applied) => applied,
            };
            crashpoint!("before-judgment");
            let judged = match applied.judge(&plan)? {
                Admission::Rejected(violations) => {
                    return Ok(Admission::Rejected(violations));
                }
                Admission::Accepted(judged) => judged,
            };
            Ok(Admission::Accepted(judged.finish(&delta, env)?))
        })
    })();
    // The never-reissue law spans the abort: every aborted attempt still
    // handed the host its mints — the closure returned them from `reserve`,
    // and a rejection carries the offending facts back as data — so the
    // escaped `Q` high-water burns regardless of the abort's shape
    // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`; the
    // counters-only commit writes exactly the dirty marks — no generation
    // bump, no cache advance). The in-process floor is raised even when
    // the disk write fails; a flush failure is parked and retried at the
    // next write begin. A sealed rejection is never replaced by
    // that flush error (or by a later decoration failure).
    let flush = match &outcome {
        Ok(Admission::Accepted(_)) => Ok(()),
        Ok(Admission::Rejected(_)) | Err(_) => flush_escaped_fresh_ids(env, &delta),
    };
    // The one rejection exit: every theory rejection — phase 2's key
    // set, phase 3's containment/capacity set — passes here, so the cited
    // facts decode here, ONCE, while the delta's provisional intern ids
    // are still resolvable: the abort burned its escaped *fresh* ids but
    // never its interns (intern ids never escape — hosts see values, not
    // words), so a later decode would still misread a novel `str` field
    // as a dangling id (`docs/architecture/30-dependencies.md` §
    // rendering the rejection). Decoration is best-effort: a `read_txn`
    // or decode failure leaves the sealed set undecoded rather than
    // converting a Rejected into an Err.
    let report = match outcome {
        Ok(Admission::Rejected(violations)) => {
            let violations = decorate_rejected(violations, schema, env, &delta);
            return Ok(Admission::Rejected(violations));
        }
        Err(other) => {
            return Err(match flush {
                Err(flush_err) => flush_err,
                Ok(()) => other,
            });
        }
        Ok(Admission::Accepted(report)) => {
            env.clear_pending_fresh_flush();
            report
        }
    };
    commit_span.set_flag(true);
    Ok(Admission::Accepted(report))
}

/// Decodes cited facts for a sealed rejection. Any secondary failure
/// (`ReadersFull`, `Corruption`, LMDB) returns the undecoded set — the
/// sealed citations are the contract, not the decoration.
fn decorate_rejected(
    violations: Violations,
    schema: &crate::schema::Schema,
    env: &Environment,
    delta: &WriteDelta<'_>,
) -> Violations {
    match env.read_txn() {
        Ok(view) => match decode_cited_facts(&violations, schema, &view, delta) {
            Ok(pairs) => Violations::from_pairs(pairs),
            Err(_) => violations,
        },
        Err(_) => violations,
    }
}

/// Decodes every citation's offending fact bytes into owned
/// [`CitedFact`] values — relation resolved through the violated
/// statement (a key's own relation; a containment's SOURCE, because the
/// judgment speaks about sources; a capacity statement's TARGET, the convicted
/// parent), `str` fields resolved pending-first through the rejecting
/// delta, then the committed dictionary.
///
/// # Errors
///
/// `Corruption` on undecodable fact bytes or a genuinely dangling intern
/// id (pending and committed both miss); `Lmdb` on dictionary reads. The
/// caller keeps the sealed rejection when this fails.
fn decode_cited_facts(
    violations: &Violations,
    schema: &crate::schema::Schema,
    view: &crate::storage::env::ReadTxn<'_>,
    delta: &WriteDelta<'_>,
) -> Result<CitedCitations> {
    use crate::error::CitedFact;
    let mut cited: Vec<(Violation, Box<[CitedFact]>)> = Vec::with_capacity(violations.len());
    for (violation, _) in violations.as_slice() {
        let (relation, facts): (_, Vec<&[u8]>) = match violation {
            Violation::Functionality { .. } => {
                let crate::schema::StatementView::Key(_, key) =
                    schema.statement(violation.statement_id(schema))
                else {
                    unreachable!("a Functionality citation names a key statement");
                };
                (
                    key.relation,
                    std::iter::once(violation.fact())
                        .chain(violation.incumbent())
                        .collect(),
                )
            }
            Violation::Containment { fact, .. } => {
                let crate::schema::StatementView::Containment(_, containment) =
                    schema.statement(violation.statement_id(schema))
                else {
                    unreachable!("a Containment citation names a containment statement");
                };
                (containment.source.relation, vec![fact.as_ref()])
            }
            Violation::Capacity { fact, .. } => {
                let crate::schema::StatementView::Capacity(_, capacity) =
                    schema.statement(violation.statement_id(schema))
                else {
                    unreachable!("a Capacity citation names a capacity statement");
                };
                (capacity.target.relation, vec![fact.as_ref()])
            }
        };
        let layout = schema.relation(relation).layout();
        let expected = layout.fact_width();
        let decoded = facts
            .into_iter()
            .map(|bytes| {
                if bytes.len() != expected {
                    return Err(Error::Corruption(CorruptionError::MalformedValue(
                        "cited fact width",
                    )));
                }
                let values = crate::encoding::decode_values(layout.encoded(bytes), |id| {
                    let id = crate::encoding::InternId::from_raw(id);
                    let raw = match delta.pending_raw(id) {
                        Some(raw) => raw,
                        None => crate::storage::dict::resolve(view, id)?,
                    };
                    std::str::from_utf8(raw)
                        .map(Box::from)
                        .map_err(|_| Error::Corruption(CorruptionError::NonUtf8Intern(id.raw())))
                })?;
                Ok(CitedFact::new(
                    relation,
                    layout.field_count(),
                    values.into_boxed_slice(),
                ))
            })
            .collect::<Result<Box<[CitedFact]>>>()?;
        cited.push((violation.clone(), decoded));
    }
    Ok(cited.into_boxed_slice())
}

/// The counters-only commit of a successful no-op write: exactly the
/// dirty `Q` marks — no generation bump, no cache advance, no intern
/// flush, no dict next-id. Sound because the generation identifies
/// *query-visible* state (`F`/`M`/`U`/`R`) and `Q` marks are write-path
/// bookkeeping no query reads: every image, memo, and cache key stays
/// valid, and the tx-id-advances-iff-data-changed rule is untouched.
/// With no dirty marks no transaction begins — LMDB sees nothing. The
/// same [`commit_bounded`] durability boundary as the full commit: one
/// mechanism, two callers. Raises the in-process high-water before the
/// disk write so a failed flush cannot rewind `reserve` in this process.
pub(crate) fn flush_escaped_fresh_ids(env: &Environment, delta: &WriteDelta<'_>) -> Result<()> {
    env.note_escaped_fresh(delta.dirty_fresh_marks());
    let mut marks = env.take_pending_fresh_flush();
    for (rel, field, next) in delta.dirty_fresh_marks() {
        marks.join(rel, field, next);
    }
    persist_or_park(env, marks)
}

/// Retries a parked escaped-id burn. Called at write begin: a still-failing
/// flush returns the error and leaves the store poisoned for `reserve`
/// until `Q` is durable.
pub(crate) fn flush_pending_escaped_fresh_ids(env: &Environment) -> Result<()> {
    persist_or_park(env, env.take_pending_fresh_flush())
}

fn persist_or_park(env: &Environment, marks: FreshMarks) -> Result<()> {
    if marks.is_empty() {
        return Ok(());
    }
    match persist_fresh_marks(env, &marks) {
        Ok(()) => Ok(()),
        Err(error) => {
            env.park_fresh_flush(marks);
            Err(error)
        }
    }
}

fn persist_fresh_marks(env: &Environment, marks: &FreshMarks) -> Result<()> {
    #[cfg(test)]
    if env.consume_fresh_flush_failure() {
        return Err(Error::Lmdb(crate::error::LmdbFailure::Mdb(
            heed::MdbError::MapFull,
        )));
    }
    commit_bounded(|| {
        let mut txn = env.write_txn()?;
        let data = txn.env().data();
        let mut count = 0u64;
        let mut span = obs::span(obs::names::COUNTERS_FLUSH);
        for ((rel, field), next) in marks.iter() {
            let key = keys::fresh_key(rel, field);
            data.put(txn.raw_mut(), &key, next.to_le_bytes().as_slice())?;
            count += 1;
        }
        span.set_count(count);
        span.end();
        let _s = obs::span(obs::names::LMDB_COMMIT);
        txn.commit()
    })
}

/// Phase 4: folds row-count deltas into `S`, writes row-id high-waters,
/// fresh next-values (`Q`), pending dictionary entries, and the
/// dictionary next-id. Parked escaped-id burns merge into the `Q` puts
/// so a later successful commit makes them durable.
pub(super) fn flush_counters(
    txn: &mut WriteTxn<'_>,
    delta: &WriteDelta<'_>,
    row_id_next: &BTreeMap<RelationId, u64>,
    env: &Environment,
) -> Result<()> {
    let data = txn.env().data();
    for (rel, count_delta) in delta.row_count_deltas() {
        if count_delta == 0 {
            continue;
        }
        let key = keys::stat_key(rel, StatKind::RowCount);
        let current = match data.get(txn.raw(), &key)? {
            Some(bytes) => crate::storage::stored_u64(bytes, "S row count")?,
            None => 0,
        };
        let updated = match current.checked_add_signed(count_delta) {
            Some(n) => n,
            None if count_delta < 0 => {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "S row count underflow",
                )));
            }
            None => {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "S row count overflow",
                )));
            }
        };
        data.put(txn.raw_mut(), &key, updated.to_le_bytes().as_slice())?;
        crashpoint!("mid-write-s");
    }
    for (rel, next) in row_id_next {
        let key = keys::stat_key(*rel, StatKind::RowIdHighWater);
        data.put(txn.raw_mut(), &key, next.to_le_bytes().as_slice())?;
    }
    let mut q_marks = FreshMarks::default();
    for (rel, field, next) in delta.fresh_marks() {
        q_marks.join(rel, field, next);
    }
    q_marks.join_all(env.peek_pending_fresh_flush());
    for ((rel, field), next) in q_marks {
        let key = keys::fresh_key(rel, field);
        data.put(txn.raw_mut(), &key, next.to_le_bytes().as_slice())?;
    }
    if let Some(interns) = delta.interns() {
        for (raw, id) in interns.entries() {
            crate::storage::dict::put_pending(txn, raw, id)?;
        }
        txn.put_dict_next_id(interns.next_id())?;
    }
    Ok(())
}
