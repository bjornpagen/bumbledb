use super::{FamilyFunction, ParameterSourceLimits, SourceBudget, constant, count_root};
use crate::{
    BoolOp4, Error, Event, ExactArithmetic, ExactRational, ParameterExpectationObservation,
    ParameterFunction, Result,
};

impl FamilyFunction {
    /// Sum the signed function over finite legal outcomes satisfying evidence,
    /// keeping the actual parameter. No designated probability law is required.
    /// # Errors
    /// Foreign evidence, function/source/solver capacities or cancellation.
    pub fn outcome_sum(
        &self,
        evidence: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterFunction> {
        let checked = self.align_to(&self.space, limits, work)?;
        let evidence = evidence.align_to(&self.space, work.control())?;
        let context = self
            .space
            .0
            .parameter
            .as_ref()
            .ok_or(Error::MissingParameter)?;
        let mut budget = SourceBudget::new(limits, work.control())?;
        budget.extent(context.fibres.len(), work.control())?;
        crate::function::raw::Budget::new(limits.functions, work.control())?
            .cells(context.fibres.len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(context.fibres.len())?;
        for fibre in context.fibres.iter() {
            budget.step(work.control())?;
            let mut total = constant(&context.domain, ExactRational::zero(), limits, work)?
                .restrict(&fibre.region, limits.parameters.region, work)?;
            for (region, value) in checked.pieces() {
                budget.step(work.control())?;
                let region = region.apply(BoolOp4::AND, &evidence, work.control())?;
                let count = count_root(
                    &self.space,
                    region.root,
                    context.mask,
                    fibre.code,
                    work.control(),
                )?;
                if count != 0 {
                    let term = value
                        .restrict(&fibre.region, limits.parameters.region, work)?
                        .mul(
                            &constant(&context.domain, ExactRational::from(count), limits, work)?,
                            limits.parameters.region,
                            work,
                        )?;
                    total = total.add(&term, limits.parameters.region, work)?;
                }
            }
            pieces.push(total);
        }
        ParameterFunction::new(
            context.domain.clone(),
            &pieces,
            limits.parameters.region,
            limits.functions,
            work,
        )
    }

    /// Signed expectation with parameter-dependent payoff values. Every term
    /// uses the same parameter; zero evidence remains undefined.
    /// # Errors
    /// Foreign evidence, missing law, capacities or cancellation.
    pub fn expectation(
        &self,
        evidence: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterExpectationObservation<Self>> {
        let evidence = evidence.align_to(&self.space, work.control())?;
        let mass = evidence.parameter_mass(limits, work)?;
        let numerator = self
            .multiply(&Self::density(&self.space, limits, work)?, limits, work)?
            .outcome_sum(&evidence, limits, work)?;
        ParameterExpectationObservation::new(self.clone(), evidence, numerator, mass, limits, work)
    }
}
