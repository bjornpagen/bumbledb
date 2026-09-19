//! Kronecker's finite search. An integer polynomial's primitive rational factor
//! has integer coefficients and divides every nonzero integer evaluation. For
//! degree k, k+1 values determine that factor. Both signs are searched, modulo
//! one global sign. Every candidate is checked by exact polynomial division.
use super::{Dense, Operation, Rat};
use crate::{Capacity, Error, Result};

struct Sample {
    x: Rat,
    positive_divisors: Vec<Rat>,
}

impl Operation<'_, '_> {
    /// Clear all coefficient denominators. Primitivity is unnecessary: Gauss's
    /// lemma still places every primitive rational factor in Z[x] dividing f.
    fn integer_polynomial(&mut self, polynomial: &Dense) -> Result<Dense> {
        let mut scale = Rat::one();
        for coefficient in &polynomial.0 {
            self.step()?;
            let denominator = coefficient.denominator_value(self.root.work)?;
            scale = scale.mul(&denominator, self.root.work)?;
        }
        let mut result = Vec::new();
        result.try_reserve_exact(polynomial.0.len())?;
        for coefficient in &polynomial.0 {
            self.step()?;
            result.push(coefficient.mul(&scale, self.root.work)?);
        }
        Ok(Dense(result))
    }

    fn divisors(&mut self, integer: &Rat) -> Result<Vec<Rat>> {
        self.step()?;
        if integer.is_zero() || !integer.is_integer() {
            return Err(Error::AlgebraicInvariant);
        }
        let value = if integer.is_negative() {
            Rat::zero().sub(integer, self.root.work)?
        } else {
            integer.clone()
        };
        let mut result = Vec::new();
        let mut trial = 1u64;
        loop {
            self.step()?;
            let divisor = Rat::from(trial);
            let quotient = value.div(&divisor, self.root.work)?;
            let comparison = quotient.sub(&divisor, self.root.work)?;
            if comparison.is_negative() {
                break;
            }
            if quotient.is_integer() {
                result.try_reserve(2)?;
                result.push(divisor);
                if !comparison.is_zero() {
                    result.push(quotient);
                }
            }
            trial = trial
                .checked_add(1)
                .ok_or(Error::Capacity(Capacity::AlgebraicIdentitySteps))?;
        }
        Ok(result)
    }

    fn sample(&mut self, polynomial: &Dense, next: &mut u64) -> Result<Sample> {
        loop {
            self.step()?;
            // 0, 1, -1, 2, -2, ...; distinct nodes, regardless of skipped roots.
            let magnitude = *next / 2 + *next % 2;
            let positive = Rat::from(magnitude);
            let x = if *next != 0 && (*next).is_multiple_of(2) {
                Rat::zero().sub(&positive, self.root.work)?
            } else {
                positive
            };
            *next = next
                .checked_add(1)
                .ok_or(Error::Capacity(Capacity::AlgebraicIdentitySteps))?;
            let value = self.root.evaluate(polynomial, &x)?;
            if !value.is_zero() {
                return Ok(Sample {
                    x,
                    positive_divisors: self.divisors(&value)?,
                });
            }
        }
    }

    fn lagrange(&mut self, samples: &[Sample]) -> Result<Vec<Dense>> {
        let mut result = Vec::new();
        result.try_reserve_exact(samples.len())?;
        for (i, sample) in samples.iter().enumerate() {
            self.step()?;
            let mut basis = vec![Rat::one()];
            let mut divisor = Rat::one();
            for (j, other) in samples.iter().enumerate() {
                self.step()?;
                if i == j {
                    continue;
                }
                let difference = sample.x.sub(&other.x, self.root.work)?;
                divisor = divisor.mul(&difference, self.root.work)?;
                let mut next = Vec::new();
                next.try_reserve_exact(basis.len() + 1)?;
                next.resize(basis.len() + 1, Rat::zero());
                for (power, value) in basis.iter().enumerate() {
                    self.step()?;
                    let constant = value.mul(&other.x, self.root.work)?;
                    next[power] = next[power].sub(&constant, self.root.work)?;
                    next[power + 1] = next[power + 1].add(value, self.root.work)?;
                }
                basis = next;
            }
            for coefficient in &mut basis {
                self.step()?;
                *coefficient = coefficient.div(&divisor, self.root.work)?;
            }
            result.push(Dense(basis));
        }
        Ok(result)
    }

    fn interpolate(
        &mut self,
        samples: &[Sample],
        basis: &[Dense],
        indices: &[usize],
    ) -> Result<Dense> {
        self.step()?;
        if self.candidates >= self.limits.candidates {
            return Err(Error::Capacity(Capacity::AlgebraicCandidates));
        }
        self.candidates += 1;
        let mut result = Vec::new();
        result.try_reserve_exact(samples.len())?;
        result.resize(samples.len(), Rat::zero());
        for ((sample, basis), &index) in samples.iter().zip(basis).zip(indices) {
            self.step()?;
            let count = sample.positive_divisors.len();
            let positive = &sample.positive_divisors[index % count];
            let value = if index >= count {
                Rat::zero().sub(positive, self.root.work)?
            } else {
                positive.clone()
            };
            for (out, coefficient) in result.iter_mut().zip(&basis.0) {
                self.step()?;
                *out = out.add(&coefficient.mul(&value, self.root.work)?, self.root.work)?;
            }
        }
        let mut result = Dense(result);
        result.trim();
        Ok(result)
    }

    pub(super) fn split(&mut self, polynomial: &Dense) -> Result<Option<(Dense, Dense)>> {
        self.step()?;
        let degree = polynomial.0.len().saturating_sub(1);
        if degree < 2 {
            return Ok(None);
        }
        let integer = self.integer_polynomial(polynomial)?;
        let mut samples = Vec::new();
        samples.try_reserve_exact(degree / 2 + 1)?;
        let mut next = 0;
        for factor_degree in 1..=degree / 2 {
            while samples.len() <= factor_degree {
                samples.push(self.sample(&integer, &mut next)?);
            }
            let basis = self.lagrange(&samples)?;
            let mut indices = Vec::new();
            indices.try_reserve_exact(samples.len())?;
            indices.resize(samples.len(), 0);
            loop {
                let candidate = self.interpolate(&samples, &basis, &indices)?;
                if candidate.0.len() == factor_degree + 1 && candidate.0.iter().all(Rat::is_integer)
                {
                    let (quotient, remainder) = self.root.divide(polynomial, &candidate)?;
                    if remainder.is_zero() {
                        return Ok(Some((candidate, quotient)));
                    }
                }
                // All signed values, with only the first value made positive:
                // negating an integer factor changes no rational factorization.
                let mut carry = true;
                for (i, (index, sample)) in indices.iter_mut().zip(&samples).enumerate().rev() {
                    self.step()?;
                    let radix = sample
                        .positive_divisors
                        .len()
                        .checked_mul(if i == 0 { 1 } else { 2 })
                        .ok_or(Error::Allocation)?;
                    *index += 1;
                    if *index < radix {
                        carry = false;
                        break;
                    }
                    *index = 0;
                }
                if carry {
                    break;
                }
            }
        }
        // Every possible integer factor of each degree <= floor(n/2) was
        // tested. Any reducible polynomial has such a proper rational factor.
        Ok(None)
    }
}
