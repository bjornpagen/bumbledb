//! Typed output adapter. It runs only after input predicates; unlike a plain
//! projection it cannot skip suffixes that might contain a producer error.
use std::sync::Arc;
use crate::exec::run::{Bindings, Flow, LeafBatch, Sink};
use crate::exec::sink::FindSpec;
use crate::schema::ValueType;
use crate::{Error, F64, FindIndex, ScalarError, ScalarExpr, Value, VarId};
use super::EitherSink;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputProgram {
    pub(crate) find: usize,
    pub(crate) expression: ScalarExpr,
    pub(crate) inputs: Vec<(VarId, usize, ValueType)>,
}

pub(super) struct ComputedSink {
    pub(super) inner: EitherSink,
    programs: Vec<(usize, Arc<OutputProgram>)>,
    bindings: Bindings,
    slots: usize,
    pub(super) error: Option<Error>,
}

pub(super) fn lower(finds: &[FindSpec], slots: usize) -> (Vec<FindSpec>, Vec<(usize, Arc<OutputProgram>)>, usize) {
    let mut next = slots;
    let mut programs = Vec::new();
    let finds = finds.iter().map(|find| match find {
        FindSpec::Compute(program) => {
            let slot = next;
            next += 1;
            programs.push((slot, Arc::clone(program)));
            FindSpec::Var { slot, width: 1 }
        },
        other => other.clone(),
    }).collect();
    (finds, programs, next)
}

impl ComputedSink {
    pub(super) fn new(inner: EitherSink, programs: Vec<(usize, Arc<OutputProgram>)>, slots: usize, total: usize) -> Self {
        Self { inner, programs, bindings: Bindings::new(total), slots, error: None }
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], slots: usize, shared: &[(usize, usize)]) {
        let (finds, programs, total) = lower(finds, slots);
        self.slots = slots;
        self.programs = programs;
        self.bindings.resize(total);
        self.inner.aim(&finds, total, shared);
    }

    fn row(&mut self) {
        if self.error.is_some() { return; }
        for (slot, program) in &self.programs {
            let value = crate::scalar::evaluate_in_operation(&program.expression, |var| {
                let (_, slot, ty) = program.inputs.iter().find(|(id, _, _)| *id == var)
                    .ok_or(ScalarError::UnboundVariable(var))?;
                let word = self.bindings.get(*slot);
                match ty {
                    ValueType::U64 => Ok(Value::U64(word)),
                    ValueType::I64 => Ok(Value::I64((word ^ (1 << 63)).cast_signed())),
                    ValueType::F64 => Ok(Value::F64(F64::from_order_key(word).expect("canonical input word"))),
                    ValueType::Bool => Ok(Value::Bool(word != 0)),
                    _ => Err(ScalarError::NotNumeric),
                }
            });
            let word = match value {
                Ok(Value::U64(value)) => value,
                Ok(Value::I64(value)) => value.cast_unsigned() ^ (1 << 63),
                Ok(Value::F64(value)) => value.to_order_key(),
                Ok(Value::Bool(value)) => u64::from(value),
                Err(source) => { self.error = Some(Error::Scalar { find: FindIndex(program.find), source }); return; },
                _ => unreachable!("validated scalar output type"),
            };
            self.bindings.set(*slot, word);
        }
        self.inner.emit(&self.bindings);
    }
}

impl Sink for ComputedSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..self.slots { self.bindings.set(slot, bindings.get(slot)); }
        self.row();
        Flow::Continue
    }
    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        for &entry in batch.survivors {
            for slot in 0..self.slots {
                let value = match batch.source_of(slot) {
                    crate::exec::run::LeafSource::Key(word) => batch.key(entry, word),
                    crate::exec::run::LeafSource::Outer => batch.bindings.get(slot),
                };
                self.bindings.set(slot, value);
            }
            self.row();
        }
        Flow::Continue
    }
}
