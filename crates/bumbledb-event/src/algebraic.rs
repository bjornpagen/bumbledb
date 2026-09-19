//! Exact univariate real-root certificates. Sturm chains prove isolation and
//! signs; rational interval refinement never changes an answer into an estimate.
//! A defining polynomial is not necessarily a minimal polynomial. Consequently
//! algebraic equality is checked, not derived from its presentation or bytes.
use std::cmp::Ordering;
use std::sync::Arc;

use crate::{
    Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational, ParameterId, Result,
};

mod dense;
use dense::{Dense, Operation};

/// Polynomial degree, retained roots and visited solver operations. Bigint
/// steps/extent use the caller's shared `ExactArithmetic` budget separately.
#[derive(Debug, Clone, Copy)]
pub struct RootLimits {
    pub degree: u32,
    pub roots: usize,
    pub steps: usize,
}

impl Default for RootLimits {
    fn default() -> Self {
        Self {
            degree: 256,
            roots: 4096,
            steps: 1_000_000,
        }
    }
}

#[derive(Debug)]
struct RootBasis {
    parameter: ParameterId,
    polynomial: ExactPolynomial,
    square_free: Dense,
    sturm: Vec<Dense>,
}

/// An owned exact real number described by a polynomial and a checked isolating
/// interval. Equal endpoints denote a rational root. Otherwise the interval is
/// open, both endpoints are nonroots, and exactly one distinct real root lies
/// inside it. Multiple roots of the defining polynomial denote one value.
///
/// Do not compare descriptions as values. `compare` and `sign` establish exact
/// numeric judgments, including across different defining polynomials/names.
#[derive(Debug, Clone)]
pub struct AlgebraicRoot {
    basis: Arc<RootBasis>,
    lower: ExactRational,
    upper: ExactRational,
}

impl ExactPolynomial {
    /// Isolate every distinct real root in ascending order. Constant nonzero
    /// polynomials have an empty roster; the zero polynomial refuses because its
    /// roots are not a finite set. Parameter assignments are never discretized.
    /// # Errors
    /// Another parameter occurs, zero polynomial, limits or cancellation.
    pub fn isolate_roots(
        &self,
        parameter: ParameterId,
        limits: RootLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<AlgebraicRoot>> {
        let mut op = Operation::new(limits, work)?;
        let basis = Arc::new(op.basis(self, parameter)?);
        let bound = op.bound(&basis.square_free)?;
        let lower = ExactRational::zero().sub(&bound, op.work)?;
        let count = op.count(&basis.sturm, &lower, &bound)?;
        if count > limits.roots {
            return Err(Error::Capacity(Capacity::AlgebraicRoots));
        }
        let mut pending = Vec::new();
        pending.try_reserve(1)?;
        pending.push((lower, bound, count));
        let mut result = Vec::new();
        result.try_reserve_exact(count)?;
        while let Some((lower, upper, count)) = pending.pop() {
            op.step()?;
            if count == 0 {
                continue;
            }
            if count == 1 {
                result.push(AlgebraicRoot {
                    basis: basis.clone(),
                    lower,
                    upper,
                });
                continue;
            }
            let mut cut = op.midpoint(&lower, &upper)?;
            // Choose an interior nonroot. Each retry is a different rational;
            // a nonzero degree-d polynomial cannot exhaust d+1 candidates.
            while op.evaluate(&basis.square_free, &cut)?.is_zero() {
                op.step()?;
                cut = op.midpoint(&lower, &cut)?;
            }
            let left = op.count(&basis.sturm, &lower, &cut)?;
            let right = count.checked_sub(left).ok_or(Error::AlgebraicInvariant)?;
            pending.try_reserve(2)?;
            // Stack order emits the smaller root intervals first.
            pending.push((cut.clone(), upper, right));
            pending.push((lower, cut, left));
        }
        op.step()?;
        Ok(result)
    }
}

impl AlgebraicRoot {
    /// Reconstruct an exact root certificate from an untrusted description.
    /// The whole polynomial, endpoint order and isolation claim are checked.
    /// No serialized Sturm chain or claimed root count is trusted.
    /// # Errors
    /// Invalid/nonisolating interval, another parameter, limits or cancellation.
    pub fn from_interval(
        polynomial: &ExactPolynomial,
        parameter: ParameterId,
        lower: ExactRational,
        upper: ExactRational,
        limits: RootLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        op.work.validate(&lower)?;
        op.work.validate(&upper)?;
        let basis = op.basis(polynomial, parameter)?;
        if limits.roots == 0 {
            return Err(Error::Capacity(Capacity::AlgebraicRoots));
        }
        match op.compare(&lower, &upper)? {
            Ordering::Greater => return Err(Error::InvalidRootInterval),
            Ordering::Equal => {
                if !op.evaluate(&basis.square_free, &lower)?.is_zero() {
                    return Err(Error::InvalidRootInterval);
                }
            }
            Ordering::Less => {
                if op.evaluate(&basis.square_free, &lower)?.is_zero()
                    || op.evaluate(&basis.square_free, &upper)?.is_zero()
                    || op.count(&basis.sturm, &lower, &upper)? != 1
                {
                    return Err(Error::InvalidRootInterval);
                }
            }
        }
        op.step()?;
        Ok(Self {
            basis: Arc::new(basis),
            lower,
            upper,
        })
    }

