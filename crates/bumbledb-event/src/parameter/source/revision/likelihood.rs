use super::{ParameterRestriction, ParameterRevisedSource, coordinate_inclusion};
use crate::{
    Error, ExactArithmetic, FamilyFunction, ParameterFunction, ParameterRegion,
    ParameterSourceLimits, PolynomialSigns, Result, Space, SpaceId,
};

/// An explicitly interpreted nonnegative family likelihood and its owned result.
/// The supplied scale is retained. Its normalizer need not be a probability.
/// This is not an import of posterior targets or a fresh observation identity.
#[derive(Debug, Clone)]
pub struct ParameterLikelihood {
    prior: Space,
    likelihood: FamilyFunction,
    normalizer: ParameterFunction,
    positive: ParameterRegion,
    revised: Option<ParameterRevisedSource>,
}

impl ParameterLikelihood {
    #[must_use]
    pub fn prior(&self) -> &Space {
        &self.prior
    }
    #[must_use]
    pub fn likelihood(&self) -> &FamilyFunction {
        &self.likelihood
    }
    #[must_use]
    pub fn normalizer(&self) -> &ParameterFunction {
        &self.normalizer
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        &self.positive
    }
    #[must_use]
    pub fn revised(&self) -> Option<&ParameterRevisedSource> {
        self.revised.as_ref()
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.revised.is_none()
    }
}

impl Space {
    /// Reweight each outcome by an explicitly supplied nonnegative function at
    /// the same actual parameter. Keep all outcomes on the positive-normalizer
    /// domain. An everywhere-zero normalizer returns an owned impossible receipt.
    /// Reapplying a likelihood is multiplication, not idempotent evidence reuse.
    /// # Errors
    /// Foreign function, negative factors (including zero-prior worlds), missing
    /// law, capacities or cancellation.
    pub fn parameter_likelihood(
        &self,
        identity: SpaceId,
        likelihood: &FamilyFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterLikelihood> {
        let likelihood = likelihood.align_to(self, limits, work)?;
        if !likelihood.is_nonnegative(limits, work)? {
            return Err(Error::NegativeMass);
        }
        let normalizer = likelihood
            .expectation(&self.full(), limits, work)?
            .numerator()
            .clone();
        let positive = normalizer.where_sign(
            PolynomialSigns::POSITIVE,
            limits.parameters.region,
            limits.functions,
            work,
        )?;
        let revised = if positive.is_empty() {
            None
        } else {
            let restriction = ParameterRestriction::new(identity, self, &positive, limits, work)?;
            let factor = likelihood
                .refine(restriction.refinement(), limits, work)?
                .pullback(restriction.inclusion(), limits, work)?;
            let domain = restriction
                .space()
                .parameter_domain()
                .ok_or(Error::MissingParameter)?;
            let divisor = FamilyFunction::from_parameter(
                restriction.space(),
                &normalizer.on_domain(domain, limits.parameters.region, limits.functions, work)?,
                limits,
                work,
            )?;
            let posterior = FamilyFunction::density(restriction.space(), limits, work)?
                .multiply(&factor, limits, work)?
                .divide(&divisor, limits, work)?
                .designate(limits, work)?;
            let translation =
                coordinate_inclusion(&posterior, restriction.refined_prior(), limits, work)?;
            Some(ParameterRevisedSource {
                restriction,
                translation,
            })
        };
        work.control().checkpoint()?;
        Ok(ParameterLikelihood {
            prior: self.clone(),
            likelihood,
            normalizer,
            positive,
            revised,
        })
    }
}
