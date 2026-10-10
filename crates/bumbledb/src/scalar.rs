//! Typed scalar expressions for computed query outputs: typing, and the
//! row-at-a-time evaluator. Partial operations are stage outputs, never
//! speculative filter terms.
use crate::exec::kernel::numeric::DefaultFloatEnvironment;
use crate::schema::ValueType;
use crate::{F64, F64CastError, Value, VarId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericCast {
    ToF64,
    ToF64Exact,
    ToI64Exact,
    ToU64Exact,
}

/// Exact integer quotient rounding, shared by every scalar binding scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    TowardZero,
    NearestTiesAwayFromZero,
    NearestTiesToEven,
}

impl Rounding {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TowardZero => "towardZero",
            Self::NearestTiesAwayFromZero => "nearestTiesAwayFromZero",
            Self::NearestTiesToEven => "nearestTiesToEven",
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "towardZero" => Some(Self::TowardZero),
            "nearestTiesAwayFromZero" => Some(Self::NearestTiesAwayFromZero),
            "nearestTiesToEven" => Some(Self::NearestTiesToEven),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarExpr {
    Var(VarId),
    Literal(Value),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),
    MulDiv {
        a: Box<Self>,
        b: Box<Self>,
        divisor: Box<Self>,
        rounding: Rounding,
    },
    Measure(Box<Self>),
    Cast {
        kind: NumericCast,
        expr: Box<Self>,
    },
    IsNaN(Box<Self>),
    IsFinite(Box<Self>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarError {
    UnboundVariable(VarId),
    TypeMismatch,
    NotNumeric,
    Cast(F64CastError),
    Overflow,
    DivisionByZero,
    NonPositiveDivisor,
    UnboundedMeasure,
    /// F64 arithmetic under a float environment that is not the IEEE default.
    NonDefaultFloatEnvironment,
    TooDeep,
}

impl std::fmt::Display for ScalarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scalar computation: {self:?}")
    }
}
impl std::error::Error for ScalarError {}

