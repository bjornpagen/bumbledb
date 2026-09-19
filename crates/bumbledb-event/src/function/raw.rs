//! Internal scalar partitions over the raw Boolean cube. Unlike public Events,
//! intermediate cofactors/abstractions must not be completed against support.
use super::FunctionLimits;
use crate::arena::{Operation, Ref};
use crate::{BoolOp4, Capacity, Control, Error, ExactArithmetic, ExactRational, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Term {
    pub root: Ref,
    pub value: ExactRational,
}

pub(crate) struct Budget {
    pub limits: FunctionLimits,
    steps: usize,
}
impl Budget {
    pub fn new(limits: FunctionLimits, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        Ok(Self { limits, steps: 0 })
    }
    pub fn step(&mut self, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        if self.steps >= self.limits.steps {
            return Err(Error::Capacity(Capacity::FunctionSteps));
        }
        self.steps += 1;
        Ok(())
    }
    pub fn cells(&self, cells: usize) -> Result<()> {
        if cells > self.limits.cells {
            Err(Error::Capacity(Capacity::FunctionCells))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct Collector {
    cells: Vec<(Vec<u8>, Term)>,
    indices: HashMap<Vec<u8>, usize>,
}
impl Collector {
    fn push(
        &mut self,
        root: Ref,
        value: ExactRational,
        op: &mut Operation<'_>,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        budget.step(work.control())?;
        if root == 0 || value.is_zero() {
            return Ok(());
        }
        let bytes = value.to_bytes(work)?;
        if let Some(&index) = self.indices.get(&bytes) {
            self.cells[index].1.root = op.apply(BoolOp4::OR, self.cells[index].1.root, root)?;
        } else {
            budget.cells(self.cells.len() + 1)?;
            self.cells.try_reserve(1)?;
            self.indices.try_reserve(1)?;
            self.indices.insert(bytes.clone(), self.cells.len());
            self.cells.push((bytes, Term { root, value }));
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Vec<Term>> {
        self.cells.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut result = Vec::new();
        result.try_reserve_exact(self.cells.len())?;
        result.extend(self.cells.into_iter().map(|(_, term)| term));
        Ok(result)
    }
}

pub(crate) fn merge(
    terms: Vec<Term>,
    op: &mut Operation<'_>,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<Term>> {
    let mut out = Collector::default();
    for term in terms {
        out.push(term.root, term.value, op, budget, work)?;
    }
    out.finish()
}

pub(crate) fn mask(
    terms: &[Term],
    predicate: Ref,
    op: &mut Operation<'_>,
    budget: &mut Budget,
) -> Result<Vec<Term>> {
    budget.cells(terms.len())?;
    let mut out = Vec::new();
    out.try_reserve_exact(terms.len())?;
    for term in terms {
        op.step()?;
        let root = op.apply(BoolOp4::AND, term.root, predicate)?;
        if root != 0 {
            out.push(Term {
                root,
                value: term.value.clone(),
            });
        }
    }
    Ok(out)
}

fn coverage(terms: &[Term], op: &mut Operation<'_>) -> Result<Ref> {
    let mut covered = 0;
    for term in terms {
        covered = op.apply(BoolOp4::OR, covered, term.root)?;
    }
    Ok(covered)
}

pub(crate) fn add(
    left: &[Term],
    right: &[Term],
    op: &mut Operation<'_>,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<Term>> {
    let mut out = Collector::default();
    let a = coverage(left, op)?;
    let b = coverage(right, op)?;
    for term in left {
        let root = op.apply(BoolOp4::DIFFERENCE, term.root, b)?;
        out.push(root, term.value.clone(), op, budget, work)?;
    }
    for term in right {
        let root = op.apply(BoolOp4::DIFFERENCE, term.root, a)?;
        out.push(root, term.value.clone(), op, budget, work)?;
    }
    for first in left {
        for second in right {
            budget.step(work.control())?;
            let root = op.apply(BoolOp4::AND, first.root, second.root)?;
            if root != 0 {
                out.push(
                    root,
                    first.value.add(&second.value, work)?,
                    op,
                    budget,
                    work,
                )?;
            }
        }
    }
    out.finish()
}

pub(crate) fn multiply(
    left: &[Term],
    right: &[Term],
    op: &mut Operation<'_>,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<Term>> {
    let mut out = Collector::default();
    for first in left {
        for second in right {
            budget.step(work.control())?;
            let root = op.apply(BoolOp4::AND, first.root, second.root)?;
            if root != 0 {
                out.push(
                    root,
                    first.value.mul(&second.value, work)?,
                    op,
                    budget,
                    work,
                )?;
            }
        }
    }
    out.finish()
}

pub(crate) fn abstract_sum(
    mut terms: Vec<Term>,
    hidden: u64,
    op: &mut Operation<'_>,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<Term>> {
    let mut hidden = hidden;
    while hidden != 0 {
        budget.step(work.control())?;
        let coordinate =
            u8::try_from(hidden.trailing_zeros()).map_err(|_| Error::FunctionInvariant)?;
        hidden &= hidden - 1;
        let mut low = Vec::new();
        let mut high = Vec::new();
        low.try_reserve_exact(terms.len())?;
        high.try_reserve_exact(terms.len())?;
        for term in &terms {
            low.push(Term {
                root: op.cofactor(term.root, coordinate, false)?,
                value: term.value.clone(),
            });
            high.push(Term {
                root: op.cofactor(term.root, coordinate, true)?,
                value: term.value.clone(),
            });
        }
        terms = add(&low, &high, op, budget, work)?;
    }
    Ok(terms)
}

pub(crate) fn select(
    bit: Ref,
    low: &[Term],
    high: &[Term],
    op: &mut Operation<'_>,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<Term>> {
    let mut out = Collector::default();
    for term in low {
        let root = op.apply(BoolOp4::DIFFERENCE, term.root, bit)?;
        out.push(root, term.value.clone(), op, budget, work)?;
    }
    for term in high {
        let root = op.apply(BoolOp4::AND, term.root, bit)?;
        out.push(root, term.value.clone(), op, budget, work)?;
    }
    out.finish()
}
