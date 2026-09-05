//! Deterministic binary64 operations. Exact aggregation uses only integers;
//! scalar arithmetic executes one hardware operation under a thread-local
//! numerical environment guard. No fast-math/reassociation/FMA is involved.

mod accumulator;
pub(crate) mod environment;

pub(crate) use accumulator::ExactF64Accumulator;
use bumbledb_theory::F64;
pub(crate) use environment::NumericalGuard;

/// A float reduction exceeded the representable number of contributing
/// bindings. This is independent of finite/infinite/NaN numerical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatCardinalityOverflow;

impl core::fmt::Display for FloatCardinalityOverflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("F64 reduction cardinality exceeds u64::MAX")
    }
}

impl std::error::Error for FloatCardinalityOverflow {}

/// The target has no implemented numerical control-register bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedNumericalPlatform;

impl core::fmt::Display for UnsupportedNumericalPlatform {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("guarded F64 arithmetic requires aarch64 or x86_64")
    }
}

impl std::error::Error for UnsupportedNumericalPlatform {}

/// Canonical scalar numerical operations for the core. These operations do
/// not deduplicate inputs: query binding distinctness belongs to the query
/// engine, before reduction. Empty reductions return `None`, not a fake group.
pub struct F64Math;

impl F64Math {
    /// One nearest-even addition with gradual underflow and canonical output.
    /// # Errors
    /// [`UnsupportedNumericalPlatform`] outside the implemented CPU roster.
    pub fn add(left: F64, right: F64) -> Result<F64, UnsupportedNumericalPlatform> {
        Ok(NumericalGuard::enter()?.add(left, right))
    }

    /// One nearest-even subtraction with canonical output.
    /// # Errors
    /// [`UnsupportedNumericalPlatform`] outside the implemented CPU roster.
    pub fn subtract(left: F64, right: F64) -> Result<F64, UnsupportedNumericalPlatform> {
        Ok(NumericalGuard::enter()?.subtract(left, right))
    }

    /// One nearest-even multiplication, never contracted with adjacent nodes.
    /// # Errors
    /// [`UnsupportedNumericalPlatform`] outside the implemented CPU roster.
    pub fn multiply(left: F64, right: F64) -> Result<F64, UnsupportedNumericalPlatform> {
        Ok(NumericalGuard::enter()?.multiply(left, right))
    }

    /// One nearest-even division. Division by zero has canonical IEEE output.
    /// # Errors
    /// [`UnsupportedNumericalPlatform`] outside the implemented CPU roster.
    pub fn divide(left: F64, right: F64) -> Result<F64, UnsupportedNumericalPlatform> {
        Ok(NumericalGuard::enter()?.divide(left, right))
    }

    /// Exact accumulation followed by a single nearest-even rounding.
    /// # Errors
    /// [`FloatCardinalityOverflow`] for more than `u64::MAX` inputs.
    pub fn sum(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<Option<F64>, FloatCardinalityOverflow> {
        Ok(Self::accumulate(values)?.sum())
    }

    /// Round the exact total divided by its exact count, not a rounded sum.
    /// # Errors
    /// [`FloatCardinalityOverflow`] for more than `u64::MAX` inputs.
    pub fn mean(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<Option<F64>, FloatCardinalityOverflow> {
        Ok(Self::accumulate(values)?.mean())
    }

    /// Share one exact total/count within a single input stage.
    /// # Errors
    /// [`FloatCardinalityOverflow`] for more than `u64::MAX` inputs.
    pub fn sum_and_mean(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<Option<(F64, F64)>, FloatCardinalityOverflow> {
        let accumulator = Self::accumulate(values)?;
        Ok(accumulator.sum().zip(accumulator.mean()))
    }

    fn accumulate(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<ExactF64Accumulator, FloatCardinalityOverflow> {
        let mut accumulator = ExactF64Accumulator::default();
        for value in values {
            accumulator.push(value)?;
        }
        Ok(accumulator)
    }
}

#[cfg(test)]
mod tests;
