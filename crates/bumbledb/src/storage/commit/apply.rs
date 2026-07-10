use std::collections::BTreeMap;

use crate::error::Result;
use crate::obs;
use crate::storage::env::Environment;
use crate::storage::keys::MAX_KEY;

use super::plan::CommitPlan;
use super::{Applied, Applier};

/// Executes the plan against LMDB in canonical order: phase 1 all deletes,
/// then phase 2 all inserts. Opens the LMDB write transaction here —
/// nothing touched a data page before this call (the 50-storage doc's
/// lock-window rule), and the plan derivation already happened outside it.
/// A dumb executor by construction: every key byte and probe marker comes
/// from the plan; only the row-id plumbing and the desync/neighbor probes
/// live here, because ids and probe results are not derivable.
///
/// # Errors
///
/// `FunctionalityViolation` when two live facts claim one key — the same
/// guard (scalar) or overlapping intervals in one scalar-prefix group
/// (pointwise); `Lmdb` on storage failure; `Corruption` on base state
/// disagreeing with what the plan proved. On any error the transaction is
/// dropped — nothing persists.
pub fn apply<'env>(plan: &CommitPlan<'_>, env: &'env Environment) -> Result<Applied<'env>> {
    let txn = env.write_txn()?;
    let mut applier = Applier {
        txn,
        data: env.data(),
        row_id_next: BTreeMap::new(),
        key: [0; MAX_KEY],
    };

    {
        let mut span = obs::span(obs::names::APPLY_DELETES, obs::Category::Commit);
        for op in &plan.deletes {
            applier.delete_fact(op)?;
        }
        span.set_args(plan.deletes.len() as u64, 0);
    }
    {
        let mut span = obs::span(obs::names::APPLY_INSERTS, obs::Category::Commit);
        for op in &plan.inserts {
            applier.insert_fact(op)?;
        }
        span.set_args(plan.inserts.len() as u64, 0);
    }

    Ok(Applied {
        txn: applier.txn,
        row_id_next: applier.row_id_next,
    })
}
