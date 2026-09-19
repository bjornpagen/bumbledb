//! Canonical rational polynomials in named real parameters. These names denote
//! shared unknown values, not stochastic outcome bits and not implicit priors.
//! Equality here is polynomial identity, not equality on a constrained domain.
use std::collections::HashMap;

use crate::{Capacity, Error, ExactArithmetic, ExactRational, Result};

mod wire;

/// Application-supplied identity of one captured real parameter. Reusing the
/// name reuses its value. Different names imply neither independence nor a law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(pub [u8; 32]);

/// Limits bound inputs and intermediate normal forms, not only the final answer.
/// `steps` counts visited terms/factors and construction operations, not bigint
/// limbs or sorting comparisons. Exact arithmetic has its own shared budget.
#[derive(Debug, Clone, Copy)]
pub struct PolynomialLimits {
    pub terms: usize,
    pub factors: usize,
    pub degree: u32,
    pub steps: usize,
    pub bytes: usize,
}

impl Default for PolynomialLimits {
    fn default() -> Self {
        Self {
            terms: 65_536,
            factors: 256,
            degree: 65_536,
            steps: 1_000_000,
            bytes: 16 * 1024 * 1024,
        }
    }
}

/// Constructor input and immutable normal-form view. Construction accepts
/// unordered/repeated parameters and monomials, zero powers and zero terms.
/// Published terms have strictly ordered positive powers, nonzero coefficients,
/// and unique lexicographically ordered monomials.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolynomialTerm {
    pub coefficient: ExactRational,
    pub powers: Box<[(ParameterId, u32)]>,
}

/// A sparse expanded normal form over Q. Constant zero has no terms. Domain
/// assumptions are deliberately absent: p² differs from p even if some future
/// source constrains p to {0,1}. Source-domain judgments require their own proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactPolynomial {
    terms: Box<[PolynomialTerm]>,
}

impl ExactPolynomial {
    #[must_use]
    pub fn zero() -> Self {
        Self {
            terms: Box::new([]),
        }
    }

    #[must_use]
    pub fn one() -> Self {
        Self::constant(ExactRational::one())
    }

    /// Operations and export recheck coefficient extent under their budget.
    #[must_use]
    pub fn constant(coefficient: ExactRational) -> Self {
        if coefficient.is_zero() {
            Self::zero()
        } else {
            Self {
                terms: Box::new([PolynomialTerm {
                    coefficient,
                    powers: Box::new([]),
                }]),
            }
        }
    }

    #[must_use]
    pub fn parameter(parameter: ParameterId) -> Self {
        Self {
            terms: Box::new([PolynomialTerm {
                coefficient: ExactRational::one(),
                powers: Box::new([(parameter, 1)]),
            }]),
        }
    }

