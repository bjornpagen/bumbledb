//! Exact finite scalar functions on Event spaces. Scalar cells are arithmetic
//! objects, never schema weight columns. Boolean regions remain first class.
use std::sync::Arc;

use crate::arena::Operation;
use crate::{
    BoolOp4, Control, DensityPiece, Error, Event, ExactArithmetic, ExactRational, LawLimits,
    Result, Space,
};

mod image;
pub(crate) mod raw;

use raw::{Budget, Term};

/// Per-operation finite-function shape/work bounds, separate from graph and
/// exact-arithmetic budgets. These are not a global retained-memory quota.
#[derive(Debug, Clone, Copy)]
pub struct FunctionLimits {
    pub cells: usize,
    pub steps: usize,
    pub memo_entries: usize,
}
impl Default for FunctionLimits {
    fn default() -> Self {
        Self {
            cells: 65_536,
            steps: 1_000_000,
            memo_entries: 100_000,
        }
    }
}

/// One piece of an exact scalar function. Inputs are disjoint, including zero
/// pieces. Unspecified legal worlds receive zero. Signed values are allowed.
#[derive(Debug, Clone)]
pub struct FunctionPiece {
    pub region: Event,
    pub value: ExactRational,
}

#[derive(Debug, Clone)]
pub struct FiniteFunction {
    space: Space,
    terms: Arc<[Term]>,
}

impl FiniteFunction {
    /// Admit a piecewise exact function and merge equal nonzero values.
    /// # Errors
    /// Context mismatch, overlapping pieces, graph/arithmetic/shape limits or
    /// cancellation. Inputs are all validated before zero/empty simplification.
    pub fn new(
        space: &Space,
        pieces: &[FunctionPiece],
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        budget.cells(pieces.len())?;
        let mut terms = Vec::new();
        terms.try_reserve_exact(pieces.len())?;
        for piece in pieces {
            budget.step(control)?;
            let region = piece.region.align_to(space, control)?;
            piece.value.to_bytes(work)?;
            terms.push(Term {
                root: region.root,
                value: piece.value.clone(),
            });
        }
        let mut arena = space.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let mut covered = 0;
        for term in &mut terms {
            term.root = op.apply(BoolOp4::AND, space.0.support, term.root)?;
            if op.apply(BoolOp4::AND, covered, term.root)? != 0 {
                return Err(Error::FunctionOverlap);
            }
            covered = op.apply(BoolOp4::OR, covered, term.root)?;
        }
        Self::publish(space, &terms, &mut op, &mut budget, work)
    }

    /// # Errors
    /// Scalar extent, function/graph limits or cancellation.
    pub fn constant(
        space: &Space,
        value: ExactRational,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::new(
            space,
            &[FunctionPiece {
                region: space.full(),
                value,
            }],
            limits,
            work,
        )
    }

    /// Capture the space's designated joint density without changing its law.
    /// # Errors
    /// Missing law, limits or cancellation.
    pub fn density(
        space: &Space,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let pieces = space.density_pieces().ok_or(Error::MissingLaw)?;
        let mut inputs = Vec::new();
        inputs.try_reserve_exact(pieces.len())?;
        for (region, density) in pieces {
            inputs.push(FunctionPiece {
                region,
                value: density.clone(),
            });
        }
        Self::new(space, &inputs, limits, work)
    }

    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }
    #[must_use]
    pub fn is_nonnegative(&self) -> bool {
        self.terms.iter().all(|term| !term.value.is_negative())
    }
    pub fn pieces(&self) -> impl ExactSizeIterator<Item = (Event, &ExactRational)> {
        self.terms
            .iter()
            .map(|term| (self.space.event(term.root), &term.value))
    }
    /// Read an exact value at a legal world; this never samples a source.
    /// # Errors
    /// Illegal world, poisoned owner or cancellation.
    pub fn at(&self, world: u64, control: &dyn Control) -> Result<ExactRational> {
        control.checkpoint()?;
        self.space.full().contains(world)?;
        for (region, value) in self.pieces() {
            control.checkpoint()?;
            if region.contains(world)? {
                return Ok(value.clone());
            }
        }
        Ok(ExactRational::zero())
    }

    /// Align an independently owned presentation of exactly this named context.
    /// # Errors
    /// Context, graph/arithmetic/function limits or cancellation.
    pub fn align_to(
        &self,
        target: &Space,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.space.full().align_to(target, work.control())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.terms.len())?;
        for (region, value) in self.pieces() {
            pieces.push(FunctionPiece {
                region,
                value: value.clone(),
            });
        }
        Self::new(target, &pieces, limits, work)
    }

    /// Checked cross-owner equality includes the designated space context.
    /// # Errors
    /// Context mismatch, graph limits or cancellation.
    pub fn equivalent(&self, other: &Self, control: &dyn Control) -> Result<bool> {
        other.space.full().align_to(&self.space, control)?;
        if self.terms.len() != other.terms.len() {
            return Ok(false);
        }
        for ((a, left), (b, right)) in self.pieces().zip(other.pieces()) {
            if left != right || a != b.align_to(&self.space, control)? {
                return Ok(false);
            }
        }
        control.checkpoint()?;
        Ok(true)
    }

    /// Pointwise addition; overlapping regions add their values exactly.
    /// # Errors
    /// Context, graph/arithmetic/shape/work limits or cancellation.
    pub fn add(
        &self,
        other: &Self,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, false, limits, work)
    }
    /// Pointwise multiplication, never an implicit independence assertion.
    /// # Errors
    /// As `add`.
    pub fn multiply(
        &self,
        other: &Self,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, true, limits, work)
    }
    fn binary(
        &self,
        other: &Self,
        multiply: bool,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let mut right = Vec::new();
        right.try_reserve_exact(other.terms.len())?;
        other.space.full().align_to(&self.space, control)?;
        for (region, value) in other.pieces() {
            right.push(Term {
                root: region.align_to(&self.space, control)?.root,
                value: value.clone(),
            });
        }
        let mut arena = self.space.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let mut budget = Budget::new(limits, control)?;
        let left = self.supported(&mut op, &mut budget)?;
        let right = raw::mask(&right, self.space.0.support, &mut op, &mut budget)?;
        let terms = if multiply {
            raw::multiply(&left, &right, &mut op, &mut budget, work)?
        } else {
            raw::add(&left, &right, &mut op, &mut budget, work)?
        };
        Self::publish(&self.space, &terms, &mut op, &mut budget, work)
    }

    /// Designate this normalized nonnegative function as a new joint law.
    /// # Errors
    /// Negative values, non-unit total, limits or cancellation; no rescaling.
    pub fn designate(&self, limits: LawLimits, work: &mut ExactArithmetic<'_>) -> Result<Space> {
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.terms.len())?;
        for (region, value) in self.pieces() {
            pieces.push(DensityPiece {
                region,
                density: value.clone(),
            });
        }
        self.space.with_density(&pieces, limits, work)
    }

    pub(crate) fn supported(
        &self,
        op: &mut Operation<'_>,
        budget: &mut Budget,
    ) -> Result<Vec<Term>> {
        raw::mask(&self.terms, self.space.0.support, op, budget)
    }
    pub(crate) fn publish(
        space: &Space,
        terms: &[Term],
        op: &mut Operation<'_>,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let terms = raw::mask(terms, space.0.support, op, budget)?;
        let mut terms = raw::merge(terms, op, budget, work)?;
        for term in &mut terms {
            term.root = space.0.complete(op, term.root)?;
        }
        work.control().checkpoint()?;
        Ok(Self {
            space: space.clone(),
            terms: terms.into(),
        })
    }
}
