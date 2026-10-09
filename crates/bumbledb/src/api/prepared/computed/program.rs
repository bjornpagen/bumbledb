//! A computed output's expression compiled to a postfix program over 64-lane
//! registers. Typing happens at compile time, so every op is monomorphic. F64
//! results are canonicalized after every op (one NaN, one zero), integer ops
//! are checked, and each lane keeps its first error in program order: postfix
//! order is the recursive evaluator's order, so a lane reports exactly the
//! error that evaluator reports for the same binding.
#![expect(
    clippy::inline_always,
    reason = "SIMD bodies inline into the dispatched target-feature context"
)]

use bumbledb_theory::{F64, FloatMeasureError, Interval};
use fearless_simd::{Bytes, Select, Simd, SimdBase, SimdMask, f64x4, u64x4};

use crate::exec::kernel::numeric::DefaultFloatEnvironment;
use crate::scalar::{NumericCast, Rounding, ScalarError, ScalarExpr, mul_div_i64, mul_div_u64};
use crate::schema::{IntervalElement, ValueType};
use crate::{F64CastError, Value, VarId};

/// Lanes per register.
pub(super) const LANES: usize = 64;

/// One value per lane in its register encoding: `u64`, `i64` bits,
/// canonical F64 bits, or 0/1.
pub(super) type Register = [u64; LANES];

const SIGN: u64 = 1 << 63;
const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// The register encoding of one expression value; intervals take two
/// registers, start then end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    U64,
    I64,
    F64,
    Bool,
    Interval(IntervalElement),
}

/// How a binding word becomes a register word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decode {
    Raw,
    /// Biased I64 word to `i64` bits.
    Biased,
    /// F64 order key to canonical F64 bits.
    OrderKey,
    Bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cast {
    I64ToF64,
    U64ToF64,
    I64ToF64Exact,
    U64ToF64Exact,
    F64ToI64Exact,
    F64ToU64Exact,
    U64ToI64Exact,
    I64ToU64Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Load {
        column: usize,
        decode: Decode,
    },
    Const(u64),
    AddU64,
    SubU64,
    MulU64,
    DivU64,
    AddI64,
    SubI64,
    MulI64,
    DivI64,
    NegI64,
    AddF64,
    SubF64,
    MulF64,
    DivF64,
    NegF64,
    MulDivU64(Rounding),
    MulDivI64(Rounding),
    MeasureU64,
    MeasureI64,
    MeasureF64,
    Cast(Cast),
    IsNaN,
    IsFinite,
    /// Every lane fails here; the program ends.
    Fail(ScalarError),
}

/// How the result register becomes a binding word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encode {
    Raw,
    Biased,
    OrderKey,
}

/// A compiled computed output.
#[derive(Clone, Debug)]
pub(super) struct Program {
    ops: Vec<Op>,
    depth: usize,
    encode: Encode,
}

/// Per-lane first error, in program order. `failed` is the truth; `first`
/// holds a meaningful error only for lanes whose bit is set, so clearing is
/// one store.
pub(super) struct Errors {
    first: [ScalarError; LANES],
    failed: u64,
}

impl Errors {
    pub(super) fn new() -> Self {
        Self {
            first: [ScalarError::Overflow; LANES],
            failed: 0,
        }
    }

    pub(super) fn clear(&mut self) {
        self.failed = 0;
    }

    fn fail(&mut self, lanes: u64, error: ScalarError) {
        let mut fresh = lanes & !self.failed;
        self.failed |= fresh;
        while fresh != 0 {
            self.first[fresh.trailing_zeros() as usize] = error;
            fresh &= fresh - 1;
        }
    }

    pub(super) fn of(&self, lane: usize) -> Option<ScalarError> {
        ((self.failed >> lane) & 1 != 0).then(|| self.first[lane])
    }
}

impl Program {
    /// Compiles `expression`; `input` maps a variable to its first binding
    /// column and type.
    pub(super) fn compile(
        expression: &ScalarExpr,
        input: impl Fn(VarId) -> Option<(usize, ValueType)>,
    ) -> Self {
        let mut compiler = Compiler {
            ops: Vec::new(),
            depth: 0,
            max_depth: 0,
            input,
        };
        let encode = match compiler.emit(expression, 0) {
            Ok(Kind::U64 | Kind::Bool) | Err(Ended) => Encode::Raw,
            Ok(Kind::I64) => Encode::Biased,
            Ok(Kind::F64) => Encode::OrderKey,
            Ok(Kind::Interval(_)) => {
                compiler.ops.push(Op::Fail(ScalarError::NotNumeric));
                Encode::Raw
            }
        };
        Self {
            ops: compiler.ops,
            depth: compiler.max_depth.max(1),
            encode,
        }
    }