impl ScalarExpr {
    pub fn variables(&self) -> impl Iterator<Item = VarId> + '_ {
        let mut pending = vec![self];
        std::iter::from_fn(move || {
            while let Some(expr) = pending.pop() {
                match expr {
                    Self::Var(var) => return Some(*var),
                    Self::Literal(_) => {}
                    Self::MulDiv { a, b, divisor, .. } => {
                        pending.extend([divisor.as_ref(), b.as_ref(), a.as_ref()]);
                    }
                    Self::Measure(value)
                    | Self::Negate(value)
                    | Self::Cast { expr: value, .. }
                    | Self::IsNaN(value)
                    | Self::IsFinite(value) => pending.push(value),
                    Self::Add(a, b)
                    | Self::Subtract(a, b)
                    | Self::Multiply(a, b)
                    | Self::Divide(a, b) => {
                        pending.push(b);
                        pending.push(a);
                    }
                }
            }
            None
        })
    }

    /// Check the whole tree before execution; there is no mixed promotion.
    /// # Errors
    /// A typed scalar-shape refusal: unknown variable, or an operand type
    /// outside the numeric promotion table.
    pub fn result_type(
        &self,
        mut variable: impl FnMut(VarId) -> Option<ValueType>,
    ) -> Result<ValueType, ScalarError> {
        self.type_at(&mut variable, 0)
    }

    fn type_at(
        &self,
        variable: &mut impl FnMut(VarId) -> Option<ValueType>,
        depth: usize,
    ) -> Result<ValueType, ScalarError> {
        if depth > 128 {
            return Err(ScalarError::TooDeep);
        }
        let numeric = |ty| matches!(ty, ValueType::I64 | ValueType::U64 | ValueType::F64);
        let unary = |value: &Self, variable: &mut _| value.type_at(variable, depth + 1);
        match self {
            Self::Var(var) => variable(*var).ok_or(ScalarError::UnboundVariable(*var)),
            Self::Literal(Value::I64(_)) => Ok(ValueType::I64),
            Self::Literal(Value::U64(_)) => Ok(ValueType::U64),
            Self::Literal(Value::F64(_)) => Ok(ValueType::F64),
            Self::Literal(Value::Bool(_)) => Ok(ValueType::Bool),
            Self::Literal(Value::IntervalU64(_)) => Ok(ValueType::Interval {
                element: crate::schema::IntervalElement::U64,
            }),
            Self::Literal(Value::IntervalI64(_)) => Ok(ValueType::Interval {
                element: crate::schema::IntervalElement::I64,
            }),
            Self::Literal(Value::IntervalF64(_)) => Ok(ValueType::Interval {
                element: crate::schema::IntervalElement::F64,
            }),
            Self::Literal(_) => Err(ScalarError::NotNumeric),
            Self::Measure(value) => match unary(value, variable)? {
                ValueType::Interval {
                    element: crate::schema::IntervalElement::F64,
                } => Ok(ValueType::F64),
                ValueType::Interval { .. } | ValueType::FixedInterval { .. } => Ok(ValueType::U64),
                _ => Err(ScalarError::TypeMismatch),
            },
            Self::MulDiv { a, b, divisor, .. } => {
                let a = unary(a, variable)?;
                let b = unary(b, variable)?;
                let d = unary(divisor, variable)?;
                if a == b && a == d && matches!(a, ValueType::I64 | ValueType::U64) {
                    Ok(a)
                } else {
                    Err(ScalarError::TypeMismatch)
                }
            }
            Self::Negate(value) => {
                let ty = unary(value, variable)?;
                if matches!(ty, ValueType::I64 | ValueType::F64) {
                    Ok(ty)
                } else {
                    Err(ScalarError::TypeMismatch)
                }
            }
            Self::IsNaN(value) | Self::IsFinite(value) => {
                if unary(value, variable)? == ValueType::F64 {
                    Ok(ValueType::Bool)
                } else {
                    Err(ScalarError::TypeMismatch)
                }
            }
            Self::Cast { kind, expr } => {
                if !numeric(unary(expr, variable)?) {
                    return Err(ScalarError::NotNumeric);
                }
                Ok(match kind {
                    NumericCast::ToF64 | NumericCast::ToF64Exact => ValueType::F64,
                    NumericCast::ToI64Exact => ValueType::I64,
                    NumericCast::ToU64Exact => ValueType::U64,
                })
            }
            Self::Add(a, b) | Self::Subtract(a, b) | Self::Multiply(a, b) | Self::Divide(a, b) => {
                let a = unary(a, variable)?;
                let b = unary(b, variable)?;
                if a != b {
                    Err(ScalarError::TypeMismatch)
                } else if numeric(a) {
                    Ok(a)
                } else {
                    Err(ScalarError::NotNumeric)
                }
            }
        }
    }
}

/// Evaluates scalar expressions one binding at a time under a float
/// environment checked once at construction.
pub struct ScalarEvaluator {
    float: DefaultFloatEnvironment,
}

impl ScalarEvaluator {
    /// # Errors
    /// [`ScalarError::NonDefaultFloatEnvironment`] when this thread's float
    /// environment is not the IEEE default.
    pub fn new() -> Result<Self, ScalarError> {
        DefaultFloatEnvironment::check()
            .map(|float| Self { float })
            .map_err(|_| ScalarError::NonDefaultFloatEnvironment)
    }

    /// # Errors
    /// As [`ScalarExpr::result_type`].
    pub fn type_of(
        expression: &ScalarExpr,
        variable: impl FnMut(VarId) -> Option<ValueType>,
    ) -> Result<ValueType, ScalarError> {
        expression.result_type(variable)
    }

    /// # Errors
    /// The variable source's refusal, or a typed evaluation fault
    /// (division shape, unrepresentable cast).
    pub fn evaluate(
        &self,
        expression: &ScalarExpr,
        mut variable: impl FnMut(VarId) -> Result<Value, ScalarError>,
    ) -> Result<Value, ScalarError> {
        evaluate(expression, Some(self.float), &mut variable, 0)
    }
}

