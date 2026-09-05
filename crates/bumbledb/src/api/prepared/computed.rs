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
use crate::{Error, F64, FindIndex, ScalarError, ScalarExpr, Value, VarId};

/// One computed find's sealed program: the find position (diagnostics),
/// the validated expression, and its inputs — per referenced variable,
/// the binding slot it reads and the type its word decodes as. Sealed at
/// prepare from validation's typing; never re-derived at execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputProgram {
    pub(crate) find: usize,
    pub(crate) expression: ScalarExpr,
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
                next += 1;
                programs.push((slot, Arc::clone(program)));
                FindSpec::Var { slot, width: 1 }
            }
            other => other.clone(),
        })
        .collect();
    (finds, programs, next)
}

impl ComputedSink {
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
        }
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], slots: usize, shared: &[(usize, usize)]) {
        let (finds, programs, total) = lower(finds, slots);
        self.slots = slots;
        self.programs = programs;
        self.bindings.resize(total);
        self.inner.aim(&finds, total, shared);
    }

    /// Evaluate every program over the staged binding row, then forward.
    /// A scalar failure records the FIRST error and drops the row —
    /// after that, every row of the spoiled execution is dropped and
    /// `finalize` refuses publication.
    fn row(&mut self) {
        if self.error.is_some() {
            return;
        }
        for (slot, program) in &self.programs {
            let value = crate::scalar::evaluate_in_operation(&program.expression, |var| {
                let (_, slot, ty) = program
                    .inputs
                    .iter()
                    .find(|(id, _, _)| *id == var)
                    .ok_or(ScalarError::UnboundVariable(var))?;
                let word = self.bindings.get(*slot);
                match ty {
                    ValueType::U64 => Ok(Value::U64(word)),
                    ValueType::I64 => Ok(Value::I64((word ^ (1 << 63)).cast_signed())),
                    ValueType::F64 => Ok(Value::F64(
                        F64::from_order_key(word).expect("validated canonical F64 binding word"),
                    )),
                    ValueType::Bool => Ok(Value::Bool(word != 0)),
                    _ => Err(ScalarError::NotNumeric),
                }
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
        self.inner.emit(&self.bindings);
    }
}

impl Sink for ComputedSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..self.slots {
            self.bindings.set(slot, bindings.get(slot));
        }
        self.row();
        Flow::Continue
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        for &entry in batch.survivors {
            for slot in 0..self.slots {
                let value = match batch.source_of(slot) {
                    LeafSource::Key(word) => batch.key(entry, word),
                    LeafSource::Outer => batch.bindings.get(slot),
                };
                self.bindings.set(slot, value);
            }
            self.row();
        }
        Flow::Continue
    }
}