    /// Registers the program needs.
    pub(super) fn depth(&self) -> usize {
        self.depth
    }

    /// Runs the first `lanes` lanes of `columns` (binding words per column)
    /// and writes binding words to `out`; failing lanes are recorded in
    /// `errors`. `float` is the checked float environment, if any.
    #[inline(always)]
    pub(super) fn run<S: Simd>(
        &self,
        simd: S,
        lanes: usize,
        columns: &[Register],
        float: Option<DefaultFloatEnvironment>,
        stack: &mut [Register],
        out: &mut Register,
        errors: &mut Errors,
    ) {
        let mut sp = 0usize;
        for op in &self.ops {
            match *op {
                Op::Load { column, decode } => {
                    let reg = &mut stack[sp];
                    for (word, &binding) in reg[..lanes].iter_mut().zip(&columns[column]) {
                        *word = match decode {
                            Decode::Raw => binding,
                            Decode::Biased => binding ^ SIGN,
                            Decode::OrderKey => order_key_to_bits(binding),
                            Decode::Bool => u64::from(binding != 0),
                        };
                    }
                    sp += 1;
                }
                Op::Const(word) => {
                    stack[sp][..lanes].fill(word);
                    sp += 1;
                }
                Op::AddU64 => sp = binary_vector(simd, lanes, stack, sp, errors, add_u64),
                Op::SubU64 => sp = binary_vector(simd, lanes, stack, sp, errors, sub_u64),
                Op::AddI64 => sp = binary_vector(simd, lanes, stack, sp, errors, add_i64),
                Op::SubI64 => sp = binary_vector(simd, lanes, stack, sp, errors, sub_i64),
                Op::MulU64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |a, b| {
                        a.checked_mul(b).ok_or(ScalarError::Overflow)
                    });
                }
                Op::DivU64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |a, b| {
                        a.checked_div(b).ok_or(ScalarError::DivisionByZero)
                    });
                }
                Op::MulI64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |a, b| {
                        a.cast_signed()
                            .checked_mul(b.cast_signed())
                            .map(i64::cast_unsigned)
                            .ok_or(ScalarError::Overflow)
                    });
                }
                Op::DivI64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |a, b| {
                        if b == 0 {
                            return Err(ScalarError::DivisionByZero);
                        }
                        a.cast_signed()
                            .checked_div(b.cast_signed())
                            .map(i64::cast_unsigned)
                            .ok_or(ScalarError::Overflow)
                    });
                }
                Op::NegI64 => unary_lanes(&mut stack[sp - 1][..lanes], errors, |a| {
                    a.cast_signed()
                        .checked_neg()
                        .map(i64::cast_unsigned)
                        .ok_or(ScalarError::Overflow)
                }),
                Op::NegF64 => {
                    for word in &mut stack[sp - 1][..lanes] {
                        *word = canonical(*word ^ SIGN);
                    }
                }
                Op::AddF64 | Op::SubF64 | Op::MulF64 | Op::DivF64 => {
                    if float.is_none() {
                        errors.fail(u64::MAX, ScalarError::NonDefaultFloatEnvironment);
                        return;
                    }
                    sp = float_binary(simd, lanes, stack, sp, *op);
                }
                Op::MulDivU64(rounding) => {
                    sp = ternary_lanes(lanes, stack, sp, errors, |a, b, d| {
                        mul_div_u64(a, b, d, rounding)
                    });
                }
                Op::MulDivI64(rounding) => {
                    sp = ternary_lanes(lanes, stack, sp, errors, |a, b, d| {
                        mul_div_i64(a.cast_signed(), b.cast_signed(), d.cast_signed(), rounding)
                            .map(i64::cast_unsigned)
                    });
                }
                Op::MeasureU64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |start, end| {
                        if end == u64::MAX {
                            Err(ScalarError::UnboundedMeasure)
                        } else {
                            Ok(end.wrapping_sub(start))
                        }
                    });
                }
                Op::MeasureI64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |start, end| {
                        if end.cast_signed() == i64::MAX {
                            Err(ScalarError::UnboundedMeasure)
                        } else {
                            Ok(end.wrapping_sub(start))
                        }
                    });
                }
                Op::MeasureF64 => {
                    sp = binary_lanes(lanes, stack, sp, errors, |start, end| {
                        let Some(span) = Interval::new(F64::from_bits(start), F64::from_bits(end))
                        else {
                            // Only lanes past the batch hold empty spans.
                            return Ok(0);
                        };
                        span.length()
                            .map(F64::to_bits)
                            .map_err(|error| match error {
                                FloatMeasureError::Unbounded => ScalarError::UnboundedMeasure,
                                FloatMeasureError::Overflow => ScalarError::Overflow,
                            })
                    });
                }
                Op::Cast(cast) => {
                    unary_lanes(&mut stack[sp - 1][..lanes], errors, |a| cast_word(cast, a));
                }
                Op::IsNaN => {
                    for word in &mut stack[sp - 1][..lanes] {
                        *word = u64::from(*word == CANONICAL_NAN);
                    }
                }
                Op::IsFinite => {
                    for word in &mut stack[sp - 1][..lanes] {
                        *word = u64::from(*word & EXPONENT != EXPONENT);
                    }
                }
                Op::Fail(error) => {
                    errors.fail(u64::MAX, error);
                    return;
                }
            }
        }
        debug_assert_eq!(sp, 1, "a program leaves exactly its result");
        for (word, &value) in out[..lanes].iter_mut().zip(&stack[0]) {
            *word = match self.encode {
                Encode::Raw => value,
                Encode::Biased => value ^ SIGN,
                Encode::OrderKey => bits_to_order_key(value),
            };
        }
    }
}

