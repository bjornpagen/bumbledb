//! Explicit integration over one named unknown parameter. Original Events and
//! per-parameter observations remain owned; integration never changes support.
use crate::function::raw::Budget;
use crate::{
    BoolOp4, Error, Event, ExactArithmetic, ExactPolynomial, ExactRational, FamilyFunction,
    FiniteFunction, GuardedRationalFunction, ParameterCell, ParameterExpectationObservation,
    ParameterFunction, ParameterProbabilityObservation, ParameterRegion, ParameterSourceLimits,
    PolynomialSigns, Result, Space,
};

/// An explicit Beta prior on an existing univariate source's full [0,1] domain.
/// This additional measurement commitment does not replace the source's fibre
/// law or collapse its actual parameter worlds. No confidence field creates it.
#[derive(Debug, Clone)]
pub struct BetaSource {
    source: Space,
    alpha: ExactRational,
    beta: ExactRational,
}

/// Exact contraction certificate: a total function agrees with `polynomial`
/// outside `exceptions`, a checked finite set of parameter points. Positive
/// Beta priors have no atoms, so these points contribute zero to this integral.
/// The original function and exceptions remain distinguishable and owned.
#[derive(Debug, Clone)]
pub struct BetaIntegral {
    source: BetaSource,
    function: ParameterFunction,
    polynomial: ExactPolynomial,
    exceptions: ParameterRegion,
    value: ExactRational,
}
impl BetaIntegral {
    #[must_use]
    pub fn source(&self) -> &BetaSource {
        &self.source
    }
    #[must_use]
    pub fn function(&self) -> &ParameterFunction {
        &self.function
    }
    #[must_use]
    pub fn polynomial(&self) -> &ExactPolynomial {
        &self.polynomial
    }
    #[must_use]
    pub fn exceptions(&self) -> &ParameterRegion {
        &self.exceptions
    }
    #[must_use]
    pub fn value(&self) -> &ExactRational {
        &self.value
    }
}

/// The evidence-weighted result of one explicit parameter binding. Both raw
/// integrands are contracted before division; the original conditional function
/// is never averaged, including when it has zero-evidence endpoint holes.
#[derive(Debug, Clone)]
pub struct BetaObservation<T> {
    source: BetaSource,
    original: T,
    numerator: BetaIntegral,
    evidence: BetaIntegral,
    value: Option<ExactRational>,
}
pub type BetaProbabilityObservation = BetaObservation<ParameterProbabilityObservation>;
pub type BetaExpectationObservation<F = FiniteFunction> =
    BetaObservation<ParameterExpectationObservation<F>>;

impl<T> BetaObservation<T> {
    #[must_use]
    pub fn source(&self) -> &BetaSource {
        &self.source
    }
    #[must_use]
    pub fn original(&self) -> &T {
        &self.original
    }
    #[must_use]
    pub fn numerator(&self) -> &BetaIntegral {
        &self.numerator
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &BetaIntegral {
        &self.evidence
    }
    #[must_use]
    pub fn value(&self) -> Option<&ExactRational> {
        self.value.as_ref()
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.value.is_none()
    }
}

fn has_sector(
    region: &ParameterRegion,
    budget: &mut Budget,
    work: &ExactArithmetic<'_>,
) -> Result<bool> {
    for (cell, present) in region.cells() {
        budget.step(work.control())?;
        if present && matches!(cell, ParameterCell::Open { .. }) {
            return Ok(true);
        }
    }
    Ok(false)
}

impl BetaSource {
    /// Capture an explicitly supplied prior, retaining the original source law.
    /// Supported observations have total, polynomial-almost-everywhere raw
    /// integrands. No arbitrary rational or semialgebraic integration is claimed.
    /// # Errors
    /// Missing source/law, nonpositive shapes, a domain other than full [0,1],
    /// arithmetic/solver capacities or cancellation.
    pub fn new(
        source: &Space,
        alpha: ExactRational,
        beta: ExactRational,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let domain = source.parameter_domain().ok_or(Error::MissingParameter)?;
        let _pieces = source.parameter_density_pieces().ok_or(Error::MissingLaw)?;
        // Validate shapes even for a constant source or impossible observation.
        ExactPolynomial::zero().integrate_beta(
            domain.parameter(),
            &alpha,
            &beta,
            limits.parameters.region.polynomial,
            work,
        )?;
        let p = ExactPolynomial::parameter(domain.parameter());
        let nonnegative = ParameterRegion::from_polynomial(
            domain.parameter(),
            &p,
            PolynomialSigns::NON_NEGATIVE,
            limits.parameters.region,
            work,
        )?;
        let upper = ExactPolynomial::one().sub(&p, limits.parameters.region.polynomial, work)?;
        let at_most_one = ParameterRegion::from_polynomial(
            domain.parameter(),
            &upper,
            PolynomialSigns::NON_NEGATIVE,
            limits.parameters.region,
            work,
        )?;
        let unit = nonnegative.apply(BoolOp4::AND, &at_most_one, limits.parameters.region, work)?;
        if !domain
            .region()
            .equivalent(&unit, limits.parameters.region, work)?
        {
            return Err(Error::UnsupportedBetaDomain);
        }
        work.control().checkpoint()?;
        Ok(Self {
            source: source.clone(),
            alpha,
            beta,
        })
    }
    #[must_use]
    pub fn source(&self) -> &Space {
        &self.source
    }
    #[must_use]
    pub fn alpha(&self) -> &ExactRational {
        &self.alpha
    }
    #[must_use]
    pub fn beta(&self) -> &ExactRational {
        &self.beta
    }

