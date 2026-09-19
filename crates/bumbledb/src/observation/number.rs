//! Source-retaining arithmetic over completed observations. Every numerical
//! result owns its written operands; cancellation and failure publish no node.
use std::sync::Arc;

use crate::event::{
    Capacity, ExactArithmetic, ExactRational, NumberLimits, NumberOp, ParameterDomain,
    PartialNumber, PolynomialSigns,
};
use crate::{ExpectationAnswer, ExpectationValue, ProbabilityAnswer, ProbabilityValue, Result};

mod import;
pub use import::{ObservationNumberCodecLimits, ObservationNumberImport};
mod predicate;
pub use predicate::{
    ObservationPredicate, ObservationPredicateExpr, ObservationPredicateImport, PredicateEvents,
    PredicateRefinement,
};

/// Selecting a numerical component never discards the original observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationComponent {
    Value,
    Numerator,
    EvidenceMass,
}

#[derive(Debug, Clone, Copy)]
pub struct ObservationNumberLimits {
    pub numbers: NumberLimits,
    /// Expanded expression size, including repeated occurrences of shared nodes.
    pub nodes: usize,
    pub depth: usize,
}
impl Default for ObservationNumberLimits {
    fn default() -> Self {
        Self {
            numbers: NumberLimits::default(),
            nodes: 65_536,
            depth: 256,
        }
    }
}

/// An inspectable owned expression. Numerical equivalence of two expressions
/// does not merge their evidence, sources, or payoff functions.
#[derive(Debug, Clone)]
pub enum ObservationNumberExpr {
    Literal(ExactRational),
    Probability {
        observation: Arc<ProbabilityAnswer>,
        component: ObservationComponent,
    },
    Expectation {
        observation: Arc<ExpectationAnswer>,
        component: ObservationComponent,
    },
    Binary {
        op: NumberOp,
        left: ObservationNumber,
        right: ObservationNumber,
    },
    Negate(ObservationNumber),
    Abs(ObservationNumber),
    Pow {
        value: ObservationNumber,
        exponent: u32,
    },
    OnDomain {
        value: ObservationNumber,
        domain: ParameterDomain,
    },
}

#[derive(Debug)]
struct NumberData {
    expression: ObservationNumberExpr,
    value: PartialNumber,
    nodes: usize,
    depth: usize,
}

/// An exact partial number and its complete immutable derivation. Source laws
/// are measured at the leaves; arithmetic never assumes stochastic independence.
#[derive(Debug, Clone)]
pub struct ObservationNumber(Arc<NumberData>);

