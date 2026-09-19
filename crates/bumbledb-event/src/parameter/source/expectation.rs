//! Signed finite payoffs under a shared-parameter law. Contraction returns an
//! owned partial function of the parameter, never a prior-averaged scalar.
use super::{ParameterSourceLimits, measure::constant};
use crate::function::raw::Budget;
use crate::{
    BoolOp4, Event, ExactArithmetic, ExactRational, FiniteFunction, ParameterDomain,
    ParameterFunction, ParameterRegion, Result, Space,
};

/// An owned signed expectation under a parameter family. A total finite payoff
/// has an explicit zero default; a query value roster must establish coverage
/// separately. The evidence and complete original mass functions remain owned.
#[derive(Debug, Clone)]
pub struct ParameterExpectationObservation {
    function: FiniteFunction,
    evidence: Event,
    numerator: ParameterFunction,
    evidence_mass: ParameterFunction,
    conditional: ParameterFunction,
}

impl ParameterExpectationObservation {
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
    pub fn numerator(&self) -> &ParameterFunction {
        &self.numerator
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &ParameterFunction {
        &self.evidence_mass
    }
    #[must_use]
    pub fn conditional(&self) -> &ParameterFunction {
        &self.conditional
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        self.conditional.defined_on()
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.conditional.is_nowhere_defined()
    }
    /// Inspect the exact signed expectation at a rational parameter value.
    /// Undefined/outside values return `None`, even for a zero payoff.
    /// # Errors
    /// Arithmetic, function/solver limits or cancellation.
    pub fn value_at(
        &self,
        value: &ExactRational,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ExactRational>> {
        self.conditional
            .value_at(value, limits.parameters.region, limits.functions, work)
    }
}

impl FiniteFunction {
    /// Contract a total finite signed payoff under its designated family law,
    /// conditional on evidence at the same actual parameter. No prior over the
    /// parameter is inferred. Zero payoffs still require a law and valid context.
    /// # Errors
    /// Foreign evidence, missing parameter/law, capacities or cancellation.
    pub fn parameter_expectation(
        &self,
        evidence: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterExpectationObservation> {
        let control = work.control();
        let mut budget = Budget::new(limits.functions, control)?;
        budget.cells(self.pieces().len())?;
        let evidence = evidence.align_to(self.space(), control)?;
        let evidence_mass = evidence.parameter_mass(limits, work)?;
        let domain = evidence_mass.ambient();
        let mut numerator = scalar(domain, ExactRational::zero(), limits, work)?;
        for (region, value) in self.pieces() {
            budget.step(control)?;
            let mass = region
                .apply(BoolOp4::AND, &evidence, control)?
                .parameter_mass(limits, work)?;
            let term = mass.mul(
                &scalar(domain, value.clone(), limits, work)?,
                limits.parameters.region,
                limits.functions,
                work,
            )?;
            numerator = numerator.add(&term, limits.parameters.region, limits.functions, work)?;
        }
        let conditional = numerator.div(
            &evidence_mass,
            limits.parameters.region,
            limits.functions,
            work,
        )?;
        control.checkpoint()?;
        Ok(ParameterExpectationObservation {
            function: self.clone(),
            evidence,
            numerator,
            evidence_mass,
            conditional,
        })
    }
}

fn scalar(
    domain: &ParameterDomain,
    value: ExactRational,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<ParameterFunction> {
    ParameterFunction::new(
        domain.clone(),
        &[constant(domain, value, limits, work)?],
        limits.parameters.region,
        limits.functions,
        work,
    )
}
