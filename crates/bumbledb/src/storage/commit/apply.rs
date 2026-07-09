use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::obs;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::MAX_KEY;

use super::{judgment, Applied, Applier};

/// Applies the delta to LMDB in canonical order: phase 1 all deletes, then
/// phase 2 all inserts. Opens the LMDB write transaction here — nothing
/// touched a data page before this call (the 50-storage doc's lock-window
/// rule).
///
/// # Errors
///
/// `FunctionalityViolation` when two live facts claim one key — the same
/// guard (scalar) or overlapping intervals in one scalar-prefix group
/// (pointwise); `Lmdb` on storage failure; `Corruption` on malformed base
/// state. On any error the transaction is dropped — nothing persists.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema shapes).
pub fn apply<'env, 's>(delta: WriteDelta<'s>, env: &'env Environment) -> Result<Applied<'env, 's>> {
    // Selection literals encode once per commit, before the write lock:
    // the resolution reads only the committed dictionary (frozen for the
    // single writer) plus the delta's pending interns.
    let selections = {
        let view = env.read_txn()?;
        judgment::Selections::encode(&delta, &view)?
    };
    let txn = env.write_txn()?;
    let mut applier = Applier {
        txn,
        data: env.data(),
        row_id_next: BTreeMap::new(),
        deleted_guards: BTreeSet::new(),
        inserted_guards: BTreeSet::new(),
        selections,
        key: [0; MAX_KEY],
        guard: Vec::new(),
    };

    // Phase 1: all deletes, then phase 2: all inserts — the canonical order
    // that makes user operation order semantically irrelevant. Every entry
    // applies: the delta's net dispositions were proved against committed
    // state at op time. Counts read from the delta's own entries — no new
    // tallies.
    {
        let mut deletes = 0u64;
        let mut span = obs::span(obs::names::APPLY_DELETES, obs::Category::Commit);
        for (rel, fact_bytes) in delta.deletes() {
            deletes += 1;
            applier.delete_fact(delta.schema(), rel, fact_bytes)?;
        }
        span.set_args(deletes, 0);
    }
    {
        let mut inserts = 0u64;
        let mut span = obs::span(obs::names::APPLY_INSERTS, obs::Category::Commit);
        for (rel, fact_bytes) in delta.inserts() {
            inserts += 1;
            applier.insert_fact(delta.schema(), rel, fact_bytes)?;
        }
        span.set_args(inserts, 0);
    }

    let Applier {
        txn,
        row_id_next,
        deleted_guards,
        inserted_guards,
        selections,
        ..
    } = applier;
    Ok(Applied {
        txn,
        delta,
        row_id_next,
        deleted_guards,
        inserted_guards,
        selections,
    })
}
