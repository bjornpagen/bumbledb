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
    key_scratch: &mut Vec<u64>,
    bindings: &mut Bindings,
    sink: &mut S,
    counters: &mut C,
) -> Result<()> {
    let field_types: Vec<bumbledb_theory::schema::ValueType> = schema
        .relation(plan.relation)
        .fields()
        .iter()
        .map(|f| f.value_type)
        .collect();
    let mut row = RowWords::new(&field_types);
    if !key_probe_row(
        plan,
        source,
        schema,
        interner,
        params,
        &mut row,
        key_scratch,
    )? {
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
