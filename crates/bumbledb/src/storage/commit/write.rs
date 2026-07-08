use std::collections::BTreeMap;

use crate::error::{CorruptionError, Error, FkViolation, Result};
use crate::obs;
use crate::schema::RelationId;
use crate::storage::delta::WriteDelta;
use crate::storage::env::{Environment, WriteTxn};
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

use super::restrict::check_restrict;
use super::{apply, Applied, CommitReport};

/// The full commit (docs/architecture/40-storage.md): apply (phases 1-2), FK validation against the
/// final state (phase 3), counter flush (phase 4), LMDB commit (phase 5).
/// Any error anywhere aborts — nothing persists.
///
/// # Errors
///
/// `UniqueViolation`/`ForeignKeyViolation` on constraint violations in the
/// final state; `Lmdb`/`Corruption` on storage failure.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths,
/// well-formed R keys this same commit wrote).
pub fn commit(delta: WriteDelta<'_>, env: &Environment) -> Result<CommitReport> {
    // An all-no-op delta commits without touching query-visible state:
    // the tx id does not advance and no cached image is invalidated. But
    // a *successful* commit persists every serial value it issued — the
    // closure may have returned those ids to the host — so dirty `Q`
    // marks flush even here (`flush_escaped_serials`). Pending interns
    // are still dropped: intern ids never escape (hosts see values, not
    // words), and re-issuing an unflushed provisional id is the
    // established abort semantics.
    if delta.is_empty() {
        obs::event(obs::names::COMMIT_NOOP, obs::Category::Commit, 0, 0);
        let generation = {
            let rtxn = env.read_txn()?;
            rtxn.generation()?
        };
        flush_escaped_serials(env.write_txn()?, &delta)?;
        return Ok(CommitReport {
            changed: false,
            new_generation: generation,
        });
    }

    let mut commit_span = obs::span(obs::names::COMMIT, obs::Category::Commit);
    let applied = apply(delta, env)?;
    if !applied.changed {
        obs::event(obs::names::COMMIT_NOOP, obs::Category::Commit, 0, 0);
        let generation = applied.txn.generation()?;
        // Nothing was applied (every disposition was a base no-op), so
        // the open transaction is clean: reuse it for the escaped
        // serials, or abort it if none are dirty.
        flush_escaped_serials(applied.txn, &applied.delta)?;
        return Ok(CommitReport {
            changed: false,
            new_generation: generation,
        });
    }
    let Applied {
        mut txn,
        delta,
        row_id_next,
        deleted_guards,
        inserted_guards,
        fk_probes,
        ..
    } = applied;
    let data = env.data();
    let mut key: KeyBuf = [0; MAX_KEY];

    // Phase 3a: forward FK validation — every inserted fact's targets must
    // resolve in the final state (the write txn reads its own writes).
    let forward_span = obs::span_args(
        obs::names::FK_FORWARD,
        obs::Category::Commit,
        fk_probes.len() as u64,
        0,
    );
    for ((target_relation, target_constraint, guard), probe) in &fk_probes {
        let u_len = keys::unique_key(&mut key, *target_relation, *target_constraint, guard);
        if data.get(txn.raw(), &key[..u_len])?.is_none() {
            return Err(Error::ForeignKeyViolation {
                relation: probe.source_relation,
                constraint: probe.source_constraint,
                violation: FkViolation::MissingTarget {
                    fact_bytes: probe.fact_bytes.clone().into_boxed_slice(),
                },
            });
        }
    }

    forward_span.end();

    check_restrict(&txn, data, &mut key, &deleted_guards, &inserted_guards)?;

    // Phase 4: counters — row counts, row-id high-waters, serial sequences,
    // pending dictionary entries and the dictionary next-id.
    {
        let mut span = obs::span(obs::names::COUNTERS_FLUSH, obs::Category::Commit);
        let interns = delta.pending_interns().count() as u64;
        flush_counters(&mut txn, env, &delta, &row_id_next)?;
        span.set_args(interns, 0);
    }

    // The storage tx id advances exactly once per state-changing commit.
    let new_generation = txn.generation()? + 1;
    txn.put_generation(new_generation)?;

    // Phase 5: LMDB commit (fsync per environment defaults) — the
    // fsync-bound number, isolated.
    {
        let _s = obs::span(obs::names::LMDB_COMMIT, obs::Category::Commit);
        txn.commit()?;
    }
    commit_span.set_args(1, 0);
    Ok(CommitReport {
        changed: true,
        new_generation,
    })
}