    #[must_use]
    pub fn terms(&self) -> &[PolynomialTerm] {
        &self.terms
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub(crate) fn validate(
        &self,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        Operation::new(limits, work)?.validate(self)
    }

    /// Combine equal monomials and remove zero coefficients. Every supplied
    /// term is checked before simplification, including zero coefficients.
    /// # Errors
    /// Input/intermediate extents, arithmetic budget, allocation or cancellation.
    pub fn from_terms(
        terms: &[PolynomialTerm],
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.term_extent(terms.len())?;
        let mut builder = Builder::default();
        for term in terms {
            op.validate_term(term)?;
            let powers = op.normalize_powers(&term.powers)?;
            builder.insert(powers, term.coefficient.clone(), &mut op)?;
        }
        builder.finish(&mut op)
    }

    /// # Errors
    /// Input/intermediate extents, arithmetic budget, allocation or cancellation.
    pub fn add(
        &self,
        other: &Self,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        op.validate(other)?;
        op.add(self, other, false)
    }

    /// # Errors
    /// As `add`.
    pub fn sub(
        &self,
        other: &Self,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        op.validate(other)?;
        op.add(self, other, true)
    }

    /// # Errors
    /// As `add`. Both operands are validated even if either is zero.
    pub fn mul(
        &self,
        other: &Self,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        op.validate(other)?;
        op.mul(self, other)
    }

    /// Natural power; zero power is one. This creates no outcome coordinates
    /// and makes no independence assertion about experiments.
    /// # Errors
    /// As `add`. The input is validated even for exponent zero.
    pub fn pow(
        &self,
        exponent: u32,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        op.pow(self, exponent)
    }

    /// Simultaneous named substitution. Unmentioned parameters are retained;
    /// replacement expressions are not themselves recursively substituted.
    /// Every binding participates in validation, including unused replacements.
    /// # Errors
    /// Duplicate names, input/intermediate extents, arithmetic or cancellation.
    pub fn substitute(
        &self,
        bindings: &[(ParameterId, Self)],
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        let mut by_name = HashMap::new();
        for (name, value) in bindings {
            op.validate(value)?;
            by_name.try_reserve(1)?;
            if by_name.insert(*name, value).is_some() {
                return Err(Error::ParameterBinding);
            }
        }
        let mut result = Self::zero();
        for term in &self.terms {
            op.step()?;
            let mut product = Self::constant(term.coefficient.clone());
            for &(name, power) in &term.powers {
                op.step()?;
                let fallback = Self::parameter(name);
                let value = by_name.get(&name).copied().unwrap_or(&fallback);
                let powered = op.pow(value, power)?;
                product = op.mul(&product, &powered)?;
            }
            result = op.add(&result, &product, false)?;
        }
        op.validate(&result)?;
        Ok(result)
    }

    /// Evaluate exactly at a rational assignment. No domain membership or
    /// probability validity is inferred. Unused bindings are permitted and
    /// validated; required names must occur exactly once.
    /// # Errors
    /// Missing/duplicate names, arithmetic/structural limits or cancellation.
    pub fn evaluate(
        &self,
        bindings: &[(ParameterId, ExactRational)],
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ExactRational> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        let mut by_name = HashMap::new();
        for (name, value) in bindings {
            op.step()?;
            op.work.validate(value)?;
            by_name.try_reserve(1)?;
            if by_name.insert(*name, value).is_some() {
                return Err(Error::ParameterBinding);
            }
        }
        let mut result = ExactRational::zero();
        for term in &self.terms {
            op.step()?;
            let mut product = term.coefficient.clone();
            for (name, exponent) in &term.powers {
                op.step()?;
                let value = by_name.get(name).ok_or(Error::ParameterBinding)?;
                product = product.mul(&value.pow(*exponent, op.work)?, op.work)?;
            }
            result = result.add(&product, op.work)?;
        }
        op.step()?;
        Ok(result)
    }

    /// Integrate one named parameter against an explicitly supplied Beta(a,b)
    /// prior on its full [0,1] interval. Other parameters stay symbolic. This
    /// algebraic moment operation neither admits a source domain nor justifies
    /// independence from it. A source constructor must check that obligation.
    /// Integrate unnormalized evidence first and divide afterwards.
    /// # Errors
    /// Nonpositive shapes, intermediate limits, arithmetic or cancellation.
    pub fn integrate_beta(
        &self,
        parameter: ParameterId,
        alpha: &ExactRational,
        beta: &ExactRational,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        op.work.validate(alpha)?;
        op.work.validate(beta)?;
        if alpha.is_zero() || alpha.is_negative() || beta.is_zero() || beta.is_negative() {
            return Err(Error::InvalidBetaPrior);
        }
        let total = alpha.add(beta, op.work)?;
        let mut degrees = Vec::new();
        degrees.try_reserve_exact(self.terms.len())?;
        for term in &self.terms {
            op.step()?;
            let exponent = term
                .powers
                .binary_search_by_key(&parameter, |p| p.0)
                .map_or(0, |index| term.powers[index].1);
            degrees.push(exponent);
        }
        degrees.sort_unstable();
        degrees.dedup();
        let mut moments = Vec::new();
        moments.try_reserve_exact(degrees.len())?;
        let mut moment = ExactRational::one();
        let mut previous = 0;
        for &degree in &degrees {
            for k in previous..degree {
                op.step()?;
                let offset = ExactRational::from(u64::from(k));
                let a = alpha.add(&offset, op.work)?;
                let ab = total.add(&offset, op.work)?;
                moment = moment.mul(&a.div(&ab, op.work)?, op.work)?;
            }
            moments.push(moment.clone());
            previous = degree;
        }
        let mut builder = Builder::default();
        for term in &self.terms {
            op.step()?;
            let mut powers = Vec::new();
            powers.try_reserve_exact(term.powers.len())?;
            let mut degree = 0;
            for &(name, exponent) in &term.powers {
                op.step()?;
                if name == parameter {
                    degree = exponent;
                } else {
                    powers.push((name, exponent));
                }
            }
            let index = degrees
                .binary_search(&degree)
                .map_err(|_| Error::InvalidPolynomial)?;
            let coefficient = term.coefficient.mul(&moments[index], op.work)?;
            builder.insert(powers.into_boxed_slice(), coefficient, &mut op)?;
        }
        builder.finish(&mut op)
    }
}

struct Operation<'a, 'b> {
    limits: PolynomialLimits,
    steps: usize,
    work: &'a mut ExactArithmetic<'b>,
}

impl<'a, 'b> Operation<'a, 'b> {
    fn new(limits: PolynomialLimits, work: &'a mut ExactArithmetic<'b>) -> Result<Self> {
        let mut op = Self {
            limits,
            steps: 0,
            work,
        };
        op.step()?;
        Ok(op)
    }