/// Compilation stopped at a `Fail`.
struct Ended;

struct Compiler<F> {
    ops: Vec<Op>,
    depth: usize,
    max_depth: usize,
    input: F,
}

impl<F: Fn(VarId) -> Option<(usize, ValueType)>> Compiler<F> {
    fn push(&mut self, op: Op, popped: usize, pushed: usize) {
        self.ops.push(op);
        self.depth = self.depth - popped + pushed;
        self.max_depth = self.max_depth.max(self.depth);
    }

    fn fail(&mut self, error: ScalarError) -> Result<Kind, Ended> {
        self.ops.push(Op::Fail(error));
        Err(Ended)
    }

    fn emit(&mut self, expr: &ScalarExpr, depth: usize) -> Result<Kind, Ended> {
        if depth > 128 {
            return self.fail(ScalarError::TooDeep);
        }
        match expr {
            ScalarExpr::Var(var) => match (self.input)(*var) {
                Some((column, ty)) => self.load(column, ty),
                None => self.fail(ScalarError::UnboundVariable(*var)),
            },
            ScalarExpr::Literal(value) => self.literal(value),
            ScalarExpr::Negate(value) => match self.emit(value, depth + 1)? {
                Kind::I64 => Ok(self.op(Op::NegI64, 1, Kind::I64)),
                Kind::F64 => Ok(self.op(Op::NegF64, 1, Kind::F64)),
                _ => self.fail(ScalarError::TypeMismatch),
            },
            ScalarExpr::Add(a, b)
            | ScalarExpr::Subtract(a, b)
            | ScalarExpr::Multiply(a, b)
            | ScalarExpr::Divide(a, b) => {
                let left = self.emit(a, depth + 1)?;
                let right = self.emit(b, depth + 1)?;
                let op = match (expr, left, right) {
                    (ScalarExpr::Add(..), Kind::U64, Kind::U64) => Op::AddU64,
                    (ScalarExpr::Subtract(..), Kind::U64, Kind::U64) => Op::SubU64,
                    (ScalarExpr::Multiply(..), Kind::U64, Kind::U64) => Op::MulU64,
                    (ScalarExpr::Divide(..), Kind::U64, Kind::U64) => Op::DivU64,
                    (ScalarExpr::Add(..), Kind::I64, Kind::I64) => Op::AddI64,
                    (ScalarExpr::Subtract(..), Kind::I64, Kind::I64) => Op::SubI64,
                    (ScalarExpr::Multiply(..), Kind::I64, Kind::I64) => Op::MulI64,
                    (ScalarExpr::Divide(..), Kind::I64, Kind::I64) => Op::DivI64,
                    (ScalarExpr::Add(..), Kind::F64, Kind::F64) => Op::AddF64,
                    (ScalarExpr::Subtract(..), Kind::F64, Kind::F64) => Op::SubF64,
                    (ScalarExpr::Multiply(..), Kind::F64, Kind::F64) => Op::MulF64,
                    (ScalarExpr::Divide(..), Kind::F64, Kind::F64) => Op::DivF64,
                    _ => return self.fail(ScalarError::TypeMismatch),
                };
                Ok(self.op(op, 2, left))
            }
            ScalarExpr::MulDiv {
                a,
                b,
                divisor,
                rounding,
            } => {
                let kinds = (
                    self.emit(a, depth + 1)?,
                    self.emit(b, depth + 1)?,
                    self.emit(divisor, depth + 1)?,
                );
                match kinds {
                    (Kind::U64, Kind::U64, Kind::U64) => {
                        Ok(self.op(Op::MulDivU64(*rounding), 3, Kind::U64))
                    }
                    (Kind::I64, Kind::I64, Kind::I64) => {
                        Ok(self.op(Op::MulDivI64(*rounding), 3, Kind::I64))
                    }
                    _ => self.fail(ScalarError::TypeMismatch),
                }
            }
            ScalarExpr::Measure(value) => match self.emit(value, depth + 1)? {
                Kind::Interval(IntervalElement::U64) => Ok(self.op(Op::MeasureU64, 2, Kind::U64)),
                Kind::Interval(IntervalElement::I64) => Ok(self.op(Op::MeasureI64, 2, Kind::U64)),
                Kind::Interval(IntervalElement::F64) => Ok(self.op(Op::MeasureF64, 2, Kind::F64)),
                _ => self.fail(ScalarError::TypeMismatch),
            },
            ScalarExpr::Cast { kind, expr } => {
                let from = self.emit(expr, depth + 1)?;
                let (cast, to) = match (kind, from) {
                    (NumericCast::ToF64 | NumericCast::ToF64Exact, Kind::F64)
                    | (NumericCast::ToI64Exact, Kind::I64)
                    | (NumericCast::ToU64Exact, Kind::U64) => return Ok(from),
                    (NumericCast::ToF64, Kind::I64) => (Cast::I64ToF64, Kind::F64),
                    (NumericCast::ToF64, Kind::U64) => (Cast::U64ToF64, Kind::F64),
                    (NumericCast::ToF64Exact, Kind::I64) => (Cast::I64ToF64Exact, Kind::F64),
                    (NumericCast::ToF64Exact, Kind::U64) => (Cast::U64ToF64Exact, Kind::F64),
                    (NumericCast::ToI64Exact, Kind::F64) => (Cast::F64ToI64Exact, Kind::I64),
                    (NumericCast::ToU64Exact, Kind::F64) => (Cast::F64ToU64Exact, Kind::U64),
                    (NumericCast::ToI64Exact, Kind::U64) => (Cast::U64ToI64Exact, Kind::I64),
                    (NumericCast::ToU64Exact, Kind::I64) => (Cast::I64ToU64Exact, Kind::U64),
                    _ => return self.fail(ScalarError::TypeMismatch),
                };
                Ok(self.op(Op::Cast(cast), 1, to))
            }
            ScalarExpr::IsNaN(value) | ScalarExpr::IsFinite(value) => {
                if self.emit(value, depth + 1)? != Kind::F64 {
                    return self.fail(ScalarError::TypeMismatch);
                }
                let op = if matches!(expr, ScalarExpr::IsNaN(_)) {
                    Op::IsNaN
                } else {
                    Op::IsFinite
                };
                Ok(self.op(op, 1, Kind::Bool))
            }
        }
    }

