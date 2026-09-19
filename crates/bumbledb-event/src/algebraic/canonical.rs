//! Numeric identity is the monic minimal polynomial over Q and the ordinal of
//! its real root. Authored factors, parameter names and isolating bounds are not
//! numeric identity. Bounded exhaustive factor search may refuse, never guess.
use super::dense::{Dense, Operation as RootOperation};
use super::{AlgebraicRoot, RootLimits};
use crate::{
    Capacity, Error, ExactArithmetic, ExactPolynomial, ExactRational as Rat, ParameterId,
    PolynomialLimits, PolynomialTerm, Result,
};

mod factor;

/// Bounds canonical real-algebraic identity construction. `candidates` counts
/// interpolated factor candidates; `steps` also bounds divisor trials and dense
/// coefficient visits. Root/polynomial limits and one shared arithmetic budget
/// remain in force. These are operation limits, not an aggregate memory quota.
#[derive(Debug, Clone, Copy)]
pub struct AlgebraicLimits {
    pub roots: RootLimits,
    pub polynomial: PolynomialLimits,
    pub candidates: usize,
    pub steps: usize,
    pub bytes: usize,
}

impl Default for AlgebraicLimits {
    fn default() -> Self {
        Self {
            roots: RootLimits::default(),
            polynomial: PolynomialLimits::default(),
            candidates: 100_000,
            steps: 1_000_000,
            bytes: 16 * 1024 * 1024,
        }
    }
}

// A numerical encoding has a formal indeterminate, never a captured source
// parameter. Region/source encodings separately retain their actual ParameterId.
const NUMERIC_PARAMETER: ParameterId = ParameterId([0; 32]);

struct Operation<'a, 'b> {
    root: RootOperation<'a, 'b>,
    limits: AlgebraicLimits,
    steps: usize,
    candidates: usize,
}

impl<'a, 'b> Operation<'a, 'b> {
    fn new(limits: AlgebraicLimits, work: &'a mut ExactArithmetic<'b>) -> Result<Self> {
        let mut result = Self {
            root: RootOperation::new(limits.roots, work)?,
            limits,
            steps: 0,
            candidates: 0,
        };
        result.step()?;
        Ok(result)
    }

    fn step(&mut self) -> Result<()> {
        self.root.work.control().checkpoint()?;
        if self.steps >= self.limits.steps {
            return Err(Error::Capacity(Capacity::AlgebraicIdentitySteps));
        }
        self.steps += 1;
        Ok(())
    }

    fn export(&mut self, dense: &Dense, parameter: ParameterId) -> Result<ExactPolynomial> {
        self.step()?;
        let mut terms = Vec::new();
        terms.try_reserve_exact(dense.0.len())?;
        for (power, coefficient) in dense.0.iter().enumerate() {
            self.step()?;
            if !coefficient.is_zero() {
                terms.push(PolynomialTerm {
                    coefficient: coefficient.clone(),
                    powers: if power == 0 {
                        Box::new([])
                    } else {
                        Box::new([(
                            parameter,
                            u32::try_from(power)
                                .map_err(|_| Error::Capacity(Capacity::AlgebraicDegree))?,
                        )])
                    },
                });
            }
        }
        ExactPolynomial::from_terms(&terms, self.limits.polynomial, self.root.work)
    }

    fn minimal(&mut self, value: &AlgebraicRoot) -> Result<Dense> {
        self.root.validate_root(value)?;
        value
            .polynomial()
            .validate(self.limits.polynomial, self.root.work)?;
        let mut polynomial = value.basis.square_free.clone();
        while let Some((factor, quotient)) = self.split(&polynomial)? {
            self.step()?;
            let description = self.export(&factor, value.parameter())?;
            polynomial = if value.sign(&description, self.limits.roots, self.root.work)?
                == std::cmp::Ordering::Equal
            {
                factor
            } else {
                quotient
            };
        }
        self.root.normalize(polynomial, false)
    }
}

