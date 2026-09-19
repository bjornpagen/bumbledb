//! Signed exact observations of finite payoffs. A total `FiniteFunction` has an
//! explicitly defined zero default; a query value roster must separately prove
//! its coverage before constructing that function.
use crate::{BoolOp4, Event, ExactArithmetic, ExactRational, FiniteFunction, Result, Space};

/// An owned signed conditional expectation. Zero evidence mass makes the value
/// undefined, even if the signed numerator happens to be zero. The original
/// payoff and evidence remain inspectable after their external owners are dropped.
#[derive(Debug, Clone)]
pub struct ExpectationObservation {
    function: FiniteFunction,
    evidence: Event,
    numerator: ExactRational,
    evidence_mass: ExactRational,
}

impl ExpectationObservation {
    #[must_use]
    pub fn space(&self) -> &Space {
        self.function.space()
    }
    #[must_use]
    pub fn function(&self) -> &FiniteFunction {
        &self.function
    }
    #[must_use]
    pub fn evidence(&self) -> &Event {
        &self.evidence
    }
    #[must_use]
    pub fn numerator(&self) -> &ExactRational {
        &self.numerator
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &ExactRational {
        &self.evidence_mass
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.evidence_mass.is_zero()
    }
    /// `None` is undefined conditioning; successful values can be any rational.
    /// # Errors
    /// Arithmetic limits or cancellation.
    pub fn value(&self, work: &mut ExactArithmetic<'_>) -> Result<Option<ExactRational>> {
        work.control().checkpoint()?;
        if self.is_impossible() {
            Ok(None)
        } else {
            self.numerator.div(&self.evidence_mass, work).map(Some)
        }
    }
}

impl FiniteFunction {
    /// Contract this signed total function with its designated law, conditional
    /// on evidence. Independently owned equal contexts align explicitly; even
    /// a zero function validates evidence and requires a designated law.
    /// # Errors
    /// Context mismatch, missing law, graph/arithmetic limits or cancellation.
    pub fn expectation(
        &self,
        evidence: &Event,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ExpectationObservation> {
        let control = work.control();
        let evidence = evidence.align_to(self.space(), control)?;
        let evidence_mass = evidence.mass(work)?;
        let mut numerator = ExactRational::zero();
        for (region, value) in self.pieces() {
            let mass = region.apply(BoolOp4::AND, &evidence, control)?.mass(work)?;
            numerator = numerator.add(&value.mul(&mass, work)?, work)?;
        }
        control.checkpoint()?;
        Ok(ExpectationObservation {
            function: self.clone(),
            evidence,
            numerator,
            evidence_mass,
        })
    }
}
