//! Gluing local values into a complete observable on an Event. Overlapping
//! patches must agree pointwise; numerical mass never certifies coverage.
use super::raw::Budget;
use crate::{
    BoolOp4, Error, Event, ExactArithmetic, ExactRational, FamilyFunction, FamilyFunctionPiece,
    FiniteFunction, FunctionLimits, FunctionPiece, ParameterSourceLimits, Result, Space,
};

/// A total source-owned function supplied on a named region. Values outside the
/// region are not supplied by this patch, even if the function is zero there.
#[derive(Debug, Clone)]
pub struct FunctionPatch<F> {
    pub region: Event,
    pub function: F,
}

/// Checked pointwise agreement and complete coverage on `parent`. The assembled
/// function is zero outside parent; that extension is not additional coverage.
/// Original aligned patches retain supplied zeros, overlaps and empty regions.
#[derive(Debug, Clone)]
pub struct FunctionCover<F> {
    parent: Event,
    patches: Box<[FunctionPatch<F>]>,
    function: F,
}
impl<F> FunctionCover<F> {
    #[must_use]
    pub fn parent(&self) -> &Event {
        &self.parent
    }
    #[must_use]
    pub fn patches(&self) -> &[FunctionPatch<F>] {
        &self.patches
    }
    #[must_use]
    pub fn function(&self) -> &F {
        &self.function
    }
}

trait Glue: Sized {
    type Limits: Copy;
    fn bounds(limits: Self::Limits) -> FunctionLimits;
    fn align(
        &self,
        space: &Space,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self>;
    fn zero(space: &Space, limits: Self::Limits, work: &mut ExactArithmetic<'_>) -> Result<Self>;
    fn masked(
        &self,
        region: &Event,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self>;
    fn equal(
        &self,
        other: &Self,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool>;
    fn plus(
        &self,
        other: &Self,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self>;
}

fn glue<F: Glue>(
    parent: &Event,
    patches: &[FunctionPatch<F>],
    limits: F::Limits,
    work: &mut ExactArithmetic<'_>,
) -> Result<FunctionCover<F>> {
    let space = parent.space();
    let mut budget = Budget::new(F::bounds(limits), work.control())?;
    budget.cells(patches.len())?;
    let mut aligned = Vec::new();
    aligned.try_reserve_exact(patches.len())?;
    // Context admission precedes overlap/gap checks and empty/full shortcuts.
    for patch in patches {
        budget.step(work.control())?;
        aligned.push(FunctionPatch {
            region: patch.region.align_to(&space, work.control())?,
            function: patch.function.align(&space, limits, work)?,
        });
    }
    let mut covered = space.empty();
    let mut function = F::zero(&space, limits, work)?;
    for patch in &aligned {
        budget.step(work.control())?;
        let region = patch.region.apply(BoolOp4::AND, parent, work.control())?;
        let overlap = region.apply(BoolOp4::AND, &covered, work.control())?;
        if !patch.function.masked(&overlap, limits, work)?.equal(
            &function.masked(&overlap, limits, work)?,
            limits,
            work,
        )? {
            return Err(Error::FunctionCoverConflict);
        }
        let fresh = region.apply(BoolOp4::DIFFERENCE, &covered, work.control())?;
        function = function.plus(&patch.function.masked(&fresh, limits, work)?, limits, work)?;
        covered = covered.apply(BoolOp4::OR, &region, work.control())?;
    }
    if covered != *parent {
        return Err(Error::FunctionCoverGap);
    }
    work.control().checkpoint()?;
    Ok(FunctionCover {
        parent: parent.clone(),
        patches: aligned.into(),
        function,
    })
}

impl FiniteFunction {
    /// Zero this total function outside an Event without changing its source.
    /// # Errors
    /// Context, function/arithmetic capacities or cancellation.
    pub fn mask(
        &self,
        region: &Event,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let region = region.align_to(self.space(), work.control())?;
        let mut budget = Budget::new(limits, work.control())?;
        budget.cells(self.pieces().len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.pieces().len())?;
        for (cell, value) in self.pieces() {
            budget.step(work.control())?;
            pieces.push(FunctionPiece {
                region: cell.apply(BoolOp4::AND, &region, work.control())?,
                value: value.clone(),
            });
        }
        Self::new(self.space(), &pieces, limits, work)
    }
    /// Assemble supplied local functions. Agreeing overlaps do not add or
    /// multiply the value. Coverage is structural, including zero-mass worlds.
    /// # Errors
    /// Foreign contexts, disagreeing overlap, missing coverage, limits or cancellation.
    pub fn glue(
        parent: &Event,
        patches: &[FunctionPatch<Self>],
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<FunctionCover<Self>> {
        glue(parent, patches, limits, work)
    }
}
impl Glue for FiniteFunction {
    type Limits = FunctionLimits;
    fn bounds(limits: Self::Limits) -> FunctionLimits {
        limits
    }
    fn align(
        &self,
        space: &Space,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.align_to(space, limits, work)
    }
    fn zero(space: &Space, limits: Self::Limits, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        Self::constant(space, ExactRational::zero(), limits, work)
    }
    fn masked(
        &self,
        region: &Event,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.mask(region, limits, work)
    }
    fn equal(&self, other: &Self, _: Self::Limits, work: &mut ExactArithmetic<'_>) -> Result<bool> {
        self.equivalent(other, work.control())
    }
    fn plus(
        &self,
        other: &Self,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.add(other, limits, work)
    }
}

impl FamilyFunction {
    /// Zero this total function outside an Event. Actual parameter holes are
    /// never filled; the supplied function is already admitted on every world.
    /// # Errors
    /// Context, parameter/function/arithmetic capacities or cancellation.
    pub fn mask(
        &self,
        region: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let region = region.align_to(self.space(), work.control())?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        budget.cells(self.pieces().len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.pieces().len())?;
        for (cell, value) in self.pieces() {
            budget.step(work.control())?;
            pieces.push(FamilyFunctionPiece {
                region: cell.apply(BoolOp4::AND, &region, work.control())?,
                value: value.clone(),
            });
        }
        Self::new(self.space(), &pieces, limits, work)
    }
    /// Assemble compatible local parameter-dependent values with exact coverage.
    /// Agreement is checked on actual overlap worlds, including guard points;
    /// it does not require the functions to agree outside their supplied regions.
    /// # Errors
    /// Foreign contexts, disagreement, missing coverage, limits or cancellation.
    pub fn glue(
        parent: &Event,
        patches: &[FunctionPatch<Self>],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<FunctionCover<Self>> {
        glue(parent, patches, limits, work)
    }
}
impl Glue for FamilyFunction {
    type Limits = ParameterSourceLimits;
    fn bounds(limits: Self::Limits) -> FunctionLimits {
        limits.functions
    }
    fn align(
        &self,
        space: &Space,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.align_to(space, limits, work)
    }
    fn zero(space: &Space, limits: Self::Limits, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        Self::new(space, &[], limits, work)
    }
    fn masked(
        &self,
        region: &Event,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.mask(region, limits, work)
    }
    fn equal(
        &self,
        other: &Self,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        self.equivalent(other, limits, work)
    }
    fn plus(
        &self,
        other: &Self,
        limits: Self::Limits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.add(other, limits, work)
    }
}
