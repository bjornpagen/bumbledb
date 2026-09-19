//! Exact semialgebraic regions on one named real parameter. Algebraic boundary
//! points and open sectors form a finite logical partition, never a distribution.
use std::cmp::Ordering;
use std::sync::Arc;

use crate::{
    AlgebraicRoot, BoolOp4, Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational,
    ParameterId, PolynomialLimits, Result, RootLimits,
};

mod function;
mod piecewise;
pub(crate) mod source;
mod wire;
pub use function::GuardedRationalFunction;
pub use piecewise::ParameterFunction;
pub use source::{
    FamilyFunction, FamilyFunctionPiece, ParameterConditioning, ParameterDensityPiece,
    ParameterExpectationObservation, ParameterGuard, ParameterJeffrey, ParameterLikelihood,
    ParameterProbabilityObservation, ParameterRefinement, ParameterRestriction,
    ParameterRevisedSource, ParameterSourceLimits, ParameterWorld, WorldCardinality,
};
pub use wire::ParameterCodecLimits;

/// A subset of the three exact polynomial signs: negative, zero, positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolynomialSigns(u8);
impl PolynomialSigns {
    pub const NONE: Self = Self(0);
    pub const NEGATIVE: Self = Self(1);
    pub const ZERO: Self = Self(2);
    pub const NON_POSITIVE: Self = Self(3);
    pub const POSITIVE: Self = Self(4);
    pub const NON_ZERO: Self = Self(5);
    pub const NON_NEGATIVE: Self = Self(6);
    pub const ANY: Self = Self(7);
    #[must_use]
    pub const fn new(bits: u8) -> Option<Self> {
        if bits < 8 { Some(Self(bits)) } else { None }
    }
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn contains(self, sign: Ordering) -> bool {
        let bit = match sign {
            Ordering::Less => 1,
            Ordering::Equal => 2,
            Ordering::Greater => 4,
        };
        self.0 & bit != 0
    }
    #[must_use]
    pub const fn negated(self) -> Self {
        Self((self.0 & 2) | ((self.0 & 1) << 2) | ((self.0 & 4) >> 2))
    }
}

/// Each root operation obeys `roots`; all arithmetic shares the caller's work
/// budget. `steps` additionally bounds visited domain cells and solver calls.
#[derive(Debug, Clone, Copy)]
pub struct ParameterLimits {
    pub polynomial: PolynomialLimits,
    pub roots: RootLimits,
    pub cells: usize,
    pub steps: usize,
}
impl Default for ParameterLimits {
    fn default() -> Self {
        Self {
            polynomial: PolynomialLimits::default(),
            roots: RootLimits::default(),
            cells: 65_537,
            steps: 1_000_000,
        }
    }
}

/// Exact scalar witnesses. The parameter name of an algebraic description is
/// its formal indeterminate; using the number does not alias source identities.
#[derive(Debug, Clone)]
pub enum RealWitness {
    Rational(ExactRational),
    Algebraic(AlgebraicRoot),
}

/// Cells cover the entire real line. Open sectors exclude their endpoints;
/// isolated boundary points have their own membership, including zero mass.
#[derive(Debug, Clone, Copy)]
pub enum ParameterCell<'a> {
    Point(&'a AlgebraicRoot),
    Open {
        lower: Option<&'a AlgebraicRoot>,
        upper: Option<&'a AlgebraicRoot>,
    },
}

#[derive(Debug)]
struct Partition {
    roots: Box<[AlgebraicRoot]>,
    membership: Box<[bool]>,
}

/// A possibly empty semialgebraic subset of one named real coordinate. Every
/// retained boundary is necessary: a point with identical membership on both
/// adjoining open sectors is removed. Endpoint descriptions are not canonical
/// numeric bytes, so equality requires the fallible `equivalent` judgment.
#[derive(Debug, Clone)]
pub struct ParameterRegion {
    parameter: ParameterId,
    partition: Arc<Partition>,
    complemented: bool,
}

/// An admitted inhabited parameter domain. Empty *regions* remain valid, but
/// cannot silently designate a source with no parameter assignments.
#[derive(Debug, Clone)]
pub struct ParameterDomain {
    region: ParameterRegion,
}

impl ParameterDomain {
    /// # Errors
    /// An empty parameter region. The region is already mathematically checked.
    pub fn new(region: ParameterRegion) -> Result<Self> {
        if region.is_empty() {
            return Err(Error::EmptyParameterDomain);
        }
        Ok(Self { region })
    }
    #[must_use]
    pub fn region(&self) -> &ParameterRegion {
        &self.region
    }
    #[must_use]
    pub fn parameter(&self) -> ParameterId {
        self.region.parameter()
    }
}