    /// Emits `op`, which pops `popped` registers and pushes one `kind`.
    fn op(&mut self, op: Op, popped: usize, kind: Kind) -> Kind {
        self.push(op, popped, 1);
        kind
    }

    fn load(&mut self, column: usize, ty: ValueType) -> Result<Kind, Ended> {
        if let Some(element) = ty.interval_element() {
            let decode = match element {
                IntervalElement::U64 => Decode::Raw,
                IntervalElement::I64 => Decode::Biased,
                IntervalElement::F64 => Decode::OrderKey,
            };
            self.push(Op::Load { column, decode }, 0, 1);
            self.push(
                Op::Load {
                    column: column + 1,
                    decode,
                },
                0,
                1,
            );
            return Ok(Kind::Interval(element));
        }
        let (decode, kind) = match ty {
            ValueType::U64 => (Decode::Raw, Kind::U64),
            ValueType::I64 => (Decode::Biased, Kind::I64),
            ValueType::F64 => (Decode::OrderKey, Kind::F64),
            ValueType::Bool => (Decode::Bool, Kind::Bool),
            _ => return self.fail(ScalarError::NotNumeric),
        };
        self.push(Op::Load { column, decode }, 0, 1);
        Ok(kind)
    }

    fn literal(&mut self, value: &Value) -> Result<Kind, Ended> {
        let (kind, first, second) = match value {
            Value::U64(v) => (Kind::U64, *v, None),
            Value::I64(v) => (Kind::I64, v.cast_unsigned(), None),
            Value::F64(v) => (Kind::F64, v.to_bits(), None),
            Value::Bool(v) => (Kind::Bool, u64::from(*v), None),
            Value::IntervalU64(span) => (
                Kind::Interval(IntervalElement::U64),
                span.start(),
                Some(span.end()),
            ),
            Value::IntervalI64(span) => (
                Kind::Interval(IntervalElement::I64),
                span.start().cast_unsigned(),
                Some(span.end().cast_unsigned()),
            ),
            Value::IntervalF64(span) => (
                Kind::Interval(IntervalElement::F64),
                span.start().to_bits(),
                Some(span.end().to_bits()),
            ),
            _ => return self.fail(ScalarError::NotNumeric),
        };
        self.push(Op::Const(first), 0, 1);
        if let Some(second) = second {
            self.push(Op::Const(second), 0, 1);
        }
        Ok(kind)
    }
}

