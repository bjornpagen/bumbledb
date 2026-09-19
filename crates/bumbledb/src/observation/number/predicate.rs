//! Source-retaining strict partial truth algebra. A predicate's identity is its
//! written derivation; numerical truth equivalence is an explicit operation.
use super::{ObservationNumber, ObservationNumberLimits};
use crate::Result;
use crate::event::{
    BoolOp4, Capacity, Error, ExactArithmetic, NumberPredicate, ParameterDomain, ParameterRegion,
    PolynomialSigns,
};
use std::sync::Arc;

mod guards;
mod import;
pub use guards::{PredicateEvents, PredicateRefinement};
pub use import::ObservationPredicateImport;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub enum ObservationPredicateExpr {
    Region {
        domain: ParameterDomain,
        region: ParameterRegion,
    },
    Sign {
        number: ObservationNumber,
        signs: PolynomialSigns,
    },
    Negate(ObservationPredicate),
    Binary {
        op: BoolOp4,
        left: ObservationPredicate,
        right: ObservationPredicate,
    },
    OnDomain {
        value: ObservationPredicate,
        domain: ParameterDomain,
    },
}

#[derive(Debug)]
struct PredicateData {
    expression: ObservationPredicateExpr,
    predicate: NumberPredicate,
    nodes: usize,
    depth: usize,
}

/// An exact true/false/undefined partition with its complete owned derivation.
/// Boolean operations are strict: both operands must be defined, even when the
/// selected truth table is constant. No source law or independence is invented.
#[derive(Debug, Clone)]
pub struct ObservationPredicate(Arc<PredicateData>);

fn shape(children: &[(usize, usize)], limits: ObservationNumberLimits) -> Result<(usize, usize)> {
    let mut nodes = 1usize;
    let mut depth = 1usize;
    for &(size, height) in children {
        nodes = nodes
            .checked_add(size)
            .ok_or(Error::Capacity(Capacity::ProgramNodes))?;
        depth = depth.max(height.saturating_add(1));
    }
    if nodes > limits.nodes || depth > limits.depth.min(256) {
        return Err(Error::Capacity(Capacity::ProgramNodes).into());
    }
    Ok((nodes, depth))
}

impl ObservationPredicate {
    /// A total membership predicate on an explicit ambient domain. The complete
    /// authored region remains in the derivation, including outside-domain parts.
    /// No measured source, law or independence claim is introduced.
    /// # Errors
    /// Foreign parameter names, expression/solver/arithmetic bounds or cancellation.
    pub fn region(
        domain: &ParameterDomain,
        region: &ParameterRegion,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[], limits)?;
        let predicate = NumberPredicate::region(domain, region, limits.numbers, work)?;
        Ok(Self::new(
            ObservationPredicateExpr::Region {
                domain: domain.clone(),
                region: region.clone(),
            },
            predicate,
            size,
        ))
    }

    fn new(
        expression: ObservationPredicateExpr,
        predicate: NumberPredicate,
        (nodes, depth): (usize, usize),
    ) -> Self {
        Self(Arc::new(PredicateData {
            expression,
            predicate,
            nodes,
            depth,
        }))
    }

    pub(super) fn where_sign(
        number: &ObservationNumber,
        signs: PolynomialSigns,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[number.expression_shape()], limits)?;
        let predicate = number.value().where_sign(signs, limits.numbers, work)?;
        Ok(Self::new(
            ObservationPredicateExpr::Sign {
                number: number.clone(),
                signs,
            },
            predicate,
            size,
        ))
    }

    fn expression_shape(&self) -> (usize, usize) {
        (self.0.nodes, self.0.depth)
    }

    #[must_use]
    pub fn expression(&self) -> &ObservationPredicateExpr {
        &self.0.expression
    }

    #[must_use]
    pub fn predicate(&self) -> &NumberPredicate {
        &self.0.predicate
    }

    /// Negation swaps true and false, leaving every undefined assignment intact.
    /// # Errors
    /// Expression/solver/arithmetic bounds or cancellation.
    pub fn negate(
        &self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self.expression_shape()], limits)?;
        self.predicate().validate(limits.numbers, work)?;
        Ok(Self::new(
            ObservationPredicateExpr::Negate(self.clone()),
            self.predicate().negate(),
            size,
        ))
    }

    /// Apply any binary truth table on the common defined domain, retaining both
    /// operands even for constant, duplicate or logically redundant branches.
    /// # Errors
    /// Domain mismatch, expression/solver/arithmetic bounds or cancellation.
    pub fn apply(
        &self,
        op: BoolOp4,
        rhs: &Self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self.expression_shape(), rhs.expression_shape()], limits)?;
        let predicate = self
            .predicate()
            .apply(op, rhs.predicate(), limits.numbers, work)?;
        Ok(Self::new(
            ObservationPredicateExpr::Binary {
                op,
                left: self.clone(),
                right: rhs.clone(),
            },
            predicate,
            size,
        ))
    }

    /// Restrict only the numerical ambient domain. Original observations and
    /// their source domains remain in the child derivation.
    /// # Errors
    /// Foreign/extended domains, expression/solver/arithmetic bounds or cancellation.
    pub fn on_domain(
        &self,
        domain: &ParameterDomain,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self.expression_shape()], limits)?;
        let predicate = self.predicate().on_domain(domain, limits.numbers, work)?;
        Ok(Self::new(
            ObservationPredicateExpr::OnDomain {
                value: self.clone(),
                domain: domain.clone(),
            },
            predicate,
            size,
        ))
    }

    /// Equality of the three truth regions, including undefinedness. This never
    /// identifies the predicates' original sources or their written expressions.
    /// # Errors
    /// Domain mismatch, expression/solver/arithmetic bounds or cancellation.
    pub fn equivalent(
        &self,
        rhs: &Self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        for value in [self, rhs] {
            if value.0.nodes > limits.nodes || value.0.depth > limits.depth.min(256) {
                return Err(Error::Capacity(Capacity::ProgramNodes).into());
            }
        }
        Ok(self
            .predicate()
            .equivalent(rhs.predicate(), limits.numbers, work)?)
    }
}
