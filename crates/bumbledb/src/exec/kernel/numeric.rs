//! Deterministic binary64 operations. Exact aggregation is integer-only;
//! arithmetic is one plain `f64` operation per node, canonicalized, under a
//! float environment checked to be the IEEE default.

mod accumulator;
mod environment;

pub(crate) use accumulator::ExactF64Accumulator;
use bumbledb_theory::F64;
pub(crate) use environment::DefaultFloatEnvironment;
pub use environment::NonDefaultFloatEnvironment;

/// A float reduction exceeded `u64::MAX` contributing bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatCardinalityOverflow;

impl core::fmt::Display for FloatCardinalityOverflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("F64 reduction cardinality exceeds u64::MAX")
    }
}

impl std::error::Error for FloatCardinalityOverflow {}

/// Canonical binary64 operations for hosts. Each arithmetic call checks the
/// thread's float environment once. Reductions do not deduplicate their
/// inputs, and an empty reduction is `None`.
pub struct F64Math;

impl F64Math {
    /// One nearest-even addition with canonical output.
    ///
    /// # Errors
    /// [`NonDefaultFloatEnvironment`] when the thread's float environment is
    /// not the IEEE default.
    pub fn add(left: F64, right: F64) -> Result<F64, NonDefaultFloatEnvironment> {
        Ok(DefaultFloatEnvironment::check()?.add(left, right))
    }

    /// One nearest-even subtraction with canonical output.
    ///
    /// # Errors
    /// As [`F64Math::add`].
    pub fn subtract(left: F64, right: F64) -> Result<F64, NonDefaultFloatEnvironment> {
        Ok(DefaultFloatEnvironment::check()?.subtract(left, right))
    }

    /// One nearest-even multiplication, never fused with another node.
    ///
    /// # Errors
    /// As [`F64Math::add`].
    pub fn multiply(left: F64, right: F64) -> Result<F64, NonDefaultFloatEnvironment> {
        Ok(DefaultFloatEnvironment::check()?.multiply(left, right))
    }

    /// One nearest-even division; division by zero is IEEE infinity or NaN.
    ///
    /// # Errors
    /// As [`F64Math::add`].
    pub fn divide(left: F64, right: F64) -> Result<F64, NonDefaultFloatEnvironment> {
        Ok(DefaultFloatEnvironment::check()?.divide(left, right))
    }

    /// The exact sum, rounded once to nearest even.
    ///
    /// # Errors
    /// [`FloatCardinalityOverflow`] for more than `u64::MAX` inputs.
    pub fn sum(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<Option<F64>, FloatCardinalityOverflow> {
        Ok(Self::accumulate(values)?.sum())
    }

    /// The exact sum divided by the exact count, rounded once.
    ///
    /// # Errors
    /// [`FloatCardinalityOverflow`] for more than `u64::MAX` inputs.
    pub fn mean(
        values: impl IntoIterator<Item = F64>,
    ) -> Result<Option<F64>, FloatCardinalityOverflow> {
        Ok(Self::accumulate(values)?.mean())
    }

    /// [`F64Math::sum`] and [`F64Math::mean`] from one pass.
    ///
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
