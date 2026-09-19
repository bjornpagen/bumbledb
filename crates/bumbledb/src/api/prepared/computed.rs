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

mod events;
mod expectation;
mod pack;
#[cfg(test)]
mod tests;

/// One computed find's sealed program: the find position (diagnostics),
/// the validated expression, and its inputs — per referenced variable,
/// the binding slot it reads and the type its word decodes as. Sealed at
/// prepare from validation's typing; never re-derived at execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputProgram {
    pub(crate) find: usize,
    /// Every written rule represented by this normalized arm. DNF collapse
    /// must not discard the logical identities used by Event diagnostics.
    pub(crate) rules: Vec<u16>,
    pub(crate) expression: FindTerm,
    pub(crate) inputs: Vec<(VarId, usize, crate::ir::validate::QueryType)>,
}

impl OutputProgram {
    fn width(&self) -> usize {
        match self.expression {
            FindTerm::Probability { .. } => 4,
            FindTerm::Segments { .. } | FindTerm::Event(_) | FindTerm::Pack { .. } => 2,
            _ => 1,
        }
    }
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
    faults: events::Faults,
    pack: Option<pack::Pack>,
    expectations: Option<expectation::Expectations>,
    pub(super) expectation_inputs: Vec<crate::observation::ExpectationInput>,
    pub(super) work: Option<crate::work::WorkContext>,
    generation: Option<crate::work::GenerationHandle>,
    stage: Option<usize>,
    pub(super) observations: super::observations::ObservationRegistry,
    pub(super) arithmetic: crate::event::ArithmeticBudget,
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
                let width = program.width();
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
        self.faults.clear();
        self.expectation_inputs.clear();
        if let Some(expectations) = &mut self.expectations {
            expectations.reset();
        }
        if let Some(pack) = &mut self.pack {
            pack.reset();
        }
        if self.bindings.slot_count() == 0 {
            self.bindings
                .resize(self.slots + self.programs.iter().map(|(_, p)| p.width()).sum::<usize>());
        }
        self.inner.reset();
    }

    pub(super) fn release_memory(&mut self) {
        self.observations.clear();
        self.arithmetic = crate::event::ArithmeticBudget::default();
        self.error = None;
        self.faults = events::Faults::default();
        self.expectation_inputs = Vec::new();
        if let Some(expectations) = &mut self.expectations {
            expectations.reset();
        }
        if let Some(pack) = &mut self.pack {
            pack.reset();
        }
        self.generation = None;
        self.work = None;
        self.bindings = Bindings::new(0);
        self.pieces = Vec::new();
        self.inner.release_memory();
    }

    pub(super) fn new(
        inner: EitherSink,
        programs: Vec<(usize, Arc<OutputProgram>)>,
        slots: usize,
        total: usize,
        finds: &[FindSpec],
    ) -> Self {
        let pack = programs
            .iter()
            .find(|(_, p)| matches!(p.expression, FindTerm::Pack { .. }))
            .map(|(slot, p)| pack::Pack::new(finds, *slot, p.find));
        let expectations = programs
            .iter()
            .any(|(_, p)| matches!(p.expression, FindTerm::Expectation { .. }))
            .then(|| expectation::Expectations::new(finds, &programs));
        Self {
            expectations,
            expectation_inputs: Vec::new(),
            pack,
            inner,
            programs,
            bindings: Bindings::new(total),
            slots,
            error: None,
            pieces: Vec::new(),
            faults: events::Faults::default(),
            work: None,
            generation: None,
            stage: None,
            observations: super::observations::ObservationRegistry::default(),
            arithmetic: crate::event::ArithmeticBudget::default(),
        }
    }

    pub(super) fn bind_events(
        &mut self,
        generation: &crate::work::GenerationHandle,
        stage: Option<usize>,
    ) {
        self.generation = Some(generation.clone());
        self.stage = stage;
    }

    /// Called once at the stage boundary, after every arm and binding. Resource
    /// and scalar errors are separate immediate refusals; partial sets never escape.
    pub(super) fn finish_events(&mut self) -> crate::Result<()> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if let Some(pack) = &mut self.pack {
            let result = pack.finish(
                self.generation
                    .as_ref()
                    .ok_or(crate::event::Error::UnknownKey)?,
                self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?,
                self.stage,
                &mut self.faults,
                &mut self.inner,
            );
            if let Err(error) = result {
                self.error = Some(error.clone());
                return Err(error);
            }
        }
        let expectation_result = if let Some(expectations) = &mut self.expectations {
            expectations
                .finish(
                    self.generation
                        .as_ref()
                        .ok_or(crate::event::Error::UnknownKey)?,
                    self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?,
                    self.stage,
                    &mut self.faults,
                )
                .map(|inputs| self.expectation_inputs = inputs)
        } else {
            Ok(())
        };
        let result = expectation_result.and_then(|()| self.faults.finish());
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    fn event_outputs(&mut self) -> crate::Result<bool> {
        let mut admitted = true;
        for (slot, program) in &self.programs {
            if !matches!(
                program.expression,
                FindTerm::Event(_) | FindTerm::Test(_) | FindTerm::Probability { .. }
            ) {
                continue;
            }
            let words = events::evaluate(
                &self.bindings,
                program,
                self.stage,
                self.generation
                    .as_ref()
                    .ok_or(crate::event::Error::UnknownKey)?,
                self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?,
                &mut self.faults,
            )?;
            if let Some(words) = words {
                for (offset, word) in words.into_iter().take(program.width()).enumerate() {
                    self.bindings.set(*slot + offset, word);
                }
            } else {
                admitted = false;
            }
        }
        Ok(admitted)
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], slots: usize, shared: &[(usize, usize)]) {
        let (finds, programs, total) = lower(finds, slots);
        if let Some(pack) = &mut self.pack {
            let (slot, _) = programs
                .iter()
                .find(|(_, p)| matches!(p.expression, FindTerm::Pack { .. }))
                .expect("Event Pack heads stay Event Pack");
            pack.aim(&finds, *slot);
        }
        if let Some(expectations) = &mut self.expectations {
            expectations.aim(&finds, &programs);
        }
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
        let admitted = match self.event_outputs() {
            Ok(admitted) => admitted,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
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
        if let Err(error) = self.observation_outputs() {
            self.error = Some(error);
            return;
        }
        if let Err(error) = self.scalar_outputs() {
            self.error = Some(error);
            return;
        }
        if !admitted {
            return;
        }
        if let Some(result) = self.aggregate_row() {
            if let Err(error) = result {
                self.error = Some(error);
            }
            return;
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

    fn scalar_outputs(&mut self) -> crate::Result<()> {
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
                    return Err(Error::Scalar {
                        find: FindIndex(program.find),
                        source,
                    });
                }
                Ok(_) => unreachable!("validated scalar output type"),
            };
            self.bindings.set(*slot, word);
        }
        Ok(())
    }

    fn aggregate_row(&mut self) -> Option<crate::Result<()>> {
        if let Err(error) = self.expectation_row() {
            return Some(Err(error));
        }
        self.pack_row()
    }

    fn observation_outputs(&mut self) -> crate::Result<()> {
        use crate::ir::validate::QueryType;
        use crate::number_expr::ObservationOperand;
        if self.programs.iter().all(|(_, program)| {
            !matches!(
                program.expression,
                FindTerm::Number(_) | FindTerm::Predicate(_) | FindTerm::PredicateTest { .. }
            )
        }) {
            return Ok(());
        }
        let control = self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?;
        let mut arithmetic = crate::event::ExactArithmetic::borrow(&mut self.arithmetic, control);
        let limits = crate::ObservationNumberCodecLimits::default();
        for (slot, program) in &self.programs {
            if !matches!(
                program.expression,
                FindTerm::Number(_) | FindTerm::Predicate(_) | FindTerm::PredicateTest { .. }
            ) {
                continue;
            }
            let operand = |var| {
                let (_, slot, ty) = program
                    .inputs
                    .iter()
                    .find(|(id, _, _)| *id == var)
                    .expect("validated numerical input");
                let word = self.bindings.get(*slot);
                Ok(match ty {
                    QueryType::Stored(ValueType::U64) => ObservationOperand::Integer(word.into()),
                    QueryType::Stored(ValueType::I64) => {
                        ObservationOperand::Integer((word ^ (1 << 63)).cast_signed().into())
                    }
                    QueryType::Observation(kind) => match self.observations.get(*kind, word)? {
                        crate::AnswerValue::Predicate(value) => {
                            ObservationOperand::Predicate(value.value().clone())
                        }
                        crate::AnswerValue::Number(value) => {
                            ObservationOperand::Number(value.value().clone())
                        }
                        crate::AnswerValue::Probability(value) => {
                            ObservationOperand::Probability(value.clone())
                        }
                        crate::AnswerValue::Expectation(value) => {
                            ObservationOperand::Expectation(value.clone())
                        }
                        _ => unreachable!("owned observation"),
                    },
                    QueryType::Stored(_) => unreachable!("validated numerical input type"),
                })
            };
            let word = match &program.expression {
                FindTerm::Number(expression) => {
                    let value = expression.evaluate(operand, &limits, &mut arithmetic)?;
                    let value =
                        crate::ObservationNumberImport::checked(value, limits, &mut arithmetic)?;
                    self.observations.insert_number(value)?
                }
                FindTerm::Predicate(expression) => {
                    let value = expression.evaluate(operand, &limits, &mut arithmetic)?;
                    let value =
                        crate::ObservationPredicateImport::checked(value, limits, &mut arithmetic)?;
                    self.observations.insert_predicate(value)?
                }
                FindTerm::PredicateTest {
                    predicate,
                    quantifier,
                } => {
                    let value = predicate.evaluate(operand, &limits, &mut arithmetic)?;
                    u64::from(quantifier.evaluate(&value))
                }
                _ => unreachable!("only numerical/predicate programs"),
            };
            self.bindings.set(*slot, word);
        }
        Ok(())
    }

    fn expectation_row(&mut self) -> crate::Result<()> {
        let Some(expectations) = &mut self.expectations else {
            return Ok(());
        };
        for (slot, program) in &self.programs {
            if !matches!(program.expression, FindTerm::Expectation { .. }) {
                continue;
            }
            let token = expectations.observe(
                &self.bindings,
                program,
                self.generation
                    .as_ref()
                    .ok_or(crate::event::Error::UnknownKey)?,
                self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?,
            )?;
            self.bindings.set(*slot, token);
        }
        Ok(())
    }

    fn pack_row(&mut self) -> Option<crate::Result<()>> {
        let pack = self.pack.as_mut()?;
        let (_, program) = self
            .programs
            .iter()
            .find(|(_, p)| matches!(p.expression, FindTerm::Pack { .. }))
            .expect("Event Pack program");
        Some((|| {
            pack.observe(
                &self.bindings,
                program,
                self.generation
                    .as_ref()
                    .ok_or(crate::event::Error::UnknownKey)?,
                self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?,
            )
        })())
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
    let ty = ty.stored().ok_or(ScalarError::TypeMismatch)?;
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