/// The counters-only commit of a successful no-op write: exactly the
/// dirty `Q` marks — no generation bump, no image eviction, no intern
/// flush, no dict next-id. Sound because the generation identifies
/// *query-visible* state (`F`/`M`/`U`/`R`) and `Q` marks are write-path
/// bookkeeping no query reads: every image, memo, and cache key stays
/// valid, and the tx-id-advances-iff-data-changed rule is untouched.
/// With no dirty marks the transaction aborts — LMDB sees nothing.
fn flush_escaped_serials(mut txn: WriteTxn<'_>, delta: &WriteDelta<'_>) -> Result<()> {
    if delta.dirty_serial_marks().next().is_none() {
        txn.abort();
        return Ok(());
    }
    let data = txn.env().data();
    let mut key: KeyBuf = [0; MAX_KEY];
    let mut marks = 0u64;
    let mut span = obs::span(obs::names::COUNTERS_FLUSH, obs::Category::Commit);
    for (rel, field, next) in delta.dirty_serial_marks() {
        let len = keys::serial_key(&mut key, rel, field);
        data.put(txn.raw_mut(), &key[..len], next.to_le_bytes().as_slice())?;
        marks += 1;
    }
    span.set_args(0, marks);
    span.end();
    let _s = obs::span(obs::names::LMDB_COMMIT, obs::Category::Commit);
    txn.commit()
}

/// Phase 4: folds row-count deltas into `S`, writes row-id high-waters,
/// serial next-values (`Q`), pending dictionary entries, and the
/// dictionary next-id.
fn flush_counters(
    txn: &mut WriteTxn<'_>,
    env: &Environment,
    delta: &WriteDelta<'_>,
    row_id_next: &BTreeMap<RelationId, u64>,
) -> Result<()> {
    let data = env.data();
    let mut key: KeyBuf = [0; MAX_KEY];
    for (rel, count_delta) in delta.row_count_deltas() {
        if count_delta == 0 {
            continue;
        }
        let len = keys::stat_key(&mut key, rel, StatKind::RowCount);
        let current =
            match data.get(txn.raw(), &key[..len])? {
                Some(bytes) => u64::from_le_bytes(bytes.try_into().map_err(|_| {
                    Error::Corruption(CorruptionError::MalformedValue("S row count"))
                })?),
                None => 0,
            };
        let updated = current
            .checked_add_signed(count_delta)
            .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                "S row count underflow",
            )))?;
        data.put(txn.raw_mut(), &key[..len], updated.to_le_bytes().as_slice())?;
    }
    for (rel, next) in row_id_next {
        let len = keys::stat_key(&mut key, *rel, StatKind::RowIdHighWater);
        data.put(txn.raw_mut(), &key[..len], next.to_le_bytes().as_slice())?;
    }
    for (rel, field, next) in delta.serial_marks() {
        let len = keys::serial_key(&mut key, rel, field);
        data.put(txn.raw_mut(), &key[..len], next.to_le_bytes().as_slice())?;
    }
    for (tag, raw, id) in delta.pending_interns() {
        crate::storage::dict::put_pending(txn, tag, raw, id)?;
    }
    if let Some(dict_next) = delta.dict_next() {
        txn.put_dict_next_id(dict_next)?;
    }
    Ok(())
}
