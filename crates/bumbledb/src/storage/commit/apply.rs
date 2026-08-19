use std::collections::BTreeMap;

use crate::error::{Admission, Result, Violations};
use crate::obs;
use crate::storage::env::Environment;
use crate::storage::keys::MAX_KEY;

use super::plan::CommitPlan;
use super::{Applied, Applier};

/// Executes the plan against the catalog in canonical order: phase 1 all
/// deletes, then phase 2 all inserts. Opens the LMDB write transaction
/// here — nothing touched a data page before this call (the 50-storage
/// doc's lock-window rule), and the plan derivation already happened
/// outside it. A dumb executor by construction: every key byte and probe
/// marker comes from the plan; only the row-id plumbing and the
/// desync/neighbor probes live here, because ids and probe results are
/// not derivable.
///
/// # Errors
///
/// `Admission::Rejected` when two live facts claim one key — the same determinant
/// (scalar) or overlapping intervals in one scalar-prefix group
/// (pointwise) — carrying the COMPLETE set of violated key statements:
/// conflicts record and phase 2 finishes the scan before the rejection
/// seals (`docs/architecture/30-dependencies.md` § judged on final
/// states). `Lmdb` on storage failure; `Corruption` on base state
/// disagreeing with what the plan proved. On any error the transaction is
/// dropped — nothing persists.
pub fn apply<'env>(
    plan: &CommitPlan<'_>,
    env: &'env Environment,
) -> Result<Admission<Applied<'env>>> {
    let schema = plan.selections.schema();
    let mut txn = env.write_txn()?;
    let (row_id_next, violations) = {
        let mut catalog = txn.catalog();
        let mut applier = Applier {
            catalog: &mut catalog,
            schema,
            row_id_next: BTreeMap::new(),
            key: [0; MAX_KEY],
            violations: Vec::new(),
        };

        {
            let mut span = obs::span(obs::names::APPLY_DELETES);
            for op in &plan.deletes {
                applier.delete_fact(op)?;
            }
            span.set_count(plan.deletes.len() as u64);
        }
        {
            let mut span = obs::span(obs::names::APPLY_INSERTS);
            for op in &plan.inserts {
                applier.insert_fact(op)?;
            }
            span.set_count(plan.inserts.len() as u64);
        }
        (applier.row_id_next, applier.violations)
    };

    // Key violations preempt the judgment phase: the containment probes
    // are defined over the keyed final state, which this apply failed to
    // produce — the rejection is every violated key statement, sealed.
    Ok(match Violations::seal(violations) {
        Admission::Rejected(violations) => Admission::Rejected(violations),
        Admission::Accepted(()) => Admission::Accepted(Applied { txn, row_id_next }),
    })
}