/// Order keys below 2^63 flip every bit, the others only the sign.
fn order_key_to_bits(key: u64) -> u64 {
    key ^ ((key >> 63).wrapping_sub(1) | SIGN)
}

/// Nonnegative bits flip only the sign, negative bits every bit.
fn bits_to_order_key(bits: u64) -> u64 {
    bits ^ ((bits >> 63).wrapping_neg() | SIGN)
}

/// One NaN and one zero.
fn canonical(bits: u64) -> u64 {
    F64::from_bits(bits).to_bits()
}

fn cast_word(cast: Cast, a: u64) -> Result<u64, ScalarError> {
    let float = F64::from_bits(a);
    let signed = a.cast_signed();
    let error = ScalarError::Cast;
    match cast {
        Cast::I64ToF64 => Ok(F64::from_i64(signed).to_bits()),
        Cast::U64ToF64 => Ok(F64::from_u64(a).to_bits()),
        Cast::I64ToF64Exact => F64::from_i64_exact(signed).map(F64::to_bits).map_err(error),
        Cast::U64ToF64Exact => F64::from_u64_exact(a).map(F64::to_bits).map_err(error),
        Cast::F64ToI64Exact => float.to_i64_exact().map(i64::cast_unsigned).map_err(error),
        Cast::F64ToU64Exact => float.to_u64_exact().map_err(error),
        Cast::U64ToI64Exact => i64::try_from(a)
            .map(i64::cast_unsigned)
            .map_err(|_| error(F64CastError::OutOfRange)),
        Cast::I64ToU64Exact => u64::try_from(signed).map_err(|_| error(F64CastError::OutOfRange)),
    }
}

/// Pops `b`, applies `f` lane-wise into `a`, and returns the new stack top.
fn binary_lanes(
    lanes: usize,
    stack: &mut [Register],
    sp: usize,
    errors: &mut Errors,
    f: impl Fn(u64, u64) -> Result<u64, ScalarError>,
) -> usize {
    let (low, high) = stack.split_at_mut(sp - 1);
    let (a, b) = (&mut low[sp - 2], &high[0]);
    for lane in 0..lanes {
        match f(a[lane], b[lane]) {
            Ok(value) => a[lane] = value,
            Err(error) => errors.fail(1 << lane, error),
        }
    }
    sp - 1
}

