//! Numerical values share one execution budget. Explicit guard interpretation
//! returns ordinary interned Events and retains every written transport fault.
use super::{ComputedSink, OutputProgram};
use crate::event::ExactArithmetic;
use crate::ir::validate::QueryType;
use crate::number_expr::ObservationOperand;
use crate::{FindTerm, GuardExpr, ObservationNumberCodecLimits, Result, VarId};

impl OutputProgram {
    fn operand(
        &self,
        var: VarId,
        bindings: &crate::exec::run::Bindings,
        observations: &super::super::observations::ObservationRegistry,
    ) -> Result<ObservationOperand> {
        let (_, slot, ty) = self
            .inputs
            .iter()
            .find(|(id, _, _)| *id == var)
            .expect("validated numerical input");
        let word = bindings.get(*slot);
        Ok(match ty {
            QueryType::Stored(crate::schema::ValueType::U64) => {
                ObservationOperand::Integer(word.into())
            }
            QueryType::Stored(crate::schema::ValueType::I64) => {
                ObservationOperand::Integer((word ^ (1 << 63)).cast_signed().into())
            }
            QueryType::Observation(kind) => match observations.get(*kind, word)? {
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
    }
}

impl ComputedSink {
    pub(super) fn observation_outputs(&mut self) -> Result<bool> {
        let mut admitted = true;
        for index in 0..self.programs.len() {
            let (slot, program) = &self.programs[index];
            if !matches!(
                program.expression,
                FindTerm::Guard(_)
                    | FindTerm::Number(_)
                    | FindTerm::Predicate(_)
                    | FindTerm::PredicateTest { .. }
            ) {
                continue;
            }
            if matches!(program.expression, FindTerm::Guard(_)) {
                let (slot, program) = (*slot, std::sync::Arc::clone(program));
                let FindTerm::Guard(expression) = &program.expression else {
                    unreachable!()
                };
                admitted &= self.guard_output(slot, &program, expression)?;
                continue;
            }
            let control = self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?;
            let mut arithmetic = ExactArithmetic::borrow(&mut self.arithmetic, control);
            let limits = ObservationNumberCodecLimits::default();
            let operand = |var| program.operand(var, &self.bindings, &self.observations);
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
                _ => unreachable!("numerical output"),
            };
            self.bindings.set(*slot, word);
        }
        Ok(admitted)
    }

    fn guard_output(
        &mut self,
        slot: usize,
        program: &OutputProgram,
        expression: &GuardExpr,
    ) -> Result<bool> {
        let control = self.work.as_ref().ok_or(crate::event::Error::UnknownKey)?;
        let mut arithmetic = ExactArithmetic::borrow(&mut self.arithmetic, control);
        let limits = ObservationNumberCodecLimits::default();
        let predicate = expression.predicate.evaluate(
            |var| program.operand(var, &self.bindings, &self.observations),
            &limits,
            &mut arithmetic,
        )?;
        let mut companions = Vec::new();
        companions
            .try_reserve_exact(expression.companions.len())
            .map_err(crate::event::Error::from)?;
        for companion in &expression.companions {
            companions.push(companion.evaluate(
                |var| program.operand(var, &self.bindings, &self.observations),
                &limits,
                &mut arithmetic,
            )?);
        }
        let interpretation =
            expression
                .plan
                .interpret(&predicate, &companions, &limits, &mut arithmetic)?;
        let interner = crate::image::intern::InternerHandle::new(
            self.generation
                .as_ref()
                .ok_or(crate::event::Error::UnknownKey)?,
            control,
        );
        let input = expression
            .operation
            .input()
            .map(|var| {
                let (_, slot, _) = program
                    .inputs
                    .iter()
                    .find(|(id, _, _)| *id == var)
                    .expect("validated guard input");
                interner.resolve_event([self.bindings.get(*slot), self.bindings.get(*slot + 1)])
            })
            .transpose()?;
        if let (Some(input), Some(expected)) =
            (&input, interpretation.expected_input(expression.operation))
        {
            match input.align_to(expected, control) {
                Ok(_) => {}
                Err(crate::event::Error::SpaceMismatch) => {
                    for &rule in &program.rules {
                        self.faults.insert(crate::EventOperandFault {
                            stage: self.stage,
                            rule,
                            find: program.find,
                            operand: 0,
                            source: crate::EventOperandSource::Variable(
                                expression.operation.input().expect("guard transport input"),
                            ),
                            category: crate::EventFaultCategory::SpaceMismatch,
                            expected_space: expected.full().to_bytes(control)?.into_boxed_slice(),
                            offending_value: input.to_bytes(control)?.into_boxed_slice(),
                        })?;
                    }
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            }
        }
        let event = interpretation.event(expression.operation, input.as_ref(), control)?;
        let words = interner.intern_event(&event)?.key().words();
        self.bindings.set(slot, words[0]);
        self.bindings.set(slot + 1, words[1]);
        Ok(true)
    }
}
