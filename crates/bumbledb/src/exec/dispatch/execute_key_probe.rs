use super::{KeyProbePlan, key_probe_fact::key_probe_row};
use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::exec::run::{Bindings, Sink};
use crate::image::canon::RowWords;
use crate::image::intern::InternerHandle;
use crate::image::view::Const;
use crate::schema::Schema;

/// # Errors
/// Storage failure, stopped work, or corrupt stored bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
pub fn execute_key_probe<S: Sink, C: crate::exec::run::Counters>(
    plan: &KeyProbePlan,
    source: &QuerySource<'_>,
    schema: &Schema,
    interner: &InternerHandle<'_>,
    params: &[Const],
    row: &mut RowWords,
    key_scratch: &mut crate::image::view::ResolvedWords,
    bindings: &mut Bindings,
    sink: &mut S,
    counters: &mut C,
) -> Result<()> {
    if !key_probe_row(plan, source, schema, interner, params, row, key_scratch)? {
        return Ok(());
    }

    bindings.reset();
    for var in &plan.vars {
        let words = row.span_words(var.field);
        debug_assert_eq!(var.width, words.len(), "the SlotWidth layout");
        for (offset, word) in words.iter().enumerate() {
            bindings.set(var.slot + offset, *word);
        }
    }
    sink.emit(bindings);
    counters.emit();
    Ok(())
}
