//! Complete-binding admission and evaluation. Validation walks written operands
//! independently of value evaluation; saturation never erases participation.
use std::collections::BTreeMap;

use super::{Bindings, OutputProgram};
use crate::event::{BoolOp4, Control, FixedPointLimits, ModalOp, Space, WorldRelation};
use crate::work::{GenerationHandle, WorkContext};
use crate::{
    Error, Event, EventExpr, EventFaultCategory, EventOperandFault, EventScope, EventTest,
    FindTerm, FixedPointKind, RelationExpr, RelationProductOp, RelationViewOp, VarId,
};

#[derive(Default)]
pub(super) struct Faults {
    values: Vec<EventOperandFault>,
    bytes: usize,
    refusal: Option<crate::event::Error>,
}

impl Faults {
    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.bytes = 0;
        self.refusal = None;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub(super) fn insert(&mut self, fault: EventOperandFault) -> crate::Result<()> {
        if let Some(error) = self.refusal {
            return Err(error.into());
        }
        let Err(position) = self.values.binary_search(&fault) else {
            return Ok(());
        };
        let bytes = fault
            .expected_space
            .len()
            .saturating_add(fault.offending_value.len())
            .saturating_add(std::mem::size_of::<EventOperandFault>());
        if self.values.len() == 65_536 || self.bytes.saturating_add(bytes) > 16 * 1024 * 1024 {
            let error = crate::event::Error::Capacity(crate::event::Capacity::Diagnostics);
            self.refusal = Some(error);
            return Err(error.into());
        }
        if self.values.try_reserve(1).is_err() {
            self.refusal = Some(crate::event::Error::Allocation);
            return Err(crate::event::Error::Allocation.into());
        }
        self.bytes += bytes;
        self.values.insert(position, fault);
        Ok(())
    }

    pub(super) fn finish(&mut self) -> crate::Result<()> {
        if let Some(error) = self.refusal {
            return Err(error.into());
        }
        if self.is_empty() {
            return Ok(());
        }
        self.bytes = 0;
        Err(Error::EventFaults(
            std::mem::take(&mut self.values).into_boxed_slice(),
        ))
    }
}

/// Output is absent only when this binding had a semantic fault; its complete
/// descriptors remain in `faults`. This absence is never published as a row.
pub(super) fn evaluate(
    bindings: &Bindings,
    program: &OutputProgram,
    stage: Option<usize>,
    generation: &GenerationHandle,
    work: &WorkContext,
    faults: &mut Faults,
) -> crate::Result<Option<[u64; 4]>> {
    let variables = program
        .expression
        .event_variables()
        .expect("Event operands");
    work.checkpoint()
        .map_err(super::super::source::work_error)?;
    let interner = crate::image::intern::InternerHandle::new(generation, work);
    let mut values = BTreeMap::new();
    for &(var, slot, _) in &program.inputs {
        values.insert(
            var,
            interner.resolve_event([bindings.get(slot), bindings.get(slot + 1)])?,
        );
    }
    let roots = match &program.expression {
        FindTerm::Event(expr) => vec![expr],
        FindTerm::Test(test) => test.roots(),
        FindTerm::Probability { event, given } => vec![event, given],
        _ => unreachable!("Event output"),
    };
    let space = roots
        .iter()
        .find_map(|expr| expr.output_space().cloned())
        .unwrap_or_else(|| values[&variables[0]].space());
    let mut admission = ScopeAdmission {
        values: &values,
        program,
        stage,
        work,
        faults,
        operand: 0,
        refused: false,
        inputs: Vec::new(),
    };
    admission
        .inputs
        .try_reserve_exact(variables.len())
        .map_err(crate::event::Error::from)?;
    for root in roots {
        admission.visit(root, &space)?;
    }
    if admission.refused {
        return Ok(None);
    }
    let mut inputs = admission.inputs.iter();
    let mut evaluation = Evaluation::default();
    let mut words = [0; 4];
    match &program.expression {
        FindTerm::Event(expr) => words[..2].copy_from_slice(
            &interner
                .intern_event(&region(expr, &mut inputs, work, &mut evaluation)?)?
                .key()
                .words(),
        ),
        FindTerm::Test(expr) => {
            words[0] = u64::from(test(expr, &mut inputs, work, &mut evaluation)?);
        }
        FindTerm::Probability { event, given } => {
            let event = region(event, &mut inputs, work, &mut evaluation)?;
            let given = region(given, &mut inputs, work, &mut evaluation)?;
            if !event.space().is_measured() {
                return Err(crate::event::Error::MissingLaw.into());
            }
            words[..2].copy_from_slice(&interner.intern_event(&event)?.key().words());
            words[2..].copy_from_slice(&interner.intern_event(&given)?.key().words());
        }
        _ => unreachable!("only Event programs enter this evaluator"),
    }
    Ok(Some(words))
}