impl ParameterRegion {
    #[must_use]
    pub fn empty(parameter: ParameterId) -> Self {
        Self::constant(parameter, false)
    }
    #[must_use]
    pub fn full(parameter: ParameterId) -> Self {
        Self::constant(parameter, true)
    }
    fn constant(parameter: ParameterId, value: bool) -> Self {
        Self {
            parameter,
            partition: Arc::new(Partition {
                roots: Box::new([]),
                membership: Box::new([value]),
            }),
            complemented: false,
        }
    }
    #[must_use]
    pub fn parameter(&self) -> ParameterId {
        self.parameter
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.partition.roots.is_empty() && !self.member(0)
    }
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.partition.roots.is_empty() && self.member(0)
    }
    #[must_use]
    pub fn complement(&self) -> Self {
        Self {
            complemented: !self.complemented,
            ..self.clone()
        }
    }
    fn member(&self, index: usize) -> bool {
        self.partition.membership[index] ^ self.complemented
    }
    pub fn cells(&self) -> impl ExactSizeIterator<Item = (ParameterCell<'_>, bool)> {
        (0..self.partition.membership.len()).map(|i| {
            let cell = if i % 2 == 1 {
                ParameterCell::Point(&self.partition.roots[i / 2])
            } else {
                ParameterCell::Open {
                    lower: (i / 2).checked_sub(1).map(|j| &self.partition.roots[j]),
                    upper: self.partition.roots.get(i / 2),
                }
            };
            (cell, self.member(i))
        })
    }

    /// Solve a polynomial sign predicate over the real line. Every open sector
    /// is sampled only after all polynomial roots have been isolated. A zero
    /// polynomial has a constant zero sign; it is not an empty-domain failure.
    /// # Errors
    /// Another parameter occurs, intermediate capacities or cancellation.
    pub fn from_polynomial(
        parameter: ParameterId,
        polynomial: &ExactPolynomial,
        signs: PolynomialSigns,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        polynomial.validate(limits.polynomial, op.work)?;
        if polynomial.is_zero() {
            op.extent(1)?;
            return Ok(Self::constant(parameter, signs.contains(Ordering::Equal)));
        }
        let roots = polynomial.isolate_roots(parameter, limits.roots, op.work)?;
        let count = roots
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::Capacity(Capacity::ParameterCells))?;
        op.extent(count)?;
        let mut membership = Vec::new();
        membership.try_reserve_exact(count)?;
        for i in 0..=roots.len() {
            op.step()?;
            let sample = op.sample(&roots, i)?;
            let value = polynomial.evaluate(&[(parameter, sample)], limits.polynomial, op.work)?;
            let sign = if value.is_zero() {
                Ordering::Equal
            } else if value.is_negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
            membership.push(signs.contains(sign));
            if i < roots.len() {
                membership.push(signs.contains(Ordering::Equal));
            }
        }
        op.finish(parameter, roots, &membership)
    }

    /// Apply any binary Boolean operation after exact boundary alignment. Both
    /// scopes and all input root descriptions participate even for empty/full.
    /// # Errors
    /// Parameter-name mismatch, capacities or cancellation.
    pub fn apply(
        &self,
        operation: BoolOp4,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        if self.parameter != other.parameter {
            return Err(Error::ParameterScopeMismatch);
        }
        op.validate(self)?;
        op.validate(other)?;
        let mut roots = Vec::new();
        let mut membership = Vec::new();
        membership.try_reserve(1)?;
        membership.push(operation.evaluate(self.member(0), other.member(0)));
        let (mut a, mut b) = (0, 0);
        while a < self.partition.roots.len() || b < other.partition.roots.len() {
            op.step()?;
            let order = match (self.partition.roots.get(a), other.partition.roots.get(b)) {
                (Some(x), Some(y)) => x.compare(y, limits.roots, op.work)?,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => break,
            };
            let take_a = order != Ordering::Greater;
            let take_b = order != Ordering::Less;
            let root = if take_a {
                self.partition.roots[a].clone()
            } else {
                other.partition.roots[b].clone()
            };
            let point_a = self.member(2 * a + usize::from(take_a));
            let point_b = other.member(2 * b + usize::from(take_b));
            a += usize::from(take_a);
            b += usize::from(take_b);
            op.extent(membership.len().saturating_add(2))?;
            roots.try_reserve(1)?;
            membership.try_reserve(2)?;
            roots.push(root);
            membership.push(operation.evaluate(point_a, point_b));
            membership.push(operation.evaluate(self.member(2 * a), other.member(2 * b)));
        }
        op.finish(self.parameter, roots, &membership)
    }

    /// # Errors
    /// As `apply`; equality is semantic, not presentation/interval equality.
    pub fn equivalent(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        Ok(self.apply(BoolOp4::XOR, other, limits, work)?.is_empty())
    }
    /// # Errors
    /// As `apply`.
    pub fn included(
        &self,
        other: &Self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        Ok(self
            .apply(BoolOp4::DIFFERENCE, other, limits, work)?
            .is_empty())
    }

    /// Test an exact scalar point. An algebraic description's formal variable
    /// name does not bind or rename this region's captured parameter.
    /// # Errors
    /// Input/intermediate capacities or cancellation, even for constant regions.
    pub fn contains(
        &self,
        point: &RealWitness,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        match point {
            RealWitness::Rational(value) => op.work.validate(value)?,
            RealWitness::Algebraic(value) => {
                value.sign(&ExactPolynomial::zero(), limits.roots, op.work)?;
            }
        }
        let mut left = 0;
        let mut right = self.partition.roots.len();
        while left < right {
            op.step()?;
            let middle = left + (right - left) / 2;
            let order = match point {
                RealWitness::Rational(value) => {
                    self.partition.roots[middle].compare_rational(value, limits.roots, op.work)?
                }
                RealWitness::Algebraic(value) => {
                    self.partition.roots[middle].compare(value, limits.roots, op.work)?
                }
            };
            match order {
                Ordering::Equal => return Ok(self.member(2 * middle + 1)),
                Ordering::Less => left = middle + 1,
                Ordering::Greater => right = middle,
            }
        }
        op.step()?;
        Ok(self.member(2 * left))
    }

    /// Return an exact member, including an algebraic singleton when no rational
    /// member exists. `None` proves the region empty, not a failed search.
    /// # Errors
    /// Input/intermediate capacities or cancellation.
    pub fn witness(
        &self,
        limits: ParameterLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<RealWitness>> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        for i in 0..self.partition.membership.len() {
            op.step()?;
            if self.member(i) {
                return Ok(Some(if i % 2 == 1 {
                    RealWitness::Algebraic(self.partition.roots[i / 2].clone())
                } else {
                    RealWitness::Rational(op.sample(&self.partition.roots, i / 2)?)
                }));
            }
        }
        Ok(None)
    }
}

