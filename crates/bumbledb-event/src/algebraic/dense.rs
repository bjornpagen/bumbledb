//! Bounded rational Euclidean arithmetic. Sturm remainders may only be rescaled
//! by a positive factor; making each remainder monic would change its signs.
use std::cmp::Ordering;

use super::{AlgebraicRoot, RootBasis, RootLimits, sign};
use crate::{
    Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational as Rat, ParameterId, Result,
};

#[derive(Debug, Clone)]
pub(super) struct Dense(Vec<Rat>);

impl Dense {
    pub(super) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    fn trim(&mut self) {
        while self.0.last().is_some_and(Rat::is_zero) {
            self.0.pop();
        }
    }
}

pub(super) struct Operation<'a, 'b> {
    limits: RootLimits,
    steps: usize,
    pub(super) work: &'a mut ExactArithmetic<'b>,
}

impl<'a, 'b> Operation<'a, 'b> {
    pub(super) fn new(limits: RootLimits, work: &'a mut ExactArithmetic<'b>) -> Result<Self> {
        let mut op = Self {
            limits,
            steps: 0,
            work,
        };
        op.step()?;
        Ok(op)
    }

    pub(super) fn step(&mut self) -> Result<()> {
        self.work.control().checkpoint()?;
        if self.steps >= self.limits.steps {
            return Err(Error::Capacity(Capacity::AlgebraicSteps));
        }
        self.steps += 1;
        Ok(())
    }

    fn extent(&mut self, degree: usize) -> Result<()> {
        self.step()?;
        if degree > self.limits.degree as usize {
            return Err(Error::Capacity(Capacity::AlgebraicDegree));
        }
        Ok(())
    }

    fn zeros(&mut self, length: usize) -> Result<Vec<Rat>> {
        self.step()?;
        self.steps = self
            .steps
            .checked_add(length)
            .filter(|steps| *steps <= self.limits.steps)
            .ok_or(Error::Capacity(Capacity::AlgebraicSteps))?;
        let mut values = Vec::new();
        values.try_reserve_exact(length)?;
        values.resize(length, Rat::zero());
        self.work.control().checkpoint()?;
        Ok(values)
    }

    pub(super) fn import_polynomial(
        &mut self,
        value: &ExactPolynomial,
        parameter: ParameterId,
    ) -> Result<Dense> {
        let mut degree = 0;
        for term in value.terms() {
            self.step()?;
            self.work.validate(&term.coefficient)?;
            for &(name, exponent) in &term.powers {
                self.step()?;
                if name != parameter {
                    return Err(Error::NotUnivariate);
                }
                degree = degree.max(exponent as usize);
            }
        }
        self.extent(degree)?;
        let mut coefficients = self.zeros(degree + 1)?;
        for term in value.terms() {
            self.step()?;
            let power = term.powers.first().map_or(0, |p| p.1 as usize);
            coefficients[power] = term.coefficient.clone();
        }
        let mut result = Dense(coefficients);
        result.trim();
        Ok(result)
    }

    pub(super) fn validate_root(&mut self, root: &AlgebraicRoot) -> Result<()> {
        self.step()?;
        if self.limits.roots == 0 {
            return Err(Error::Capacity(Capacity::AlgebraicRoots));
        }
        self.work.validate(&root.lower)?;
        self.work.validate(&root.upper)?;
        // Recheck every retained arithmetic input under this caller's limits,
        // including the authored repeated polynomial and its derived chain.
        self.import_polynomial(root.polynomial(), root.parameter())?;
        for polynomial in &root.basis.sturm {
            self.extent(polynomial.0.len().saturating_sub(1))?;
            for value in &polynomial.0 {
                self.step()?;
                self.work.validate(value)?;
            }
        }
        Ok(())
    }

    pub(super) fn basis(
        &mut self,
        value: &ExactPolynomial,
        parameter: ParameterId,
    ) -> Result<RootBasis> {
        let input = self.import_polynomial(value, parameter)?;
        if input.is_zero() {
            return Err(Error::IndeterminateRoots);
        }
        let square_free = self.square_free(&input)?;
        let sturm = self.sturm(&square_free)?;
        Ok(RootBasis {
            parameter,
            polynomial: value.clone(),
            square_free,
            sturm,
        })
    }

    pub(super) fn evaluate(&mut self, polynomial: &Dense, x: &Rat) -> Result<Rat> {
        let mut result = Rat::zero();
        for coefficient in polynomial.0.iter().rev() {
            self.step()?;
            result = result.mul(x, self.work)?.add(coefficient, self.work)?;
        }
        Ok(result)
    }

    pub(super) fn compare(&mut self, a: &Rat, b: &Rat) -> Result<Ordering> {
        self.step()?;
        Ok(sign(&a.sub(b, self.work)?))
    }

    pub(super) fn midpoint(&mut self, a: &Rat, b: &Rat) -> Result<Rat> {
        self.step()?;
        a.add(b, self.work)?.div(&Rat::from(2u64), self.work)
    }

    fn derivative(&mut self, polynomial: &Dense) -> Result<Dense> {
        let mut result = Vec::new();
        result.try_reserve_exact(polynomial.0.len().saturating_sub(1))?;
        for (i, value) in polynomial.0.iter().enumerate().skip(1) {
            self.step()?;
            result.push(value.mul(&Rat::from(i as u64), self.work)?);
        }
        Ok(Dense(result))
    }

