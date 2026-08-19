use super::{FactOperand, KeyProbePlan, fact_operand, key_probe_fact};
use crate::error::Result;
use crate::exec::run::{Bindings, Sink};
use crate::image::view::Const;
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;

/// Executes the key probe: key bytes from constants, one `U`/`M` get,
/// one `F` fetch, remaining filters on the fact bytes, then the single
/// binding through the ordinary sink (sinks are reused, not special-cased
/// — a key-probe rule inside a multi-rule query unions through the same
/// spanning seen-set as every other rule). The emit is counted like a
/// join emit (the rule loop's union accounting). Multi-word variables
/// (intervals, bytes<N>) occupy their whole slot span.
///
/// # Errors
///
/// `Lmdb`/`Corruption` from the storage reads. A missing key or a failed
/// filter is not an error: the result is simply empty.
#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)] // the prepared query's split borrows,
// exactly like `run_join`'s — bundling
// would only rename the same eight things
pub fn execute_key_probe<S: Sink, C: crate::exec::run::Counters, Cat: CatalogRead>(
    plan: &KeyProbePlan,
    catalog: &Cat,
    schema: &Schema,
    params: &[Const],
    key_scratch: &mut Vec<u8>,
    bindings: &mut Bindings,
    sink: &mut S,
    counters: &mut C,
) -> Result<()> {
    let Some(stored) = key_probe_fact(plan, catalog, schema, params, key_scratch)? else {
        return Ok(());
    };
    let fact = schema
        .relation(plan.relation)
        .layout()
        .encoded(stored.as_ref());
    // The single binding, through the ordinary sink (the aggregate-find
    // key-probe path; plain-variable key_probes take the direct lane).
    // Interval variables occupy their two-slot span.
    bindings.reset();
    for var in &plan.vars {
        match fact_operand(fact, var.field)? {
            FactOperand::Word(word) => bindings.set(var.slot, word),
            FactOperand::Pair(start, end) => {
                debug_assert_eq!(var.width, 2, "the SlotWidth layout");
                bindings.set(var.slot, start);
                bindings.set(var.slot + 1, end);
            }
            FactOperand::Block { words, count } => {
                debug_assert_eq!(var.width, usize::from(count), "the SlotWidth layout");
                for (offset, word) in words[..usize::from(count)].iter().enumerate() {
                    bindings.set(var.slot + offset, *word);
                }
            }
        }
    }
    sink.emit(bindings);
    counters.emit();
    Ok(())
}
