use super::{fact_word, guard_probe_fact, GuardPlan};
use crate::error::Result;
use crate::exec::run::{Bindings, Sink};
use crate::image::view::Const;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;

/// Executes the guard probe: guard key from constants, one `U`/`M` get,
/// one `F` fetch, remaining filters on the fact bytes, then the single
/// binding through the ordinary sink (sinks are reused, not special-cased).
///
/// # Errors
///
/// `Lmdb`/`Corruption` from the storage reads. A missing key or a failed
/// filter is not an error: the result is simply empty.
pub fn execute_guard<S: Sink>(
    plan: &GuardPlan,
    txn: &ReadTxn<'_>,
    schema: &Schema,
    params: &[Const],
    key_scratch: &mut Vec<u8>,
    bindings: &mut Bindings,
    sink: &mut S,
) -> Result<()> {
    let Some(fact) = guard_probe_fact(plan, txn, schema, params, key_scratch)? else {
        return Ok(());
    };
    // The single binding, through the ordinary sink (the aggregate-find
    // guard path; plain-variable guards take the direct lane, docs/perf/
    // PRD 11).
    bindings.reset();
    for (slot, (field, _)) in plan.vars.iter().enumerate() {
        bindings.set(slot, fact_word(schema, plan, fact, *field));
    }
    sink.emit(bindings);
    Ok(())
}