fn shape(
    children: &[&ObservationNumber],
    limits: ObservationNumberLimits,
) -> Result<(usize, usize)> {
    let mut nodes = 1usize;
    let mut depth = 1usize;
    for child in children {
        nodes = nodes
            .checked_add(child.0.nodes)
            .ok_or(crate::event::Error::Capacity(Capacity::ProgramNodes))?;
        depth = depth.max(child.0.depth.saturating_add(1));
    }
    // A hard ceiling bounds recursive Debug/drop even if a caller raises limits.
    if nodes > limits.nodes || depth > limits.depth.min(256) {
        return Err(crate::event::Error::Capacity(Capacity::ProgramNodes).into());
    }
    Ok((nodes, depth))
}
impl ObservationNumber {
    pub(super) fn expression_shape(&self) -> (usize, usize) {
        (self.0.nodes, self.0.depth)
    }
    fn new(
        expression: ObservationNumberExpr,
        value: PartialNumber,
        (nodes, depth): (usize, usize),
    ) -> Self {
        Self(Arc::new(NumberData {
            expression,
            value,
            nodes,
            depth,
        }))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn literal(
        value: ExactRational,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[], limits)?;
        let number = PartialNumber::Fixed(Some(value.clone()));
        number.validate(limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Literal(value),
            number,
            size,
        ))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn probability(
        observation: ProbabilityAnswer,
        component: ObservationComponent,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[], limits)?;
        let value = match observation.value() {
            ProbabilityValue::Fixed { observation, value } => {
                PartialNumber::Fixed(match component {
                    ObservationComponent::Value => value.clone(),
                    ObservationComponent::Numerator => Some(observation.numerator().clone()),
                    ObservationComponent::EvidenceMass => Some(observation.evidence_mass().clone()),
                })
            }
            ProbabilityValue::Parameter(observation) => PartialNumber::Parameter(
                match component {
                    ObservationComponent::Value => observation.conditional(),
                    ObservationComponent::Numerator => observation.numerator(),
                    ObservationComponent::EvidenceMass => observation.evidence_mass(),
                }
                .clone(),
            ),
        };
        value.validate(limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Probability {
                observation: Arc::new(observation),
                component,
            },
            value,
            size,
        ))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn expectation(
        observation: ExpectationAnswer,
        component: ObservationComponent,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[], limits)?;
        let value = match observation.value() {
            ExpectationValue::Fixed { observation, value } => {
                PartialNumber::Fixed(match component {
                    ObservationComponent::Value => value.clone(),
                    ObservationComponent::Numerator => Some(observation.numerator().clone()),
                    ObservationComponent::EvidenceMass => Some(observation.evidence_mass().clone()),
                })
            }
            ExpectationValue::Parameter(observation) => parameter_value(observation, component),
            ExpectationValue::Family(observation) => parameter_value(observation, component),
        };
        value.validate(limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Expectation {
                observation: Arc::new(observation),
                component,
            },
            value,
            size,
        ))
    }
    #[must_use]
    pub fn value(&self) -> &PartialNumber {
        &self.0.value
    }
    #[must_use]
    pub fn expression(&self) -> &ObservationNumberExpr {
        &self.0.expression
    }
    /// Explicit numerical-domain restriction; the original evidence remains in
    /// the derivation. This does not condition or replace its source law.
    /// # Errors
    /// Foreign/extended domains, expression/arithmetic/solver limits or cancellation.
    pub fn on_domain(
        &self,
        domain: &ParameterDomain,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self], limits)?;
        let value = self.value().on_domain(domain, limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::OnDomain {
                value: self.clone(),
                domain: domain.clone(),
            },
            value,
            size,
        ))
    }
    /// # Errors
    /// Incompatible numerical domains, expression/arithmetic/solver limits or cancellation.
    pub fn apply(
        &self,
        op: NumberOp,
        rhs: &Self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self, rhs], limits)?;
        let value = self.value().apply(op, rhs.value(), limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Binary {
                op,
                left: self.clone(),
                right: rhs.clone(),
            },
            value,
            size,
        ))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn negate(
        &self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self], limits)?;
        let value = self.value().negate(limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Negate(self.clone()),
            value,
            size,
        ))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn abs(
        &self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self], limits)?;
        let value = self.value().abs(limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Abs(self.clone()),
            value,
            size,
        ))
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn pow(
        &self,
        exponent: u32,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let size = shape(&[self], limits)?;
        let value = self.value().pow(exponent, limits.numbers, work)?;
        Ok(Self::new(
            ObservationNumberExpr::Pow {
                value: self.clone(),
                exponent,
            },
            value,
            size,
        ))
    }
    /// Numerical equality including defined domains, separate from provenance.
    /// # Errors
    /// Domain mismatch, arithmetic/solver limits or cancellation.
    pub fn equivalent(
        &self,
        rhs: &Self,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        shape(&[self, rhs], limits)?;
        Ok(self.value().equivalent(rhs.value(), limits.numbers, work)?)
    }
    /// # Errors
    /// Expression, arithmetic or solver limits and cancellation.
    pub fn where_sign(
        &self,
        signs: PolynomialSigns,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ObservationPredicate> {
        ObservationPredicate::where_sign(self, signs, limits, work)
    }
    /// Pointwise comparison, retaining both operands and undefined assignments.
    /// # Errors
    /// Incompatible numerical domains, expression/arithmetic/solver limits or cancellation.
    pub fn compare(
        &self,
        rhs: &Self,
        signs: PolynomialSigns,
        limits: ObservationNumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ObservationPredicate> {
        self.apply(NumberOp::Subtract, rhs, limits, work)?
            .where_sign(signs, limits, work)
    }
}
fn parameter_value<F>(
    observation: &crate::event::ParameterExpectationObservation<F>,
    component: ObservationComponent,
) -> PartialNumber {
    PartialNumber::Parameter(
        match component {
            ObservationComponent::Value => observation.conditional(),
            ObservationComponent::Numerator => observation.numerator(),
            ObservationComponent::EvidenceMass => observation.evidence_mass(),
        }
        .clone(),
    )
}
