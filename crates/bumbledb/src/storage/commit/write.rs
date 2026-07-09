use std::collections::BTreeMap;

use crate::error::{CorruptionError, Error, Result};
use crate::obs;
use crate::schema::RelationId;
use crate::storage::delta::WriteDelta;
use crate::storage::env::{Environment, WriteTxn};
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

use super::{apply, judgment, Applied, CommitReport};

/// The full commit (docs/architecture/50-storage.md): apply (phases 1-2),
/// the judgment phase (phase 3 — containment source and target sides),
/// counter flush (phase 4), LMDB commit (phase 5). Any error anywhere
/// aborts — nothing persists.
///
/// # Errors
///
/// `FunctionalityViolation` on a key statement violated by the final
/// state; `ContainmentViolation` on a containment statement the final
/// state violates — a source left without its target, or a deleted
/// target key a surviving source still requires; `Lmdb`/`Corruption`
/// on storage failure.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema shapes).
pub fn commit(delta: WriteDelta<'_>, env: &Environment) -> Result<CommitReport> {
    // The empty delta is the *only* no-op commit shape — net dispositions
    // make every recorded entry a genuine state change. It commits without
    // touching query-visible state: the tx id does not advance and no
    // cached image is invalidated. But a *successful* commit persists
    // every serial value it issued — the closure may have returned those
    // ids to the host — so dirty `Q` marks flush even here
    // (`flush_escaped_serials`). Pending interns are still dropped: intern
    // ids never escape (hosts see values, not words), and re-issuing an
    // unflushed provisional id is the established abort semantics.
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
    let Applied {
        mut txn,
        delta,
        row_id_next,
        deleted_guards,
        inserted_guards,
        selections,
    } = apply(delta, env)?;

    // Phase 3, the judgment phase: final-state probes inside this same
    // write transaction (LMDB write txns read their own writes) — the
    // containment source side over inserted facts, then the target side
    // over the disestablished guard tuples
    // (`deleted_guards − inserted_guards`).
    judgment::check_source(&txn, env.data(), &delta, &selections)?;
    judgment::check_target(
        &txn,
        env.data(),
        &delta,
        &selections,
        &deleted_guards,
        &inserted_guards,
    )?;

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
