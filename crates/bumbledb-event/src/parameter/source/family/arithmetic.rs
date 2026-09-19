use super::{Budget, FamilyFunction, FamilyFunctionPiece, ParameterSourceLimits, constant};
use crate::{BoolOp4, Error, Event, ExactArithmetic, ExactRational, PolynomialSigns, Result};

#[derive(Clone, Copy)]
enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl FamilyFunction {
    /// # Errors
    /// Foreign context, capacities or cancellation. Omitted worlds stay zero.
    pub fn add(
        &self,
        other: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Add, limits, work)
    }
    /// # Errors
    /// Same checked contract as `add`.
    pub fn sub(
        &self,
        other: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Subtract, limits, work)
    }
    /// Pointwise multiplication, without any independence assumption.
    /// # Errors
    /// Same checked contract as `add`.
    pub fn multiply(
        &self,
        other: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Multiply, limits, work)
    }
    /// Total pointwise division. A zero divisor at any legal world refuses;
    /// this never silently deletes worlds or shrinks the parameter domain.
    /// # Errors
    /// Undefined function, foreign context, capacities or cancellation.
    pub fn divide(
        &self,
        other: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Divide, limits, work)
    }

    fn complete(
        &self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<FamilyFunctionPiece>> {
        let mut budget = Budget::new(limits.functions, work.control())?;
        budget.cells(self.terms.len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.terms.len())?;
        let mut covered = self.space.empty();
        for (region, value) in self.pieces() {
            budget.step(work.control())?;
            covered = covered.apply(BoolOp4::OR, &region, work.control())?;
            pieces.push(FamilyFunctionPiece {
                region,
                value: value.clone(),
            });
        }
        if !covered.is_full() {
            budget.cells(pieces.len() + 1)?;
            pieces.try_reserve(1)?;
            pieces.push(FamilyFunctionPiece {
                region: covered.complement(),
                value: constant(
                    self.space
                        .parameter_domain()
                        .ok_or(Error::MissingParameter)?,
                    ExactRational::zero(),
                    limits,
                    work,
                )?,
            });
        }
        Ok(pieces)
    }

    fn binary(
        &self,
        other: &Self,
        kind: Binary,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let left = self
            .align_to(&self.space, limits, work)?
            .complete(limits, work)?;
        let right = other
            .align_to(&self.space, limits, work)?
            .complete(limits, work)?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        let mut pieces = Vec::new();
        for a in &left {
            for b in &right {
                budget.step(work.control())?;
                let region = a.region.apply(BoolOp4::AND, &b.region, work.control())?;
                if region.is_empty() {
                    continue;
                }
                let value = match kind {
                    Binary::Add => a.value.add(&b.value, limits.parameters.region, work)?,
                    Binary::Subtract => a.value.sub(&b.value, limits.parameters.region, work)?,
                    Binary::Multiply => a.value.mul(&b.value, limits.parameters.region, work)?,
                    Binary::Divide => a.value.div(&b.value, limits.parameters.region, work)?,
                };
                budget.cells(pieces.len() + 1)?;
                pieces.try_reserve(1)?;
                pieces.push(FamilyFunctionPiece { region, value });
            }
        }
        Self::new(&self.space, &pieces, limits, work)
    }

    /// Numerical equality at every actual world of the aligned source. Different
    /// rational definitions or cell decompositions can be equivalent.
    /// # Errors
    /// Foreign context, capacities or cancellation.
    pub fn equivalent(
        &self,
        other: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        self.sub(other, limits, work)?
            .has_no_sign(PolynomialSigns::NON_ZERO, limits, work)
    }

    /// # Errors
    /// Function/solver capacities or cancellation; checks actual active regions.
    pub fn is_nonnegative(
        &self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        self.align_to(&self.space, limits, work)?.has_no_sign(
            PolynomialSigns::NEGATIVE,
            limits,
            work,
        )
    }

    fn has_no_sign(
        &self,
        signs: PolynomialSigns,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        let mut budget = Budget::new(limits.functions, work.control())?;
        for (region, value) in self.pieces() {
            budget.step(work.control())?;
            let active = self.active(&region, limits, work)?;
            if !value
                .restrict(&active, limits.parameters.region, work)?
                .where_sign(signs, limits.parameters.region, work)?
                .is_empty()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Represent the exact sign set as an ordinary Event. A predicate cutting
    /// a sealed guard cell requires explicit refinement first; it is not rounded.
    /// # Errors
    /// Refinement required, function/solver capacities or cancellation.
    pub fn where_sign(
        &self,
        signs: PolynomialSigns,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Event> {
        let checked = self.align_to(&self.space, limits, work)?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        let mut result = self.space.empty();
        let mut covered = self.space.empty();
        for (region, value) in checked.pieces() {
            budget.step(work.control())?;
            let active = checked.active(&region, limits, work)?;
            let predicate = value
                .restrict(&active, limits.parameters.region, work)?
                .where_sign(signs, limits.parameters.region, work)?;
            let guard = self.space.parameter_event(&predicate, limits, work)?;
            result = result.apply(
                BoolOp4::OR,
                &region.apply(BoolOp4::AND, &guard, work.control())?,
                work.control(),
            )?;
            covered = covered.apply(BoolOp4::OR, &region, work.control())?;
        }
        if signs.contains(std::cmp::Ordering::Equal) {
            result = result.apply(BoolOp4::OR, &covered.complement(), work.control())?;
        }
        Ok(result)
    }
}