    fn divide(&mut self, dividend: &Dense, divisor: &Dense) -> Result<(Dense, Dense)> {
        self.step()?;
        let leading = divisor.0.last().ok_or(Error::AlgebraicInvariant)?;
        let mut remainder = dividend.clone();
        let length = dividend.0.len().saturating_sub(divisor.0.len()) + 1;
        let mut quotient = self.zeros(length)?;
        while !remainder.is_zero() && remainder.0.len() >= divisor.0.len() {
            self.step()?;
            let shift = remainder.0.len() - divisor.0.len();
            let factor = remainder
                .0
                .last()
                .ok_or(Error::AlgebraicInvariant)?
                .div(leading, self.work)?;
            quotient[shift] = factor.clone();
            for (index, coefficient) in divisor.0.iter().enumerate() {
                self.step()?;
                let term = coefficient.mul(&factor, self.work)?;
                remainder.0[index + shift] = remainder.0[index + shift].sub(&term, self.work)?;
            }
            remainder.trim();
        }
        let mut quotient = Dense(quotient);
        quotient.trim();
        Ok((quotient, remainder))
    }

    fn normalize(&mut self, mut polynomial: Dense, positive_only: bool) -> Result<Dense> {
        self.step()?;
        if let Some(leading) = polynomial.0.last() {
            let factor = if positive_only && leading.is_negative() {
                Rat::zero().sub(leading, self.work)?
            } else {
                leading.clone()
            };
            for value in &mut polynomial.0 {
                self.step()?;
                *value = value.div(&factor, self.work)?;
            }
        }
        Ok(polynomial)
    }

    pub(super) fn gcd(&mut self, mut a: Dense, mut b: Dense) -> Result<Dense> {
        while !b.is_zero() {
            self.step()?;
            let (_, remainder) = self.divide(&a, &b)?;
            a = b;
            b = self.normalize(remainder, false)?;
        }
        self.normalize(a, false)
    }

    pub(super) fn square_free(&mut self, polynomial: &Dense) -> Result<Dense> {
        if polynomial.is_zero() {
            return Err(Error::IndeterminateRoots);
        }
        let derivative = self.derivative(polynomial)?;
        let common = self.gcd(polynomial.clone(), derivative)?;
        let (quotient, remainder) = self.divide(polynomial, &common)?;
        if !remainder.is_zero() {
            return Err(Error::AlgebraicInvariant);
        }
        self.normalize(quotient, false)
    }

    pub(super) fn sturm(&mut self, square_free: &Dense) -> Result<Vec<Dense>> {
        let mut sequence = Vec::new();
        sequence.try_reserve(2)?;
        sequence.push(square_free.clone());
        let derivative = self.derivative(square_free)?;
        if derivative.is_zero() {
            return Ok(sequence);
        }
        sequence.push(self.normalize(derivative, true)?);
        loop {
            self.step()?;
            let n = sequence.len();
            let (_, mut remainder) = self.divide(&sequence[n - 2], &sequence[n - 1])?;
            if remainder.is_zero() {
                break;
            }
            for coefficient in &mut remainder.0 {
                self.step()?;
                *coefficient = Rat::zero().sub(coefficient, self.work)?;
            }
            sequence.try_reserve(1)?;
            sequence.push(self.normalize(remainder, true)?);
        }
        Ok(sequence)
    }

    fn variations(&mut self, sequence: &[Dense], at: &Rat) -> Result<usize> {
        let mut previous = Ordering::Equal;
        let mut count = 0;
        for polynomial in sequence {
            self.step()?;
            let current = sign(&self.evaluate(polynomial, at)?);
            if current == Ordering::Equal {
                continue;
            }
            if previous != Ordering::Equal && current != previous {
                count += 1;
            }
            previous = current;
        }
        Ok(count)
    }

    /// V(lower)-V(upper) counts distinct roots in (lower,upper]. Public root
    /// intervals additionally require that neither endpoint is a defining root.
    pub(super) fn count(&mut self, sequence: &[Dense], lower: &Rat, upper: &Rat) -> Result<usize> {
        self.step()?;
        let left = self.variations(sequence, lower)?;
        let right = self.variations(sequence, upper)?;
        left.checked_sub(right).ok_or(Error::AlgebraicInvariant)
    }

    /// Cauchy's strict bound `1 + max |a_i/a_n|` on every complex root modulus.
    pub(super) fn bound(&mut self, polynomial: &Dense) -> Result<Rat> {
        let leading = polynomial.0.last().ok_or(Error::AlgebraicInvariant)?;
        let mut bound = Rat::zero();
        for coefficient in polynomial.0.iter().take(polynomial.0.len() - 1) {
            self.step()?;
            let ratio = coefficient.div(leading, self.work)?;
            let magnitude = if ratio.is_negative() {
                Rat::zero().sub(&ratio, self.work)?
            } else {
                ratio
            };
            if self.compare(&magnitude, &bound)? == Ordering::Greater {
                bound = magnitude;
            }
        }
        bound.add(&Rat::one(), self.work)
    }
}