struct ScopeAdmission<'a> {
    values: &'a BTreeMap<VarId, Event>,
    program: &'a OutputProgram,
    stage: Option<usize>,
    work: &'a WorkContext,
    faults: &'a mut Faults,
    operand: usize,
    refused: bool,
    inputs: Vec<Event>,
}

impl ScopeAdmission<'_> {
    fn visit(&mut self, expr: &EventExpr, expected: &Space) -> crate::Result<()> {
        self.work
            .checkpoint()
            .map_err(super::super::source::work_error)?;
        match expr {
            EventExpr::Bound(_) => Ok(()),
            EventExpr::FixedPoint { scope, body, .. } => self.visit(body, scope.carrier().space()),
            EventExpr::Var(var) | EventExpr::Empty(var) | EventExpr::Full(var) => {
                self.leaf(*var, expected)
            }
            EventExpr::Map {
                operation,
                map,
                input,
            } => {
                let (required, _) = map.map_spaces(*operation).expect("validated map import");
                self.visit(input, required)
            }
            EventExpr::Relation { relation, .. } => self.relation(relation),
            EventExpr::Modal {
                operation,
                relation,
                input,
            } => {
                self.relation(relation)?;
                let role = relation.role().expect("validated relation role");
                let context = if *operation == ModalOp::Post {
                    role.input()
                } else {
                    role.output()
                };
                self.visit(input, context.space)
            }
            _ => {
                for child in crate::event_expr::children(expr) {
                    self.visit(child, expected)?;
                }
                Ok(())
            }
        }
    }

    fn relation(&mut self, expr: &RelationExpr) -> crate::Result<()> {
        self.work
            .checkpoint()
            .map_err(super::super::source::work_error)?;
        match expr {
            RelationExpr::Bind { faces, region } => self.visit(
                region,
                faces.faces().expect("validated pair").region().space,
            ),
            RelationExpr::Identity { .. } => Ok(()),
            RelationExpr::Test { faces, predicate } => self.visit(
                predicate,
                faces.faces().expect("validated pair").input().space,
            ),
            RelationExpr::Not(value)
            | RelationExpr::Converse(value)
            | RelationExpr::Star {
                relation: value, ..
            } => self.relation(value),
            RelationExpr::Apply { left, right, .. } | RelationExpr::Product { left, right, .. } => {
                self.relation(left)?;
                self.relation(right)
            }
        }
    }

    fn leaf(&mut self, var: VarId, expected: &Space) -> crate::Result<()> {
        let operand = self.operand;
        self.operand += 1;
        match self.values[&var].align_to(expected, self.work) {
            Ok(value) => self.inputs.push(value),
            Err(crate::event::Error::SpaceMismatch) => {
                self.refused = true;
                let expected_space = expected.full().to_bytes(self.work)?;
                let offending_value = self.values[&var].to_bytes(self.work)?;
                for &rule in &self.program.rules {
                    self.faults.insert(EventOperandFault {
                        stage: self.stage,
                        rule,
                        find: self.program.find,
                        operand,
                        source: crate::EventOperandSource::Variable(var),
                        category: EventFaultCategory::SpaceMismatch,
                        expected_space: expected_space.clone().into_boxed_slice(),
                        offending_value: offending_value.clone().into_boxed_slice(),
                    })?;
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}

#[derive(Default)]
struct Evaluation {
    bounds: Vec<Event>,
    limits: FixedPointLimits,
    iterations: u64,
    steps: u64,
}

impl Evaluation {
    fn step(&mut self, control: &dyn Control) -> crate::event::Result<()> {
        control.checkpoint()?;
        if self.steps == self.limits.program_steps {
            return Err(crate::event::Error::Capacity(
                crate::event::Capacity::ProgramSteps,
            ));
        }
        self.steps += 1;
        Ok(())
    }
    fn application(&mut self) -> crate::event::Result<()> {
        if self.iterations == self.limits.iterations {
            return Err(crate::event::Error::Capacity(
                crate::event::Capacity::FixedPointIterations,
            ));
        }
        self.iterations += 1;
        Ok(())
    }
    fn remaining(&self) -> FixedPointLimits {
        FixedPointLimits {
            iterations: self.limits.iterations - self.iterations,
            program_steps: self.limits.program_steps - self.steps,
        }
    }
}

fn fixed_point(
    kind: FixedPointKind,
    scope: &EventScope,
    body: &EventExpr,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
    evaluation: &mut Evaluation,
) -> crate::event::Result<Event> {
    let carrier = scope.carrier();
    let greatest = kind == FixedPointKind::Greatest;
    let index = evaluation.bounds.len();
    evaluation.bounds.try_reserve(1)?;
    evaluation.bounds.push(if greatest {
        carrier.space().full()
    } else {
        carrier.space().empty()
    });
    // External leaves stay fixed. Each application consumes the same written
    // occurrence roster; the caller resumes just past it after stabilization.
    let start = inputs.clone();
    let result = (|| {
        for _ in 0..=carrier.atoms() {
            evaluation.application()?;
            *inputs = start.clone();
            let next =
                region(body, inputs, control, evaluation)?.align_to(carrier.space(), control)?;
            let previous = &evaluation.bounds[index];
            if next == *previous {
                return Ok(next);
            }
            let ordered = if greatest {
                next.signature(previous, control)?
            } else {
                previous.signature(&next, control)?
            };
            if !ordered.included() {
                return Err(crate::event::Error::FixedPointInvariant);
            }
            evaluation.bounds[index] = next;
        }
        Err(crate::event::Error::FixedPointInvariant)
    })();
    evaluation.bounds.truncate(index);
    result
}

fn region(
    expr: &EventExpr,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
    evaluation: &mut Evaluation,
) -> crate::event::Result<Event> {
    evaluation.step(control)?;
    match expr {
        EventExpr::Bound(depth) => {
            Ok(evaluation.bounds[evaluation.bounds.len() - 1 - usize::from(depth.0)].clone())
        }
        EventExpr::FixedPoint { kind, scope, body } => {
            fixed_point(*kind, scope, body, inputs, control, evaluation)
        }
        EventExpr::Var(_) => Ok(inputs.next().expect("admitted leaf occurrence").clone()),
        EventExpr::Empty(_) => Ok(inputs.next().expect("admitted anchor").space().empty()),
        EventExpr::Full(_) => Ok(inputs.next().expect("admitted anchor").space().full()),
        EventExpr::Not(a) => Ok(region(a, inputs, control, evaluation)?.complement()),
        EventExpr::Apply { op, left, right } => {
            let a = region(left, inputs, control, evaluation)?;
            let b = region(right, inputs, control, evaluation)?.align_to(&a.space(), control)?;
            a.apply(*op, &b, control)
        }
        EventExpr::Ite {
            condition,
            high,
            low,
        } => {
            let c = region(condition, inputs, control, evaluation)?;
            let h = region(high, inputs, control, evaluation)?.align_to(&c.space(), control)?;
            let l = region(low, inputs, control, evaluation)?.align_to(&c.space(), control)?;
            c.ite(&h, &l, control)
        }
        EventExpr::Cardinality {
            minimum,
            maximum,
            events,
        } => {
            let mut events = events
                .iter()
                .map(|expr| region(expr, inputs, control, evaluation))
                .collect::<crate::event::Result<Vec<_>>>()?;
            let space = events[0].space();
            for value in &mut events {
                *value = value.align_to(&space, control)?;
            }
            space.cardinality(&events, *minimum, *maximum, control)
        }
        EventExpr::Map {
            operation,
            map,
            input,
        } => map.evaluate_map(
            *operation,
            &region(input, inputs, control, evaluation)?,
            control,
        ),
        EventExpr::Relation {
            operation,
            relation: expr,
        } => {
            let value = relation(expr, inputs, control, evaluation)?;
            match operation {
                RelationViewOp::Region => Ok(value.region().clone()),
                RelationViewOp::Domain => value.domain(control),
                RelationViewOp::Range => value.range(control),
            }
        }
        EventExpr::Modal {
            operation,
            relation: expr,
            input,
        } => {
            let value = relation(expr, inputs, control, evaluation)?;
            let predicate = region(input, inputs, control, evaluation)?;
            match operation {
                ModalOp::May => value.may(&predicate, control),
                ModalOp::All => value.all(&predicate, control),
                ModalOp::Must => value.must(&predicate, control),
                ModalOp::Post => value.post(&predicate, control),
            }
        }
    }
}

fn relation(
    expr: &RelationExpr,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
    evaluation: &mut Evaluation,
) -> crate::event::Result<WorldRelation> {
    evaluation.step(control)?;
    match expr {
        RelationExpr::Bind {
            faces,
            region: input,
        } => WorldRelation::new(
            &faces.faces().expect("validated pair").product(),
            &region(input, inputs, control, evaluation)?,
            control,
        ),
        RelationExpr::Identity { faces } => {
            WorldRelation::identity(&faces.faces().expect("validated pair").product(), control)
        }
        RelationExpr::Test { faces, predicate } => WorldRelation::test(
            &faces.faces().expect("validated pair").product(),
            &region(predicate, inputs, control, evaluation)?,
            control,
        ),
        RelationExpr::Not(value) => Ok(relation(value, inputs, control, evaluation)?.complement()),
        RelationExpr::Converse(value) => {
            Ok(relation(value, inputs, control, evaluation)?.converse())
        }
        RelationExpr::Apply { op, left, right } => {
            let left = relation(left, inputs, control, evaluation)?;
            let right = relation(right, inputs, control, evaluation)?;
            left.apply(*op, &right, control)
        }
        RelationExpr::Product {
            operation,
            plan,
            left,
            right,
        } => {
            let left = relation(left, inputs, control, evaluation)?;
            let right = relation(right, inputs, control, evaluation)?;
            let (plan, _) = plan.product().expect("validated composition plan");
            match operation {
                RelationProductOp::Compose => plan.compose(&left, &right, control),
                RelationProductOp::LeftResidual => plan.left_residual(&left, &right, control),
                RelationProductOp::RightResidual => plan.right_residual(&left, &right, control),
            }
        }
        RelationExpr::Star {
            plan,
            relation: value,
        } => {
            let value = relation(value, inputs, control, evaluation)?;
            let plan = plan.product().expect("validated closure plan").0;
            let result = plan
                .star_program(&value, control)?
                .least(evaluation.remaining(), control)?;
            evaluation.iterations += result.iterations();
            evaluation.steps += result.program_steps();
            WorldRelation::new(plan.products()[2], result.event(), control)
        }
    }
}

fn test(
    expr: &EventTest,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
    evaluation: &mut Evaluation,
) -> crate::event::Result<bool> {
    match expr {
        EventTest::IsEmpty(a) => Ok(region(a, inputs, control, evaluation)?.is_empty()),
        EventTest::IsFull(a) => Ok(region(a, inputs, control, evaluation)?.is_full()),
        EventTest::Subset(a, b)
        | EventTest::Equal(a, b)
        | EventTest::Disjoint(a, b)
        | EventTest::Covers(a, b) => {
            let a = region(a, inputs, control, evaluation)?;
            let b = region(b, inputs, control, evaluation)?.align_to(&a.space(), control)?;
            let signature = a.signature(&b, control)?;
            Ok(match expr {
                EventTest::Subset(..) => signature.included(),
                EventTest::Equal(..) => signature.equal(),
                EventTest::Disjoint(..) => signature.disjoint(),
                EventTest::Covers(..) => signature.full(BoolOp4::OR),
                _ => unreachable!("binary test"),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_binders_share_iteration_and_instruction_budgets() {
        use crate::{FixedPointKind as K, PredicateDepth};
        let space = Space::new(crate::event::SpaceId([219; 32]), 1, &()).unwrap();
        let scope = EventScope::capture(&space, &()).unwrap();
        let expr = EventExpr::FixedPoint {
            kind: K::Least,
            scope: scope.clone(),
            body: Box::new(EventExpr::FixedPoint {
                kind: K::Greatest,
                scope,
                body: Box::new(EventExpr::Apply {
                    op: BoolOp4::OR,
                    left: Box::new(EventExpr::Bound(PredicateDepth(1))),
                    right: Box::new(EventExpr::Bound(PredicateDepth(0))),
                }),
            }),
        };
        expr.validate_shape().unwrap();
        for (iterations, steps, expected) in [
            (3, 100, Some(crate::event::Capacity::FixedPointIterations)),
            (4, 8, Some(crate::event::Capacity::ProgramSteps)),
            (4, 9, None),
        ] {
            let mut evaluation = Evaluation {
                limits: FixedPointLimits {
                    iterations,
                    program_steps: steps,
                },
                ..Evaluation::default()
            };
            let result = region(&expr, &mut [].iter(), &(), &mut evaluation);
            if let Some(capacity) = expected {
                assert_eq!(result.unwrap_err(), crate::event::Error::Capacity(capacity));
            } else {
                assert!(result.unwrap().is_full());
                assert_eq!((evaluation.iterations, evaluation.steps), (4, 9));
            }
            assert!(evaluation.bounds.is_empty());
        }
    }

    struct StopAfter(std::cell::Cell<usize>);
    impl Control for StopAfter {
        fn checkpoint(&self) -> crate::event::Result<()> {
            let left = self.0.get();
            if left == 0 {
                return Err(crate::event::Error::Cancelled);
            }
            self.0.set(left - 1);
            Ok(())
        }
    }

    #[test]
    fn star_inside_a_binder_shares_budgets_and_cancellation_never_returns_an_approximant() {
        use crate::event::{
            AdmittedDescriptor, CoordinateMap, DescriptorLimits, FibreProduct, RelationalProduct,
            SpaceId,
        };
        let states = Space::new(SpaceId([220; 32]), 1, &()).unwrap();
        let env = Space::new(SpaceId([221; 32]), 0, &()).unwrap();
        let base = CoordinateMap::coordinates(&states, &env, &[], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap();
        let pair = FibreProduct::new(SpaceId([222; 32]), &base, &base, &()).unwrap();
        let plan = RelationalProduct::new(SpaceId([223; 32]), &pair, &pair, &pair, &()).unwrap();
        let import =
            |value| crate::EventImport::capture(&value, DescriptorLimits::default(), &()).unwrap();
        let star = EventExpr::Relation {
            operation: RelationViewOp::Region,
            relation: Box::new(RelationExpr::Star {
                plan: import(AdmittedDescriptor::Composition(plan)),
                relation: Box::new(RelationExpr::Identity {
                    faces: import(AdmittedDescriptor::Fibre(pair.clone())),
                }),
            }),
        };
        let expr = EventExpr::FixedPoint {
            kind: FixedPointKind::Least,
            scope: EventScope::capture(pair.space(), &()).unwrap(),
            body: Box::new(star),
        };
        expr.validate_shape().unwrap();
        let mut completed = Evaluation::default();
        let expected = region(&expr, &mut [].iter(), &(), &mut completed).unwrap();
        assert!(
            completed.iterations > 2,
            "nested Star applications count too"
        );
        for limits in [
            FixedPointLimits {
                iterations: completed.iterations - 1,
                program_steps: completed.steps,
            },
            FixedPointLimits {
                iterations: completed.iterations,
                program_steps: completed.steps - 1,
            },
        ] {
            let mut limited = Evaluation {
                limits,
                ..Evaluation::default()
            };
            assert!(matches!(
                region(&expr, &mut [].iter(), &(), &mut limited),
                Err(crate::event::Error::Capacity(_))
            ));
            assert!(limited.bounds.is_empty());
        }
        for checks in [0, 1, 2, 3, 10, 25] {
            let mut cancelled = Evaluation::default();
            assert_eq!(
                region(
                    &expr,
                    &mut [].iter(),
                    &StopAfter(std::cell::Cell::new(checks)),
                    &mut cancelled
                )
                .unwrap_err(),
                crate::event::Error::Cancelled
            );
            assert!(cancelled.bounds.is_empty());
        }
        assert_eq!(
            region(&expr, &mut [].iter(), &(), &mut Evaluation::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn diagnostic_capacity_refuses_without_publishing_a_partial_set() {
        let value = EventOperandFault {
            stage: None,
            rule: 0,
            find: 0,
            operand: 1,
            source: crate::EventOperandSource::Variable(VarId(1)),
            category: EventFaultCategory::SpaceMismatch,
            expected_space: Box::new([]),
            offending_value: Box::new([]),
        };
        let mut faults = Faults::default();
        faults.insert(value.clone()).unwrap();
        faults.bytes = 16 * 1024 * 1024;
        // Exact duplicates consume no additional diagnostic budget.
        faults.insert(value.clone()).unwrap();
        let mut second = value;
        second.operand = 2;
        assert!(matches!(
            faults.insert(second),
            Err(Error::Event(crate::event::Error::Capacity(
                crate::event::Capacity::Diagnostics
            )))
        ));
        assert!(matches!(
            faults.finish(),
            Err(Error::Event(crate::event::Error::Capacity(
                crate::event::Capacity::Diagnostics
            )))
        ));
    }
}