/// Evaluates one binding. `float` is the checked environment, if any; F64
/// arithmetic without one fails with
/// [`ScalarError::NonDefaultFloatEnvironment`].
#[cfg(test)]
pub(crate) fn evaluate_binding(
    expression: &ScalarExpr,
    float: Option<DefaultFloatEnvironment>,
    mut variable: impl FnMut(VarId) -> Result<Value, ScalarError>,
) -> Result<Value, ScalarError> {
    evaluate(expression, float, &mut variable, 0)
}

fn evaluate(
    expr: &ScalarExpr,
    float: Option<DefaultFloatEnvironment>,
    variable: &mut impl FnMut(VarId) -> Result<Value, ScalarError>,
    depth: usize,
) -> Result<Value, ScalarError> {
    if depth > 128 {
        return Err(ScalarError::TooDeep);
    }
    let eval = |value: &ScalarExpr, variable: &mut _| evaluate(value, float, variable, depth + 1);
    match expr {
        ScalarExpr::Var(var) => variable(*var),
        ScalarExpr::Literal(value) => Ok(value.clone()),
        ScalarExpr::Measure(value) => match eval(value, variable)? {
            Value::IntervalU64(span) => span
                .duration()
                .map(Value::U64)
                .ok_or(ScalarError::UnboundedMeasure),
            Value::IntervalI64(span) => span
                .duration()
                .map(Value::U64)
                .ok_or(ScalarError::UnboundedMeasure),
            Value::IntervalF64(span) => {
                span.length().map(Value::F64).map_err(|error| match error {
                    bumbledb_theory::FloatMeasureError::Unbounded => ScalarError::UnboundedMeasure,
                    bumbledb_theory::FloatMeasureError::Overflow => ScalarError::Overflow,
                })
            }
            _ => Err(ScalarError::TypeMismatch),
        },
        ScalarExpr::MulDiv {
            a,
            b,
            divisor,
            rounding,
        } => mul_div(
            eval(a, variable)?,
            eval(b, variable)?,
            eval(divisor, variable)?,
            *rounding,
        ),
        ScalarExpr::Negate(value) => match eval(value, variable)? {
            Value::F64(value) => Ok(Value::F64(value.negated())),
            Value::I64(value) => value
                .checked_neg()
                .map(Value::I64)
                .ok_or(ScalarError::Overflow),
            _ => Err(ScalarError::TypeMismatch),
        },
        ScalarExpr::IsNaN(value) | ScalarExpr::IsFinite(value) => {
            let Value::F64(value) = eval(value, variable)? else {
                return Err(ScalarError::TypeMismatch);
            };
            Ok(Value::Bool(if matches!(expr, ScalarExpr::IsNaN(_)) {
                value.is_nan()
            } else {
                value.is_finite()
            }))
        }
        ScalarExpr::Cast { kind, expr } => cast(*kind, eval(expr, variable)?),
        ScalarExpr::Add(a, b)
        | ScalarExpr::Subtract(a, b)
        | ScalarExpr::Multiply(a, b)
        | ScalarExpr::Divide(a, b) => {
            let (a, b) = (eval(a, variable)?, eval(b, variable)?);
            match (a, b) {
                (Value::F64(a), Value::F64(b)) => {
                    let float = float.ok_or(ScalarError::NonDefaultFloatEnvironment)?;
                    Ok(Value::F64(match expr {
                        ScalarExpr::Add(..) => float.add(a, b),
                        ScalarExpr::Subtract(..) => float.subtract(a, b),
                        ScalarExpr::Multiply(..) => float.multiply(a, b),
                        ScalarExpr::Divide(..) => float.divide(a, b),
                        _ => unreachable!(),
                    }))
                }
                (Value::I64(a), Value::I64(b)) => {
                    if matches!(expr, ScalarExpr::Divide(..)) && b == 0 {
                        return Err(ScalarError::DivisionByZero);
                    }
                    match expr {
                        ScalarExpr::Add(..) => a.checked_add(b),
                        ScalarExpr::Subtract(..) => a.checked_sub(b),
                        ScalarExpr::Multiply(..) => a.checked_mul(b),
                        ScalarExpr::Divide(..) => a.checked_div(b),
                        _ => unreachable!(),
                    }
                    .map(Value::I64)
                    .ok_or(ScalarError::Overflow)
                }
                (Value::U64(a), Value::U64(b)) => {
                    if matches!(expr, ScalarExpr::Divide(..)) && b == 0 {
                        return Err(ScalarError::DivisionByZero);
                    }
                    match expr {
                        ScalarExpr::Add(..) => a.checked_add(b),
                        ScalarExpr::Subtract(..) => a.checked_sub(b),
                        ScalarExpr::Multiply(..) => a.checked_mul(b),
                        ScalarExpr::Divide(..) => a.checked_div(b),
                        _ => unreachable!(),
                    }
                    .map(Value::U64)
                    .ok_or(ScalarError::Overflow)
                }
                _ => Err(ScalarError::TypeMismatch),
            }
        }
    }
}