struct Operation<'a, 'b> {
    limits: ParameterLimits,
    steps: usize,
    work: &'a mut ExactArithmetic<'b>,
}
impl<'a, 'b> Operation<'a, 'b> {
    fn new(limits: ParameterLimits, work: &'a mut ExactArithmetic<'b>) -> Result<Self> {
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
            return Err(Error::Capacity(Capacity::ParameterSteps));
        }
        self.steps += 1;
        Ok(())
    }
    fn extent(&mut self, cells: usize) -> Result<()> {
        self.step()?;
        if cells > self.limits.cells {
            return Err(Error::Capacity(Capacity::ParameterCells));
        }
        Ok(())
    }
    fn validate(&mut self, region: &ParameterRegion) -> Result<()> {
        self.extent(region.partition.membership.len())?;
        for root in &region.partition.roots {
            self.step()?;
            root.polynomial()
                .validate(self.limits.polynomial, self.work)?;
            root.sign(&ExactPolynomial::zero(), self.limits.roots, self.work)?;
        }
        Ok(())
    }
    fn polynomial(&mut self, parameter: ParameterId, value: &ExactPolynomial) -> Result<()> {
        self.step()?;
        value.validate(self.limits.polynomial, self.work)?;
        for term in value.terms() {
            for &(name, _) in &term.powers {
                self.step()?;
                if name != parameter {
                    return Err(Error::ParameterScopeMismatch);
                }
            }
        }
        Ok(())
    }
    fn sample(&mut self, roots: &[AlgebraicRoot], sector: usize) -> Result<ExactRational> {
        self.step()?;
        match (
            sector.checked_sub(1).and_then(|i| roots.get(i)),
            roots.get(sector),
        ) {
            (None, None) => Ok(ExactRational::zero()),
            (None, Some(right)) => right.interval().0.sub(&ExactRational::one(), self.work),
            (Some(left), None) => left.interval().1.add(&ExactRational::one(), self.work),
            (Some(left), Some(right)) => left.rational_between(right, self.limits.roots, self.work),
        }
    }
    fn finish(
        &mut self,
        parameter: ParameterId,
        roots: Vec<AlgebraicRoot>,
        membership: &[bool],
    ) -> Result<ParameterRegion> {
        self.step()?;
        let mut kept = Vec::new();
        let mut cells = Vec::new();
        cells.try_reserve(1)?;
        cells.push(membership[0]);
        for (i, root) in roots.into_iter().enumerate() {
            self.step()?;
            let (left, point, right) = (
                membership[2 * i],
                membership[2 * i + 1],
                membership[2 * i + 2],
            );
            if left == point && point == right {
                continue;
            }
            kept.try_reserve(1)?;
            cells.try_reserve(2)?;
            kept.push(root);
            cells.push(point);
            cells.push(right);
        }
        self.step()?;
        Ok(ParameterRegion {
            parameter,
            partition: Arc::new(Partition {
                roots: kept.into_boxed_slice(),
                membership: cells.into_boxed_slice(),
            }),
            complemented: false,
        })
    }
}
