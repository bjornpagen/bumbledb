use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::obs;
use crate::storage::delta::{Disposition, WriteDelta};
use crate::storage::env::Environment;
use crate::storage::keys::MAX_KEY;

use super::{Applied, Applier};

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
    let txn = env.write_txn()?;
    let mut applier = Applier {
        txn,
        data: env.data(),
        changed: false,
        row_id_next: BTreeMap::new(),
        deleted_guards: BTreeSet::new(),
        inserted_guards: BTreeSet::new(),
        key: [0; MAX_KEY],
        guard: Vec::new(),
    };

    // Phase 1: all deletes, then phase 2: all inserts — the canonical order
    // that makes user operation order semantically irrelevant. Counts read
    // from the delta's own disposition entries — no new tallies.
    {
        let mut deletes = 0u64;
        let mut span = obs::span(obs::names::APPLY_DELETES, obs::Category::Commit);
        for (rel, fact_bytes, disposition) in delta.entries() {
            if disposition == Disposition::Delete {
                deletes += 1;
                applier.delete_fact(delta.schema(), rel, fact_bytes)?;
            }
        }
        span.set_args(deletes, 0);
    }
    {
        let mut inserts = 0u64;
        let mut span = obs::span(obs::names::APPLY_INSERTS, obs::Category::Commit);
        for (rel, fact_bytes, disposition) in delta.entries() {
            if disposition == Disposition::Insert {
                inserts += 1;
                applier.insert_fact(delta.schema(), rel, fact_bytes)?;
            }
        }
        span.set_args(inserts, 0);
    }

    let Applier {
        txn,
        changed,
        row_id_next,
        deleted_guards,
        inserted_guards,
        ..
    } = applier;
    Ok(Applied {
        txn,
        delta,
        changed,
        row_id_next,
        deleted_guards,
        inserted_guards,
    })
}