fn cast(kind: NumericCast, value: Value) -> Result<Value, ScalarError> {
    let err = ScalarError::Cast;
    match (kind, value) {
        (NumericCast::ToF64 | NumericCast::ToF64Exact, Value::F64(value)) => Ok(Value::F64(value)),
        (NumericCast::ToF64, Value::I64(value)) => Ok(Value::F64(F64::from_i64(value))),
        (NumericCast::ToF64, Value::U64(value)) => Ok(Value::F64(F64::from_u64(value))),
        (NumericCast::ToF64Exact, Value::I64(value)) => {
            F64::from_i64_exact(value).map(Value::F64).map_err(err)
        }
        (NumericCast::ToF64Exact, Value::U64(value)) => {
            F64::from_u64_exact(value).map(Value::F64).map_err(err)
        }
        (NumericCast::ToI64Exact, Value::F64(value)) => {
            value.to_i64_exact().map(Value::I64).map_err(err)
        }
        (NumericCast::ToU64Exact, Value::F64(value)) => {
            value.to_u64_exact().map(Value::U64).map_err(err)
        }
        (NumericCast::ToI64Exact, Value::I64(value)) => Ok(Value::I64(value)),
        (NumericCast::ToU64Exact, Value::U64(value)) => Ok(Value::U64(value)),
        (NumericCast::ToI64Exact, Value::U64(value)) => i64::try_from(value)
            .map(Value::I64)
            .map_err(|_| err(F64CastError::OutOfRange)),
        (NumericCast::ToU64Exact, Value::I64(value)) => u64::try_from(value)
            .map(Value::U64)
            .map_err(|_| err(F64CastError::OutOfRange)),
        _ => Err(ScalarError::TypeMismatch),
    }
}

/// Fixed-width product, exact quotient/remainder, one final public range check.
fn mul_div(a: Value, b: Value, divisor: Value, rounding: Rounding) -> Result<Value, ScalarError> {
    match (a, b, divisor) {
        (Value::U64(a), Value::U64(b), Value::U64(d)) => {
            mul_div_u64(a, b, d, rounding).map(Value::U64)
        }
        (Value::I64(a), Value::I64(b), Value::I64(d)) => {
            mul_div_i64(a, b, d, rounding).map(Value::I64)
        }
        _ => Err(ScalarError::TypeMismatch),
    }
}

/// `round(a * b / d)` over the exact 128-bit product.
pub(crate) fn mul_div_u64(a: u64, b: u64, d: u64, rounding: Rounding) -> Result<u64, ScalarError> {
    if d == 0 {
        return Err(ScalarError::DivisionByZero);
    }
    let rounded = rounded_quotient(u128::from(a) * u128::from(b), u128::from(d), rounding);
    u64::try_from(rounded).map_err(|_| ScalarError::Overflow)
}

/// `round(a * b / d)` over the exact 128-bit product; `d` must be positive.
pub(crate) fn mul_div_i64(a: i64, b: i64, d: i64, rounding: Rounding) -> Result<i64, ScalarError> {
    if d == 0 {
        return Err(ScalarError::DivisionByZero);
    }
    if d < 0 {
        return Err(ScalarError::NonPositiveDivisor);
    }
    let product = i128::from(a) * i128::from(b);
    let magnitude = rounded_quotient(
        product.unsigned_abs(),
        u128::from(d.unsigned_abs()),
        rounding,
    );
    // A signed 64-bit product magnitude is at most 2^126.
    let rounded = i128::try_from(magnitude).expect("64-bit product fits signed wide magnitude");
    let signed = if product < 0 { -rounded } else { rounded };
    i64::try_from(signed).map_err(|_| ScalarError::Overflow)
}

