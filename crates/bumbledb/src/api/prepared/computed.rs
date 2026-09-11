//! The computed-output sink adapter: typed scalar stage outputs
//! (`FindTerm::Compute`) evaluated per surviving binding, after every
//! input predicate of the rule has run. Unlike a plain projection it can
//! never license a suffix skip (`SkipCapability::Forbidden`) and never
//! accepts a fused leaf scan: every computed output must see its complete
//! binding row, and a producer error must not be hidden by work elision
//! (chapter 12's stage error boundary).
//!
//! Errors are sticky: the first scalar failure is recorded and every
//! later row is dropped — `finalize` refuses to publish any answer for
//! this execution (`Q-ATOMIC`: no partial published result). The FPU
//! environment is established ONCE per engine operation by the query
//! entry (`execute.rs` holds the [`NumericalGuard`] across the whole
//! run), not per row and not per arithmetic node.
//!
//! [`NumericalGuard`]: crate::exec::kernel::numeric::NumericalGuard
use std::sync::Arc;

use super::EitherSink;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafSource, Sink};
use crate::exec::sink::FindSpec;
use crate::schema::ValueType;
use crate::{Error, F64, FindIndex, FindTerm, ScalarError, Value, VarId};

#[cfg(test)]
mod tests;

/// One computed find's sealed program: the find position (diagnostics),
/// the validated expression, and its inputs — per referenced variable,
/// the binding slot it reads and the type its word decodes as. Sealed at
/// prepare from validation's typing; never re-derived at execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputProgram {
    pub(crate) find: usize,
    pub(crate) expression: FindTerm,
    pub(crate) inputs: Vec<(VarId, usize, ValueType)>,
}

/// The adapter: evaluates each program into its appended output slot,
/// then forwards the widened binding row to the inner sink (projection
/// or aggregate), whose find specs were lowered to read those slots.
pub(in crate::api) struct ComputedSink {
    pub(super) inner: EitherSink,
    programs: Vec<(usize, Arc<OutputProgram>)>,
    bindings: Bindings,
    /// The rule's real slot count; slots at and past it are outputs.
    slots: usize,
    pieces: Vec<(usize, [[u64; 2]; 2], usize, usize)>,
    /// The first scalar failure of this execution; sticky until reset.
    pub(super) error: Option<Error>,
}

/// A lowered find-spec list: the rewritten specs, the `(slot, program)`
/// pairs that fill the appended output slots, and the widened slot count.
pub(super) type Lowered = (Vec<FindSpec>, Vec<(usize, Arc<OutputProgram>)>, usize);

/// Lowers a find-spec list for the inner sink: every `Compute` becomes a
/// fresh appended `Var` slot, and the programs that fill those slots are
/// returned beside the widened slot count.
pub(super) fn lower(finds: &[FindSpec], slots: usize) -> Lowered {
    let mut next = slots;
    let mut programs = Vec::new();
    let finds = finds
        .iter()
        .map(|find| match find {
            FindSpec::Compute(program) => {
                let slot = next;
                let width = if matches!(program.expression, FindTerm::Segments { .. }) {
                    2
                } else {
                    1
                };
                next += width;
                programs.push((slot, Arc::clone(program)));
                FindSpec::Var { slot, width }
            }
            other => other.clone(),
        })
        .collect();
    (finds, programs, next)
}

impl ComputedSink {
    pub(super) fn reset(&mut self) {
        self.error = None;
        if self.bindings.slot_count() == 0 {
            self.bindings.resize(
                self.slots
                    + self
                        .programs
                        .iter()
                        .map(|(_, p)| {
                            if matches!(p.expression, FindTerm::Segments { .. }) {
                                2
                            } else {
                                1
                            }
                        })
                        .sum::<usize>(),
            );
        }
        self.inner.reset();
    }

    pub(super) fn release_memory(&mut self) {
        self.error = None;
        self.bindings = Bindings::new(0);
        self.pieces = Vec::new();
        self.inner.release_memory();
    }