    fn step(&mut self) -> Result<()> {
        self.work.control().checkpoint()?;
        if self.steps >= self.limits.steps {
            return Err(Error::Capacity(Capacity::PolynomialSteps));
        }
        self.steps += 1;
        Ok(())
    }

    fn term_extent(&self, size: usize) -> Result<()> {
        if size > self.limits.terms {
            return Err(Error::Capacity(Capacity::PolynomialTerms));
        }
        Ok(())
    }

    fn factor_extent(&self, size: usize) -> Result<()> {
        if size > self.limits.factors {
            return Err(Error::Capacity(Capacity::PolynomialFactors));
        }
        Ok(())
    }

    fn degree(&self, a: u32, b: u32) -> Result<u32> {
        a.checked_add(b)
            .filter(|d| *d <= self.limits.degree)
            .ok_or(Error::Capacity(Capacity::PolynomialDegree))
    }

    fn validate(&mut self, value: &ExactPolynomial) -> Result<()> {
        self.step()?;
        self.term_extent(value.terms.len())?;
        for term in &value.terms {
            self.validate_term(term)?;
        }
        Ok(())
    }

    fn validate_term(&mut self, term: &PolynomialTerm) -> Result<()> {
        self.step()?;
        self.work.validate(&term.coefficient)?;
        self.factor_extent(term.powers.len())?;
        let mut degree = 0;
        for &(_, exponent) in &term.powers {
            self.step()?;
            degree = self.degree(degree, exponent)?;
        }
        Ok(())
    }

    fn normalize_powers(
        &mut self,
        powers: &[(ParameterId, u32)],
    ) -> Result<Box<[(ParameterId, u32)]>> {
        let mut sorted = Vec::new();
        sorted.try_reserve_exact(powers.len())?;
        sorted.extend_from_slice(powers);
        sorted.sort_unstable_by_key(|p| p.0);
        let mut result = Vec::<(ParameterId, u32)>::new();
        result.try_reserve_exact(powers.len())?;
        for (parameter, exponent) in sorted {
            self.step()?;
            if exponent == 0 {
                continue;
            }
            if let Some(last) = result.last_mut().filter(|p| p.0 == parameter) {
                last.1 = self.degree(last.1, exponent)?;
            } else {
                result.push((parameter, exponent));
            }
        }
        Ok(result.into_boxed_slice())
    }

