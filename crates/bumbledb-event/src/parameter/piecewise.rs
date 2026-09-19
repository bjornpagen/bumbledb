//! Partial piecewise rational functions over one shared parameter assignment.
//! Pieces select disjoint domains; they are never stochastic alternatives.
use crate::function::raw::Budget;
use crate::{
    BoolOp4, Error, ExactArithmetic, ExactRational, FunctionLimits, GuardedRationalFunction,
    ParameterDomain, ParameterLimits, ParameterRegion, PolynomialSigns, Result,
};

/// An exact partial function on an inhabited ambient parameter domain. Pieces
/// have disjoint defined regions. Outside their union the result is undefined,
/// not zero. Empty pieces are discarded only after validating all their inputs.
#[derive(Debug, Clone)]
pub struct ParameterFunction {
    ambient: ParameterDomain,
    defined: ParameterRegion,
    pieces: Box<[GuardedRationalFunction]>,
}

impl ParameterFunction {
    /// # Errors
    /// Foreign domains, overlapping pieces, capacities or cancellation.
    pub fn new(
        ambient: ParameterDomain,
        pieces: &[GuardedRationalFunction],
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut budget = Budget::new(functions, work.control())?;
        budget.cells(pieces.len())?;
        ambient
            .region()
            .equivalent(ambient.region(), parameters, work)?;
        let mut retained = Vec::new();
        retained.try_reserve_exact(pieces.len())?;
        let mut defined = ParameterRegion::empty(ambient.parameter());
        for piece in pieces {
            budget.step(work.control())?;
            if !ambient
                .region()
                .equivalent(piece.ambient().region(), parameters, work)?
            {
                return Err(Error::ParameterDomainMismatch);
            }
            let piece = piece.restrict(ambient.region(), parameters, work)?;
            if !defined
                .apply(BoolOp4::AND, piece.defined_on(), parameters, work)?
                .is_empty()
            {
                return Err(Error::FunctionOverlap);
            }
            defined = defined.apply(BoolOp4::OR, piece.defined_on(), parameters, work)?;
            if !piece.is_nowhere_defined() {
                retained.push(piece);
            }
        }
        budget.step(work.control())?;
        Ok(Self {
            ambient,
            defined,
            pieces: retained.into_boxed_slice(),
        })
    }
    #[must_use]
    pub fn ambient(&self) -> &ParameterDomain {
        &self.ambient
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        &self.defined
    }
    #[must_use]
    pub fn pieces(&self) -> &[GuardedRationalFunction] {
        &self.pieces
    }
    #[must_use]
    pub fn is_nowhere_defined(&self) -> bool {
        self.defined.is_empty()
    }

    /// # Errors
    /// Input/intermediate capacities or cancellation, including outside points.
    pub fn value_at(
        &self,
        value: &ExactRational,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ExactRational>> {
        let mut budget = Budget::new(functions, work.control())?;
        budget.cells(self.pieces.len())?;
        work.validate(value)?;
        self.ambient
            .region()
            .equivalent(self.ambient.region(), parameters, work)?;
        let mut result = None;
        for piece in &self.pieces {
            budget.step(work.control())?;
            if let Some(value) = piece.value_at(value, parameters, work)? {
                result = Some(value);
            }
        }
        Ok(result)
    }
    /// # Errors
    /// Input/intermediate capacities or cancellation. Undefined points are
    /// excluded for every sign request, including `ANY`.
    pub fn where_sign(
        &self,
        signs: PolynomialSigns,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterRegion> {
        let mut budget = Budget::new(functions, work.control())?;
        budget.cells(self.pieces.len())?;
        self.ambient
            .region()
            .equivalent(self.ambient.region(), parameters, work)?;
        let mut result = ParameterRegion::empty(self.ambient.parameter());
        for piece in &self.pieces {
            budget.step(work.control())?;
            result = result.apply(
                BoolOp4::OR,
                &piece.where_sign(signs, parameters, work)?,
                parameters,
                work,
            )?;
        }
        Ok(result)
    }
    /// # Errors
    /// Foreign scope, capacities or cancellation; retains the ambient domain.
    pub fn restrict(
        &self,
        region: &ParameterRegion,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.defined.apply(BoolOp4::AND, region, parameters, work)?;
        let mut pieces = Vec::new();
        let mut budget = Budget::new(functions, work.control())?;
        budget.cells(self.pieces.len())?;
        pieces.try_reserve_exact(self.pieces.len())?;
        for piece in &self.pieces {
            budget.step(work.control())?;
            pieces.push(piece.restrict(region, parameters, work)?);
        }
        Self::new(self.ambient.clone(), &pieces, parameters, functions, work)
    }
    /// # Errors
    /// Domain mismatch, capacities or cancellation. Both inputs retain holes.
    pub fn add(
        &self,
        other: &Self,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Add, parameters, functions, work)
    }
    /// # Errors
    /// As `add`.
    pub fn sub(
        &self,
        other: &Self,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Subtract, parameters, functions, work)
    }
    /// # Errors
    /// As `add`; multiplication by zero does not erase undefined points.
    pub fn mul(
        &self,
        other: &Self,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Multiply, parameters, functions, work)
    }
    /// # Errors
    /// As `add`; divisor zeros are additionally excluded.
    pub fn div(
        &self,
        other: &Self,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Divide, parameters, functions, work)
    }
    fn binary(
        &self,
        other: &Self,
        kind: Binary,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.where_sign(PolynomialSigns::ANY, parameters, functions, work)?;
        other.where_sign(PolynomialSigns::ANY, parameters, functions, work)?;
        if !self
            .ambient
            .region()
            .equivalent(other.ambient.region(), parameters, work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        let mut budget = Budget::new(functions, work.control())?;
        let mut pieces = Vec::new();
        for a in &self.pieces {
            for b in &other.pieces {
                budget.step(work.control())?;
                let value = match kind {
                    Binary::Add => a.add(b, parameters, work)?,
                    Binary::Subtract => a.sub(b, parameters, work)?,
                    Binary::Multiply => a.mul(b, parameters, work)?,
                    Binary::Divide => a.div(b, parameters, work)?,
                };
                if !value.is_nowhere_defined() {
                    budget.cells(pieces.len() + 1)?;
                    pieces.try_reserve(1)?;
                    pieces.push(value);
                }
            }
        }
        Self::new(self.ambient.clone(), &pieces, parameters, functions, work)
    }
    /// Numerical partial-function equality, separate from source/provenance identity.
    /// # Errors
    /// Domain/scope mismatch, capacities or cancellation.
    pub fn equivalent(
        &self,
        other: &Self,
        parameters: ParameterLimits,
        functions: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        let difference = self.sub(other, parameters, functions, work)?;
        Ok(self.defined.equivalent(&other.defined, parameters, work)?
            && difference
                .where_sign(PolynomialSigns::NON_ZERO, parameters, functions, work)?
                .is_empty())
    }
}
#[derive(Clone, Copy)]
enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
}
