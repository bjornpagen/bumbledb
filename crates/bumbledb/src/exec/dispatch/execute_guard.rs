use super::{fact_operand, guard_probe_fact, FactOperand, GuardPlan};
use crate::error::Result;
use crate::exec::run::{Bindings, Sink};
use crate::image::view::Const;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;

/// Executes the guard probe: key bytes from constants, one `U`/`M` get,
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
    // guard path; plain-variable guards take the direct lane).
    // Interval variables occupy their two-slot span.
    bindings.reset();
    for var in &plan.vars {
        match fact_operand(schema, plan.relation, fact, var.field) {
            FactOperand::Word(word) => bindings.set(var.slot, word),
            FactOperand::Pair(start, end) => {
                debug_assert_eq!(var.width, 2, "the SlotWidth layout");
                bindings.set(var.slot, start);
                bindings.set(var.slot + 1, end);
            }
        }
    }
    sink.emit(bindings);
    Ok(())
}