    #[must_use]
    pub fn polynomial(&self) -> &ExactPolynomial {
        &self.basis.polynomial
    }
    #[must_use]
    pub fn parameter(&self) -> ParameterId {
        self.basis.parameter
    }
    /// Equal endpoints denote an exact rational; other endpoints are strict
    /// rational bounds, not the value or a tolerance-based equality criterion.
    #[must_use]
    pub fn interval(&self) -> (&ExactRational, &ExactRational) {
        (&self.lower, &self.upper)
    }

    /// Exact sign of a polynomial at this algebraic value. GCD detects zero;
    /// disjoint real-root intervals then establish the nonzero sign. Merely
    /// observing the same sign at two endpoints would be insufficient.
    /// # Errors
    /// Another parameter occurs, capacities or cancellation.
    pub fn sign(
        &self,
        polynomial: &ExactPolynomial,
        limits: RootLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Ordering> {
        let mut op = Operation::new(limits, work)?;
        op.validate_root(self)?;
        let polynomial = op.import_polynomial(polynomial, self.parameter())?;
        if self.lower == self.upper {
            return Ok(sign(&op.evaluate(&polynomial, &self.lower)?));
        }
        if polynomial.is_zero() {
            return Ok(Ordering::Equal);
        }
        let common = op.gcd(self.basis.square_free.clone(), polynomial.clone())?;
        let common_sturm = op.sturm(&common)?;
        if op.count(&common_sturm, &self.lower, &self.upper)? == 1 {
            return Ok(Ordering::Equal);
        }
        let square_free = op.square_free(&polynomial)?;
        let sturm = op.sturm(&square_free)?;
        let mut interval = Interval::from(self);
        loop {
            op.step()?;
            if interval.lower == interval.upper {
                return Ok(sign(&op.evaluate(&polynomial, &interval.lower)?));
            }
            if op.count(&sturm, &interval.lower, &interval.upper)? == 0 {
                let sample = op.midpoint(&interval.lower, &interval.upper)?;
                let result = sign(&op.evaluate(&polynomial, &sample)?);
                if result == Ordering::Equal {
                    return Err(Error::AlgebraicInvariant);
                }
                return Ok(result);
            }
            interval.refine(&self.basis, &mut op)?;
        }
    }

    /// Compare exact real values, including distinct presentations of the same
    /// root. Different parameter names here are formal polynomial indeterminates;
    /// numerical comparison neither binds nor equates their source identities.
    /// # Errors
    /// Input/intermediate capacities or cancellation. No approximate answer.
    pub fn compare(
        &self,
        other: &Self,
        limits: RootLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Ordering> {
        let mut op = Operation::new(limits, work)?;
        op.validate_root(self)?;
        op.validate_root(other)?;
        let common = op.gcd(
            self.basis.square_free.clone(),
            other.basis.square_free.clone(),
        )?;
        let sturm = op.sturm(&common)?;
        let mut a = Interval::from(self);
        let mut b = Interval::from(other);
        loop {
            op.step()?;
            if a.lower == a.upper && b.lower == b.upper {
                return op.compare(&a.lower, &b.lower);
            }
            if op.compare(&a.upper, &b.lower)? != Ordering::Greater {
                return Ok(Ordering::Less);
            }
            if op.compare(&a.lower, &b.upper)? != Ordering::Less {
                return Ok(Ordering::Greater);
            }
            if a.lower == a.upper {
                if op.evaluate(&other.basis.square_free, &a.lower)?.is_zero() {
                    return Ok(Ordering::Equal);
                }
            } else if b.lower == b.upper {
                if op.evaluate(&self.basis.square_free, &b.lower)?.is_zero() {
                    return Ok(Ordering::Equal);
                }
            } else {
                let lower = if op.compare(&a.lower, &b.lower)? == Ordering::Greater {
                    &a.lower
                } else {
                    &b.lower
                };
                let upper = if op.compare(&a.upper, &b.upper)? == Ordering::Less {
                    &a.upper
                } else {
                    &b.upper
                };
                if op.count(&sturm, lower, upper)? == 1 {
                    return Ok(Ordering::Equal);
                }
            }
            a.refine(&self.basis, &mut op)?;
            b.refine(&other.basis, &mut op)?;
        }
    }
}

pub(super) fn sign(value: &ExactRational) -> Ordering {
    if value.is_zero() {
        Ordering::Equal
    } else if value.is_negative() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

struct Interval {
    lower: ExactRational,
    upper: ExactRational,
}
impl From<&AlgebraicRoot> for Interval {
    fn from(root: &AlgebraicRoot) -> Self {
        Self {
            lower: root.lower.clone(),
            upper: root.upper.clone(),
        }
    }
}
impl Interval {
    fn refine(&mut self, basis: &RootBasis, op: &mut Operation<'_, '_>) -> Result<()> {
        op.step()?;
        if self.lower == self.upper {
            return Ok(());
        }
        let middle = op.midpoint(&self.lower, &self.upper)?;
        if op.evaluate(&basis.square_free, &middle)?.is_zero() {
            self.lower = middle.clone();
            self.upper = middle;
        } else {
            match op.count(&basis.sturm, &self.lower, &middle)? {
                0 => self.lower = middle,
                1 => self.upper = middle,
                _ => return Err(Error::AlgebraicInvariant),
            }
        }
        Ok(())
    }
}
