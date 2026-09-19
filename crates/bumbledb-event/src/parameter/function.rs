//! Partial rational functions retain a nonempty ambient domain, their exact
//! numerator/denominator and all inherited holes. Cancelling an expression never
//! makes a previously undefined point defined.
use super::{
    Operation, ParameterDomain, ParameterLimits, ParameterRegion, PolynomialSigns, RealWitness,
};
use crate::{BoolOp4, Error, ExactArithmetic, ExactPolynomial, ExactRational, Result};

/// An exact partial rational function over one named real parameter. The domain
/// and arithmetic presentation are owned. This is not itself a probability law
/// or an Event observation; numerical equivalence does not equate evidence mass.
#[derive(Debug, Clone)]
pub struct GuardedRationalFunction {
    ambient: ParameterDomain,
    defined: ParameterRegion,
    numerator: ExactPolynomial,
    denominator: ExactPolynomial,
}

impl GuardedRationalFunction {
    /// Retain `numerator/denominator` on the part of an admitted ambient domain
    /// where the denominator is nonzero. A nowhere-defined function is valid;
    /// it does not turn the nonempty ambient source domain into a conflict.
    /// # Errors
    /// Foreign parameter names, unsupported solving, capacities or cancellation.
    pub fn new(
        ambient: ParameterDomain,
        numerator: ExactPolynomial,
        denominator: ExactPolynomial,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(ambient.region())?;
        op.polynomial(ambient.parameter(), &numerator)?;
        op.polynomial(ambient.parameter(), &denominator)?;
        let nonzero = ParameterRegion::from_polynomial(
            ambient.parameter(),
            &denominator,
            PolynomialSigns::NON_ZERO,
            limits,
            op.work,
        )?;
        let defined = ambient
            .region()
            .apply(BoolOp4::AND, &nonzero, limits, op.work)?;
        op.step()?;
        Ok(Self {
            ambient,
            defined,
            numerator,
            denominator,
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
    pub fn numerator(&self) -> &ExactPolynomial {
        &self.numerator
    }
    #[must_use]
    pub fn denominator(&self) -> &ExactPolynomial {
        &self.denominator
    }
    #[must_use]
    pub fn is_nowhere_defined(&self) -> bool {
        self.defined.is_empty()
    }

    fn validate(&self, op: &mut Operation<'_, '_>) -> Result<()> {
        op.validate(self.ambient.region())?;
        op.validate(&self.defined)?;
        op.polynomial(self.ambient.parameter(), &self.numerator)?;
        op.polynomial(self.ambient.parameter(), &self.denominator)
    }

    /// Capture a smaller inhabited ambient domain without extending the
    /// function or erasing inherited holes. Unlike `restrict`, this changes
    /// the retained ambient source domain explicitly.
    /// # Errors
    /// Domain extension, foreign parameter, capacities or cancellation.
    pub fn on_domain(
        &self,
        domain: &ParameterDomain,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.restrict(self.ambient.region(), limits, work)?;
        if !domain
            .region()
            .included(self.ambient.region(), limits, work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        Self::new(
            domain.clone(),
            self.numerator.clone(),
            self.denominator.clone(),
            limits,
            work,
        )?
        .restrict(&self.defined, limits, work)
    }

    /// Further restrict the function while retaining its ambient domain and
    /// original numerator/denominator. Restriction may exclude every point.
    /// # Errors
    /// Parameter scope mismatch, capacities or cancellation.
    pub fn restrict(
        &self,
        region: &ParameterRegion,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        let defined = self.defined.apply(BoolOp4::AND, region, limits, op.work)?;
        op.step()?;
        Ok(Self {
            defined,
            ..self.clone()
        })
    }

    /// # Errors
    /// Distinct ambient domains, parameter mismatch, capacities or cancellation.
    pub fn add(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Add, limits, work)
    }
    /// # Errors
    /// As `add`.
    pub fn sub(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Subtract, limits, work)
    }
    /// # Errors
    /// As `add`; zero factors do not erase undefined input points.
    pub fn mul(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Multiply, limits, work)
    }
    /// # Errors
    /// As `add`; additionally excludes every zero of the divisor.
    pub fn div(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.binary(other, Binary::Divide, limits, work)
    }
    fn binary(
        &self,
        other: &Self,
        kind: Binary,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        other.validate(&mut op)?;
        if !self
            .ambient
            .region()
            .equivalent(other.ambient.region(), limits, op.work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        let (n, d) = match kind {
            Binary::Add | Binary::Subtract => {
                let a = self
                    .numerator
                    .mul(&other.denominator, limits.polynomial, op.work)?;
                let b = other
                    .numerator
                    .mul(&self.denominator, limits.polynomial, op.work)?;
                let n = if matches!(kind, Binary::Add) {
                    a.add(&b, limits.polynomial, op.work)?
                } else {
                    a.sub(&b, limits.polynomial, op.work)?
                };
                (
                    n,
                    self.denominator
                        .mul(&other.denominator, limits.polynomial, op.work)?,
                )
            }
            Binary::Multiply => (
                self.numerator
                    .mul(&other.numerator, limits.polynomial, op.work)?,
                self.denominator
                    .mul(&other.denominator, limits.polynomial, op.work)?,
            ),
            Binary::Divide => (
                self.numerator
                    .mul(&other.denominator, limits.polynomial, op.work)?,
                self.denominator
                    .mul(&other.numerator, limits.polynomial, op.work)?,
            ),
        };
        let guard = self
            .defined
            .apply(BoolOp4::AND, &other.defined, limits, op.work)?;
        Self::new(self.ambient.clone(), n, d, limits, op.work)?.restrict(&guard, limits, op.work)
    }

    /// # Errors
    /// Input/intermediate limits or cancellation. Zero numerators produce an
    /// empty defined region, not a fabricated value or an ambient conflict.
    pub fn reciprocal(
        &self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        Self::new(
            self.ambient.clone(),
            self.denominator.clone(),
            self.numerator.clone(),
            limits,
            op.work,
        )?
        .restrict(&self.defined, limits, op.work)
    }

    /// Evaluate at an exact rational. `None` denotes an undefined or out-of-domain
    /// point. No rounding or denominator cancellation changes that domain.
    /// # Errors
    /// Input/intermediate limits or cancellation, including unused operands.
    pub fn value_at(
        &self,
        point: &ExactRational,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ExactRational>> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        op.work.validate(point)?;
        if !self
            .defined
            .contains(&RealWitness::Rational(point.clone()), limits, op.work)?
        {
            return Ok(None);
        }
        let binding = [(self.ambient.parameter(), point.clone())];
        let numerator = self
            .numerator
            .evaluate(&binding, limits.polynomial, op.work)?;
        let denominator = self
            .denominator
            .evaluate(&binding, limits.polynomial, op.work)?;
        Ok(Some(numerator.div(&denominator, op.work)?))
    }

    /// The exact region where this *defined* function has one of the requested
    /// signs. Negative denominators reverse signs; zero denominators never join
    /// the result, including a request for all three signs.
    /// # Errors
    /// Input/intermediate capacities, unsupported solving or cancellation.
    pub fn where_sign(
        &self,
        signs: PolynomialSigns,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterRegion> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        let parameter = self.ambient.parameter();
        let positive = ParameterRegion::from_polynomial(
            parameter,
            &self.denominator,
            PolynomialSigns::POSITIVE,
            limits,
            op.work,
        )?;
        let negative = ParameterRegion::from_polynomial(
            parameter,
            &self.denominator,
            PolynomialSigns::NEGATIVE,
            limits,
            op.work,
        )?;
        let normal =
            ParameterRegion::from_polynomial(parameter, &self.numerator, signs, limits, op.work)?;
        let reversed = ParameterRegion::from_polynomial(
            parameter,
            &self.numerator,
            signs.negated(),
            limits,
            op.work,
        )?;
        let normal = normal.apply(BoolOp4::AND, &positive, limits, op.work)?;
        let reversed = reversed.apply(BoolOp4::AND, &negative, limits, op.work)?;
        normal
            .apply(BoolOp4::OR, &reversed, limits, op.work)?
            .apply(BoolOp4::AND, &self.defined, limits, op.work)
    }

    /// Numeric equality as partial functions: same ambient and defined domain,
    /// and equal values everywhere there. This does NOT identify the original
    /// denominator/evidence presentation, source law or provenance.
    /// # Errors
    /// Parameter mismatch, intermediate limits or cancellation.
    pub fn equivalent(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        let mut op = Operation::new(limits, work)?;
        self.validate(&mut op)?;
        other.validate(&mut op)?;
        if !self
            .ambient
            .region()
            .equivalent(other.ambient.region(), limits, op.work)?
            || !self.defined.equivalent(&other.defined, limits, op.work)?
        {
            return Ok(false);
        }
        let difference = self.sub(other, limits, op.work)?;
        Ok(difference
            .where_sign(PolynomialSigns::NON_ZERO, limits, op.work)?
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