fn rounded_quotient(product: u128, divisor: u128, rounding: Rounding) -> u128 {
    let q = product / divisor;
    let r = product % divisor;
    let increment = match rounding {
        Rounding::TowardZero => false,
        Rounding::NearestTiesAwayFromZero => r != 0 && r >= divisor - r,
        Rounding::NearestTiesToEven => {
            r > divisor - r || (r == divisor - r && !q.is_multiple_of(2))
        }
    };
    // Products of two 64-bit values leave room for this one increment.
    q + u128::from(increment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(expr: ScalarExpr) -> Result<Value, ScalarError> {
        ScalarEvaluator::new()
            .unwrap()
            .evaluate(&expr, |v| Err(ScalarError::UnboundVariable(v)))
    }

    fn quotient(a: Value, b: Value, d: Value, rounding: Rounding) -> ScalarExpr {
        ScalarExpr::MulDiv {
            a: Box::new(ScalarExpr::Literal(a)),
            b: Box::new(ScalarExpr::Literal(b)),
            divisor: Box::new(ScalarExpr::Literal(d)),
            rounding,
        }
    }

    #[test]
    fn float_arithmetic_without_a_checked_environment_is_refused() {
        let literal = |value| Box::new(ScalarExpr::Literal(value));
        let unbound = |v| Err(ScalarError::UnboundVariable(v));
        let float_sum = ScalarExpr::Add(
            literal(Value::F64(F64::from(1.0))),
            literal(Value::F64(F64::from(2.0))),
        );
        assert_eq!(
            evaluate_binding(&float_sum, None, unbound),
            Err(ScalarError::NonDefaultFloatEnvironment)
        );
        let integer_sum = ScalarExpr::Add(literal(Value::I64(1)), literal(Value::I64(2)));
        assert_eq!(
            evaluate_binding(&integer_sum, None, unbound),
            Ok(Value::I64(3))
        );
        let negated = ScalarExpr::Negate(literal(Value::F64(F64::from(2.0))));
        assert_eq!(
            evaluate_binding(&negated, None, unbound),
            Ok(Value::F64(F64::from(-2.0)))
        );
        assert_eq!(run(float_sum), Ok(Value::F64(F64::from(3.0))));
    }

    #[test]
    fn rounded_quotient_uses_exact_wide_product_and_final_range() {
        use Rounding::{
            NearestTiesAwayFromZero as Away, NearestTiesToEven as Even, TowardZero as Zero,
        };
        for (a, mode, expected) in [
            (5, Zero, 2),
            (5, Away, 3),
            (5, Even, 2),
            (-5, Zero, -2),
            (-5, Away, -3),
            (-5, Even, -2),
            (7, Even, 4),
            (-7, Even, -4),
        ] {
            assert_eq!(
                run(quotient(Value::I64(a), Value::I64(1), Value::I64(2), mode)),
                Ok(Value::I64(expected))
            );
        }
        for mode in [Zero, Away, Even] {
            assert_eq!(
                run(quotient(
                    Value::U64(u64::MAX),
                    Value::U64(2),
                    Value::U64(2),
                    mode
                )),
                Ok(Value::U64(u64::MAX))
            );
            assert_eq!(
                run(quotient(
                    Value::I64(i64::MIN),
                    Value::I64(-1),
                    Value::I64(2),
                    mode
                )),
                Ok(Value::I64(1 << 62))
            );
            assert_eq!(
                run(quotient(
                    Value::I64(i64::MIN),
                    Value::I64(-1),
                    Value::I64(1),
                    mode
                )),
                Err(ScalarError::Overflow)
            );
            assert_eq!(
                run(quotient(
                    Value::I64(i64::MIN),
                    Value::I64(1),
                    Value::I64(1),
                    mode
                )),
                Ok(Value::I64(i64::MIN))
            );
            assert_eq!(
                run(quotient(
                    Value::U64(u64::MAX),
                    Value::U64(u64::MAX),
                    Value::U64(u64::MAX),
                    mode
                )),
                Ok(Value::U64(u64::MAX))
            );
        }
        assert_eq!(
            run(quotient(Value::I64(1), Value::I64(1), Value::I64(-2), Zero)),
            Err(ScalarError::NonPositiveDivisor)
        );
        assert_eq!(
            run(quotient(Value::U64(1), Value::U64(1), Value::U64(0), Zero)),
            Err(ScalarError::DivisionByZero)
        );
        assert_eq!(
            run(quotient(Value::I64(1), Value::U64(1), Value::I64(2), Zero)),
            Err(ScalarError::TypeMismatch)
        );
        assert_eq!(
            run(ScalarExpr::Multiply(
                Box::new(ScalarExpr::Literal(Value::U64(u64::MAX))),
                Box::new(ScalarExpr::Literal(Value::U64(2)))
            )),
            Err(ScalarError::Overflow)
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn small_signed_quotients_match_nearest_integer_distance_oracle() {
        // Select among integers by distance to the rational; no production
        // quotient/remainder rounding logic participates in this oracle.
        for a in -15i64..=15 {
            for b in -5i64..=5 {
                for d in 1i64..=11 {
                    for mode in [
                        Rounding::TowardZero,
                        Rounding::NearestTiesAwayFromZero,
                        Rounding::NearestTiesToEven,
                    ] {
                        let product = a * b;
                        let expected = if mode == Rounding::TowardZero {
                            product / d
                        } else {
                            (-76i64..=76)
                                .min_by_key(|q| {
                                    (
                                        (q * d - product).abs(),
                                        match mode {
                                            Rounding::NearestTiesAwayFromZero => -q.abs(),
                                            Rounding::NearestTiesToEven => q.abs() % 2,
                                            Rounding::TowardZero => unreachable!(),
                                        },
                                    )
                                })
                                .unwrap()
                        };
                        assert_eq!(
                            run(quotient(Value::I64(a), Value::I64(b), Value::I64(d), mode)),
                            Ok(Value::I64(expected))
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn measure_preserves_interval_domains_and_typechecks_without_rows() {
        use crate::Interval;
        let measure = |value| run(ScalarExpr::Measure(Box::new(ScalarExpr::Literal(value))));
        assert_eq!(
            measure(Value::IntervalI64(
                Interval::new(i64::MIN, i64::MAX - 1).unwrap()
            )),
            Ok(Value::U64(u64::MAX - 1))
        );
        assert_eq!(
            measure(Value::IntervalU64(Interval::ray(0).unwrap())),
            Err(ScalarError::UnboundedMeasure)
        );
        assert_eq!(
            measure(Value::IntervalF64(
                Interval::new(F64::from(-1.5), F64::from(2.5)).unwrap()
            )),
            Ok(Value::F64(F64::from(4.0)))
        );
        assert_eq!(
            measure(Value::IntervalF64(
                Interval::new(F64::from(-f64::MAX), F64::from(f64::MAX)).unwrap()
            )),
            Err(ScalarError::Overflow)
        );
        assert_eq!(
            measure(Value::IntervalF64(
                Interval::new(F64::from(f64::NEG_INFINITY), F64::from(0.0)).unwrap()
            )),
            Err(ScalarError::UnboundedMeasure)
        );
        assert_eq!(
            ScalarExpr::Measure(Box::new(ScalarExpr::Var(VarId(0))))
                .result_type(|_| Some(ValueType::U64)),
            Err(ScalarError::TypeMismatch)
        );
        assert_eq!(
            quotient(
                Value::F64(F64::from(1.0)),
                Value::F64(F64::from(2.0)),
                Value::F64(F64::from(3.0)),
                Rounding::TowardZero
            )
            .result_type(|_| None),
            Err(ScalarError::TypeMismatch)
        );
    }
}