    pub(super) fn new(
        inner: EitherSink,
        programs: Vec<(usize, Arc<OutputProgram>)>,
        slots: usize,
        total: usize,
    ) -> Self {
        Self {
            inner,
            programs,
            bindings: Bindings::new(total),
            slots,
            error: None,
            pieces: Vec::new(),
        }
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], slots: usize, shared: &[(usize, usize)]) {
        let (finds, programs, total) = lower(finds, slots);
        self.slots = slots;
        self.programs = programs;
        self.bindings.resize(total);
        self.inner.aim(&finds, total, shared);
    }

    /// Generate interval pieces before any partial scalar output. Empty pieces
    /// eliminate the binding. Odometer traversal forms the ordinary Cartesian
    /// product without recursion or materializing a product-sized buffer.
    fn row(&mut self) {
        if self.error.is_some() {
            return;
        }
        self.pieces.clear();
        for (slot, program) in &self.programs {
            let FindTerm::Segments { op, left, right } = &program.expression else {
                continue;
            };
            let a = read_value(&self.bindings, program, *left).expect("validated interval input");
            let b = read_value(&self.bindings, program, *right).expect("validated interval input");
            let segments = match (a, b) {
                (Value::IntervalU64(a), Value::IntervalU64(b)) => {
                    op.apply(a, b).map(|s| s.map(|s| [s.start(), s.end()]))
                }
                (Value::IntervalI64(a), Value::IntervalI64(b)) => op.apply(a, b).map(|s| {
                    s.map(|s| {
                        [
                            s.start().cast_unsigned() ^ (1 << 63),
                            s.end().cast_unsigned() ^ (1 << 63),
                        ]
                    })
                }),
                (Value::IntervalF64(a), Value::IntervalF64(b)) => op
                    .apply(a, b)
                    .map(|s| s.map(|s| [s.start().to_order_key(), s.end().to_order_key()])),
                _ => unreachable!("validated same-kind intervals"),
            };
            let mut values = [[0; 2]; 2];
            let mut len = 0;
            for segment in segments.into_iter().flatten() {
                values[len] = segment;
                len += 1;
            }
            if len == 0 {
                return;
            }
            self.pieces.push((*slot, values, len, 0));
        }
        for (slot, program) in &self.programs {
            let FindTerm::Compute(expression) = &program.expression else {
                continue;
            };
            let value = crate::scalar::evaluate_in_operation(expression, |var| {
                read_value(&self.bindings, program, var)
            });
            let word = match value {
                Ok(Value::U64(value)) => value,
                Ok(Value::I64(value)) => value.cast_unsigned() ^ (1 << 63),
                Ok(Value::F64(value)) => value.to_order_key(),
                Ok(Value::Bool(value)) => u64::from(value),
                Err(source) => {
                    self.error = Some(Error::Scalar {
                        find: FindIndex(program.find),
                        source,
                    });
                    return;
                }
                Ok(_) => unreachable!("validated scalar output type"),
            };
            self.bindings.set(*slot, word);
        }
        loop {
            for &(slot, values, _, position) in &self.pieces {
                self.bindings.set(slot, values[position][0]);
                self.bindings.set(slot + 1, values[position][1]);
            }
            if self.inner.emit(&self.bindings).is_terminal() {
                return;
            }
            let mut advanced = false;
            for (_, _, len, position) in self.pieces.iter_mut().rev() {
                *position += 1;
                if *position < *len {
                    advanced = true;
                    break;
                }
                *position = 0;
            }
            if !advanced {
                return;
            }
        }
    }

    fn flow_after_row(&self) -> Flow {
        if self.error.is_some() {
            Flow::Error
        } else {
            Flow::from_sink_progress(self.inner.progress())
        }
    }
}

impl Sink for ComputedSink {
    fn retains_binding_slot(&self, slot: usize) -> bool {
        self.inner.retains_binding_slot(slot)
    }

    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..self.slots {
            self.bindings.set(slot, bindings.get(slot));
        }
        self.row();
        self.flow_after_row()
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        for &entry in batch.survivors {
            if self.flow_after_row().is_terminal() {
                break;
            }
            for slot in 0..self.slots {
                let value = match batch.source_of(slot) {
                    LeafSource::Key(word) => batch.key(entry, word),
                    LeafSource::Outer => batch.bindings.get(slot),
                };
                self.bindings.set(slot, value);
            }
            self.row();
        }
        self.flow_after_row()
    }

    fn progress(&self) -> crate::exec::sink::SinkProgress {
        if self.error.is_some() {
            crate::exec::sink::SinkProgress::Error
        } else {
            self.inner.progress()
        }
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        self.error.take().or_else(|| self.inner.take_error())
    }
}

fn read_value(
    bindings: &Bindings,
    program: &OutputProgram,
    var: VarId,
) -> Result<Value, ScalarError> {
    let (_, slot, ty) = program
        .inputs
        .iter()
        .find(|(id, _, _)| *id == var)
        .ok_or(ScalarError::UnboundVariable(var))?;
    let word = bindings.get(*slot);
    if let Some(element) = ty.interval_element() {
        let end = bindings.get(*slot + 1);
        return Ok(match element {
            crate::schema::IntervalElement::U64 => {
                Value::IntervalU64(crate::Interval::new(word, end).expect("valid interval binding"))
            }
            crate::schema::IntervalElement::I64 => Value::IntervalI64(
                crate::Interval::new(
                    (word ^ (1 << 63)).cast_signed(),
                    (end ^ (1 << 63)).cast_signed(),
                )
                .expect("valid interval binding"),
            ),
            crate::schema::IntervalElement::F64 => Value::IntervalF64(
                crate::Interval::new(
                    F64::from_order_key(word).expect("canonical endpoint"),
                    F64::from_order_key(end).expect("canonical endpoint"),
                )
                .expect("valid interval binding"),
            ),
        });
    }
    match ty {
        ValueType::U64 => Ok(Value::U64(word)),
        ValueType::I64 => Ok(Value::I64((word ^ (1 << 63)).cast_signed())),
        ValueType::F64 => Ok(Value::F64(
            F64::from_order_key(word).expect("canonical float"),
        )),
        ValueType::Bool => Ok(Value::Bool(word != 0)),
        _ => Err(ScalarError::NotNumeric),
    }
}