fn ternary_lanes(
    lanes: usize,
    stack: &mut [Register],
    sp: usize,
    errors: &mut Errors,
    f: impl Fn(u64, u64, u64) -> Result<u64, ScalarError>,
) -> usize {
    let (low, high) = stack.split_at_mut(sp - 2);
    let a = &mut low[sp - 3];
    for lane in 0..lanes {
        match f(a[lane], high[0][lane], high[1][lane]) {
            Ok(value) => a[lane] = value,
            Err(error) => errors.fail(1 << lane, error),
        }
    }
    sp - 2
}

fn unary_lanes(a: &mut [u64], errors: &mut Errors, f: impl Fn(u64) -> Result<u64, ScalarError>) {
    for (lane, word) in a.iter_mut().enumerate() {
        match f(*word) {
            Ok(value) => *word = value,
            Err(error) => errors.fail(1 << lane, error),
        }
    }
}

/// A wrapping lane op and the lanes where it overflowed.
type VectorOp<S> = fn(u64x4<S>, u64x4<S>) -> (u64x4<S>, u64);

#[inline(always)]
fn binary_vector<S: Simd>(
    simd: S,
    lanes: usize,
    stack: &mut [Register],
    sp: usize,
    errors: &mut Errors,
    f: VectorOp<S>,
) -> usize {
    let (low, high) = stack.split_at_mut(sp - 1);
    let (a, b) = (&mut low[sp - 2], &high[0]);
    let mut overflow = 0u64;
    for chunk in 0..lanes.div_ceil(4) {
        let lanes = chunk * 4..chunk * 4 + 4;
        let (value, bad) = f(
            u64x4::from_slice(simd, &a[lanes.clone()]),
            u64x4::from_slice(simd, &b[lanes.clone()]),
        );
        value.store_slice(&mut a[lanes]);
        overflow |= bad << (chunk * 4);
    }
    errors.fail(overflow, ScalarError::Overflow);
    sp - 1
}

#[inline(always)]
fn add_u64<S: Simd>(a: u64x4<S>, b: u64x4<S>) -> (u64x4<S>, u64) {
    let sum = a + b;
    (sum, sum.simd_lt(a).to_bitmask())
}

#[inline(always)]
fn sub_u64<S: Simd>(a: u64x4<S>, b: u64x4<S>) -> (u64x4<S>, u64) {
    (a - b, a.simd_lt(b).to_bitmask())
}

/// Signed overflow: both operands share a sign the sum lacks.
#[inline(always)]
fn add_i64<S: Simd>(a: u64x4<S>, b: u64x4<S>) -> (u64x4<S>, u64) {
    let sum = a + b;
    let sign = ((a ^ sum) & (b ^ sum)) >> 63u32;
    (sum, sign.simd_ne(0u64).to_bitmask())
}

/// Signed overflow: the operands differ in sign and the difference takes
/// the subtrahend's.
#[inline(always)]
fn sub_i64<S: Simd>(a: u64x4<S>, b: u64x4<S>) -> (u64x4<S>, u64) {
    let difference = a - b;
    let sign = ((a ^ b) & (a ^ difference)) >> 63u32;
    (difference, sign.simd_ne(0u64).to_bitmask())
}

#[inline(always)]
fn float_binary<S: Simd>(
    simd: S,
    lanes: usize,
    stack: &mut [Register],
    sp: usize,
    op: Op,
) -> usize {
    let (low, high) = stack.split_at_mut(sp - 1);
    let (a, b) = (&mut low[sp - 2], &high[0]);
    let nan = u64x4::splat(simd, CANONICAL_NAN);
    let zero = u64x4::splat(simd, 0);
    for chunk in 0..lanes.div_ceil(4) {
        let lanes = chunk * 4..chunk * 4 + 4;
        let x = f64x4::from_bytes(u64x4::from_slice(simd, &a[lanes.clone()]).to_bytes());
        let y = f64x4::from_bytes(u64x4::from_slice(simd, &b[lanes.clone()]).to_bytes());
        let result = match op {
            Op::AddF64 => x + y,
            Op::SubF64 => x - y,
            Op::MulF64 => x * y,
            _ => x / y,
        };
        let bits = u64x4::from_bytes(result.to_bytes());
        let canonical = result.simd_ne(result).select(nan, bits);
        let canonical = (canonical & !SIGN).simd_eq(0u64).select(zero, canonical);
        canonical.store_slice(&mut a[lanes]);
    }
    sp - 1
}
