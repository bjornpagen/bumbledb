//! Shared typed scalar output language for queries and schema migrations.
//! Partial operations are stage outputs, never speculative filter terms.
use crate::exec::kernel::numeric::{NumericalGuard, environment};
use crate::{F64, F64CastError, Value, VarId};
use crate::schema::ValueType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericCast { ToF64, ToF64Exact, ToI64Exact, ToU64Exact }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarExpr {
    Var(VarId),
    Literal(Value),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),
    Cast { kind: NumericCast, expr: Box<Self> },
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
    UnsupportedPlatform,
    TooDeep,
}

impl std::fmt::Display for ScalarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "scalar computation: {self:?}") }
}
impl std::error::Error for ScalarError {}

impl ScalarExpr {
    pub fn variables(&self) -> impl Iterator<Item = VarId> + '_ {
        let mut pending = vec![self];
        std::iter::from_fn(move || {
            while let Some(expr) = pending.pop() {
                match expr {
                    Self::Var(var) => return Some(*var),
                    Self::Literal(_) => {},
                    Self::Negate(value) | Self::Cast { expr: value, .. } | Self::IsNaN(value) | Self::IsFinite(value) => pending.push(value),
                    Self::Add(a,b) | Self::Subtract(a,b) | Self::Multiply(a,b) | Self::Divide(a,b) => { pending.push(b); pending.push(a); }
                }
            }
            None
        })
    }

    pub(crate) fn map_variables(&mut self, map: &impl Fn(VarId) -> VarId) {
        match self {
            Self::Var(var) => *var = map(*var),
            Self::Literal(_) => {},
            Self::Negate(value) | Self::Cast { expr: value, .. } | Self::IsNaN(value) | Self::IsFinite(value) => value.map_variables(map),
            Self::Add(a,b) | Self::Subtract(a,b) | Self::Multiply(a,b) | Self::Divide(a,b) => { a.map_variables(map); b.map_variables(map); }
        }
    }

    /// Check the whole tree before execution; there is no mixed promotion.
    pub fn result_type(&self, mut variable: impl FnMut(VarId) -> Option<ValueType>) -> Result<ValueType, ScalarError> {
        self.type_at(&mut variable, 0)
    }

    fn type_at(&self, variable: &mut impl FnMut(VarId) -> Option<ValueType>, depth: usize) -> Result<ValueType, ScalarError> {
        if depth > 128 { return Err(ScalarError::TooDeep); }
        let numeric = |ty| matches!(ty, ValueType::I64 | ValueType::U64 | ValueType::F64);
        let unary = |value: &Self, variable: &mut _| value.type_at(variable, depth + 1);
        match self {
            Self::Var(var) => variable(*var).ok_or(ScalarError::UnboundVariable(*var)),
            Self::Literal(Value::I64(_)) => Ok(ValueType::I64),
            Self::Literal(Value::U64(_)) => Ok(ValueType::U64),
            Self::Literal(Value::F64(_)) => Ok(ValueType::F64),
            Self::Literal(Value::Bool(_)) => Ok(ValueType::Bool),
            Self::Literal(_) => Err(ScalarError::NotNumeric),
            Self::Negate(value) => {
                let ty = unary(value, variable)?;
                if matches!(ty, ValueType::I64 | ValueType::F64) { Ok(ty) } else { Err(ScalarError::TypeMismatch) }
            }
            Self::IsNaN(value) | Self::IsFinite(value) => {
                if unary(value, variable)? == ValueType::F64 { Ok(ValueType::Bool) } else { Err(ScalarError::TypeMismatch) }
            }
            Self::Cast { kind, expr } => {
                if !numeric(unary(expr, variable)?) { return Err(ScalarError::NotNumeric); }
                Ok(match kind { NumericCast::ToF64 | NumericCast::ToF64Exact => ValueType::F64,
                    NumericCast::ToI64Exact => ValueType::I64, NumericCast::ToU64Exact => ValueType::U64 })
            }
            Self::Add(a,b) | Self::Subtract(a,b) | Self::Multiply(a,b) | Self::Divide(a,b) => {
                let a = unary(a, variable)?;
                let b = unary(b, variable)?;
                if a != b { Err(ScalarError::TypeMismatch) } else if numeric(a) { Ok(a) } else { Err(ScalarError::NotNumeric) }
            }
        }
    }
}

/// One thread-bound numerical execution operation. Create once, evaluate all
/// admitted rows, then drop before callbacks/suspension; Drop restores the host.
pub struct ScalarEvaluator { _guard: NumericalGuard }