    fn add(
        &mut self,
        a: &ExactPolynomial,
        b: &ExactPolynomial,
        subtract: bool,
    ) -> Result<ExactPolynomial> {
        let mut builder = Builder::default();
        for term in &a.terms {
            self.step()?;
            builder.insert(term.powers.clone(), term.coefficient.clone(), self)?;
        }
        for term in &b.terms {
            self.step()?;
            let coefficient = if subtract {
                ExactRational::zero().sub(&term.coefficient, self.work)?
            } else {
                term.coefficient.clone()
            };
            builder.insert(term.powers.clone(), coefficient, self)?;
        }
        builder.finish(self)
    }

    fn mul(&mut self, a: &ExactPolynomial, b: &ExactPolynomial) -> Result<ExactPolynomial> {
        let mut builder = Builder::default();
        for x in &a.terms {
            for y in &b.terms {
                self.step()?;
                // Merge two canonical monomials without an oversized temporary
                // concatenation when most named parameters are shared.
                let mut powers = Vec::new();
                let mut xi = x.powers.iter().peekable();
                let mut yi = y.powers.iter().peekable();
                let mut degree = 0;
                while xi.peek().is_some() || yi.peek().is_some() {
                    self.step()?;
                    let next = match (xi.peek(), yi.peek()) {
                        (Some(x), Some(y)) if x.0 == y.0 => {
                            let pair = (x.0, self.degree(x.1, y.1)?);
                            xi.next();
                            yi.next();
                            pair
                        }
                        (Some(x), Some(y)) if x.0 < y.0 => {
                            *xi.next().ok_or(Error::InvalidPolynomial)?
                        }
                        (_, Some(_)) => *yi.next().ok_or(Error::InvalidPolynomial)?,
                        (Some(_), None) => *xi.next().ok_or(Error::InvalidPolynomial)?,
                        (None, None) => break,
                    };
                    self.factor_extent(powers.len().saturating_add(1))?;
                    degree = self.degree(degree, next.1)?;
                    powers.try_reserve(1)?;
                    powers.push(next);
                }
                let coefficient = x.coefficient.mul(&y.coefficient, self.work)?;
                builder.insert(powers.into_boxed_slice(), coefficient, self)?;
            }
        }
        builder.finish(self)
    }

    fn pow(&mut self, value: &ExactPolynomial, mut exponent: u32) -> Result<ExactPolynomial> {
        self.step()?;
        let mut result = ExactPolynomial::one();
        let mut base = value.clone();
        while exponent != 0 {
            self.step()?;
            if exponent & 1 != 0 {
                result = self.mul(&result, &base)?;
            }
            exponent >>= 1;
            if exponent != 0 {
                base = self.mul(&base, &base)?;
            }
        }
        self.validate(&result)?;
        Ok(result)
    }
}

#[derive(Default)]
struct Builder {
    terms: HashMap<Box<[(ParameterId, u32)]>, ExactRational>,
}

impl Builder {
    fn insert(
        &mut self,
        powers: Box<[(ParameterId, u32)]>,
        coefficient: ExactRational,
        op: &mut Operation<'_, '_>,
    ) -> Result<()> {
        op.step()?;
        if coefficient.is_zero() {
            return Ok(());
        }
        if let Some(previous) = self.terms.get_mut(&powers) {
            let sum = previous.add(&coefficient, op.work)?;
            if sum.is_zero() {
                self.terms.remove(&powers);
            } else {
                *previous = sum;
            }
        } else {
            op.term_extent(self.terms.len().saturating_add(1))?;
            self.terms.try_reserve(1)?;
            self.terms.insert(powers, coefficient);
        }
        Ok(())
    }

    fn finish(self, op: &mut Operation<'_, '_>) -> Result<ExactPolynomial> {
        op.step()?;
        let mut terms = Vec::new();
        terms.try_reserve_exact(self.terms.len())?;
        for (powers, coefficient) in self.terms {
            terms.push(PolynomialTerm {
                coefficient,
                powers,
            });
        }
        terms.sort_unstable_by(|a, b| a.powers.cmp(&b.powers));
        op.step()?;
        Ok(ExactPolynomial {
            terms: terms.into_boxed_slice(),
        })
    }
}
