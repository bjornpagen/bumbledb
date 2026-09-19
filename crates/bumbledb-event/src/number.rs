//! Exact partial numerical values. This is arithmetic on observations, not a
//! construction of a joint law. Source/evidence provenance belongs to the
//! enclosing observation; numerical equivalence never identifies its source.
use std::cmp::Ordering;

use crate::{
    BoolOp4, Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational, FunctionLimits,
    GuardedRationalFunction, ParameterDomain, ParameterFunction, ParameterLimits, ParameterRegion,
    PolynomialSigns, Result,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct NumberLimits {
    pub parameters: ParameterLimits,
    pub functions: FunctionLimits,
}

/// A rational value on a single fixed assignment, or a partial function on an
/// inhabited named parameter domain. `None` is undefined, never numerical zero.
/// The current parameter solver is univariate; combining different parameter
/// names requires an explicit multivariate construction, not name erasure.
#[derive(Debug, Clone)]
pub enum PartialNumber {
    Fixed(Option<ExactRational>),
    Parameter(ParameterFunction),
}

/// A predicate's complete truth partition. Undefined assignments remain a
/// third region under negation; complementing the true region alone is wrong.
#[derive(Debug, Clone)]
pub struct NumberPredicate {
    cases: PredicateCases,
}

#[derive(Debug, Clone)]
enum PredicateCases {
    Fixed(Option<bool>),
    Parameter {
        ambient: ParameterDomain,
        holds: ParameterRegion,
        fails: ParameterRegion,
        undefined: ParameterRegion,
    },
}

/// Borrowed, exact predicate regions. For fixed values, the domain has one
/// assignment and `None` means that assignment is undefined.
#[derive(Debug, Clone, Copy)]
pub enum NumberPredicateView<'a> {
    Fixed(Option<bool>),
    Parameter {
        ambient: &'a ParameterDomain,
        holds: &'a ParameterRegion,
        fails: &'a ParameterRegion,
        undefined: &'a ParameterRegion,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Min,
    Max,
}

impl PartialNumber {
    /// Revalidate complete operands under the caller's shared arithmetic and
    /// solver limits, including values later masked by an undefined operand.
    /// # Errors
    /// Capacities or cancellation.
    pub fn validate(&self, limits: NumberLimits, work: &mut ExactArithmetic<'_>) -> Result<()> {
        work.validate(&ExactRational::zero())?;
        match self {
            Self::Fixed(Some(value)) => work.validate(value),
            Self::Fixed(None) => Ok(()),
            Self::Parameter(value) => value
                .where_sign(
                    PolynomialSigns::ANY,
                    limits.parameters,
                    limits.functions,
                    work,
                )
                .map(|_| ()),
        }
    }

    /// Explicitly restrict a function's ambient domain, or lift a fixed value
    /// to that domain. Undefined fixed values become nowhere-defined functions.
    /// # Errors
    /// Foreign/extended domains, capacities or cancellation.
    pub fn on_domain(
        &self,
        domain: &ParameterDomain,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.validate(limits, work)?;
        Ok(Self::Parameter(self.in_domain(domain, limits, work)?))
    }

    fn in_domain(
        &self,
        domain: &ParameterDomain,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterFunction> {
        match self {
            Self::Parameter(value) => {
                value.on_domain(domain, limits.parameters, limits.functions, work)
            }
            Self::Fixed(value) => {
                let pieces = value
                    .as_ref()
                    .map(|value| {
                        GuardedRationalFunction::new(
                            domain.clone(),
                            ExactPolynomial::constant(value.clone()),
                            ExactPolynomial::one(),
                            limits.parameters,
                            work,
                        )
                    })
                    .transpose()?;
                ParameterFunction::new(
                    domain.clone(),
                    pieces.as_slice(),
                    limits.parameters,
                    limits.functions,
                    work,
                )
            }
        }
    }

    /// # Errors
    /// Incompatible ambient domains, capacities or cancellation.
    pub fn add(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Add, rhs, limits, work)
    }
    /// # Errors
    /// As `add`. In particular, `x - x` retains all undefined assignments of x.
    pub fn sub(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Subtract, rhs, limits, work)
    }
    /// # Errors
    /// As `add`. Multiplication by zero never fills an undefined assignment.
    pub fn mul(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Multiply, rhs, limits, work)
    }
    /// Zero divisors exclude assignments from the result's defined domain.
    /// # Errors
    /// As `add`. Resource errors remain errors, not undefined numerical values.
    pub fn div(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Divide, rhs, limits, work)
    }
    /// Pointwise minimum on the intersection of the defined domains.
    /// # Errors
    /// As `add`.
    pub fn min(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Min, rhs, limits, work)
    }
    /// Pointwise maximum on the intersection of the defined domains.
    /// # Errors
    /// As `add`.
    pub fn max(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.apply(NumberOp::Max, rhs, limits, work)
    }
    /// # Errors
    /// Capacities or cancellation. Undefined points are retained.
    pub fn negate(&self, limits: NumberLimits, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        Self::Fixed(Some(ExactRational::zero())).sub(self, limits, work)
    }
    /// # Errors
    /// Capacities or cancellation. Undefined points are retained.
    pub fn abs(&self, limits: NumberLimits, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        self.max(&self.negate(limits, work)?, limits, work)
    }

    /// Natural powers, retaining the input's defined domain even at exponent
    /// zero. This is strict partial arithmetic: undefined-to-zero is undefined.
    /// # Errors
    /// Capacities or cancellation.
    pub fn pow(
        &self,
        mut exponent: u32,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.validate(limits, work)?;
        let mut result = match self {
            Self::Fixed(value) => Self::Fixed(value.as_ref().map(|_| ExactRational::one())),
            Self::Parameter(value) => {
                let one = Self::Fixed(Some(ExactRational::one())).in_domain(
                    value.ambient(),
                    limits,
                    work,
                )?;
                Self::Parameter(one.restrict(
                    value.defined_on(),
                    limits.parameters,
                    limits.functions,
                    work,
                )?)
            }
        };
        let mut factor = self.clone();
        while exponent != 0 {
            if exponent & 1 != 0 {
                result = result.mul(&factor, limits, work)?;
            }
            exponent >>= 1;
            if exponent != 0 {
                factor = factor.mul(&factor, limits, work)?;
            }
        }
        Ok(result)
    }

    /// Apply one strict exact binary operation, retaining undefined points.
    /// # Errors
    /// Incompatible ambient domains, capacities or cancellation.
    pub fn apply(
        &self,
        op: NumberOp,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.validate(limits, work)?;
        rhs.validate(limits, work)?;
        if let (Self::Fixed(left), Self::Fixed(right)) = (self, rhs) {
            let (Some(a), Some(b)) = (left, right) else {
                return Ok(Self::Fixed(None));
            };
            let result = match op {
                NumberOp::Add => a.add(b, work)?,
                NumberOp::Subtract => a.sub(b, work)?,
                NumberOp::Multiply => a.mul(b, work)?,
                NumberOp::Divide if b.is_zero() => return Ok(Self::Fixed(None)),
                NumberOp::Divide => a.div(b, work)?,
                NumberOp::Min | NumberOp::Max => {
                    let choose_a = a.sub(b, work)?.is_negative();
                    if choose_a == matches!(op, NumberOp::Min) {
                        a.clone()
                    } else {
                        b.clone()
                    }
                }
            };
            Ok(Self::Fixed(Some(result)))
        } else {
            // Parameter/parameter operations below check the entire ambient
            // domain; they never silently intersect different source domains.
            let (a, b) = match (self, rhs) {
                (Self::Parameter(a), Self::Parameter(b)) => (a.clone(), b.clone()),
                (Self::Parameter(a), _) => (a.clone(), rhs.in_domain(a.ambient(), limits, work)?),
                (_, Self::Parameter(b)) => (self.in_domain(b.ambient(), limits, work)?, b.clone()),
                _ => unreachable!("fixed pair handled above"),
            };
            let result = match op {
                NumberOp::Add => a.add(&b, limits.parameters, limits.functions, work)?,
                NumberOp::Subtract => a.sub(&b, limits.parameters, limits.functions, work)?,
                NumberOp::Multiply => a.mul(&b, limits.parameters, limits.functions, work)?,
                NumberOp::Divide => a.div(&b, limits.parameters, limits.functions, work)?,
                NumberOp::Min | NumberOp::Max => {
                    let delta = a.sub(&b, limits.parameters, limits.functions, work)?;
                    let signs = if matches!(op, NumberOp::Min) {
                        PolynomialSigns::NON_POSITIVE
                    } else {
                        PolynomialSigns::NON_NEGATIVE
                    };
                    let left =
                        delta.where_sign(signs, limits.parameters, limits.functions, work)?;
                    let right = delta.defined_on().apply(
                        BoolOp4::DIFFERENCE,
                        &left,
                        limits.parameters,
                        work,
                    )?;
                    let left = a.restrict(&left, limits.parameters, limits.functions, work)?;
                    let right = b.restrict(&right, limits.parameters, limits.functions, work)?;
                    let count = left
                        .pieces()
                        .len()
                        .checked_add(right.pieces().len())
                        .ok_or(Error::Capacity(Capacity::FunctionCells))?;
                    if count > limits.functions.cells {
                        return Err(Error::Capacity(Capacity::FunctionCells));
                    }
                    let mut pieces = Vec::new();
                    pieces.try_reserve_exact(count)?;
                    pieces.extend_from_slice(left.pieces());
                    pieces.extend_from_slice(right.pieces());
                    ParameterFunction::new(
                        a.ambient().clone(),
                        &pieces,
                        limits.parameters,
                        limits.functions,
                        work,
                    )?
                }
            };
            Ok(Self::Parameter(result))
        }
    }

    /// Pointwise sign predicate with an explicit undefined region.
    /// # Errors
    /// Capacities or cancellation; every piece participates for every mask.
    pub fn where_sign(
        &self,
        signs: PolynomialSigns,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<NumberPredicate> {
        self.validate(limits, work)?;
        let cases = match self {
            Self::Fixed(value) => PredicateCases::Fixed(value.as_ref().map(|value| {
                let sign = if value.is_zero() {
                    Ordering::Equal
                } else if value.is_negative() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                signs.contains(sign)
            })),
            Self::Parameter(value) => {
                let holds = value.where_sign(signs, limits.parameters, limits.functions, work)?;
                let fails = value.defined_on().apply(
                    BoolOp4::DIFFERENCE,
                    &holds,
                    limits.parameters,
                    work,
                )?;
                let undefined = value.ambient().region().apply(
                    BoolOp4::DIFFERENCE,
                    value.defined_on(),
                    limits.parameters,
                    work,
                )?;
                PredicateCases::Parameter {
                    ambient: value.ambient().clone(),
                    holds,
                    fails,
                    undefined,
                }
            }
        };
        Ok(NumberPredicate { cases })
    }

    /// Compare numerical values on their common defined domain. ZERO denotes
    /// equality, NEGATIVE less-than, `NON_NEGATIVE` greater-or-equal, etc.
    /// # Errors
    /// Domain mismatch, capacities or cancellation.
    pub fn compare(
        &self,
        rhs: &Self,
        signs: PolynomialSigns,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<NumberPredicate> {
        self.sub(rhs, limits, work)?.where_sign(signs, limits, work)
    }

    /// Extensional partial-function equality, including defined domains. It is
    /// distinct from a pointwise equality predicate, which retains undefinedness.
    /// # Errors
    /// Domain mismatch, capacities or cancellation.
    pub fn equivalent(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        self.validate(limits, work)?;
        rhs.validate(limits, work)?;
        match (self, rhs) {
            (Self::Fixed(a), Self::Fixed(b)) => Ok(a == b),
            (Self::Parameter(a), Self::Parameter(b)) => {
                a.equivalent(b, limits.parameters, limits.functions, work)
            }
            (Self::Parameter(a), _) => a.equivalent(
                &rhs.in_domain(a.ambient(), limits, work)?,
                limits.parameters,
                limits.functions,
                work,
            ),
            (_, Self::Parameter(b)) => self.in_domain(b.ambient(), limits, work)?.equivalent(
                b,
                limits.parameters,
                limits.functions,
                work,
            ),
        }
    }
}

impl NumberPredicate {
    /// Recheck every truth region under the caller's exact solver budget.
    /// # Errors
    /// Capacities or cancellation, including constant truth partitions.
    pub fn validate(&self, limits: NumberLimits, work: &mut ExactArithmetic<'_>) -> Result<()> {
        work.validate(&ExactRational::zero())?;
        if let PredicateCases::Parameter {
            ambient,
            holds,
            fails,
            undefined,
        } = &self.cases
        {
            for region in [ambient.region(), holds, fails, undefined] {
                region.equivalent(region, limits.parameters, work)?;
            }
        }
        Ok(())
    }

    /// Restrict the ambient domain explicitly, or lift a fixed partial truth
    /// value to it. This never fills a hole or changes an observation's source.
    /// # Errors
    /// Foreign/extended domains, capacities or cancellation.
    pub fn on_domain(
        &self,
        domain: &ParameterDomain,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.validate(limits, work)?;
        domain
            .region()
            .equivalent(domain.region(), limits.parameters, work)?;
        let cases = match &self.cases {
            PredicateCases::Fixed(value) => {
                let region = |case| {
                    if *value == case {
                        domain.region().clone()
                    } else {
                        ParameterRegion::empty(domain.parameter())
                    }
                };
                PredicateCases::Parameter {
                    ambient: domain.clone(),
                    holds: region(Some(true)),
                    fails: region(Some(false)),
                    undefined: region(None),
                }
            }
            PredicateCases::Parameter {
                ambient,
                holds,
                fails,
                undefined,
            } => {
                if !domain
                    .region()
                    .included(ambient.region(), limits.parameters, work)?
                {
                    return Err(Error::ParameterDomainMismatch);
                }
                let mut clip = |region: &ParameterRegion| {
                    region.apply(BoolOp4::AND, domain.region(), limits.parameters, work)
                };
                PredicateCases::Parameter {
                    ambient: domain.clone(),
                    holds: clip(holds)?,
                    fails: clip(fails)?,
                    undefined: clip(undefined)?,
                }
            }
        };
        Ok(Self { cases })
    }

    /// Extensional equality of all three truth regions. Unlike a strict
    /// Boolean equality operation, identical undefined regions compare equal.
    /// # Errors
    /// Foreign/different ambient domains, capacities or cancellation.
    pub fn equivalent(
        &self,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        self.validate(limits, work)?;
        rhs.validate(limits, work)?;
        let ambient = match (&self.cases, &rhs.cases) {
            (PredicateCases::Fixed(a), PredicateCases::Fixed(b)) => return Ok(a == b),
            (PredicateCases::Parameter { ambient, .. }, _)
            | (_, PredicateCases::Parameter { ambient, .. }) => ambient,
        };
        let a = self.regions(ambient, limits, work)?;
        let b = rhs.regions(ambient, limits, work)?;
        // Do both checks: a constant result must not hide a region refusal.
        let false_equal = a[0].equivalent(&b[0], limits.parameters, work)?;
        let true_equal = a[1].equivalent(&b[1], limits.parameters, work)?;
        Ok(false_equal && true_equal)
    }

    fn regions(
        &self,
        domain: &ParameterDomain,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<[ParameterRegion; 2]> {
        match &self.cases {
            PredicateCases::Fixed(value) => Ok([
                if *value == Some(false) {
                    domain.region().clone()
                } else {
                    ParameterRegion::empty(domain.parameter())
                },
                if *value == Some(true) {
                    domain.region().clone()
                } else {
                    ParameterRegion::empty(domain.parameter())
                },
            ]),
            PredicateCases::Parameter {
                ambient,
                holds,
                fails,
                ..
            } => {
                if !ambient
                    .region()
                    .equivalent(domain.region(), limits.parameters, work)?
                {
                    return Err(Error::ParameterDomainMismatch);
                }
                Ok([fails.clone(), holds.clone()])
            }
        }
    }

    /// All sixteen Boolean operations lifted strictly to partial predicates.
    /// Both operands must be defined. Even a constant truth function preserves
    /// undefinedness; choosing a three-valued short-circuit policy is explicit.
    /// # Errors
    /// Foreign/different ambient domains, capacities or cancellation.
    pub fn apply(
        &self,
        op: BoolOp4,
        rhs: &Self,
        limits: NumberLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.validate(limits, work)?;
        rhs.validate(limits, work)?;
        let ambient = match (&self.cases, &rhs.cases) {
            (PredicateCases::Fixed(a), PredicateCases::Fixed(b)) => {
                return Ok(Self {
                    cases: PredicateCases::Fixed(a.zip(*b).map(|(a, b)| op.evaluate(a, b))),
                });
            }
            (PredicateCases::Parameter { ambient, .. }, _)
            | (_, PredicateCases::Parameter { ambient, .. }) => ambient,
        };
        let left = self.regions(ambient, limits, work)?;
        let right = rhs.regions(ambient, limits, work)?;
        let mut holds = ParameterRegion::empty(ambient.parameter());
        let mut fails = holds.clone();
        for (i, a) in left.iter().enumerate() {
            for (j, b) in right.iter().enumerate() {
                let cell = a.apply(BoolOp4::AND, b, limits.parameters, work)?;
                let selected = if op.evaluate(i != 0, j != 0) {
                    &mut holds
                } else {
                    &mut fails
                };
                *selected = selected.apply(BoolOp4::OR, &cell, limits.parameters, work)?;
            }
        }
        let defined = holds.apply(BoolOp4::OR, &fails, limits.parameters, work)?;
        let undefined =
            ambient
                .region()
                .apply(BoolOp4::DIFFERENCE, &defined, limits.parameters, work)?;
        Ok(Self {
            cases: PredicateCases::Parameter {
                ambient: ambient.clone(),
                holds,
                fails,
                undefined,
            },
        })
    }

    #[must_use]
    pub fn view(&self) -> NumberPredicateView<'_> {
        match &self.cases {
            PredicateCases::Fixed(value) => NumberPredicateView::Fixed(*value),
            PredicateCases::Parameter {
                ambient,
                holds,
                fails,
                undefined,
            } => NumberPredicateView::Parameter {
                ambient,
                holds,
                fails,
                undefined,
            },
        }
    }
    /// True somewhere in the ambient domain. Undefined assignments are not witnesses.
    #[must_use]
    pub fn possibly(&self) -> bool {
        match &self.cases {
            PredicateCases::Fixed(value) => *value == Some(true),
            PredicateCases::Parameter { holds, .. } => !holds.is_empty(),
        }
    }
    /// True at every ambient assignment, requiring definedness everywhere.
    #[must_use]
    pub fn always(&self) -> bool {
        match &self.cases {
            PredicateCases::Fixed(value) => *value == Some(true),
            PredicateCases::Parameter {
                fails, undefined, ..
            } => fails.is_empty() && undefined.is_empty(),
        }
    }
    #[must_use]
    pub fn is_total(&self) -> bool {
        match &self.cases {
            PredicateCases::Fixed(value) => value.is_some(),
            PredicateCases::Parameter { undefined, .. } => undefined.is_empty(),
        }
    }
    /// Negation within the defined domain, preserving every undefined assignment.
    #[must_use]
    pub fn negate(&self) -> Self {
        let cases = match &self.cases {
            PredicateCases::Fixed(value) => PredicateCases::Fixed(value.map(|v| !v)),
            PredicateCases::Parameter {
                ambient,
                holds,
                fails,
                undefined,
            } => PredicateCases::Parameter {
                ambient: ambient.clone(),
                holds: fails.clone(),
                fails: holds.clone(),
                undefined: undefined.clone(),
            },
        };
        Self { cases }
    }
}
