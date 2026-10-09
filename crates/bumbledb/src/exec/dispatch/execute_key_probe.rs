use super::{KeyProbePlan, ProbeBuffers, ProbeCtx, key_probe_fact::key_probe_row};
use crate::error::Result;
use crate::exec::run::{Bindings, Counters, Sink};

/// Probes once and emits the matched row's binding.
/// # Errors
/// Storage failure, stopped work, or corrupt stored bytes.
pub(crate) fn execute_key_probe<S: Sink, C: Counters>(
    plan: &KeyProbePlan,
    cx: ProbeCtx<'_>,
    buf: &mut ProbeBuffers,
    bindings: &mut Bindings,
    sink: &mut S,
    counters: &mut C,
) -> Result<()> {
    if !key_probe_row(plan, cx, buf)? {
        return Ok(());
    }

    bindings.reset();
    for var in &plan.vars {
        let words = buf.row.span_words(var.field);
        debug_assert_eq!(var.width, words.len(), "the SlotWidth layout");
        for (offset, word) in words.iter().enumerate() {
            bindings.set(var.slot + offset, *word);
        }
    }
    sink.emit(bindings);
    counters.emit();
    Ok(())
}