impl ScalarEvaluator {
    pub fn new() -> Result<Self, ScalarError> {
        Ok(Self { _guard: NumericalGuard::enter().map_err(|_| ScalarError::UnsupportedPlatform)? })
    }
    pub fn type_of(expression: &ScalarExpr, variable: impl FnMut(VarId) -> Option<ValueType>) -> Result<ValueType, ScalarError> {
        expression.result_type(variable)
    }
    pub fn evaluate(&self, expression: &ScalarExpr, variable: impl FnMut(VarId) -> Result<Value, ScalarError>) -> Result<Value, ScalarError> {
        evaluate_in_operation(expression, variable)
    }
}

/// Query entry owns the guard once for the complete numerical operation.
pub(crate) fn evaluate_in_operation(expression: &ScalarExpr, mut variable: impl FnMut(VarId) -> Result<Value, ScalarError>) -> Result<Value, ScalarError> {
    evaluate(expression, &mut variable, 0)
}

fn evaluate(expr: &ScalarExpr, variable: &mut impl FnMut(VarId) -> Result<Value, ScalarError>, depth: usize) -> Result<Value, ScalarError> {
    if depth > 128 { return Err(ScalarError::TooDeep); }
    let eval = |value: &ScalarExpr, variable: &mut _| evaluate(value, variable, depth + 1);
    match expr {
        ScalarExpr::Var(var) => variable(*var),
        ScalarExpr::Literal(value) => Ok(value.clone()),
        ScalarExpr::Negate(value) => match eval(value, variable)? {
            Value::F64(value) => Ok(Value::F64(value.negated())),
            Value::I64(value) => value.checked_neg().map(Value::I64).ok_or(ScalarError::Overflow),
            _ => Err(ScalarError::TypeMismatch),
        },
        ScalarExpr::IsNaN(value) | ScalarExpr::IsFinite(value) => {
            let Value::F64(value) = eval(value, variable)? else { return Err(ScalarError::TypeMismatch); };
            Ok(Value::Bool(if matches!(expr, ScalarExpr::IsNaN(_)) { value.is_nan() } else { value.is_finite() }))
        }
        ScalarExpr::Cast { kind, expr } => cast(*kind, eval(expr, variable)?),
        ScalarExpr::Add(a,b) | ScalarExpr::Subtract(a,b) | ScalarExpr::Multiply(a,b) | ScalarExpr::Divide(a,b) => {
            let (a,b) = (eval(a, variable)?, eval(b, variable)?);
            match (a,b) {
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(match expr {
                    ScalarExpr::Add(..) => environment::add(a,b),
                    ScalarExpr::Subtract(..) => environment::subtract(a,b),
                    ScalarExpr::Multiply(..) => environment::multiply(a,b),
                    ScalarExpr::Divide(..) => environment::divide(a,b),
                    _ => unreachable!(),
                })),
                (Value::I64(a), Value::I64(b)) => {
                    if matches!(expr, ScalarExpr::Divide(..)) && b == 0 { return Err(ScalarError::DivisionByZero); }
                    match expr { ScalarExpr::Add(..) => a.checked_add(b), ScalarExpr::Subtract(..) => a.checked_sub(b),
                        ScalarExpr::Multiply(..) => a.checked_mul(b), ScalarExpr::Divide(..) => a.checked_div(b), _ => unreachable!() }
                        .map(Value::I64).ok_or(ScalarError::Overflow)
                }
                (Value::U64(a), Value::U64(b)) => {
                    if matches!(expr, ScalarExpr::Divide(..)) && b == 0 { return Err(ScalarError::DivisionByZero); }
                    match expr { ScalarExpr::Add(..) => a.checked_add(b), ScalarExpr::Subtract(..) => a.checked_sub(b),
                        ScalarExpr::Multiply(..) => a.checked_mul(b), ScalarExpr::Divide(..) => a.checked_div(b), _ => unreachable!() }
                        .map(Value::U64).ok_or(ScalarError::Overflow)
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
        (NumericCast::ToF64Exact, Value::I64(value)) => F64::from_i64_exact(value).map(Value::F64).map_err(err),
        (NumericCast::ToF64Exact, Value::U64(value)) => F64::from_u64_exact(value).map(Value::F64).map_err(err),
        (NumericCast::ToI64Exact, Value::F64(value)) => value.to_i64_exact().map(Value::I64).map_err(err),
        (NumericCast::ToU64Exact, Value::F64(value)) => value.to_u64_exact().map(Value::U64).map_err(err),
        (NumericCast::ToI64Exact, Value::I64(value)) => Ok(Value::I64(value)),
        (NumericCast::ToU64Exact, Value::U64(value)) => Ok(Value::U64(value)),
        (NumericCast::ToI64Exact, Value::U64(value)) => i64::try_from(value).map(Value::I64).map_err(|_| err(F64CastError::OutOfRange)),
        (NumericCast::ToU64Exact, Value::I64(value)) => u64::try_from(value).map(Value::U64).map_err(|_| err(F64CastError::OutOfRange)),
        _ => Err(ScalarError::TypeMismatch),
    }
}
