//! Complete-binding admission and evaluation. Validation walks written operands
//! independently of value evaluation; saturation never erases participation.
use std::collections::BTreeMap;

use super::{Bindings, OutputProgram};
use crate::event::{BoolOp4, Control, Space};
use crate::work::{GenerationHandle, WorkContext};
use crate::{
    Error, Event, EventExpr, EventFaultCategory, EventOperandFault, EventTest, FindTerm, VarId,
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
) -> crate::Result<Option<[u64; 2]>> {
    let variables: Vec<_> = match &program.expression {
        FindTerm::Event(expr) => expr.variables().collect(),
        FindTerm::Test(test) => test.variables().collect(),
        _ => unreachable!("only Event programs enter this evaluator"),
    };
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
    let words = match &program.expression {
        FindTerm::Event(expr) => interner
            .intern_event(&region(expr, &mut inputs, work)?)?
            .key()
            .words(),
        FindTerm::Test(expr) => [u64::from(test(expr, &mut inputs, work)?), 0],
        _ => unreachable!("only Event programs enter this evaluator"),
    };
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
            _ => {
                for child in crate::event_expr::children(expr) {
                    self.visit(child, expected)?;
                }
                Ok(())
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
                        variable: var,
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

fn region(
    expr: &EventExpr,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
) -> crate::event::Result<Event> {
    control.checkpoint()?;
    match expr {
        EventExpr::Var(_) => Ok(inputs.next().expect("admitted leaf occurrence").clone()),
        EventExpr::Empty(_) => Ok(inputs.next().expect("admitted anchor").space().empty()),
        EventExpr::Full(_) => Ok(inputs.next().expect("admitted anchor").space().full()),
        EventExpr::Not(a) => Ok(region(a, inputs, control)?.complement()),
        EventExpr::Apply { op, left, right } => {
            let a = region(left, inputs, control)?;
            let b = region(right, inputs, control)?.align_to(&a.space(), control)?;
            a.apply(*op, &b, control)
        }
        EventExpr::Ite {
            condition,
            high,
            low,
        } => {
            let c = region(condition, inputs, control)?;
            let h = region(high, inputs, control)?.align_to(&c.space(), control)?;
            let l = region(low, inputs, control)?.align_to(&c.space(), control)?;
            c.ite(&h, &l, control)
        }
        EventExpr::Cardinality {
            minimum,
            maximum,
            events,
        } => {
            let mut events = events
                .iter()
                .map(|expr| region(expr, inputs, control))
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
        } => map.evaluate_map(*operation, &region(input, inputs, control)?, control),
    }
}

fn test(
    expr: &EventTest,
    inputs: &mut std::slice::Iter<'_, Event>,
    control: &dyn Control,
) -> crate::event::Result<bool> {
    match expr {
        EventTest::IsEmpty(a) => Ok(region(a, inputs, control)?.is_empty()),
        EventTest::IsFull(a) => Ok(region(a, inputs, control)?.is_full()),
        EventTest::Subset(a, b)
        | EventTest::Equal(a, b)
        | EventTest::Disjoint(a, b)
        | EventTest::Covers(a, b) => {
            let a = region(a, inputs, control)?;
            let b = region(b, inputs, control)?.align_to(&a.space(), control)?;
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
    fn diagnostic_capacity_refuses_without_publishing_a_partial_set() {
        let value = EventOperandFault {
            stage: None,
            rule: 0,
            find: 0,
            operand: 1,
            variable: VarId(1),
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