impl AlgebraicRoot {
    /// Compute the unique monic irreducible polynomial over Q for this value,
    /// expressed in this certificate's formal parameter. Exhaustive bounded
    /// Kronecker factor search establishes irreducibility; a truncated search
    /// refuses rather than treating a remaining factor as irreducible.
    /// # Errors
    /// Input/intermediate limits, factor-search capacity or cancellation.
    pub fn minimal_polynomial(
        &self,
        limits: AlgebraicLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ExactPolynomial> {
        let mut op = Operation::new(limits, work)?;
        let minimal = op.minimal(self)?;
        op.export(&minimal, self.parameter())
    }

    /// Canonical BEAR v1 numeric identity: monic minimal polynomial in a fixed
    /// formal indeterminate and a zero-based ascending real-root ordinal.
    /// Equivalent reducible/repeated/scaled descriptions and different interval
    /// choices/names produce identical bytes. No source parameter is captured.
    /// # Errors
    /// Input/intermediate/search/byte limits, allocation or cancellation.
    pub fn to_bytes(
        &self,
        limits: AlgebraicLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u8>> {
        let mut op = Operation::new(limits, work)?;
        let minimal = op.minimal(self)?;
        let polynomial = op.export(&minimal, NUMERIC_PARAMETER)?;
        let roots = polynomial.isolate_roots(NUMERIC_PARAMETER, limits.roots, op.root.work)?;
        let mut ordinal = None;
        for (index, root) in roots.iter().enumerate() {
            op.step()?;
            if self.compare(root, limits.roots, op.root.work)? == std::cmp::Ordering::Equal {
                ordinal = Some(index as u64);
                break;
            }
        }
        let ordinal = ordinal.ok_or(Error::AlgebraicInvariant)?;
        let polynomial = polynomial.to_bytes(limits.polynomial, op.root.work)?;
        let length = polynomial
            .len()
            .checked_add(21)
            .filter(|&length| length <= limits.bytes)
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length)?;
        bytes.extend_from_slice(b"BEAR\x01");
        bytes.extend_from_slice(&(polynomial.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&polynomial);
        bytes.extend_from_slice(&ordinal.to_le_bytes());
        op.step()?;
        Ok(bytes)
    }

    /// Decode only canonical BEAR v1. Recomputes root existence and minimality;
    /// neither a serialized irreducibility claim nor an isolation hint is trusted.
    /// The result's formal parameter is numerical and carries no source identity.
    /// # Errors
    /// Malformed/noncanonical bytes, unknown version, capacities or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: AlgebraicLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        work.control().checkpoint()?;
        if bytes.len() > limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        if bytes.get(..4) != Some(b"BEAR") {
            return Err(Error::InvalidEncoding);
        }
        match bytes.get(4) {
            Some(1) => (),
            Some(&version) => return Err(Error::UnsupportedVersion(version)),
            None => return Err(Error::InvalidEncoding),
        }
        let length = read_length(bytes.get(5..13).ok_or(Error::InvalidEncoding)?)?;
        let end = 13usize.checked_add(length).ok_or(Error::InvalidEncoding)?;
        if end.checked_add(8) != Some(bytes.len()) {
            return Err(Error::InvalidEncoding);
        }
        let ordinal = read_length(&bytes[end..])?;
        let polynomial = ExactPolynomial::from_bytes(&bytes[13..end], limits.polynomial, work)?;
        let roots = polynomial.isolate_roots(NUMERIC_PARAMETER, limits.roots, work)?;
        let root = roots.get(ordinal).ok_or(Error::InvalidEncoding)?.clone();
        if root.to_bytes(limits, work)? != bytes {
            return Err(Error::InvalidEncoding);
        }
        Ok(root)
    }
}

fn read_length(bytes: &[u8]) -> Result<usize> {
    usize::try_from(u64::from_le_bytes(
        bytes.try_into().map_err(|_| Error::InvalidEncoding)?,
    ))
    .map_err(|_| Error::InvalidEncoding)
}
