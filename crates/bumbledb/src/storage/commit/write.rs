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
use super::{CommitReport, apply, judgment};

/// With the 10 ms-doubling backoff the worst case adds 70 ms before the typed
/// error escapes.
const COMMIT_SYNC_RETRIES: u32 = 3;

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

/// # Errors
/// # Panics
/// Only on programmer-invariant violations (validated-schema shapes).
#[expect(
    clippy::needless_pass_by_value,
    reason = "consuming the delta is the commit boundary contract"
)] 
pub fn commit(delta: WriteDelta<'_>, env: &Environment) -> Result<Admission<CommitReport>> {

    // irrelevant (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).

    if delta.is_empty() {
        obs::event(obs::names::COMMIT_NOOP, obs::TraceArgs::None);

        // `commit()`, the write region's drop guard has disarmed and

        // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
        flush_escaped_fresh_ids(env, &delta)?;
        let generation = {
            let rtxn = env.read_txn()?;
            rtxn.generation()?
        };
        return Ok(Admission::Accepted(CommitReport::Noop { generation }));
    }

    let mut commit_span = obs::span(obs::names::COMMIT);

    // pure function of (delta, schema) before the write lock. Selection

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
            let judged = match applied.judge(&plan)? {
                Admission::Rejected(violations) => {
                    return Ok(Admission::Rejected(violations));
                }
                Admission::Accepted(judged) => judged,
            };
            Ok(Admission::Accepted(judged.finish(&delta, env)?))
        })
    })();

    // (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`; the

    let flush = match &outcome {
        Ok(Admission::Accepted(_)) => Ok(()),
        Ok(Admission::Rejected(_)) | Err(_) => flush_escaped_fresh_ids(env, &delta),
    };

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

/// # Errors
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

/// Raises the in-process high-water before the disk write so a failed flush
/// cannot rewind `reserve` in this process.
pub(crate) fn flush_escaped_fresh_ids(env: &Environment, delta: &WriteDelta<'_>) -> Result<()> {
    env.note_escaped_fresh(delta.dirty_fresh_marks());
    let mut marks = env.take_pending_fresh_flush();
    for (rel, field, next) in delta.dirty_fresh_marks() {
        marks.join(rel, field, next);
    }
    persist_or_park(env, marks)
}

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