    /// Integrate a total scalar function, proving it has one polynomial on every
    /// open sector of the full domain. Isolated deviations are retained in the
    /// certificate, not erased from the function. Undefined points still refuse.
    /// # Errors
    /// Foreign domain, undefined points, nonpolynomial sectors or work limits.
    pub fn integrate(
        &self,
        function: &ParameterFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaIntegral> {
        let domain = self
            .source
            .parameter_domain()
            .ok_or(Error::MissingParameter)?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        budget.cells(function.pieces().len())?;
        if !domain.region().equivalent(
            function.ambient().region(),
            limits.parameters.region,
            work,
        )? {
            return Err(Error::ParameterDomainMismatch);
        }
        function.where_sign(
            PolynomialSigns::ANY,
            limits.parameters.region,
            limits.functions,
            work,
        )?;
        if !domain
            .region()
            .equivalent(function.defined_on(), limits.parameters.region, work)?
        {
            return Err(Error::UndefinedFunction);
        }
        let mut first = None;
        for part in function.pieces() {
            if has_sector(part.defined_on(), &mut budget, work)? {
                first = Some(part);
                break;
            }
        }
        let first = first.ok_or(Error::UnsupportedBetaIntegrand)?;
        let (polynomial, remainder) = first.numerator().div_rem_univariate(
            first.denominator(),
            domain.parameter(),
            limits.parameters.region.polynomial,
            work,
        )?;
        if !remainder.is_zero() {
            return Err(Error::UnsupportedBetaIntegrand);
        }
        let expected = GuardedRationalFunction::new(
            domain.clone(),
            polynomial.clone(),
            ExactPolynomial::one(),
            limits.parameters.region,
            work,
        )?;
        let mut exceptions = ParameterRegion::empty(domain.parameter());
        for piece in function.pieces() {
            budget.step(work.control())?;
            let different = piece
                .sub(&expected, limits.parameters.region, work)?
                .where_sign(PolynomialSigns::NON_ZERO, limits.parameters.region, work)?;
            if has_sector(&different, &mut budget, work)? {
                return Err(Error::UnsupportedBetaIntegrand);
            }
            exceptions =
                exceptions.apply(BoolOp4::OR, &different, limits.parameters.region, work)?;
        }
        let integrated = polynomial.integrate_beta(
            domain.parameter(),
            &self.alpha,
            &self.beta,
            limits.parameters.region.polynomial,
            work,
        )?;
        let value = integrated.evaluate(&[], limits.parameters.region.polynomial, work)?;
        work.control().checkpoint()?;
        Ok(BetaIntegral {
            source: self.clone(),
            function: function.clone(),
            polynomial,
            exceptions,
            value,
        })
    }

    /// Bind an existing source-owned probability observation. Raw joint mass
    /// and evidence mass are integrated before taking their ratio.
    /// # Errors
    /// Source mismatch or `integrate`'s explicit capability/resource failures.
    pub fn bind_probability(
        &self,
        observation: &ParameterProbabilityObservation,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaProbabilityObservation> {
        observation
            .space()
            .full()
            .align_to(&self.source, work.control())?;
        self.bind(
            observation.clone(),
            observation.numerator(),
            observation.evidence_mass(),
            limits,
            work,
        )
    }

    /// Bind a finite or family-function expectation without losing its payoff,
    /// original evidence, raw numerator or conditional definedness.
    /// # Errors
    /// Source mismatch or `integrate`'s explicit capability/resource failures.
    pub fn bind_expectation<F: Clone>(
        &self,
        observation: &ParameterExpectationObservation<F>,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaExpectationObservation<F>> {
        observation
            .evidence()
            .space()
            .full()
            .align_to(&self.source, work.control())?;
        self.bind(
            observation.clone(),
            observation.numerator(),
            observation.evidence_mass(),
            limits,
            work,
        )
    }

    /// Observe two Events after explicitly binding their source's shared prior.
    /// # Errors
    /// Foreign operands, unsupported integrands, capacities or cancellation.
    pub fn probability(
        &self,
        event: &Event,
        given: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaProbabilityObservation> {
        let event = event.align_to(&self.source, work.control())?;
        let given = given.align_to(&self.source, work.control())?;
        self.bind_probability(
            &event.parameter_probability(&given, limits, work)?,
            limits,
            work,
        )
    }

    /// Observe an exact finite signed payoff under this prior.
    /// # Errors
    /// Foreign operands, unsupported integrands, capacities or cancellation.
    pub fn expectation(
        &self,
        payoff: &FiniteFunction,
        given: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaExpectationObservation> {
        let payoff = payoff.align_to(&self.source, limits.functions, work)?;
        self.bind_expectation(
            &payoff.parameter_expectation(given, limits, work)?,
            limits,
            work,
        )
    }

    /// Observe a total signed parameter-dependent payoff under this prior.
    /// # Errors
    /// Foreign operands, unsupported integrands, capacities or cancellation.
    pub fn family_expectation(
        &self,
        payoff: &FamilyFunction,
        given: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaExpectationObservation<FamilyFunction>> {
        let payoff = payoff.align_to(&self.source, limits, work)?;
        self.bind_expectation(&payoff.expectation(given, limits, work)?, limits, work)
    }

    fn bind<T>(
        &self,
        original: T,
        numerator: &ParameterFunction,
        evidence: &ParameterFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaObservation<T>> {
        let numerator = self.integrate(numerator, limits, work)?;
        let evidence = self.integrate(evidence, limits, work)?;
        let value = if evidence.value.is_zero() {
            None
        } else {
            Some(numerator.value.div(&evidence.value, work)?)
        };
        work.control().checkpoint()?;
        Ok(BetaObservation {
            source: self.clone(),
            original,
            numerator,
            evidence,
            value,
        })
    }
}
